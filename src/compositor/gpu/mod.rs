//! GPU 合成器（feature `gpu`，wgpu）：设计 §4~§6；v2 修订 R1~R16。
//!
//! 内容仍只由 CPU 光栅化。这里把 [`FrameLayers`] 里的图层当纹理**镜像**（页面按脏区增量上传、
//! tabBar / fixed 按代数整层或按行上传），用整数着色器复刻 [`cpu::compose`](super::cpu::compose)
//! 的算术，再画到 surface（上屏）或离屏目标（无头 / 自检 / 测试读回）。
//! 同一份 `FrameLayers`，输出与 CPU **逐字节相同** —— 这是判据，不是愿望：启动自检、
//! `MINI_GPU_VERIFY=1` 的逐帧对拍、`tools/gpu-parity.sh` 三道守着。
//!
//! Toast / Loading / Modal、picker 面板、侧滑返回是直接画进输出缓冲、混合时读目标像素的
//! （还带浮点），这些帧走**直通**：CPU 合成整帧、整块上传、着色器原样输出（§5.3）。
//! 判断故意从宽（状态 `Some` 就算，不看 visible）：误判成直通永远是对的。
//!
//! 合成器只拿到图层的只读引用，拿不到布局 / 命中 / 事件 / 动画时钟 ——
//! 「后端只管画」是类型层面保证的。
pub mod device;
mod mirror;
pub mod pipeline;
mod readback;
pub mod selftest;
#[cfg(test)]
mod tests;

#[cfg(not(target_endian = "little"))]
compile_error!("GPU 合成器按小端字节序上传 u32 像素（直通帧），大端目标请关掉 feature gpu");

/// 宿主（桌面二进制）要用 wgpu 的类型构造窗口 surface
pub use wgpu;

use super::{cpu, FaultSpec, FrameLayers, LayerCanvas};
use device::Gpu;
use mirror::PageMirror;
use pipeline::{Pipeline, Tex, Uniforms};
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
}

/// 上屏结果
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PresentOutcome {
    Presented(GpuFrameStats),
    /// surface 暂时取不到（Timeout / Occluded）：屏上保留上一帧，宿主要安排补一帧（v2 修订 R4）
    Skipped(&'static str),
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
    pull: Option<Tex>,
    pass: Option<Tex>,
    pass_scratch: Vec<u32>,
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
            pull: None,
            pass: None,
            pass_scratch: Vec::new(),
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
        let pass_why = if o.modal.is_some() {
            Some("Modal")
        } else if o.toast.is_some() {
            Some("Toast")
        } else if o.loading.is_some() {
            Some("Loading")
        } else if o.picker.is_some() {
            Some("picker")
        } else if l.edge_back.is_some() {
            Some("侧滑返回")
        } else {
            None
        };
        let (dev, queue) = (&self.gpu.device, &self.gpu.queue);
        if let Some(_why) = pass_why {
            // 与快照一致：输出缓冲初值 0（tabBar 画布比 tabbar_px 矮时那几行保留初值）
            let n = (l.width * l.height) as usize;
            self.pass_scratch.clear();
            self.pass_scratch.resize(n, 0);
            cpu::compose(&mut self.pass_scratch, l);
            Tex::ensure(&mut self.pass, dev, "直通帧", l.width, l.height);
            // SAFETY: u32 切片按字节看（小端：B,G,R,0），长度 ×4，生命周期跟随 scratch
            let bytes = unsafe { std::slice::from_raw_parts(self.pass_scratch.as_ptr().cast::<u8>(), n * 4) };
            let sent = self.pass.as_ref().map(|t| t.upload_rows(queue, bytes, l.width, 0, l.height, l.width)).unwrap_or(0);
            return (sent, pass_why);
        }
        let mut sent = 0u64;
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
        let visible = (l.scroll_px, l.scroll_px.saturating_add(content_h as i32));
        if let Some(m) = &mut self.page {
            sent += m.sync(queue, l.page, visible);
        }
        sent += sync_layer(&mut self.tabbar, dev, queue, "tabBar", l.tabbar, None);
        sent += sync_layer(&mut self.fixed, dev, queue, "fixed", l.fixed, l.fixed_rows);
        if let Some(p) = l.pull {
            if let Some(band) = crate::host::render::pull_indicator_band(
                l.width, l.height, l.scale, p.gap_logical, p.refreshing, p.progress, p.phase,
            ) {
                Tex::ensure(&mut self.pull, dev, "下拉指示器", band.width(), band.height());
                let bytes = super::color_bytes(band.pixels());
                sent += self.pull.as_ref().map(|t| t.upload_rows(queue, bytes, band.width(), 0, band.height(), band.width())).unwrap_or(0);
            }
        }
        (sent, None)
    }

    fn uniforms(&self, l: &FrameLayers<'_>, passthrough: bool) -> Uniforms {
        let canvas = l.page.read();
        let tab = l.tabbar.map(LayerCanvas::read);
        let fix = l.fixed.map(LayerCanvas::read);
        let (row0, row_end) = match (fix, l.fixed_rows) {
            (Some(fc), Some((a, b))) => (a, l.height.min(fc.height()).min(b.saturating_add(1))),
            _ => (0, 0),
        };
        let pull_rows = l
            .pull
            .and_then(|p| {
                let gap_px = (p.gap_logical * l.scale).round() as u32;
                (gap_px >= 8 && l.width > 0).then(|| gap_px.min(l.height))
            })
            .unwrap_or(0);
        let tint = self.fault == Some(FaultSpec::Tint) && !self.selftesting;
        Uniforms {
            width: l.width,
            height: l.height,
            content_h: l.height - if l.has_tabbar { l.tabbar_px.min(l.height) } else { 0 },
            scroll_px: l.scroll_px,
            canvas_w: canvas.width(),
            canvas_h: canvas.height(),
            ring_rows: self.page.as_ref().map(|m| m.ring_rows).unwrap_or(1),
            page_bg: l.page_bg,
            has_tabbar: l.has_tabbar as u32,
            tabbar_on: tab.is_some() as u32,
            tabbar_px: l.tabbar_px,
            tabbar_w: tab.map(|t| t.width()).unwrap_or(0),
            tabbar_h: tab.map(|t| t.height()).unwrap_or(0),
            fixed_on: (row_end > row0) as u32,
            fixed_row0: row0,
            fixed_row_end: row_end,
            fixed_w: fix.map(|f| f.width()).unwrap_or(0),
            pull_rows,
            mode: passthrough as u32,
            fault_tint: tint as u32,
        }
    }

    /// 把这一帧编码进 `target`（格式 `format`）并提交。故障注入、致命错误检查都在这里
    fn render_into(&mut self, l: &FrameLayers<'_>, target: &wgpu::TextureView, format: wgpu::TextureFormat) -> Result<GpuFrameStats, GpuError> {
        self.gpu.poll_now();
        if let Some(e) = self.gpu.health.fatal() {
            return Err(e);
        }
        let t0 = Instant::now();
        let (upload_bytes, pass_why) = self.sync_layers(l);
        self.last_pass = pass_why.is_some();
        let u = self.uniforms(l, pass_why.is_some());
        self.gpu.queue.write_buffer(&self.pipe.uniforms, 0, u.bytes());
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
        let encode_ms = t0.elapsed().as_secs_f32() * 1000.0;
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
        Ok(GpuFrameStats { encode_ms, upload_bytes, passthrough: pass_why.is_some(), gpu_frame: self.gpu_frame, ..Default::default() })
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
    slot.gen = layer.gen();
    slot.rows = rows;
    let bytes = super::color_bytes(c.pixels());
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
