//! SDK 的 GPU 上屏（feature `gpu`）：宿主挂上原生 surface（Android 的 `ANativeWindow*`、
//! Apple 的 `CAMetalLayer*`）后，合成交给 [`GpuCompositor`]，引擎直接画到 surface 上。
//!
//! 与桌面窗体同一个合成器（`compositor::gpu`）：同一套门槛、自检（与 CPU 逐字节对拍）、
//! 回退规则。这里只做 SDK 这一侧的接线：
//!
//! - **挂载**：[`attach_native_surface`](MiniEngine::attach_native_surface) 按偏好与机型信息
//!   选一次；偏好是 cpu（SDK 默认）时根本不碰 GPU。挂着 surface 时宿主再改偏好 / 机型信息，
//!   由 [`refresh_native_backend`](MiniEngine::refresh_native_backend) 重新选（必要时重建合成器）。
//! - **每帧**：`compose_frame` 先试 GPU；上屏了就不做 CPU 合成，`pump_ex` 带 `PRESENTED`
//!   （宿主不要再取像素）；surface 暂时取不到带 `PENDING`，下一次 pump 补一帧。
//! - **出错**：设备丢失 / surface 丢失 / 校验错误一律**当帧**回退 CPU，`pump_ex` 带
//!   `BACKEND_CHANGED`，宿主切回像素出口。回退是粘性的：本实例之后再挂 surface 也不再试 GPU。
//!   没失败过时宿主主动的 detach / attach 照常再试（v2 修订 R5）。
//!
//! Flutter 的 Android SurfaceProducer 给的就是一块 Surface（→ `ANativeWindow`），直接能用；
//! iOS 的 Flutter 外部纹理是 CVPixelBuffer，不是 CAMetalLayer —— 那条路要新的 surface 种类，
//! 还没做。
use super::engine::MiniEngine;
use crate::compositor::gpu::{self, OpenOptions, PresentOutcome, SurfaceSpec};
use crate::compositor::select::{self, SelectInput, GPU_COMPILED};
use crate::compositor::{diag, pref, BackendKind, BackendStatus, FrameLayers, HostKind, Platform, SurfaceKind};
use std::ffi::c_void;
use std::ptr::NonNull;

/// 这一帧 GPU 上屏的结论
pub(crate) enum GpuFrame {
    /// 没有 GPU 合成器：走 CPU
    NotActive,
    Presented(gpu::GpuFrameStats),
    /// surface 暂时取不到：屏上保留上一帧，下一次 pump 补
    Skipped,
}

impl MiniEngine {
    /// 挂原生 surface 并按偏好选后端。返回挂载后的实际后端。重复调用等于先卸再挂。
    ///
    /// `native` 由调用方保证活到 [`detach_native_surface`](Self::detach_native_surface)
    /// 或实例销毁之后（Android：JNI 层持有 `ANativeWindow_fromSurface` 的那一个引用）。
    pub fn attach_native_surface(&mut self, kind: SurfaceKind, native: *mut c_void) -> BackendKind {
        self.detach_native_surface();
        self.surface = Some(kind);
        self.native_surface = NonNull::new(native).map(|p| (kind, p.as_ptr() as usize));
        // 宿主主动挂载不算「引擎引起的变化」，不置 BACKEND_CHANGED（R5）
        self.refresh_native_backend(false);
        self.backend.active
    }

    /// 卸 surface：先放掉合成器手里的 surface，之后宿主才能释放原生对象。
    ///
    /// 合成器本身（设备、管线）停到 `gpu_parked`，下次挂载直接复用（[`reattach_parked`](Self::reattach_parked)）。
    /// 也不整页重画：CPU 画布一直是全的（GPU 只是拿它们合成），CPU 出口要的只是重新合成一次。
    /// 从前这里 `mark_full`：回桌面再回来那一帧整页重画 15ms（一加 PLY110），白画
    pub fn detach_native_surface(&mut self) {
        self.native_surface = None;
        if let Some(mut comp) = self.gpu.take() {
            comp.detach_surface();
            self.gpu_parked = Some(comp);
            self.dmg.compose_dirty = true;
        }
    }

