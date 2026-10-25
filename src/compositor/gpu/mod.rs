//! GPU 合成器（feature `gpu`，wgpu）：设计 §4~§6；v2 修订 R1~R16。
//!
//! 内容仍只由 CPU 光栅化。这里把 [`FrameLayers`] 里的图层当纹理**镜像**（页面按脏区增量上传、
//! tabBar / fixed 按代数整层或按行上传），用整数着色器复刻 [`cpu::compose`](super::cpu::compose)
//! 的算术，再画到 surface（上屏）或离屏目标（无头 / 自检 / 测试读回）。
//! 同一份 `FrameLayers`，输出与 CPU **逐字节相同** —— 这是判据，不是愿望：启动自检、
//! `MINI_GPU_VERIFY=1` 的逐帧对拍、`tools/gpu-parity.sh` 三道守着。
//!
//! Toast / Loading / Modal、操作菜单、picker 面板是直接画进输出缓冲、混合时读目标像素的
//! （还带浮点），这些帧走**直通**：CPU 合成整帧，但只上传覆盖层画过的那几行，其余行着色器
//! 自己合成、按遮罩表压暗（`passthrough.rs`，§5.3 的收窄）。说不清画了哪里的（图片预览、
//! 全屏视频、侧滑叠覆盖层）才整帧上传、原样输出。
//! 判断故意从宽（状态 `Some` 就算，不看 visible）：误判成直通永远是对的。
//!
//! 合成器只拿到图层的只读引用，拿不到布局 / 命中 / 事件 / 动画时钟 ——
//! 「后端只管画」是类型层面保证的。
pub mod device;
mod mirror;
mod passthrough;
mod uniforms;
pub mod pipeline;
mod readback;
pub mod selftest;
#[cfg(test)]
mod tests;

#[cfg(not(target_endian = "little"))]
compile_error!("GPU 合成器按小端字节序上传 u32 像素（直通帧），大端目标请关掉 feature gpu");

/// 宿主（桌面二进制）要用 wgpu 的类型构造窗口 surface
pub use wgpu;

use super::{FaultSpec, FrameLayers, LayerCanvas};
use device::Gpu;
use mirror::PageMirror;
use passthrough::PassBand;
use pipeline::{Pipeline, Tex};
use std::time::Instant;

/// 判定缓存键里的「引擎版本」：包版本 + 着色器源码哈希 + 自检逻辑版本（v2 修订 R15）
pub fn engine_key() -> String {
    format!("{}+{:016x}+{}", env!("CARGO_PKG_VERSION"), fnv1a(pipeline::WGSL.as_bytes()), selftest::SELFTEST_REV)
}

const fn fnv1a(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < b.len() {
        h ^= b[i] as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    h
}

/// 对 GPU 致命、宿主要当帧回退 CPU 的错误
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuError {
    DeviceLost(String),
    Validation(String),
    SurfaceLost(String),
    Readback(String),
}

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuError::DeviceLost(m) => write!(f, "设备丢失（{m}）"),
            GpuError::Validation(m) => write!(f, "GPU 校验错误（{m}）"),
            GpuError::SurfaceLost(m) => write!(f, "surface 无法恢复（{m}）"),
            GpuError::Readback(m) => write!(f, "读回失败（{m}）"),
        }
    }
}

/// 一帧的 GPU 账
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuFrameStats {
    /// 编码 + 提交占主线程的时间（不含等 GPU、不含等 vsync）
    pub encode_ms: f32,
    /// 提交 → GPU 完成（只在调用方要求等待时有）
    pub gpu_done_ms: Option<f32>,
    /// 等 surface（acquire，含 vsync 阻塞）
    pub acquire_ms: f32,
    pub upload_bytes: u64,
    pub passthrough: bool,
    /// 从选定 GPU 起的帧号（与 `MINI_GPU_FAULT` 的 N 同一个计数器）
    pub gpu_frame: u32,
    /// 其中准备阶段（同步纹理、写 uniform）的时间，与准备阶段里直通帧 CPU 合成的时间
    pub prepare_ms: f32,
    pub pass_compose_ms: f32,
    /// `queue.present` 本身（交换链满了时驱动可能在这里等）
    pub present_ms: f32,
}

