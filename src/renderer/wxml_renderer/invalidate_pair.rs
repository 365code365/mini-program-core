//! 失效分析里「哪个旧孩子对哪个新孩子」：按位置配对，两条例外。
//!
//! 1. **fixed 的孩子单独配**。fixed 元素画在覆盖层里、不占文档流：「节点与滚动」页滚过 200px 才
//!    出现的悬浮按钮（`wx:if` 挂在页面根上），从前一出现就是「根节点数变化 → 整帧」，滑过那一下必顿。
//!    现在页面上的孩子照常配对，fixed 的那几个只比「变没变」，变了就是覆盖层重画。
//!    前提是这串兄弟里没有谁会被 `:first-child` / `:last-child` / `:nth-*` 这类按位置选的规则选中
//!    （`StyleSheet::positional_subjects`）：fixed 孩子一增一减，别的兄弟序号 / 兄弟数跟着变，样式
//!    可能就变了，而失效分析判样式只看 `attrs`。只按「规则里写的类名 / 标签」判，示例 app.wxss 里的
//!    `.entry:last-child` 不会让所有页面都分不了。
//! 2. **内容变高时**孩子个数可以不同：配到第一个标签对不上为止，剩下的新旧两段各走一遍取切线
//!    （invalidate_grow.rs）。
use super::invalidate::DiffCtx;
use super::layout_incremental::resolve;
use super::layout_snapshot::Layouts;
use super::*;

impl WxmlRenderer {
    /// 比对一个父节点的两份孩子列表。`old_o` / `new_o` 是父节点在旧 / 新树里的累计原点，
    /// `in_fixed` / `old_fixed` 是父节点在新 / 旧树里是否已经在覆盖层里
    #[allow(clippy::too_many_arguments)]
    pub(super) fn diff_children(
        old_t: &dyn Layouts,
        new_t: &dyn Layouts,
        olds: &[RenderNode],
        news: &[RenderNode],
        old_o: (f32, f32),
        new_o: (f32, f32),
        in_fixed: bool,
        old_fixed: bool,
        ctx: &mut DiffCtx,
    ) {
        // fixed 的孩子能单独配的前提：这串兄弟里没有一个会被按位置选的规则选中
        // （fixed 孩子一增一减，它们的序号 / 兄弟数跟着变，样式可能就变了，而这里判样式只看 attrs）
        let has_fixed = olds.iter().chain(news).any(|c| c.style.is_fixed);
        let split = has_fixed
            && !ctx.positional.any
            && !olds.iter().chain(news).any(|c| !c.style.is_fixed && ctx.positional.may_match(&c.tag, c.attrs.get("class").map(String::as_str)));
        if !split {
            Self::diff_list(old_t, new_t, olds.iter(), news.iter(), old_o, new_o, in_fixed, old_fixed, ctx);
            return;
        }
        Self::diff_list(
            old_t,
            new_t,
            olds.iter().filter(|c| !c.style.is_fixed),
            news.iter().filter(|c| !c.style.is_fixed),
            old_o,
            new_o,
            in_fixed,
            old_fixed,
            ctx,
        );
        if ctx.bail || ctx.fixed_changed {
            return;
        }
        let (fo, fnew) = (olds.iter().filter(|c| c.style.is_fixed), news.iter().filter(|c| c.style.is_fixed));
        if fo.clone().count() != fnew.clone().count()
            || fo.zip(fnew).any(|(o, n)| Self::subtree_differs(old_t, new_t, o, n, old_o, new_o, ctx.moved))
        {
            ctx.fixed_changed = true;
        }
    }

