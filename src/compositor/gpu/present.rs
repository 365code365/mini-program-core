//! 建合成器（门槛 → 适配器 → 设备 → 自检）与 surface 上屏（设计 §5.4、§6.2~§6.5；v2 修订 R4、R7）。
use super::device::{self, Gpu};
use super::{selftest, GpuCompositor, GpuError, PresentOutcome};
use crate::compositor::select::{self, GpuNeeds, SelectInput};
use crate::compositor::verdict::{self, CacheMode, VerdictCache, VerdictEvent};
use crate::compositor::{diag, BackendPref, BackendStatus, FaultSpec, FrameLayers};
use std::time::Instant;

/// 要上屏的原生目标
pub enum SurfaceSpec {
    /// 桌面窗口（`Arc<winit::window::Window>` 之类实现了窗口句柄的类型 `.into()` 过来）
    Target(wgpu::SurfaceTarget<'static>),
    /// CAMetalLayer* / ANativeWindow*（调用方保证活到 detach）
    Unsafe(wgpu::SurfaceTargetUnsafe),
}

/// 建合成器要的全部输入（宿主入口一次性给齐，不在这里读环境变量 —— v2 修订 R14）
pub struct OpenOptions<'a> {
    pub select: SelectInput<'a>,
    pub fault: Option<FaultSpec>,
    pub log: bool,
    /// 输出像素尺寸
    pub width: u32,
    pub height: u32,
    /// 页面画布宽（= 输出宽）与视口裁剪余量（渲染器的 VIEWPORT_CULL_MARGIN_PX）
    pub canvas_w: u32,
    pub margin_px: u32,
    /// 用判定缓存（只有 Auto 读；强制 gpu 无视缓存但仍要过自检）
    pub use_verdict: bool,
}

/// 选一次：成功给出合成器与「GPU」结论；失败给出「CPU + 原因」结论（原因已经打过日志）
pub fn open(opts: OpenOptions<'_>, spec: Option<SurfaceSpec>) -> Result<(GpuCompositor, BackendStatus), BackendStatus> {
    let s = opts.select;
    select::pre_gpu(&s)?;
    let fail = |why: String| {
        diag::emit(&diag::fallback(&why));
        BackendStatus::cpu(s.pref, s.source, why)
    };
    let t0 = Instant::now();
    let instance = device::new_instance(s.pref).map_err(fail)?;
    let surface = match spec {
        None => None,
        Some(SurfaceSpec::Target(t)) => Some(instance.create_surface(t).map_err(|e| fail(format!("创建 surface 失败：{e}")))?),
        // SAFETY: 调用方（FFI 宿主）保证原生指针有效且活到 detach / destroy
        Some(SurfaceSpec::Unsafe(t)) => Some(unsafe { instance.create_surface_unsafe(t) }.map_err(|e| fail(format!("创建 surface 失败：{e}")))?),
    };
    let adapter = device::request_adapter(&instance, surface.as_ref()).map_err(fail)?;
    let facts = device::facts_of(&adapter);
    let key = device::verdict_key(&facts);
    let mode = if opts.use_verdict && s.pref == BackendPref::Auto { CacheMode::for_run(opts.fault.is_some()) } else { CacheMode::Off };
    let (mut cache, warn) = VerdictCache::load(&verdict::default_path(), mode);
    if let Some(w) = warn {
        diag::emit(&w);
    }
    let viewport = opts.height;
    let needs = GpuNeeds::for_output(opts.width, opts.height, opts.canvas_w, viewport, opts.margin_px);
    let cached = cache.rejection(&key);
    select::adapter_gate(s.pref, &facts, needs, select::DENY, cached.as_deref()).map_err(fail)?;
    let gpu: Gpu = device::open(instance, adapter, needs).map_err(fail)?;
    if opts.fault == Some(FaultSpec::Init) {
        return Err(fail("注入故障 init（request_device 之后当作失败）".into()));
    }
    let init_ms = t0.elapsed().as_secs_f32() * 1000.0;
    let mut comp = GpuCompositor::from_gpu(gpu, needs.array_layers, opts.fault, opts.log);
    if let Some(surface) = surface {
        comp.attach_surface(surface, opts.width, opts.height).map_err(fail)?;
    }
    let report = selftest::run(&mut comp, opts.width, opts.height, opts.fault == Some(FaultSpec::Selftest));
    if opts.log {
        diag::emit(&format!("🧪 GPU 初始化 {init_ms:.1}ms；自检 {}", report.describe()));
    }
    if let Some(why) = report.mismatch.clone() {
        cache.record(&key, VerdictEvent::SelftestMismatch);
        let _ = cache.save();
        return Err(fail(why));
    }
    if s.pref == BackendPref::Auto {
        if let Some(why) = report.auto_rejection() {
            return Err(fail(why));
        }
    }
    // 自检用的那几帧不算进故障注入的帧号：N 从第一帧真实内容起计
    comp.gpu_frame = 0;
    comp.reset_mirrors();
    let status = BackendStatus::gpu(s.pref, s.source, comp.facts().describe());
    Ok((comp.with_log(opts.log), status))
}