    /// 挂着原生 surface 时重新选一次后端。没挂返回 false（调用方走只看种类的老路）。
    ///
    /// GPU 正在用、门槛照样过 → 不重建（只更新偏好来源）；门槛不过 → 放掉合成器；
    /// 还没有合成器、门槛过了、本实例没失败过 → 建一个（含自检）。
    /// `engine_caused` 为真且实际后端变了时，下一次 `pump_ex` 带 `BACKEND_CHANGED`。
    pub(super) fn refresh_native_backend(&mut self, engine_caused: bool) -> bool {
        let Some((kind, addr)) = self.native_surface else { return false };
        let before = self.backend.active;
        let r = pref::resolve(HostKind::Sdk, None, &self.env, self.host_pref);
        let device = self.device_info;
        let input = SelectInput {
            pref: r.pref,
            source: r.source,
            platform: Platform::current(),
            device: &device,
            gpu_compiled: GPU_COMPILED,
            has_surface: true,
        };
        let next = match (select::pre_gpu(&input), self.gpu_failed.clone()) {
            (Err(status), _) => {
                self.drop_gpu();
                status
            }
            (Ok(()), Some(why)) => BackendStatus::cpu(r.pref, r.source, why),
            (Ok(()), None) => match self.gpu.as_ref() {
                Some(comp) => BackendStatus::gpu(r.pref, r.source, comp.facts().describe()),
                None => match self.reattach_parked(kind, addr) {
                    Some(comp_facts) => BackendStatus::gpu(r.pref, r.source, comp_facts),
                    None => self.open_gpu(kind, addr, input),
                },
            },
        };
        if next != self.backend {
            diag::emit(&diag::selected(&next));
        }
        if engine_caused && next.active != before {
            self.backend_changed = true;
        }
        self.backend = next;
        true
    }

    /// 停着的合成器挂上新 surface（门槛已过）。成功时 `self.gpu` 有值，返回适配器描述；
    /// 失败（新 surface 这个适配器不支持之类）放掉它，调用方照常重开
    fn reattach_parked(&mut self, kind: SurfaceKind, addr: usize) -> Option<String> {
        let mut comp = self.gpu_parked.take()?;
        let ptr = NonNull::new(addr as *mut c_void)?;
        let spec = surface_spec(kind, ptr).ok()?;
        let (pw, ph) = self.pixel_size();
        let t0 = std::time::Instant::now();
        if let Err(why) = comp.reattach_surface(spec, pw, ph) {
            diag::emit(&format!("🎨 GPU 合成器复用失败（{why}），重开"));
            return None;
        }
        diag::emit(&format!("🎨 GPU 合成器复用：只换 surface（{:.1}ms，不重开设备、不重跑自检）", t0.elapsed().as_secs_f32() * 1000.0));
        let facts = comp.facts().describe();
        self.gpu = Some(comp);
        // 镜像清空了，下一帧整张重传；页面不用重画
        self.dmg.compose_dirty = true;
        Some(facts)
    }

    /// 建合成器（门槛已过）。成功时 `self.gpu` 有值；返回结论
    fn open_gpu(&mut self, kind: SurfaceKind, addr: usize, select: SelectInput<'_>) -> BackendStatus {
        let (pref, source) = (select.pref, select.source);
        let Some(ptr) = NonNull::new(addr as *mut c_void) else {
            return BackendStatus::cpu(pref, source, select::REASON_NO_SURFACE);
        };
        let spec = match surface_spec(kind, ptr) {
            Ok(spec) => spec,
            Err(why) => return BackendStatus::cpu(pref, source, why),
        };
        let (pw, ph) = self.pixel_size();
        let opts = OpenOptions {
            select,
            fault: self.env.gpu_fault.valid(),
            log: self.env.gpu_log,
            width: pw,
            height: ph,
            canvas_w: pw,
            margin_px: crate::renderer::VIEWPORT_CULL_MARGIN_PX as u32,
            use_verdict: true,
        };
        match gpu::open(opts, Some(spec)) {
            Ok((comp, status)) => {
                self.gpu = Some(Box::new(comp));
                // 换了上屏方式：下一帧整帧合成（GPU 镜像是空的）
                self.mark_full();
                status
            }
            Err(status) => status,
        }
    }

