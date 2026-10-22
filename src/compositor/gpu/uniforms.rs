//! 这一帧的 uniform：把 [`FrameLayers`] 里的各层参数按 `compose.wgsl` 的 `struct U` 摆好。
//!
//! 从 `gpu/mod.rs` 原样搬出（那边超了 500 行）：一个函数，只把可见性从私有改成 `pub(super)`。
use super::pipeline::{self, Uniforms};
use super::GpuCompositor;
use crate::compositor::{FaultSpec, FrameLayers, LayerCanvas};

impl GpuCompositor {
    pub(super) fn uniforms(&self, l: &FrameLayers<'_>, passthrough: bool) -> Uniforms {
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
        let content_h = l.height - if l.has_tabbar { l.tabbar_px.min(l.height) } else { 0 };
        // 导航栏：与 present_to_buffer（`top_px.min(内容区高)`）、paint_top_band（`rows.min(h).min(画布高)`）同一组夹法
        let (top_px, top_rows, top_w) = match &l.top {
            Some(t) => {
                let band = t.canvas.read();
                (t.rows.min(content_h), t.rows.min(l.height).min(band.height()), band.width())
            }
            None => (0, 0, 0),
        };
        // 转场 / 侧滑（edge_back::compose / compose_shot_over 同一套参数）
        let (eb_mode, eb_off, eb_par, eb_dim, eb_shadow, eb_shadow_f, eb_has_shot) = match &l.edge_back {
            Some(e) => {
                let (off, par, dim, shadow, f) = crate::host::edge_back::gpu_params(l.width, e.offset_px, e.progress);
                let has = e.shot.is_some_and(|s| s.width == l.width && s.height == l.height && self.shot_id == Some(s.id));
                (if e.shot_on_top { 2 } else { 1 }, off, par, dim, shadow, f, has as u32)
            }
            None => (0, 0, 0, 1.0, 0, [1.0; 16], 0),
        };
        Uniforms {
            width: l.width,
            height: l.height,
            content_h,
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
            top_px,
            top_rows,
            top_w,
            eb_mode,
            eb_off,
            eb_par,
            eb_shadow,
            eb_has_shot,
            eb_dim,
            pass_y0: self.pass_band.y0,
            pass_y1: self.pass_band.y1,
            lut_on: (passthrough && self.pass_band.lut.is_some()) as u32,
            eb_shadow_f,
            lut: self.pass_band.lut.as_ref().map(pipeline::pack_lut).unwrap_or([0; 64]),
        }
    }
}