impl GpuCompositor {
    fn with_log(mut self, log: bool) -> Self {
        self.log = log;
        self
    }

    /// 自检之后清掉镜像：自检帧的内容不能被当成真实页面的「已上传」
    fn reset_mirrors(&mut self) {
        self.page = None;
        self.tabbar = Default::default();
        self.fixed = Default::default();
        self.pull = None;
        self.pass = None;
    }

    /// 选 surface 格式并 configure。只用非 sRGB 的写法（字节原样写入，不做伽马）
    fn attach_surface(&mut self, surface: wgpu::Surface<'static>, w: u32, h: u32) -> Result<(), String> {
        use wgpu::TextureFormat as F;
        let caps = surface.get_capabilities(&self.gpu.adapter);
        let pick = [F::Bgra8Unorm, F::Rgba8Unorm].into_iter().find(|f| caps.formats.contains(f));
        let (format, view_format) = match pick {
            Some(f) => (f, f),
            None => match [(F::Bgra8UnormSrgb, F::Bgra8Unorm), (F::Rgba8UnormSrgb, F::Rgba8Unorm)].into_iter().find(|(s, _)| caps.formats.contains(s)) {
                Some(p) => p,
                None => return Err(format!("surface 无非 sRGB 格式（可用：{:?}）", caps.formats)),
            },
        };
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            caps.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Auto)
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w.max(1),
            height: h.max(1),
            // Fifo 规范保证都支持；它在 acquire 时按 vsync 阻塞，宿主的节拍器在 GPU 模式下关掉
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: if view_format != format { vec![view_format] } else { vec![] },
            // 显式 sRGB：非 *Srgb 格式的字节按「已经是 sRGB 编码」解释，与 softbuffer 上屏的
            // CPU 路径同一个色彩空间。不写死的话 macOS 的 CAMetalLayer 可能按显示器色域
            // （Display P3）解释同样的字节，离屏读回逐字节一致、屏上却整体偏色（v2 修订 R8）
            color_space: wgpu::SurfaceColorSpace::Srgb,
        };
        surface.configure(&self.gpu.device, &config);
        if self.log {
            diag::emit(&format!("🧩 surface 格式 {format:?}（视图 {view_format:?}），{w}×{h}，Fifo"));
        }
        self.surface = Some(super::SurfaceState { surface, config, view_format });
        Ok(())
    }

    /// 放掉 surface（它引用着宿主的原生窗口），设备、管线、层纹理都留着：宿主回桌面时
    /// Surface 被系统收走，回来再 [`reattach_surface`](Self::reattach_surface) 就不用重开设备、重跑自检
    pub fn detach_surface(&mut self) {
        self.surface = None;
        // 层纹理也放掉（页面镜像一张就是 宽 × 环长 × 4，一加上 20MB 往上）：进了后台没人采样，
        // 挂回来时 `reattach_surface` 本来就整张重传。留着设备与管线，回来不用重开
        self.release_caches();
        self.reset_mirrors();
        self.top = Default::default();
        self.sheet_tex = None;
        self.sheet_tex_id = None;
    }

    /// 用原来的设备挂一块新 surface。层纹理清掉、下一帧整张重传（宿主不在时 CPU 画布可能变了，
    /// 镜像按增量同步的账已经对不上）；页面不用重画 —— CPU 画布一直在。
    ///
    /// 为什么要：一加 PLY110 上回桌面再回来，从 surfaceCreated 到第一帧上屏 **520ms**
    /// （重开 Vulkan 设备 + 管线 + 自检 ≈ 490ms，再整页重画 21ms），这段时间那块 SurfaceView 是空的
    pub fn reattach_surface(&mut self, spec: SurfaceSpec, w: u32, h: u32) -> Result<(), String> {
        let surface = match spec {
            SurfaceSpec::Target(t) => self.gpu.instance.create_surface(t).map_err(|e| format!("创建 surface 失败：{e}"))?,
            // SAFETY: 调用方（FFI 宿主）保证原生指针有效且活到 detach / destroy
            SurfaceSpec::Unsafe(t) => unsafe { self.gpu.instance.create_surface_unsafe(t) }.map_err(|e| format!("创建 surface 失败：{e}"))?,
        };
        if !self.gpu.adapter.is_surface_supported(&surface) {
            return Err("原来的适配器不支持这块新 surface".into());
        }
        self.attach_surface(surface, w, h)?;
        self.reset_mirrors();
        Ok(())
    }

    pub fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    /// 合成并上屏这一帧
    pub fn present(&mut self, l: &FrameLayers<'_>) -> Result<PresentOutcome, GpuError> {
        let next = self.gpu_frame + 1;
        if let Some(s) = self.surface.as_mut() {
            if s.config.width != l.width.max(1) || s.config.height != l.height.max(1) {
                s.config.width = l.width.max(1);
                s.config.height = l.height.max(1);
                s.surface.configure(&self.gpu.device, &s.config);
            }
        }
        // 先把这一帧的纹理传好（直通帧还要先 CPU 合成整帧），**再** acquire。
        //
        // 顺序反过来（先 acquire 再上传）时：Fifo 的 acquire 阻塞到下一个 vsync 才放出 drawable，
        // 这时离再下一个 vsync 只剩一个周期（120Hz 8.3ms），直通帧的 CPU 合成 + 4MB 上传
        // （1~10ms）全压在这段里，赶不上就掉帧；实测「弹 Toast + setData」场景 360 帧掉 18 帧，
        // acquire 最长等了 58ms。先准备再等，等 vsync 的那段时间就不是白等。
        //
        // prepare 里也会先 poll 再查健康旗标：设备丢失后 acquire 只会报 Validation，
        // 而丢失回调要在 poll 里才触发 —— 顺序反了，回退原因会被误报成「校验错误」
        // （优先级：设备丢失 > 校验错误 > surface 丢失，v2 修订 R6）
        let prep = self.prepare(l)?;
        let Some(s) = self.surface.as_mut() else { return Err(GpuError::SurfaceLost("没有挂载 surface".into())) };
        let t_acq = Instant::now();
        let injected = match self.fault {
            Some(FaultSpec::Timeout(n)) if n == next => Some(wgpu::CurrentSurfaceTexture::Timeout),
            Some(FaultSpec::SurfaceLost(n)) if n == next => Some(wgpu::CurrentSurfaceTexture::Lost),
            _ => None,
        };
        let surface_lost_injected = matches!(injected, Some(wgpu::CurrentSurfaceTexture::Lost));
        let mut got = injected.unwrap_or_else(|| s.surface.get_current_texture());
        let mut retried = false;
        let frame = loop {
            match got {
                wgpu::CurrentSurfaceTexture::Success(t) => break t,
                wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                    // 本帧照用，下一帧前重新 configure
                    s.surface.configure(&self.gpu.device, &s.config);
                    break t;
                }
                wgpu::CurrentSurfaceTexture::Timeout => {
                    // 注入的 timeout 也要推进帧号，否则第 N 帧会永远是 timeout
                    self.gpu_frame += 1;
                    return Ok(PresentOutcome::Skipped("等 surface 超时"));
                }
                wgpu::CurrentSurfaceTexture::Occluded => return Ok(PresentOutcome::Skipped("窗口被遮挡")),
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost if !retried && !surface_lost_injected => {
                    retried = true;
                    s.surface.configure(&self.gpu.device, &s.config);
                    got = s.surface.get_current_texture();
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    self.gpu.poll_now();
                    return Err(self.gpu.health.fatal().unwrap_or_else(|| GpuError::Validation("acquire 返回 Validation".into())));
                }
                _ => {
                    self.gpu_frame += 1;
                    return Err(GpuError::SurfaceLost(if surface_lost_injected { "注入故障 surface-lost".into() } else { "重新 configure 后仍取不到".into() }));
                }
            }
        };
        let acquire_ms = t_acq.elapsed().as_secs_f32() * 1000.0;
        let view_format = s.view_format;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor { format: Some(view_format), ..Default::default() });
        let mut stats = self.draw_submit(prep, &view, view_format)?;
        let t_present = Instant::now();
        self.gpu.queue.present(frame);
        stats.present_ms = t_present.elapsed().as_secs_f32() * 1000.0;
        stats.acquire_ms = acquire_ms;
        Ok(PresentOutcome::Presented(stats))
    }
}