/// 上屏结果
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PresentOutcome {
    Presented(GpuFrameStats),
    /// surface 暂时取不到（Timeout / Occluded）：屏上保留上一帧，宿主要安排补一帧（v2 修订 R4）
    Skipped(&'static str),
}

/// [`GpuCompositor::prepare`] 的结果：纹理已同步，等着画进目标
struct Prepared {
    upload_bytes: u64,
    pass_why: Option<&'static str>,
    ms: f32,
}

/// 有 surface 时的上屏状态
struct SurfaceState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// 往这个格式的视图里画（非 sRGB，字节原样写入）
    view_format: wgpu::TextureFormat,
}

/// 层纹理 + 上次上传时看到的代数 / 行区间
#[derive(Default)]
struct LayerSlot {
    tex: Option<Tex>,
    gen: u64,
    rows: Option<(u32, u32)>,
}

pub struct GpuCompositor {
    gpu: Gpu,
    pipe: Pipeline,
    page: Option<PageMirror>,
    tabbar: LayerSlot,
    fixed: LayerSlot,
    /// 引擎画的导航栏画布
    top: LayerSlot,
    /// 转场 / 侧滑的另一页留图，及它是哪一张（`PageShot::id`）
    shot_tex: Option<Tex>,
    shot_id: Option<u64>,
    pull: Option<Tex>,
    pass: Option<Tex>,
    pass_scratch: Vec<u32>,
    /// 最近一个直通帧里取 CPU 结果的行区间与遮罩表、CPU 合成花的时间（passthrough.rs）
    pass_band: PassBand,
    pass_compose_ms: f32,
    /// 操作菜单面板缓存纹理、它是哪一张（`SheetRaster::id`），以及这一帧的贴法（顶边行, 起, 止）
    sheet_tex: Option<Tex>,
    sheet_tex_id: Option<u64>,
    sheet_frame: Option<(i32, u32, u32)>,
    /// 直通帧一律整帧合成、整帧上传（`MINI_GPU_VERIFY` 拿 [`last_passthrough`](Self::last_passthrough)
    /// 当参照时要整帧真值；平时只合成覆盖层那一截，见 passthrough.rs）
    pub full_reference: bool,
    offscreen: Option<(wgpu::Texture, u32, u32)>,
    surface: Option<SurfaceState>,
    fault: Option<FaultSpec>,
    /// 视口高 + 裁剪余量：决定环长（层数）
    ring_layers: u32,
    gpu_frame: u32,
    /// 自检进行中：故障注入（tint / lost@N / timeout@N）一律不生效 ——
    /// 否则 tint 会先让自检判掉 GPU，gpu-parity 就测不到比较链路本身
    selftesting: bool,
    /// 最近一帧是不是直通帧（见 [`last_passthrough`](Self::last_passthrough)）
    last_pass: bool,
    pub log: bool,
}

impl GpuCompositor {
    fn from_gpu(gpu: Gpu, ring_layers: u32, fault: Option<FaultSpec>, log: bool) -> Self {
        let pipe = Pipeline::new(&gpu.device);
        Self {
            gpu,
            pipe,
            page: None,
            tabbar: LayerSlot::default(),
            fixed: LayerSlot::default(),
            top: LayerSlot::default(),
            shot_tex: None,
            shot_id: None,
            pull: None,
            pass: None,
            pass_scratch: Vec::new(),
            pass_band: PassBand::default(),
            pass_compose_ms: 0.0,
            sheet_tex: None,
            sheet_tex_id: None,
            sheet_frame: None,
            full_reference: false,
            offscreen: None,
            surface: None,
            fault,
            ring_layers,
            gpu_frame: 0,
            selftesting: false,
            last_pass: false,
            log,
        }
    }

    pub fn facts(&self) -> &super::AdapterFacts {
        &self.gpu.facts
    }

    pub fn gpu_frame(&self) -> u32 {
        self.gpu_frame
    }

