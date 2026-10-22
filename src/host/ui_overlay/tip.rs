//! 提示框（`wx.showToast` / `wx.showLoading`）的绘制。几何在 `geometry.rs`，图标在 `icons.rs`。
use super::draw::{draw_text_line, fill_rrect_aa, ALL_CORNERS};
use super::geometry::{tip_box, TipBox, TipIcon, TIP_BG, TIP_FG, TIP_RADIUS};
use super::icons::{self, Glyph};
use crate::compositor::Footprint;
use crate::text::TextRenderer;

/// 转圈的环宽（WeUI 在提示框里把 `.weui-primary-loading` 的环宽改成 4px）
const SPINNER_RING: f32 = 4.0;

/// 画一个提示框（物理像素缓冲，`sf` 是设备像素比）。`step` 是转圈走到第几步
pub(super) fn render_tip(
    buf: &mut [u32],
    w: u32,
    h: u32,
    sf: f32,
    title: &str,
    icon: &TipIcon,
    step: u32,
    tr: Option<&TextRenderer>,
) -> Footprint {
    let b = tip_box(title, icon, (w as f32 / sf, h as f32 / sf), tr);
    let p = |v: f32| v * sf;
    // 提示框没有可见的遮罩（`mask: true` 只挡触摸），所有笔画都在盒子里
    let mut fp = Footprint::NONE;
    fp.touch(p(b.y), p(b.y + b.h), h);
    draw_box(buf, w, h, sf, &b, icon, step, tr);
    fp
}

#[allow(clippy::too_many_arguments)]
fn draw_box(buf: &mut [u32], w: u32, h: u32, sf: f32, b: &TipBox, icon: &TipIcon, step: u32, tr: Option<&TextRenderer>) {
    let p = |v: f32| v * sf;
    fill_rrect_aa(buf, w, h, (p(b.x), p(b.y), p(b.w), p(b.h)), p(TIP_RADIUS), TIP_BG, 1.0, ALL_CORNERS);
    if let Some((ix, iy, s)) = b.icon {
        let rect = (p(ix), p(iy), p(s));
        match icon {
            TipIcon::Success => icons::draw_glyph(buf, w, h, Glyph::Success, rect, TIP_FG),
            TipIcon::Error => icons::draw_glyph(buf, w, h, Glyph::Warn, rect, TIP_FG),
            TipIcon::Loading => {
                let c = (p(ix + s / 2.0), p(iy + s / 2.0));
                icons::draw_spinner(buf, w, h, c, p(s), p(SPINNER_RING), step, TIP_FG);
            }
            // 远程图还没到时先空着（图片层下载完会置脏，下一次合成就有了）
            TipIcon::Image(src) => {
                icons::draw_image(buf, w, h, src, rect);
            }
            TipIcon::None => {}
        }
    }
    let Some(tr) = tr else { return };
    let size = p(b.font);
    for (i, line) in b.lines.iter().enumerate() {
        let lw = tr.measure_text(line, size);
        // 行盒内垂直居中：基线 ≈ 行盒中线 + 0.34em（与页面文字 renderer::components::text 一致）
        let baseline = p(b.text_top + i as f32 * b.line_h + b.line_h / 2.0) + size * 0.34;
        draw_text_line(buf, w, h, tr, line, (p(b.center_x) - lw / 2.0, baseline), size, TIP_FG, false, sf);
    }
}
