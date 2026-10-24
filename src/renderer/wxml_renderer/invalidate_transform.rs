//! setData 改到带静态 `transform` 的子树时的失效范围（worklet / 手势每帧改 `style="transform:…"`）。
//!
//! 从前这种子树一律整帧：失效分析只认布局盒，而 transform 子树画在别处。skyline 演示页拖一下
//! pan 手势的小圆点，之后每一帧 `__nodeStyles` 都整条带重画 + 整张重传（PLY110 上一帧 14~16ms）。
//!
//! 这里按**绘制路径同一套几何**算出旧、新两次实际画到的范围（`animation.rs::draw_with_transform`）：
//! - 纯平移：子树整体按 (tx, ty) 偏移画 → 子树各节点框（含阴影）的并集平移；
//! - 缩放 / 旋转 / 倾斜：离屏 `盒子 + pad` 经仿射贴回 → 那块离屏的变换后包围盒再放 2px（双线性）。
//!
//! 拿不准的照旧整帧：子树里还有 transform / 动画 / fixed / scroll-view / swiper，或节点自己在跑
//! CSS / `wx.createAnimation` 动画（绘制时的 transform 不是 style 里这份）。

use super::invalidate::DiffCtx;
use super::*;

/// 双线性采样会往外多带的像素（与 `compose::blit_transformed` 的取整对齐，留宽一点）
const SAMPLE_MARGIN: f32 = 2.0;

impl WxmlRenderer {
    /// 这一对节点能不能按「transform 子树」单独算范围
    pub(super) fn transform_diffable(old: &RenderNode, new: &RenderNode) -> bool {
        let plain = |n: &RenderNode| {
            n.style.animation.is_none()
                && !n.attrs.contains_key("animation")
                && !matches!(n.tag.as_str(), "scroll-view" | "swiper")
        };
        (old.style.transform.is_some() || new.style.transform.is_some()) && plain(old) && plain(new)
    }

    /// transform 子树里有没有变化；有的话把旧、新两次画到的范围记进去，算不出就整帧
    #[allow(clippy::too_many_arguments)]
    pub(super) fn diff_transform_subtree(
        old_t: &dyn super::layout_snapshot::Layouts,
        new_t: &dyn super::layout_snapshot::Layouts,
        old: &RenderNode,
        new: &RenderNode,
        (oox, ooy): (f32, f32),
        (nox, noy): (f32, f32),
        ctx: &mut DiffCtx,
    ) {
        if !Self::subtree_differs(old_t, new_t, old, new, (oox, ooy), (nox, noy), ctx.moved) {
            return;
        }
        let painted = |t: &dyn super::layout_snapshot::Layouts, n: &RenderNode, ox: f32, oy: f32| Self::transform_painted(t, n, ox, oy);
        match (painted(old_t, old, oox, ooy), painted(new_t, new, nox, noy)) {
            (Some(a), Some(b)) => {
                ctx.add(a);
                ctx.add(b);
            }
            _ => {
                ctx.bail = true;
                ctx.bail_why = format!(
                    "<{}{}> transform 子树里有拿不准画在哪的节点",
                    new.tag,
                    new.attrs.get("class").map(|c| format!(" class={}", c)).unwrap_or_default()
                );
            }
        }
    }

    /// 带 transform 的节点（连同子树）实际画到的范围（画布物理像素）。None = 拿不准
    fn transform_painted(t: &dyn super::layout_snapshot::Layouts, node: &RenderNode, ox: f32, oy: f32) -> Option<GeoRect> {
        let l = t.layout(node.taffy_node).ok()?;
        let (x, y, w, h) = (ox + l.location.x, oy + l.location.y, l.size.width, l.size.height);
        let mut extent = super::invalidate_grow::inflate(GeoRect::new(x, y, w, h), &node.style);
        for c in &node.children {
            extent = union_rect(extent, Self::plain_extent(t, c, x, y)?);
        }
        let Some(tf) = node.style.transform else { return Some(extent) };
        if crate::renderer::compose::is_identity(&tf) {
            return Some(extent);
        }
        if crate::renderer::compose::is_translate_only(&tf) {
            return Some(GeoRect::new(extent.x + tf.translate_x, extent.y + tf.translate_y, extent.width, extent.height));
        }
        let pad = crate::renderer::compose::transform_padding(w, h, &tf);
        let (tw, th) = ((w + pad * 2.0).ceil(), (h + pad * 2.0).ceil());
        if w <= 0.0 || h <= 0.0 || tw > 4096.0 || th > 4096.0 {
            // 绘制路径在这种尺寸下退回不变换绘制（draw_with_transform）
            return Some(extent);
        }
        let (x0, y0, x1, y1) = crate::renderer::compose::transformed_bounds(tw, th, (x - pad, y - pad), (x + w / 2.0, y + h / 2.0), &tf);
        let m = SAMPLE_MARGIN;
        Some(GeoRect::new(x0 - m, y0 - m, x1 - x0 + m * 2.0, y1 - y0 + m * 2.0))
    }

    /// transform 子树里普通节点框（含阴影）的并集；遇到自己会另画在别处的节点返回 None
    fn plain_extent(t: &dyn super::layout_snapshot::Layouts, node: &RenderNode, px: f32, py: f32) -> Option<GeoRect> {
        let s = &node.style;
        if s.transform.is_some() || s.animation.is_some() || s.is_fixed || node.attrs.contains_key("animation") {
            return None;
        }
        if matches!(node.tag.as_str(), "scroll-view" | "swiper") {
            return None;
        }
        let l = t.layout(node.taffy_node).ok()?;
        let (x, y) = (px + l.location.x, py + l.location.y);
        let mut r = super::invalidate_grow::inflate(GeoRect::new(x, y, l.size.width, l.size.height), s);
        for c in &node.children {
            r = union_rect(r, Self::plain_extent(t, c, x, y)?);
        }
        Some(r)
    }
}

fn union_rect(a: GeoRect, b: GeoRect) -> GeoRect {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    let (x1, y1) = ((a.x + a.width).max(b.x + b.width), (a.y + a.height).max(b.y + b.height));
    GeoRect::new(x0, y0, x1 - x0, y1 - y0)
}