    /// 同步本帧要用的图层纹理；返回（上传字节数, 是否直通, 直通原因）
    fn sync_layers(&mut self, l: &FrameLayers<'_>) -> (u64, Option<&'static str>) {
        let o = &l.overlays;
        // 导航栏（页面整体下移 + 叠导航栏画布）与转场 / 侧滑（整帧平移 + 压暗 + 阴影）在着色器里做：
        // 从前这两样都走直通 —— 引擎画导航栏的宿主（Flutter）**每一帧**都是 CPU 合成 + 整帧上传，
        // GPU 比 CPU 还慢（一加 PLY110 滑动只合成帧平均 7.2ms，CPU 2.8ms）
        let pass_why = if o.modal.is_some() {
            Some("Modal")
        } else if o.toast.is_some() {
            Some("Toast")
        } else if o.loading.is_some() {
            Some("Loading")
        } else if o.picker.is_some() {
            Some("picker")
        } else if crate::renderer::components::fullscreen_video().is_some() {
            // 全屏视频画在 CPU 合成那一步（与 Modal 同理，整帧直通）
            Some("全屏视频")
        } else if crate::host::ui_overlay::sheet_overlay_visible() {
            Some("操作菜单 / 图片预览")
        } else {
            None
        };
        let mut sent = 0u64;
        if pass_why.is_some() {
            // 直通帧：CPU 合成整帧，但只传覆盖层画过的那几行（passthrough.rs）；
            // 说不清画了哪里的才整帧上传，那时其余图层也用不上，不必同步
            let (bytes, whole) = self.sync_pass(l);
            if whole {
                return (bytes, pass_why);
            }
            sent += bytes;
        }
        let (dev, queue) = (&self.gpu.device, &self.gpu.queue);
        // 页面：宽度或代数变了就重建镜像（尺寸可能变了，旧内容一律作废）
        let canvas = l.page.read();
        let rebuild = self.page.as_ref().is_none_or(|m| m.width != canvas.width() || m.cursor.0 != l.page.gen());
        if rebuild {
            let keep = self.page.as_ref().is_some_and(|m| m.width == canvas.width() && m.layers == self.ring_layers);
            if !keep {
                self.page = Some(PageMirror::new(dev, canvas.width(), self.ring_layers));
            }
            if let Some(m) = &mut self.page {
                m.valid = None;
                m.cursor = (l.page.gen(), u64::MAX);
            }
        }
        let content_h = l.height - if l.has_tabbar { l.tabbar_px.min(l.height) } else { 0 };
        // 键盘顶起画面时底下多露出 content_shift 行（在键盘后面，但要与 CPU 合成逐字节相同，照样传上去）
        let visible = (l.scroll_px, l.scroll_px.saturating_add(content_h as i32 + l.content_shift as i32));
        if let Some(m) = &mut self.page {
            sent += m.sync(queue, l.page, visible);
        }
        sent += sync_layer(&mut self.tabbar, dev, queue, "tabBar", l.tabbar, None);
        sent += sync_layer(&mut self.fixed, dev, queue, "fixed", l.fixed, l.fixed_rows);
        sent += sync_layer(&mut self.top, dev, queue, "导航栏", l.top.as_ref().map(|t| t.canvas), None);
        // 转场 / 侧滑的另一页：一张图一次转场只传一次（按 PageShot::id 认）
        if let Some(s) = l.edge_back.as_ref().and_then(|e| e.shot).filter(|s| s.width == l.width && s.height == l.height) {
            if self.shot_id != Some(s.id) && s.pixels.len() == (s.width * s.height) as usize {
                Tex::ensure(&mut self.shot_tex, dev, "转场留图", s.width, s.height);
                // SAFETY: u32 切片按字节看（小端：B,G,R,0），与直通帧同一种布局
                let bytes = unsafe { std::slice::from_raw_parts(s.pixels.as_ptr().cast::<u8>(), s.pixels.len() * 4) };
                sent += self.shot_tex.as_ref().map(|t| t.upload_rows(queue, bytes, s.width, 0, s.height, s.width)).unwrap_or(0);
                self.shot_id = Some(s.id);
            }
        }
        if let Some(p) = l.pull {
            if let Some(band) = crate::host::render::pull_indicator_band(
                l.width, l.height, l.scale, p.gap_logical, p.refreshing, p.progress, p.phase,
            ) {
                Tex::ensure(&mut self.pull, dev, "下拉指示器", band.width(), band.height());
                let bytes = super::color_bytes(band.pixels());
                sent += self.pull.as_ref().map(|t| t.upload_rows(queue, bytes, band.width(), 0, band.height(), band.width())).unwrap_or(0);
            }
        }
        (sent, pass_why)
    }

