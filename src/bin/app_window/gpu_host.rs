//! 桌面窗体接 GPU 合成器（设计 §7.1、§7.2；v2 修订 R1、R4、R16）。
//!
//! - **无头**：整个会话建**一个常驻**的离屏合成器，`pump_one_frame` 每帧之后都合成一次 ——
//!   增量镜像（只传脏区）在每一帧都真实发生，最后那张快照才代表 GPU 的输出。
//!   `MINI_GPU_VERIFY=1`：每帧读回、与同一份图层的 `cpu::compose` 逐字节比较，
//!   不一致就以退出码 5 结束（`tools/gpu-parity.sh` 靠它守住增量路径）。
//! - **窗口**：`resumed` 里用窗口建 surface；成功就不建 softbuffer（提前建会多挂一个子层）。
//!   上屏失败（设备丢失 / surface 无法恢复 / 校验错误）当帧回退：建 softbuffer、同一帧走 CPU，
//!   不留空帧、不黑屏。回退是粘性的，本会话不再尝试 GPU。
//!
//! 没编 `gpu` feature 时这里全是空实现，窗体照常只走 CPU。
// 没编 gpu 时：待建请求、环境、校验计数这些字段只有 GPU 分支读，空实现里用不上
#![cfg_attr(not(feature = "gpu"), allow(dead_code, unused_imports))]
use super::backend_host::PendingGpu;
use mini_render::compositor::{diag, BackendStatus, HostEnv};

/// 退出码：`MINI_GPU_VERIFY` 发现 GPU 与 CPU 不一致
pub const EXIT_VERIFY_MISMATCH: i32 = 5;

/// 窗体里与 GPU 相关的全部状态（字段不随 feature 变，窗体结构体里不用写 cfg）
#[derive(Default)]
pub struct GpuSlot {
    #[cfg(feature = "gpu")]
    pub comp: Option<Box<mini_render::compositor::gpu::GpuCompositor>>,
    pub pending: Option<PendingGpu>,
    pub env: HostEnv,
    /// 逐帧校验过的帧数（防空转：脚本要求它不小于请求帧数的 90%）
    pub verified: u64,
    /// 最近一帧的 GPU 账（给 `--bench`）
    #[cfg(feature = "gpu")]
    pub last: Option<mini_render::compositor::gpu::GpuFrameStats>,
    /// 上一次上屏被跳过（Timeout / Occluded），要补一帧
    pub present_pending: bool,
    /// 无头每帧合成后都等 GPU 做完（`--bench` 要量「GPU 完成」时间）
    pub wait_frames: bool,
}

/// 最近一帧 GPU 账的扁平版（`--bench` 用；字段不随 feature 变）
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LastGpuFrame {
    pub encode_ms: f32,
    pub gpu_done_ms: Option<f32>,
    pub acquire_ms: f32,
    pub upload_bytes: u64,
    pub passthrough: bool,
    pub gpu_frame: u32,
}

impl GpuSlot {
    pub fn new(pending: Option<PendingGpu>, env: &HostEnv) -> Self {
        Self { pending, env: env.clone(), ..Self::default() }
    }

    /// 最近一帧的 GPU 账；没走 GPU 时是 None
    pub fn last_frame(&self) -> Option<LastGpuFrame> {
        #[cfg(feature = "gpu")]
        {
            self.last.map(|s| LastGpuFrame {
                encode_ms: s.encode_ms,
                gpu_done_ms: s.gpu_done_ms,
                acquire_ms: s.acquire_ms,
                upload_bytes: s.upload_bytes,
                passthrough: s.passthrough,
                gpu_frame: s.gpu_frame,
            })
        }
        #[cfg(not(feature = "gpu"))]
        {
            None
        }
    }

    /// 当前是否由 GPU 合成
    pub fn active(&self) -> bool {
        #[cfg(feature = "gpu")]
        {
            self.comp.is_some()
        }
        #[cfg(not(feature = "gpu"))]
        {
            false
        }
    }
}

#[cfg(feature = "gpu")]
pub use imp::*;

#[cfg(not(feature = "gpu"))]
pub fn probe_offscreen(_p: PendingGpu, _env: &HostEnv) -> Option<BackendStatus> {
    None
}

