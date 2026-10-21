//! swiper 的绘制：裁剪到自身盒子 → 按当前（浮点）页沿主轴偏移 → 只画与盒子相交的项 →
//! 指示点。三条绘制路径（带交互、无交互的画廊 / 离屏、离屏缓存里的 scroll-view 内容）
//! 都走这里，分页语义只有一份（从前只有带交互那条分页，画廊里的 swiper 只是「第一屏
//! 正好在可见区」，见 doc/引擎测试说明.md G-13）。
//!
//! 衔接（`circular`）模式下项可以出现在「虚拟下标」处：第 -1 项画的是最后一项、第 total 项
//! 画的是第一项，于是从末页往首页走是连续的。项在 taffy 里的位置是 `内边距 + 下标 × 项宽`，
//! 画到虚拟下标 `v` 就是在原位置上再挪 `(v − 下标 − 当前位置) × 项宽`。
//!
//! 每帧还登记本帧画到的 swiper 盒子（逻辑像素），宿主的手势仲裁据此把横划交给它（`host::input`）。

use super::*;
use crate::renderer::components::{swiper_effects, swiper_pages, SwiperConfig, SWIPER_MANAGER};
// taffy 0.12 的尺寸值是位压缩结构，读值走这几个封装（见 components::base）
use crate::renderer::components::dim_is_auto;

/// 本帧画到的一个 swiper（手势候选）
#[derive(Debug, Clone)]
pub struct SwiperRegion {
    /// 组件 id（也是全局状态表与事件绑定的键）
    pub id: String,
    /// 逻辑像素：正常流是内容坐标，fixed 是视口坐标
    pub bounds: GeoRect,
    pub is_fixed: bool,
    pub vertical: bool,
}

/// 与盒子相交的项：(真实下标, 虚拟下标)
fn visible_items(pos: f32, pad: f32, item: f32, view: f32, total: usize, circular: bool) -> Vec<(usize, i64)> {
    if total == 0 || item <= 0.0 {
        return Vec::new();
    }
    let lo = (pos - pad / item).floor() as i64 - 1;
    let hi = (pos + (view - pad) / item).ceil() as i64 + 1;
    let mut out = Vec::new();
    for v in lo..=hi {
        let left = pad + (v as f32 - pos) * item;
        if left + item <= 0.5 || left >= view - 0.5 {
            continue;
        }
        let real = if circular {
            v.rem_euclid(total as i64) as usize
        } else if v >= 0 && (v as usize) < total {
            v as usize
        } else {
            continue;
        };
        out.push((real, v));
    }
    out
}

impl WxmlRenderer {
    /// layout-type 的绘制计划：(带变换的项副本, 原点 x, 原点 y)，按层级从下到上。
    /// 项自己写了 transform 的不叠加效果（保留页面的变换）
    fn swiper_effect_plan(node: &RenderNode, kind: swiper_effects::Kind, pos: f32, item: f32, (x, y): (f32, f32), cfg: &SwiperConfig) -> Vec<(RenderNode, f32, f32)> {
        let mut list: Vec<_> = swiper_effects::items(pos, cfg.total, cfg.circular)
            .into_iter()
            .filter_map(|(real, v)| swiper_effects::fx(kind, v as f32 - pos, item, cfg.vertical).map(|f| (real, v, f)))
            .collect();
        list.sort_by_key(|(_, _, f)| f.z);
        list.into_iter()
            .filter_map(|(real, v, f)| {
                let child = node.children.get(real)?;
                let shift = (v as f32 - real as f32 - pos) * item;
                let (ox, oy) = if cfg.vertical { (x, y + shift) } else { (x + shift, y) };
                let mut c = child.clone();
                if c.style.transform.is_none() {
                    c.style.transform = Some(f.transform);
                }
                c.style.opacity *= f.opacity;
                Some((c, ox, oy))
            })
            .collect()
    }

    /// 本帧画到的 swiper 里，包含这个点的最内层那个（面积最小）。
    /// `y` 对正常流是内容坐标，对覆盖层是视口坐标
    pub fn swiper_at(&self, x: f32, y: f32, on_fixed: bool) -> Option<&SwiperRegion> {
        self.swiper_regions
            .iter()
            .filter(|r| {
                let b = &r.bounds;
                r.is_fixed == on_fixed && x >= b.x && x < b.x + b.width && y >= b.y && y < b.y + b.height
            })
            .min_by(|a, b| (a.bounds.width * a.bounds.height).total_cmp(&(b.bounds.width * b.bounds.height)))
    }