    /// 把这一帧编码进 `target`（格式 `format`）并提交。故障注入、致命错误检查都在这里
    fn render_into(&mut self, l: &FrameLayers<'_>, target: &wgpu::TextureView, format: wgpu::TextureFormat) -> Result<GpuFrameStats, GpuError> {
        let prep = self.prepare(l)?;
        self.draw_submit(prep, target, format)
    }

    /// 第一步：同步图层纹理（含直通帧的 CPU 合成与整帧上传）、写 uniform。**不需要 drawable**。
    ///
    /// 与 [`draw_submit`](Self::draw_submit) 分开，是为了让上屏路径先做完这些、再去 acquire：
    /// Fifo 的 acquire 会阻塞到 vsync，等回来再做的话，这些活（直通帧要 1~10ms）全部压在
    /// 「拿到 drawable → 下一个 vsync」那几毫秒里，赶不上就掉一帧（见 present.rs）
    fn prepare(&mut self, l: &FrameLayers<'_>) -> Result<Prepared, GpuError> {
        self.gpu.poll_now();
        if let Some(e) = self.gpu.health.fatal() {
            return Err(e);
        }
        let t0 = Instant::now();
        let (upload_bytes, pass_why) = self.sync_layers(l);
        self.last_pass = pass_why.is_some();
        let u = self.uniforms(l, pass_why.is_some());
        self.gpu.queue.write_buffer(&self.pipe.uniforms, 0, u.bytes());
        Ok(Prepared { upload_bytes, pass_why, ms: t0.elapsed().as_secs_f32() * 1000.0 })
    }

    /// 第二步：画进 `target`（drawable 或离屏目标）并提交。故障注入、致命错误检查也在这里
    fn draw_submit(&mut self, prep: Prepared, target: &wgpu::TextureView, format: wgpu::TextureFormat) -> Result<GpuFrameStats, GpuError> {
        let Prepared { upload_bytes, pass_why, ms: prep_ms } = prep;
        let t0 = Instant::now();
        let ph = &self.pipe.ph2d.view;
        fn view_of<'a>(t: &'a Option<Tex>, ph: &'a wgpu::TextureView) -> &'a wgpu::TextureView {
            t.as_ref().map(|t| &t.view).unwrap_or(ph)
        }
        let page_view = self.page.as_ref().map(|m| &m.view).unwrap_or(&self.pipe.ph_array);
        let entries = [
            wgpu::BindGroupEntry { binding: 0, resource: self.pipe.uniforms.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(page_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(view_of(&self.tabbar.tex, ph)) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(view_of(&self.fixed.tex, ph)) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(view_of(&self.pull, ph)) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(view_of(&self.pass, ph)) },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(view_of(&self.top.tex, ph)) },
            wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(view_of(&self.shot_tex, ph)) },
            wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(view_of(&self.sheet_tex, ph)) },
        ];
        let bind = self.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("合成"), layout: &self.pipe.layout, entries: &entries });
        let pipeline = self.pipe.for_format(&self.gpu.device, format).clone();
        let mut enc = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("合成") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("合成"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        }
        self.gpu.queue.submit([enc.finish()]);
        let encode_ms = prep_ms + t0.elapsed().as_secs_f32() * 1000.0;
        self.gpu_frame += 1;
        if self.log {
            let line = match pass_why {
                Some(why) => super::diag::mode_passthrough(why, upload_bytes),
                None => super::diag::mode_layered(upload_bytes),
            };
            super::diag::emit(&line);
        }
        // 故障注入：第 N 帧之后设备丢失 —— 走 wgpu 真实的丢失回调，不是假装
        if let (Some(FaultSpec::Lost(n)), false) = (self.fault, self.selftesting) {
            if self.gpu_frame == n {
                self.gpu.device.destroy();
                self.gpu.health.mark_lost("Destroyed（注入故障 lost）");
                self.gpu.poll_now();
            }
        }
        self.gpu.poll_now();
        if let Some(e) = self.gpu.health.fatal() {
            return Err(e);
        }
        Ok(GpuFrameStats {
            encode_ms,
            upload_bytes,
            passthrough: pass_why.is_some(),
            gpu_frame: self.gpu_frame,
            prepare_ms: prep_ms,
            pass_compose_ms: if pass_why.is_some() { self.pass_compose_ms } else { 0.0 },
            ..Default::default()
        })
    }

    /// 合成到离屏目标（无头 / 自检 / 测试）。`wait` 为真时等 GPU 做完并记完成时间
    pub fn compose_offscreen(&mut self, l: &FrameLayers<'_>, wait: bool) -> Result<GpuFrameStats, GpuError> {
        let fresh = self.offscreen.as_ref().is_none_or(|(_, w, h)| *w != l.width || *h != l.height);
        if fresh {
            let tex = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("离屏目标"),
                size: wgpu::Extent3d { width: l.width.max(1), height: l.height.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            self.offscreen = Some((tex, l.width, l.height));
        }
        let view = self.offscreen.as_ref().map(|(t, _, _)| t.create_view(&wgpu::TextureViewDescriptor::default())).ok_or_else(|| GpuError::Readback("离屏目标缺失".into()))?;
        let submitted = Instant::now();
        let mut stats = self.render_into(l, &view, wgpu::TextureFormat::Rgba8Unorm)?;
        if wait {
            self.gpu.wait_idle();
            stats.gpu_done_ms = Some(submitted.elapsed().as_secs_f32() * 1000.0 - stats.encode_ms);
        }
        Ok(stats)
    }

    /// 读回最近一次离屏合成的结果（RGBA，每像素 4 字节，A 恒为 255）
    pub fn read_back(&mut self) -> Result<Vec<u8>, GpuError> {
        let (tex, w, h) = self.offscreen.as_ref().ok_or_else(|| GpuError::Readback("还没有离屏合成过".into()))?;
        readback::read_rgba(&self.gpu, tex, *w, *h)
    }

    /// 最近一帧若是直通帧：它上传的那份 CPU 合成结果（逐帧校验的参照物）。
    ///
    /// 直通帧里的覆盖层（picker 面板入场、Loading 转圈）按墙上时钟动画，事后再 CPU 合成一次
    /// 相位已经变了，比出来的差异是时间不是 GPU。GPU 在直通帧里做的是「上传 + 原样输出」，
    /// 要校验的正是这一段，所以拿它上传的那份字节当参照
    pub fn last_passthrough(&self) -> Option<&[u32]> {
        self.last_pass.then_some(self.pass_scratch.as_slice())
    }

    /// 内存告警：释放直通纹理、离屏目标（下次用到再建）
    pub fn release_caches(&mut self) {
        self.pass = None;
        self.pass_scratch = Vec::new();
        self.offscreen = None;
        self.shot_tex = None;
        self.shot_id = None;
    }
}

