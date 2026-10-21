//! scroll-view 自定义下拉刷新（`refresher-enabled`）的默认样式：内容被拉下来后露出的那段空白里
//! 铺 `refresher-background`，中间画三个点。
//!
//! 微信的默认样式（`refresher-default-style`）只有 `black` / `white` / `none` 三种：三个圆点，
//! 下拉时逐渐显出，刷新中依次明暗循环。`none` 时只铺背景，内容由 `slot="refresher"` 的节点
//! 自己画（它绝对定位在内容上方，随内容一起被拉出来，见 `scroll_views::place_refresher_slot`）。
//! 圆点的尺寸与间距没有官方数字，取的是开发者工具里量到的近似值（直径 6px、间距 10px）。

use super::*;
use crate::renderer::components::parse_color_str;
use crate::{Paint, PaintStyle};
use crate::ui::scroll_view_state::RefresherPhase;

/// 圆点直径与圆心间距（逻辑 px）
const DOT_D: f32 = 6.0;
const DOT_GAP: f32 = 10.0;
/// 刷新中一轮明暗循环的时长（秒）
const CYCLE_SECS: f32 = 1.2;

impl WxmlRenderer {
    /// 在 scroll-view 顶部露出的空白里画刷新区域。`gap` 是物理像素，`(x, y, w)` 是 scroll-view 的盒子
    pub(super) fn draw_refresher_gap(
        &mut self,
        canvas: &mut Canvas,
        node: &RenderNode,
        component_id: &str,
        interaction: &InteractionManager,
        (x, y, w): (f32, f32, f32),
        gap: f32,
    ) {
        if gap < 0.5 || !super::scroll_views::bool_attr(&node.attrs, "refresher-enabled") {
            return;
        }
        let sf = self.scale_factor;
        if let Some(bg) = node.attrs.get("refresher-background").and_then(|s| parse_color_str(s)) {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            canvas.draw_rect(&GeoRect::new(x, y, w, gap), &paint);
        }
        let style = node.attrs.get("refresher-default-style").map(|s| s.trim()).unwrap_or("black");
        let base = match style {
            "none" => return,
            "white" => Color::WHITE,
            _ => Color::new(0x99, 0x99, 0x99, 255),
        };
        let threshold = node
            .attrs
            .get("refresher-threshold")
            .and_then(|v| super::scroll_views::parse_len(v, self.screen_width))
            .unwrap_or(45.0)
            .max(1.0);
        let refreshing = interaction
            .scroll_views
            .get(component_id)
            .map(|s| s.refresher == RefresherPhase::Refreshing)
            .unwrap_or(false);
        // 下拉时按进度显出；刷新中三个点依次亮起（按动画时钟走，快照可复现）
        let progress = (gap / sf / threshold).clamp(0.0, 1.0);
        let t = self.animation_time();
        if refreshing {
            self.mark_animating();
        }
        let cy = y + gap - threshold.min(gap / sf) * sf / 2.0;
        for i in 0..3 {
            let alpha = if refreshing {
                let phase = ((t / CYCLE_SECS + i as f32 / 3.0) % 1.0) * std::f32::consts::TAU;
                0.35 + 0.65 * (0.5 + 0.5 * phase.cos())
            } else {
                progress
            };
            let c = Color::new(base.r, base.g, base.b, (base.a as f32 * alpha).round() as u8);
            let cx = x + w / 2.0 + (i as f32 - 1.0) * DOT_GAP * sf;
            let paint = Paint::new().with_color(c).with_style(PaintStyle::Fill).with_anti_alias(true);
            canvas.draw_circle(cx, cy, DOT_D * sf / 2.0, &paint);
        }
    }
}