    fn drop_gpu(&mut self) {
        self.gpu_parked = None;
        if self.gpu.take().is_some() {
            self.mark_full();
        }
    }

    /// 上一帧是 GPU 上屏的、合成缓冲还是旧的：用同一组图层补一次 CPU 合成。
    /// 只在有人取像素（`mr_app_pixels*`、截图、调试 dump）时调；GPU 与 CPU 合成逐字节相同，
    /// 所以取到的就是屏上那一帧。每帧上屏本身不做这一步（那正是 GPU 省下的活）。
    pub(super) fn sync_cpu_frame(&mut self) {
        let Some((has_tabbar, bg)) = self.cpu_frame_pending.take() else { return };
        let gpu = self.gpu.take();
        self.compose_frame(has_tabbar, bg);
        self.gpu = gpu;
    }

    /// GPU 这一帧出错了：当帧回退 CPU（粘性），下一次 `pump_ex` 带 `BACKEND_CHANGED`
    pub(super) fn gpu_fell_back(&mut self, why: &str, frame: u32) {
        diag::emit(&diag::runtime_fallback(why, frame));
        self.gpu = None;
        self.gpu_parked = None;
        self.backend = self.backend.fell_back(why);
        self.gpu_failed = Some(format!("本实例 GPU 已失败过：{why}"));
        self.backend_changed = true;
    }
}

/// 宿主给的原生指针 → wgpu 的 surface 目标
fn surface_spec(kind: SurfaceKind, ptr: NonNull<c_void>) -> Result<SurfaceSpec, &'static str> {
    Ok(match kind {
        SurfaceKind::AndroidWindow => SurfaceSpec::Unsafe(gpu::wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(gpu::wgpu::rwh::RawDisplayHandle::Android(
                gpu::wgpu::rwh::AndroidDisplayHandle::new(),
            )),
            raw_window_handle: gpu::wgpu::rwh::RawWindowHandle::AndroidNdk(
                gpu::wgpu::rwh::AndroidNdkWindowHandle::new(ptr),
            ),
        }),
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        SurfaceKind::MetalLayer => {
            SurfaceSpec::Unsafe(gpu::wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(ptr.as_ptr()))
        }
        // FFI 层已按目标拦过（supported_on_target），走到这里只可能是 Rust 侧直接调用
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        SurfaceKind::MetalLayer => {
            return Err("当前编译目标不支持 CAMetalLayer");
        }
    })
}

/// 合成这一帧：有 GPU 合成器就试 GPU。写成自由函数而不是方法：调用方手里的 `layers`
/// 借着引擎的画布字段，这里只要 `gpu` 那一个字段的可变借用。出错返回 (原因, GPU 帧号)。
pub(super) fn present_on(
    gpu: &mut Option<Box<gpu::GpuCompositor>>,
    layers: &FrameLayers<'_>,
) -> Result<GpuFrame, (String, u32)> {
    let Some(comp) = gpu.as_mut() else { return Ok(GpuFrame::NotActive) };
    match comp.present(layers) {
        Ok(PresentOutcome::Presented(st)) => Ok(GpuFrame::Presented(st)),
        Ok(PresentOutcome::Skipped(_)) => Ok(GpuFrame::Skipped),
        Err(e) => Err((e.to_string(), comp.gpu_frame())),
    }
}