    /// 第一项的尺寸与它在容器里的起点（主轴，物理像素）：项宽与前边距都由布局算好了
    fn swiper_item_geometry(taffy: &Tree, node: &RenderNode, vertical: bool) -> (f32, f32) {
        let Some(first) = node.children.first() else { return (0.0, 0.0) };
        let Ok(l) = taffy.layout(first.taffy_node) else { return (0.0, 0.0) };
        if vertical {
            (l.size.height, l.location.y)
        } else {
            (l.size.width, l.location.x)
        }
    }

    /// 带交互的绘制：推进状态机（受控 current、拿到当前位置）、登记手势候选
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_swiper_container(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        interaction: &mut InteractionManager,
        kind: DrawKind,
    ) {
        let sf = self.scale_factor;
        draw_background(canvas, &node.style, x, y, w, h);
        if node.children.is_empty() || w <= 0.0 || h <= 0.0 {
            return;
        }
        let cfg = SwiperComponent::config(node);
        let logical = self.page_bounds(x, y, w, h);
        let id = Self::get_component_id(node, &logical);
        let (item, pad) = Self::swiper_item_geometry(taffy, node, cfg.vertical);
        let now = crate::renderer::components::swiper_clock_ms();
        let (pos, active, current) = SWIPER_MANAGER
            .lock()
            .map(|mut m| m.sync(&id, &cfg, item / sf, now))
            .unwrap_or((0.0, false, 0));
        if active {
            self.mark_animating();
        }
        if !self.swiper_regions.iter().any(|r| r.id == id) {
            self.swiper_regions.push(SwiperRegion { id, bounds: logical, is_fixed: self.registering_fixed, vertical: cfg.vertical });
        }
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        let view = if cfg.vertical { h } else { w };
        if let Some(fx) = swiper_effects::kind(node) {
            // Skyline layout-type / transformer-type：每项带变换与不透明度，按层级先后画
            for (child, ox, oy) in Self::swiper_effect_plan(node, fx, pos, item, (x, y), &cfg) {
                self.dispatch_draw(canvas, taffy, &child, ox, oy, interaction, kind);
            }
        } else {
            for (real, v) in visible_items(pos, pad, item, view, cfg.total, cfg.circular) {
                let shift = (v as f32 - real as f32 - pos) * item;
                let (ox, oy) = if cfg.vertical { (x, y + shift) } else { (x + shift, y) };
                if let Some(child) = node.children.get(real) {
                    self.dispatch_draw(canvas, taffy, child, ox, oy, interaction, kind);
                }
            }
        }
        canvas.restore();
        SwiperComponent::draw_indicators(node, canvas, x, y, w, h, sf, current, swiper_pages(&cfg), pos);
        // swiper 自己身上的绑定（bindchange / bindtap …）：从前这条路径提前 return，
        // 从来没登记过 —— swiper 上的事件一个都发不出去
        self.push_event_bindings(node, logical);
    }

    /// 没有交互上下文时（画廊、离屏度量、离屏缓存里的 scroll-view 内容）：同样分页、同样画指示点。
    /// 状态表里有这个 swiper 就用它当前的位置（缓存内容与屏幕上一致），否则用 `current` 属性
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_swiper_plain(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        (x, y, w, h): (f32, f32, f32, f32),
        mut draw_item: impl FnMut(&mut Self, &mut Canvas, &RenderNode, f32, f32),
    ) {
        if node.children.is_empty() || w <= 0.0 || h <= 0.0 {
            return;
        }
        let sf = self.scale_factor;
        let cfg: SwiperConfig = SwiperComponent::config(node);
        let logical = self.page_bounds(x, y, w, h);
        let id = Self::get_component_id(node, &logical);
        let pages = swiper_pages(&cfg);
        let (pos, current) = SWIPER_MANAGER
            .lock()
            .ok()
            .and_then(|m| m.get(&id).map(|s| (s.pos, s.current)))
            .unwrap_or_else(|| {
                let c = cfg.current_attr.unwrap_or(0).min(pages - 1);
                (c as f32, c)
            });
        let (item, pad) = Self::swiper_item_geometry(taffy, node, cfg.vertical);
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        let view = if cfg.vertical { h } else { w };
        if let Some(fx) = swiper_effects::kind(node) {
            for (child, ox, oy) in Self::swiper_effect_plan(node, fx, pos, item, (x, y), &cfg) {
                draw_item(self, canvas, &child, ox, oy);
            }
        } else {
            for (real, v) in visible_items(pos, pad, item, view, cfg.total, cfg.circular) {
                let shift = (v as f32 - real as f32 - pos) * item;
                let (ox, oy) = if cfg.vertical { (x, y + shift) } else { (x + shift, y) };
                if let Some(child) = node.children.get(real) {
                    draw_item(self, canvas, child, ox, oy);
                }
            }
        }
        canvas.restore();
        SwiperComponent::draw_indicators(node, canvas, x, y, w, h, sf, current, pages, pos);
    }
}

