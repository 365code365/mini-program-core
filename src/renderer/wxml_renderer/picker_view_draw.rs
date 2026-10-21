//! `<picker-view>` 的绘制：每列裁剪到自己的盒子，按滚轮位置把项沿纵向挪到选中项正对指示条；
//! 上下两片白色渐隐遮罩 + 指示条上下两根细线。带交互的那条路径还把每一列登记成纵向 swiper 候选
//! （手势仲裁见 `components::picker_view` 的模块注释）。
//!
//! 外观按微信默认样式（官方 picker-view 的 `mask-style` / `indicator-style` 未设时）：
//! 指示条高度 = 项高（第一项的布局高度，没有项时 34px），上下边框 1px #e5e5e5；
//! 遮罩是 `linear-gradient(180deg, rgba(255,255,255,.95), rgba(255,255,255,.6))`，下半片反向。
//! `indicator-style` 只认 `height`（其余属性不画），`mask-style` 未实现（按默认遮罩画）。

use super::*;
use crate::renderer::components::picker_view;
use crate::Paint;

/// 指示条高度（逻辑像素）：`indicator-style` 里写了 height 用它，否则用第一项的高
fn indicator_height(node: &RenderNode, item_h: f32) -> f32 {
    node.attrs
        .get("indicator-style")
        .and_then(|s| {
            s.split(';').find_map(|d| {
                let (k, v) = d.split_once(':')?;
                if k.trim() != "height" {
                    return None;
                }
                v.trim().trim_end_matches("px").trim().parse::<f32>().ok()
            })
        })
        .unwrap_or(item_h)
}

impl WxmlRenderer {
    /// 带交互的路径（顶层 / 子节点两种上下文都走这里）：画、登记列的手势候选、登记 picker-view 自己的绑定
    pub(super) fn draw_picker_view_interactive(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        rect: (f32, f32, f32, f32),
        interaction: &mut InteractionManager,
        kind: DrawKind,
    ) {
        self.draw_picker_view_with(canvas, taffy, node, rect, true, |r, c, item, ox, oy| {
            r.dispatch_draw(c, taffy, item, ox, oy, interaction, kind)
        });
        let logical = self.page_bounds(rect.0, rect.1, rect.2, rect.3);
        self.push_event_bindings(node, logical);
    }

    /// 列的几何（物理像素，相对 picker-view 左上角）与项高 / 项数
    fn picker_columns(taffy: &Tree, node: &RenderNode) -> Vec<(f32, f32, f32, f32, f32, usize)> {
        node.children
            .iter()
            .filter(|c| c.tag == "picker-view-column")
            .filter_map(|c| {
                let l = taffy.layout(c.taffy_node).ok()?;
                let item_h = c.children.first().and_then(|i| taffy.layout(i.taffy_node).ok()).map(|i| i.size.height).unwrap_or(0.0);
                Some((l.location.x, l.location.y, l.size.width, l.size.height, item_h, c.children.len()))
            })
            .collect()
    }