    /// 按位置配对（个数不同时：内容变高就把配不上的两段各走一遍，否则整帧）
    #[allow(clippy::too_many_arguments)]
    fn diff_list<'a>(
        old_t: &dyn Layouts,
        new_t: &dyn Layouts,
        olds: impl Iterator<Item = &'a RenderNode> + Clone,
        news: impl Iterator<Item = &'a RenderNode> + Clone,
        old_o: (f32, f32),
        new_o: (f32, f32),
        in_fixed: bool,
        old_fixed: bool,
        ctx: &mut DiffCtx,
    ) {
        let (no, nn) = (olds.clone().count(), news.clone().count());
        if no != nn && !ctx.grow {
            // 孩子多了 / 少了一个（`wx:if` 切换选中条、角标）而父节点的盒子没变：新旧两串孩子各自整棵画到哪
            // 就重画哪（布局框 + 阴影外扩）。从前一律整帧：外卖店铺页滚动时左侧分类的高亮条一换（孩子 2 -> 1），
            // 一加上那一帧整页重画 31~43ms。按位置选的规则（:first-child 等）可能因此改了别的兄弟的样式、
            // 子树里有画法≠布局的（scroll-view / swiper / 变换 / 动画）时照旧整帧
            if in_fixed || old_fixed {
                ctx.fixed_changed = true;
                return;
            }
            let positional = olds
                .clone()
                .chain(news.clone())
                .any(|c| ctx.positional.may_match(&c.tag, c.attrs.get("class").map(String::as_str)));
            let moved = ctx.moved;
            let mut acc: Option<GeoRect> = None;
            let ok = !positional
                && olds.clone().all(|o| Self::subtree_paint_bounds(old_t, resolve(o, moved), old_o, moved, &mut acc))
                && news.clone().all(|n| Self::subtree_paint_bounds(new_t, n, new_o, moved, &mut acc));
            if !ok {
                ctx.bail = true;
                ctx.bail_why = format!("结构变化：孩子 {no} 个 -> {nn} 个");
                return;
            }
            if let Some(r) = acc {
                ctx.add(r);
            }
            return;
        }
        let mut paired = 0;
        let moved = ctx.moved;
        for (o, n) in olds.clone().zip(news.clone()) {
            let o = resolve(o, moved);
            if ctx.grow && o.tag != n.tag {
                break;
            }
            Self::diff_node(old_t, new_t, o, n, old_o.0, old_o.1, new_o.0, new_o.1, in_fixed, ctx);
            if ctx.bail {
                return;
            }
            paired += 1;
        }
        if ctx.grow {
            for o in olds.skip(paired) {
                Self::tail_cut(old_t, o, old_o.0, old_o.1, old_fixed, ctx);
            }
            for n in news.skip(paired) {
                Self::tail_cut(new_t, n, new_o.0, new_o.1, in_fixed, ctx);
            }
        }
    }

    /// 一棵子树画出来盖到的范围（每个节点的布局框按阴影外扩，并进 `acc`）。
    /// 碰到画法≠布局的节点（scroll-view / swiper / 变换 / 动画 / fixed）返回 false：那种算不准，调用方整帧
    fn subtree_paint_bounds(t: &dyn Layouts, n: &RenderNode, origin: (f32, f32), moved: &HashMap<taffy::NodeId, &RenderNode>, acc: &mut Option<GeoRect>) -> bool {
        let n = resolve(n, moved);
        if matches!(n.tag.as_str(), "scroll-view" | "swiper") || n.style.transform.is_some() || n.style.animation.is_some() || n.style.is_fixed {
            return false;
        }
        let Ok(l) = t.layout(n.taffy_node) else { return false };
        let o = (origin.0 + l.location.x, origin.1 + l.location.y);
        let r = super::invalidate_grow::inflate(GeoRect::new(o.0, o.1, l.size.width, l.size.height), &n.style);
        *acc = Some(match *acc {
            None => r,
            Some(a) => {
                let (x0, y0) = (a.x.min(r.x), a.y.min(r.y));
                let (x1, y1) = ((a.x + a.width).max(r.x + r.width), (a.y + a.height).max(r.y + r.height));
                GeoRect::new(x0, y0, x1 - x0, y1 - y0)
            }
        });
        n.children.iter().all(|c| Self::subtree_paint_bounds(t, c, o, moved, acc))
    }

    /// 两棵子树画出来会不会不一样（结构、文字、属性、位置尺寸任何一样不同就算）
    #[allow(clippy::too_many_arguments)]
    pub(super) fn subtree_differs(old_t: &dyn Layouts, new_t: &dyn Layouts, o: &RenderNode, n: &RenderNode, old_o: (f32, f32), new_o: (f32, f32), moved: &HashMap<taffy::NodeId, &RenderNode>) -> bool {
        let o = resolve(o, moved);
        if o.tag != n.tag || o.text != n.text || o.attrs != n.attrs || o.children.len() != n.children.len() {
            return true;
        }
        let (Ok(ol), Ok(nl)) = (old_t.layout(o.taffy_node), new_t.layout(n.taffy_node)) else { return true };
        let oo = (old_o.0 + ol.location.x, old_o.1 + ol.location.y);
        let no = (new_o.0 + nl.location.x, new_o.1 + nl.location.y);
        let near = |a: f32, b: f32| (a - b).abs() <= 0.01;
        if !(near(oo.0, no.0) && near(oo.1, no.1) && near(ol.size.width, nl.size.width) && near(ol.size.height, nl.size.height)) {
            return true;
        }
        o.children.iter().zip(&n.children).any(|(a, b)| Self::subtree_differs(old_t, new_t, a, b, oo, no, moved))
    }
}