#[cfg(test)]
mod tests {
    use super::visible_items;

    #[test]
    fn 静止时只画当前一屏() {
        assert_eq!(visible_items(2.0, 0.0, 100.0, 100.0, 5, false), vec![(2, 2)]);
        assert_eq!(visible_items(0.0, 0.0, 50.0, 100.0, 5, false), vec![(0, 0), (1, 1)], "一屏两项");
    }

    #[test]
    fn 边距露出相邻项_衔接模式两端连着() {
        // 前后各 20px 边距、项宽 60：当前项两侧各露出 20px 的邻项
        assert_eq!(visible_items(0.0, 20.0, 60.0, 100.0, 3, true), vec![(2, -1), (0, 0), (1, 1)]);
        assert_eq!(visible_items(0.0, 20.0, 60.0, 100.0, 3, false), vec![(0, 0), (1, 1)], "非衔接：第一项前面没有");
        // 从末页往首页滑到一半：末项与首项各占一半
        assert_eq!(visible_items(2.5, 0.0, 100.0, 100.0, 3, true), vec![(2, 2), (0, 3)]);
    }
}

impl WxmlRenderer {
    /// swiper 容器的布局补丁：方向、一屏几项、两侧边距（previous-margin / next-margin 落成主轴内边距，
    /// item 按内容盒的 1/n 排 —— 相邻项露出一截由布局算出来，绘制只管偏移）。
    /// 建树时基础样式会盖掉组件 build 里设的布局，所以在 `layout::build_tree` 里再补一次。
    ///
    /// 从 `layout.rs` 原样搬来（纯搬迁：那个文件超了 500 行）
    pub(super) fn apply_swiper_container_layout(
        &self,
        node: &WxmlNode,
        ts: &mut taffy::style::Style,
        children: &[RenderNode],
        ctx: &mut ComponentContext,
    ) {
        // 方向、一屏几项、两侧边距（previous-margin / next-margin 落成主轴内边距，
        // item 按内容盒的 1/n 排 —— 相邻项露出一截由布局算出来，绘制只管偏移）
        let (vertical, per_view, m0, m1) =
            crate::renderer::components::swiper_layout_params(&node.attributes, self.screen_width);
        let sfp = ctx.scale_factor;
        if m0 > 0.0 || m1 > 0.0 {
            let lp = |v: f32| taffy::style::LengthPercentage::length(v * sfp);
            if vertical {
                ts.padding.top = lp(m0);
                ts.padding.bottom = lp(m1);
            } else {
                ts.padding.left = lp(m0);
                ts.padding.right = lp(m1);
            }
        }
        let share = 1.0 / per_view as f32;
        ts.flex_direction = if vertical { FlexDirection::Column } else { FlexDirection::Row };
        ts.flex_wrap = FlexWrap::NoWrap;
        if dim_is_auto(ts.size.width) {
            ts.size.width = percent(1.0_f32);
        }
        if dim_is_auto(ts.size.height) {
            ts.size.height = length(SwiperComponent::DEFAULT_HEIGHT * ctx.scale_factor);
        }
        // 每个 item 占一屏（或 1/n 屏）且不收缩（对齐 HTML .wx-swiper-item{flex:0 0 100%}）
        for child in children {
            if let Ok(mut style) = ctx.taffy.style(child.taffy_node).cloned() {
                style.size = if vertical {
                    taffy::geometry::Size { width: percent(1.0_f32), height: percent(share) }
                } else {
                    taffy::geometry::Size { width: percent(share), height: percent(1.0_f32) }
                };
                // 纵向时 min-width 保持 100%（与从前一致），主轴方向再按 1/n 兜住
                style.min_size.width = percent(if vertical { 1.0 } else { share });
                if vertical {
                    style.min_size.height = percent(share);
                }
                style.flex_shrink = 0.0;
                style.flex_grow = 0.0;
                ctx.taffy.set_style(child.taffy_node, style).ok();
            }
        }
    }
}