/// tabBar / fixed：代数变了才传；fixed 只传 `rows` 那几行（其余行着色器不采样）
fn sync_layer(slot: &mut LayerSlot, dev: &wgpu::Device, queue: &wgpu::Queue, label: &str, layer: Option<&LayerCanvas>, rows: Option<(u32, u32)>) -> u64 {
    let Some(layer) = layer else { return 0 };
    let c = layer.read();
    let stale_tex = slot.tex.as_ref().is_none_or(|t| t.w != c.width().max(1) || t.h != c.height().max(1));
    if stale_tex {
        slot.tex = Some(Tex::new(dev, label, c.width(), c.height()));
    }
    if !stale_tex && slot.gen == layer.gen() && slot.rows == rows {
        return 0;
    }
    let bytes = super::color_bytes(c.pixels());
    // 镜像停在上一代、这一代只改了几行（行区间也没变）：只补传那几行
    if let (false, Some((y0, y1))) = (stale_tex, layer.partial_since(slot.gen).filter(|_| slot.rows == rows)) {
        slot.gen = layer.gen();
        let (y0, y1) = (y0.min(c.height()), y1.min(c.height()));
        return slot.tex.as_ref().map(|t| t.upload_rows(queue, bytes, c.width(), y0, y1.saturating_sub(y0), c.width())).unwrap_or(0);
    }
    slot.gen = layer.gen();
    slot.rows = rows;
    let (y0, n) = match (label, rows) {
        ("fixed", Some((a, b))) => (a.min(c.height()), b.saturating_add(1).min(c.height()).saturating_sub(a.min(c.height()))),
        ("fixed", None) => (0, 0),
        _ => (0, c.height()),
    };
    slot.tex.as_ref().map(|t| t.upload_rows(queue, bytes, c.width(), y0, n, c.width())).unwrap_or(0)
}

mod present;
pub use present::{open, OpenOptions, SurfaceSpec};
pub use readback::{cpu_rgba, first_mismatch};
