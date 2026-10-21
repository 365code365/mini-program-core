//! `wx.showModal` 的绘制。几何在 `geometry.rs`（命中与绘制共用一份）。
//!
//! 颜色取 WeUI 2.x 浅色主题：遮罩 rgba(0,0,0,0.6)、白底、标题 FG-0 rgba(0,0,0,0.9)、
//! 内容 FG-1 rgba(0,0,0,0.5)、分隔线 rgba(0,0,0,0.1) 的 0.5px 细线、按下 #ECECEC；
//! 两个按钮都是 17px 粗体，颜色是 `cancelColor`（缺省 #000000）/ `confirmColor`（缺省 #576B95）。
use super::draw::{dim, draw_text_line, fill_rrect_aa, ALL_CORNERS};
use super::geometry::{modal_geom, MODAL_FOOTER_H, MODAL_RADIUS};
use super::ModalState;
use crate::text::TextRenderer;
use crate::Color;

/// 遮罩之后保留的亮度（rgba(0,0,0,0.6)）
pub(super) const MASK_KEEP: f32 = 0.4;
/// rgba(0,0,0,0.9) / rgba(0,0,0,0.5) 压在白底上
const FG_0: u32 = 0x1A1A1A;
const FG_1: u32 = 0x808080;
const PRESSED: u32 = 0xECECEC;

fn rgb(c: Color) -> u32 {
    ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32
}

pub(super) fn render_modal(buf: &mut [u32], w: u32, h: u32, sf: f32, m: &ModalState, tr: Option<&TextRenderer>) {
    dim(buf, MASK_KEEP);
    let g = modal_geom(m, (w as f32 / sf, h as f32 / sf), tr);
    let p = |v: f32| v * sf;
    fill_rrect_aa(buf, w, h, (p(g.x), p(g.y), p(g.w), p(g.h)), p(MODAL_RADIUS), 0xFFFFFF, 1.0, ALL_CORNERS);

    // 按下的那个按钮先铺底色（只圆它挨着的下角，盒子 overflow: hidden 的效果）
    let half = g.w / 2.0;
    if let Some(btn) = m.pressed_button.as_deref() {
        let (bx, bw, corners) = match (m.show_cancel, btn) {
            (true, "cancel") => (g.x, half, [false, false, false, true]),
            (true, _) => (g.x + half, half, [false, false, true, false]),
            (false, _) => (g.x, g.w, [false, false, true, true]),
        };
        let rect = (p(bx), p(g.footer_y), p(bw), p(MODAL_FOOTER_H));
        fill_rrect_aa(buf, w, h, rect, p(MODAL_RADIUS), PRESSED, 1.0, corners);
    }
    // 按钮条上沿、两个按钮之间的 0.5px 细线（`setTopLine` / `setLeftLine` 是 1px 再 scale 0.5）
    let hair = 0.5 * sf;
    fill_rrect_aa(buf, w, h, (p(g.x), p(g.footer_y), p(g.w), hair), 0.0, 0x000000, 0.1, ALL_CORNERS);
    if m.show_cancel {
        let rect = (p(g.x + half), p(g.footer_y), hair, p(MODAL_FOOTER_H));
        fill_rrect_aa(buf, w, h, rect, 0.0, 0x000000, 0.1, ALL_CORNERS);
    }

    let Some(tr) = tr else { return };
    let size = p(g.font);
    let center = p(g.x + g.w / 2.0);
    let mut line = |text: &str, top: f32, line_h: f32, cx: f32, color: u32, bold: bool| {
        let lw = tr.measure_text_weighted(text, size, 0.0, bold && tr.has_bold_face());
        let baseline = p(top + line_h / 2.0) + size * 0.34;
        draw_text_line(buf, w, h, tr, text, (cx - lw / 2.0, baseline), size, color, bold, sf);
    };
    for (i, t) in g.title_lines.iter().enumerate() {
        line(t, g.title_top + i as f32 * g.line_h, g.line_h, center, FG_0, true);
    }
    let (body_color, body_bold) = if g.body_is_main { (FG_0, true) } else { (FG_1, false) };
    for (i, t) in g.body_lines.iter().enumerate() {
        line(t, g.body_top + i as f32 * g.line_h, g.line_h, center, body_color, body_bold);
    }
    if m.show_cancel {
        line(&m.cancel_text, g.footer_y, MODAL_FOOTER_H, p(g.x + half / 2.0), rgb(m.cancel_color), true);
        line(&m.confirm_text, g.footer_y, MODAL_FOOTER_H, p(g.x + half * 1.5), rgb(m.confirm_color), true);
    } else {
        line(&m.confirm_text, g.footer_y, MODAL_FOOTER_H, center, rgb(m.confirm_color), true);
    }
}