#[cfg(not(feature = "gpu"))]
impl crate::MiniAppWindow {
    pub(crate) fn gpu_open_offscreen(&mut self, _scale: f64) {}
    pub(crate) fn gpu_headless_frame(&mut self, _wait: bool) {}
    pub(crate) fn gpu_snapshot_rgba(&mut self, _scale: f64, _has_tabbar: bool) -> Option<Vec<u8>> {
        None
    }
    pub(crate) fn gpu_open_window(&mut self, _w: &std::sync::Arc<winit::window::Window>) -> bool {
        false
    }
    pub(crate) fn gpu_finish_headless(&self) {}
}

#[cfg(feature = "gpu")]
mod imp {
    use super::*;
    use crate::app_window::{LOGICAL_HEIGHT, LOGICAL_WIDTH};
    use mini_render::compositor::gpu::{self, OpenOptions, SurfaceSpec};
    use mini_render::compositor::select::{SelectInput, GPU_COMPILED};
    use mini_render::compositor::{DeviceInfo, Platform};
    use std::sync::Arc;

    fn options<'a>(p: PendingGpu, env: &HostEnv, device: &'a DeviceInfo, scale: f64) -> OpenOptions<'a> {
        let (w, h) = ((LOGICAL_WIDTH as f64 * scale) as u32, (LOGICAL_HEIGHT as f64 * scale) as u32);
        OpenOptions {
            select: SelectInput {
                pref: p.pref,
                source: p.source,
                platform: Platform::current(),
                device,
                gpu_compiled: GPU_COMPILED,
                has_surface: true,
            },
            fault: env.gpu_fault.valid(),
            log: env.gpu_log,
            width: w,
            height: h,
            canvas_w: w,
            margin_px: mini_render::renderer::VIEWPORT_CULL_MARGIN_PX as u32,
            use_verdict: true,
        }
    }

    /// `MINI_GPU_VERIFY`：这一帧的 GPU 读回与同一份图层的 CPU 合成逐字节比较，不一致退出码 5。
    /// 分层帧现场再 CPU 合成一次（图层是只读的，结果确定）；直通帧对照合成器上传的那一份
    /// （覆盖层按墙上时钟动画，事后再合成相位会变 —— 见 `GpuCompositor::last_passthrough`）
    fn verify_or_exit(comp: &gpu::GpuCompositor, layers: &mini_render::compositor::FrameLayers<'_>, gpu_px: &[u8], frame: u64) {
        let reference = match comp.last_passthrough() {
            Some(px) => gpu::cpu_rgba(px),
            None => {
                let mut buf = vec![0u32; (layers.width * layers.height) as usize];
                mini_render::compositor::cpu::compose(&mut buf, layers);
                gpu::cpu_rgba(&buf)
            }
        };
        if let Some((x, y, c, g)) = gpu::first_mismatch(&reference, gpu_px, layers.width) {
            diag::emit(&diag::verify_mismatch(frame, x, y, c, g));
            std::process::exit(EXIT_VERIFY_MISMATCH);
        }
    }

    /// `--backend-probe`：真的建一次离屏合成器（含自检），给出结论
    pub fn probe_offscreen(p: PendingGpu, env: &HostEnv) -> Option<BackendStatus> {
        let device = DeviceInfo::default();
        let status = match gpu::open(options(p, env, &device, 2.0), None) {
            Ok((_, s)) => s,
            Err(s) => s,
        };
        diag::emit(&diag::selected(&status));
        Some(status)
    }

    impl crate::MiniAppWindow {
        /// 无头：会话开始时建常驻离屏合成器（偏好不是 cpu 时）
        pub(crate) fn gpu_open_offscreen(&mut self, scale: f64) {
            let Some(p) = self.gpu.pending.take() else { return };
            let device = DeviceInfo::default();
            let opts = options(p, &self.gpu.env, &device, scale);
            match gpu::open(opts, None) {
                Ok((c, s)) => {
                    self.gpu.comp = Some(Box::new(c));
                    self.backend = s;
                }
                Err(s) => self.backend = s,
            }
            diag::emit(&diag::selected(&self.backend));
        }

        /// GPU 这一帧出错了：当帧回退 CPU（粘性），打运行中回退日志
        fn gpu_runtime_fallback(&mut self, why: &str) {
            let frame = self.gpu.comp.as_ref().map(|c| c.gpu_frame()).unwrap_or(0);
            diag::emit(&diag::runtime_fallback(why, frame));
            self.gpu.comp = None;
            self.gpu.last = None;
            self.backend = self.backend.fell_back(why);
        }

        /// 无头：`pump_one_frame` 之后合成这一帧；`wait` 时等 GPU 做完（基准要「GPU 完成」时间）
        pub(crate) fn gpu_headless_frame(&mut self, wait: bool) {
            let Some(mut comp) = self.gpu.comp.take() else { return };
            let scale = self.scale_factor;
            let has_tabbar = self.page_stack.last().map(|p| self.is_tabbar_page(&p.path)).unwrap_or(false);
            let verify = self.gpu.env.gpu_verify;
            let result = match self.headless_layers(scale, has_tabbar) {
                None => Ok(None),
                Some(layers) => comp.compose_offscreen(&layers, wait || verify).and_then(|st| {
                    if verify {
                        let gpu_px = comp.read_back()?;
                        verify_or_exit(&comp, &layers, &gpu_px, self.gpu.verified + 1);
                    }
                    Ok(Some(st))
                }),
            };
            match result {
                Ok(st) => {
                    if verify && st.is_some() {
                        self.gpu.verified += 1;
                    }
                    self.gpu.last = st;
                    self.gpu.comp = Some(comp);
                }
                Err(e) => {
                    self.gpu.comp = Some(comp);
                    self.gpu_runtime_fallback(&e.to_string());
                }
            }
        }

        /// 无头最后那张快照：GPU 合成并读回（PNG 才代表 GPU 的输出）。没有 GPU 时返回 None
        pub(crate) fn gpu_snapshot_rgba(&mut self, scale: f64, has_tabbar: bool) -> Option<Vec<u8>> {
            let mut comp = self.gpu.comp.take()?;
            let verify = self.gpu.env.gpu_verify;
            let result = match self.headless_layers(scale, has_tabbar) {
                None => return { self.gpu.comp = Some(comp); None },
                Some(layers) => comp.compose_offscreen(&layers, true).and_then(|_| comp.read_back()).map(|px| {
                    if verify {
                        verify_or_exit(&comp, &layers, &px, self.gpu.verified + 1);
                    }
                    px
                }),
            };
            self.gpu.comp = Some(comp);
            match result {
                Ok(px) => {
                    if verify {
                        self.gpu.verified += 1;
                    }
                    Some(px)
                }
                Err(e) => {
                    // 读回失败：这张快照改用 CPU，并打回退日志（gpu-parity 据此判失败）
                    self.gpu_runtime_fallback(&e.to_string());
                    None
                }
            }
        }

        /// 无头结束：逐帧校验的汇总（脚本 grep 固定前缀，按 N 判防空转）
        pub(crate) fn gpu_finish_headless(&self) {
            if self.gpu.env.gpu_verify && self.backend.active == mini_render::compositor::BackendKind::Gpu {
                diag::emit(&diag::verify_summary(self.gpu.verified));
            }
        }

        /// 窗口：用窗口建 surface 与合成器。成功返回 true（调用方就不建 softbuffer）
        pub(crate) fn gpu_open_window(&mut self, window: &Arc<winit::window::Window>) -> bool {
            let Some(p) = self.gpu.pending.take() else { return false };
            let device = DeviceInfo::default();
            let size = window.inner_size();
            let mut opts = options(p, &self.gpu.env, &device, window.scale_factor());
            opts.width = size.width;
            opts.height = size.height;
            let spec = SurfaceSpec::Target(gpu::wgpu::SurfaceTarget::from(window.clone()));
            let ok = match gpu::open(opts, Some(spec)) {
                Ok((c, s)) => {
                    self.gpu.comp = Some(Box::new(c));
                    self.backend = s;
                    true
                }
                Err(s) => {
                    self.backend = s;
                    false
                }
            };
            diag::emit(&diag::selected(&self.backend));
            ok
        }

        /// 窗口上屏失败后的回退：建 softbuffer（同一帧随后走 CPU）
        pub(crate) fn gpu_fallback_to_softbuffer(&mut self, why: &str) {
            self.gpu_runtime_fallback(why);
            if let Some(window) = &self.window {
                match softbuffer::Context::new(window.clone()).and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone())) {
                    Ok(s) => {
                        self.surface = Some(s);
                        self.surface_size = None;
                    }
                    Err(e) => diag::emit(&format!("❌ 回退 CPU 后建 softbuffer 失败：{e}")),
                }
            }
        }
    }
}