    /// 带交互 / 不带交互两条路径共用：`draw_item(self, canvas, 项, 列原点 x, 列原点 y)`
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_picker_view_with(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        (x, y, w, h): (f32, f32, f32, f32),
        register: bool,
        mut draw_item: impl FnMut(&mut Self, &mut Canvas, &RenderNode, f32, f32),
    ) {
        let sf = self.scale_factor;
        draw_background(canvas, &node.style, x, y, w, h);
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let logical = self.page_bounds(x, y, w, h);
        let id = Self::get_component_id(node, &logical);
        let cols = Self::picker_columns(taffy, node);
        let default_item = 34.0 * sf;
        let item_of = |c: &(f32, f32, f32, f32, f32, usize)| if c.4 > 0.5 { c.4 } else { default_item };
        let first_item = cols.first().map(item_of).unwrap_or(default_item);
        let ind_h = indicator_height(node, first_item / sf) * sf;
        let band_top = y + (h - ind_h) / 2.0;
        let value = node.attrs.get("value").map(|s| s.as_str()).unwrap_or("");
        let immediate = super::scroll_views::bool_attr(&node.attrs, "immediate-change");
        let spec: Vec<(usize, f32)> = cols.iter().map(|c| (c.5, item_of(c) / sf)).collect();
        let pos = if register {
            picker_view::sync(&id, value, immediate, &spec)
        } else {
            // 离屏 / 画廊：有状态用状态（与屏幕上一致），没有按 value 属性
            picker_view::positions(&id).filter(|p| p.len() == spec.len()).unwrap_or_else(|| {
                let v = crate::renderer::components::parse_attr_json(value);
                spec.iter()
                    .enumerate()
                    .map(|(i, (n, _))| {
                        let want = v.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0).max(0.0) as usize;
                        want.min(n.saturating_sub(1)) as f32
                    })
                    .collect()
            })
        };
        let columns: Vec<&RenderNode> = node.children.iter().filter(|c| c.tag == "picker-view-column").collect();
        for (i, (col, c)) in columns.iter().zip(cols.iter()).enumerate() {
            let (cx, cy, cw, ch) = (x + c.0, y + c.1, c.2, c.3);
            let item_h = item_of(c);
            let p = pos.get(i).copied().unwrap_or(0.0);
            canvas.save();
            canvas.clip_rect(GeoRect::new(cx, cy, cw, ch));
            draw_background(canvas, &col.style, cx, cy, cw, ch);
            let first_y = col.children.first().and_then(|f| taffy.layout(f.taffy_node).ok()).map(|l| l.location.y).unwrap_or(0.0);
            // 第 p 项的顶边正对指示条顶边
            let shift = band_top - cy - first_y - p * item_h + (ind_h - item_h) / 2.0;
            for item in &col.children {
                let Ok(l) = taffy.layout(item.taffy_node) else { continue };
                let top = cy + shift + l.location.y;
                if top + l.size.height < y || top > y + h {
                    continue;
                }
                draw_item(self, canvas, item, cx, cy + shift);
            }
            canvas.restore();
            if register {
                let key = picker_view::column_key(&id, i);
                let bounds = self.page_bounds(cx, cy, cw, ch);
                if !self.swiper_regions.iter().any(|r| r.id == key) {
                    self.swiper_regions.push(SwiperRegion { id: key, bounds, is_fixed: self.registering_fixed, vertical: true });
                }
            }
        }
        if register && picker_view::needs_frame() {
            self.mark_animating();
        }
        Self::draw_picker_mask(canvas, x, y, w, h, band_top, ind_h, sf);
    }

    /// 上下遮罩（逐行画渐变）与指示条两根线
    #[allow(clippy::too_many_arguments)]
    fn draw_picker_mask(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, band_top: f32, ind_h: f32, sf: f32) {
        let top_h = (band_top - y).max(0.0);
        let bottom_y = band_top + ind_h;
        let bottom_h = (y + h - bottom_y).max(0.0);
        let rows = |canvas: &mut Canvas, y0: f32, hh: f32, top_alpha: f32, bottom_alpha: f32| {
            let n = hh.ceil() as i32;
            for r in 0..n {
                let t = if n > 1 { r as f32 / (n - 1) as f32 } else { 0.0 };
                let a = top_alpha + (bottom_alpha - top_alpha) * t;
                let paint = Paint::new().with_color(Color::new(255, 255, 255, (a * 255.0).round() as u8));
                canvas.draw_rect(&GeoRect::new(x, y0 + r as f32, w, 1.0f32.min(y0 + hh - (y0 + r as f32))), &paint);
            }
        };
        rows(canvas, y, top_h, 0.95, 0.6);
        rows(canvas, bottom_y, bottom_h, 0.6, 0.95);
        let line = Paint::new().with_color(Color::from_hex(0xE5E5E5));
        let lw = (1.0 * sf).max(1.0);
        canvas.draw_rect(&GeoRect::new(x, band_top, w, lw), &line);
        canvas.draw_rect(&GeoRect::new(x, bottom_y - lw, w, lw), &line);
    }
}

impl WxmlRenderer {
    /// picker-view 的布局补丁（微信内置样式：`display:flex`，列 `flex:1`）。
    /// 建树时基础样式会盖掉组件 build 里设的布局，所以在 `layout::build_tree` 里补
    pub(super) fn apply_picker_view_layout(ts: &mut taffy::style::Style, children: &[RenderNode], ctx: &mut ComponentContext) {
        ts.flex_direction = FlexDirection::Row;
        ts.flex_wrap = FlexWrap::NoWrap;
        ts.align_items = Some(AlignItems::STRETCH);
        for child in children.iter().filter(|c| c.tag == "picker-view-column") {
            if let Ok(mut style) = ctx.taffy.style(child.taffy_node).cloned() {
                style.flex_grow = 1.0;
                style.flex_shrink = 1.0;
                style.flex_basis = length(0.0_f32);
                style.flex_direction = FlexDirection::Column;
                style.size.height = percent(1.0_f32);
                // 列高由 picker-view 定，不被项撑开（项超出部分靠绘制时裁剪）
                style.min_size.height = length(0.0_f32);
                style.overflow.y = taffy::style::Overflow::Hidden;
                ctx.taffy.set_style(child.taffy_node, style).ok();
            }
        }
    }
}
