//! 第二遍布局修正（reflow）。
//!
//! 有些约束只有**布局算完**才知道：一个文本盒实际拿到多宽、一个包含块实际多高、
//! 一个滚动容器有没有真的溢出父级。这里在第一遍 `compute_layout` 之后按实际几何
//! 修正样式，需要时再跑一遍布局。
//!
//! 这样做的好处是**自限**：规则只在「确实出问题」的节点上生效，
//! 不像建树期的猜测那样会波及全部节点。

use taffy::prelude::*;
use taffy::style::Position;

use super::layout::count_wrapped_lines;
use super::{Tree, WxmlRenderer};
use crate::renderer::components::{
    dim_is_auto, dim_length, dim_percent, lpa_length, FontWeight, RenderNode, WhiteSpace,
};

impl WxmlRenderer {
    /// 第二遍布局修正之一：**绝对定位元素的包含块高度塌成 0 时按视口折算**。
    ///
    /// CSS 规则：`position:absolute` 的包含块是最近的定位祖先，没有则是初始包含块（视口）。
    /// 建树期已经处理了「一个定位祖先都没有」的情况，但还有一类同样常见：
    /// 定位祖先存在，而它的**使用高度是 0** —— 典型就是整屏铺底的写法
    ///
    /// ```text
    /// .page { flex: 1; position: relative }        /* 子节点全是绝对定位 → 没有在流内容 → 高 0 */
    /// .background { position: absolute; inset: 0; width: 100%; height: 100% }
    /// ```
    ///
    /// 微信/Skyline 里页面根节点就是视口高，所以 `.page` 是满屏的；我们的布局根是
    /// `height: auto`（内容高驱动滚动），于是这类页面整屏都塌掉：闪屏页的背景图不见了、
    /// 文案全挤在顶部。这里在**布局之后**用实际使用高度判断，塌成 0 就退回视口高度。
    pub(super) fn correct_absolute_heights(
        &self,
        taffy: &mut Tree,
        nodes: &[RenderNode],
        viewport_h: f32,
    ) -> bool {
        let mut changed = false;
        self.fix_abs_in(taffy, nodes, viewport_h, &mut changed);
        changed
    }

    fn fix_abs_in(&self, taffy: &mut Tree, nodes: &[RenderNode], cb_h: f32, changed: &mut bool) {
        for node in nodes {
            // 该节点作为「包含块」时的高度：自己是定位元素且有实际高度才换参照物，
            // 高度为 0 说明它自己也没被撑开，继续沿用上层的参照物（最终是视口）。
            let used_h = taffy.layout(node.taffy_node).map(|l| l.size.height).unwrap_or(0.0);
            let child_cb = if node.style.is_positioned && used_h > 1.0 { used_h } else { cb_h };

            for child in &node.children {
                let Ok(st) = taffy.style(child.taffy_node).cloned() else { continue };
                if st.position != Position::Absolute {
                    continue;
                }
                // taffy 0.12 起这些尺寸值是位压缩结构、不能 match，用读值判据代替
                let target = if let Some(p) = dim_percent(st.size.height) {
                    // `height: 50%` → 按包含块折算
                    Some(child_cb * p)
                } else if dim_is_auto(st.size.height) {
                    // `top/bottom` 都给了而高度 auto：高度 = 包含块高 - top - bottom
                    match (lpa_length(st.inset.top), lpa_length(st.inset.bottom)) {
                        (Some(t), Some(b)) => Some((child_cb - t - b).max(0.0)),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(h) = target {
                    let cur = taffy.layout(child.taffy_node).map(|l| l.size.height).unwrap_or(0.0);
                    if (cur - h).abs() > 0.5 && h > 0.5 {
                        let mut st2 = st.clone();
                        st2.size.height = length(h);
                        taffy.set_style(child.taffy_node, st2).ok();
                        *changed = true;
                    }
                }
            }
            self.fix_abs_in(taffy, &node.children, child_cb, changed);
        }
    }

    /// 第二遍布局修正之二：按**实际拿到的几何**收拾溢出。
    ///
    /// 三件事，都只在真的溢出时才动手：
    /// 1. 文本盒比父级内容区还宽 → 收到父级宽度（浏览器里行内文本就在这儿折行）；
    /// 2. 换行后行数变多 → 补高度（原来的 `correct_wrapped_text_heights`）；
    /// 3. 滚动容器比父级内容区还高 → 解除它的自动最小高度，让弹性布局把它压回去。
    pub(super) fn correct_overflow(&self, taffy: &mut Tree, nodes: &[RenderNode]) -> bool {
        let mut changed = false;
        self.fix_overflow_in(taffy, nodes, None, &mut changed);
        changed
    }

    /// `parent_inner` 是父节点的内容区尺寸（物理像素）。根节点传 `None`。
    fn fix_overflow_in(
        &self,
        taffy: &mut Tree,
        nodes: &[RenderNode],
        parent_inner: Option<(f32, f32)>,
        changed: &mut bool,
    ) {
        for node in nodes {
            if node.tag == "text" && !node.text.is_empty() {
                self.fix_text_box(taffy, node, parent_inner, changed);
            } else if node.tag == "scroll-view" {
                Self::release_scroll_min_size(taffy, node, parent_inner, changed);
            }
            let inner = Self::content_box_of(taffy, node);
            self.fix_overflow_in(taffy, &node.children, inner, changed);
        }
    }

    /// 节点的内容区尺寸（减掉 border + padding），物理像素
    fn content_box_of(taffy: &Tree, node: &RenderNode) -> Option<(f32, f32)> {
        let l = taffy.layout(node.taffy_node).ok()?;
        // taffy 0.12 的 `content_box_*` 已经把 padding/border 减掉了
        let w = l.content_box_width();
        let h = l.content_box_height();
        Some((w.max(0.0), h.max(0.0)))
    }

    /// 文本盒的宽度收窄 + 行数补高。
    ///
    /// 靠内容定宽的行内文本（`text.rs` 里 `size.width = max-content`、`flex_shrink = 0`
    /// 的那条路径）在**纵向排布**的父级里会一路溢出到看不见 —— 一段长文案会渲染成
    /// 一整行横着冲出屏幕（tea-app 品牌页的「品牌故事」正文实测 1414px 宽、只有 1 行，
    /// 协议弹窗的正文 674px 宽）。浏览器里这段文字的宽度来自交叉轴（`align-items:stretch`），
    /// 到容器边就折行。
    ///
    /// 不在建树期改成「一律按容器宽换行」：那会把**每一个**短文本的盒子都撑成整宽，
    /// 带背景色/边框的标签、徽章会跟着变形。这里只收拾「确实比父级内容区还宽」的那些，
    /// 影响面严格限制在真正出问题的节点上。
    fn fix_text_box(
        &self,
        taffy: &mut Tree,
        node: &RenderNode,
        parent_inner: Option<(f32, f32)>,
        changed: &mut bool,
    ) {
        if matches!(node.style.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre) {
            return;
        }
        let Some(default_tr) = self.text_renderer.as_deref() else { return };
        // 与建树/绘制端同一族字体，否则「按 A 字体定的盒子」用 B 字体数行数
        let family = crate::text_family::renderer_for_family(node.style.font_family.as_deref());
        let tr = family.as_deref().unwrap_or(default_tr);
        let sf = self.scale_factor;

        let Ok(layout) = taffy.layout(node.taffy_node) else { return };
        let mut box_w = layout.size.width;
        let box_h = layout.size.height;
        let pl = node.style.padding_left * sf;
        let pr = node.style.padding_right * sf;
        let pt = node.style.padding_top * sf;
        let pb = node.style.padding_bottom * sf;

        // 1) 比父级内容区还宽 → 收到父级宽度。只对「靠内容定宽」的文本盒生效：
        //    走 taffy 文本度量的那条路径（width 仍是 auto）本来就会按可用宽换行。
        let mut narrowed = None;
        if let Some((avail_w, _)) = parent_inner {
            let self_fixed_width = taffy
                .style(node.taffy_node)
                .ok()
                .map(|st| dim_length(st.size.width).is_some())
                .unwrap_or(false);
            if self_fixed_width && avail_w > 1.0 && box_w > avail_w + 0.5 {
                box_w = avail_w;
                narrowed = Some(avail_w);
            }
        }

        let avail = (box_w - pl - pr).max(1.0);
        let size = node.style.font_size * sf;
        let ls = node.style.letter_spacing * sf;
        let line_height = node
            .style
            .line_height
            .map(|lh| lh * sf)
            .unwrap_or_else(|| tr.natural_line_height_for(&node.text, size))
            .max(size);
        let bold = matches!(
            node.style.font_weight,
            FontWeight::Bold | FontWeight::W600 | FontWeight::W700 | FontWeight::W800 | FontWeight::W900
        ) && tr.has_bold_face();
        let lines = count_wrapped_lines(tr, &node.text, avail, size, ls, bold);
        let needed_h = lines as f32 * line_height + pt + pb;

        if narrowed.is_none() && needed_h <= box_h + 0.5 {
            return;
        }
        let Ok(mut st) = taffy.style(node.taffy_node).cloned() else { return };
        if let Some(w) = narrowed {
            st.size.width = length(w);
            // 自动最小宽度（`min-width:auto` = min-content）会把上面这条压回去 ——
            // 定宽叶子的 min-content 就等于它的 max-content。
            st.min_size.width = length(w);
        }
        if needed_h > box_h + 0.5 || narrowed.is_some() {
            st.size.height = length(needed_h);
            st.min_size.height = length(needed_h);
        }
        taffy.set_style(node.taffy_node, st).ok();
        *changed = true;
    }

    /// 滚动容器溢出父级时解除它的自动最小尺寸（CSS 的 scroll container 语义）。
    ///
    /// CSS 里滚动容器的**自动最小尺寸为 0**，所以 `flex:1` 的滚动区会被压到分配给它的
    /// 那点空间、内容自己滚。我们给 `scroll-view` 保留 `Overflow::Visible`（换成
    /// `Overflow::Scroll` 会让页面画布高塌掉，见 `layout.rs` 里的注释），代价就是它的
    /// 自动最小高度仍按内容算 —— 于是 `height:78%` 的弹窗里 `flex:1` 的正文区被内容
    /// 撑成 732px、把「我已阅读并同意」按钮顶到弹窗外面，而且完全不可滚。
    ///
    /// 这里不做建树期的「祖先高度是否确定」推断，直接看**实际有没有溢出父级内容区**：
    /// - 弹窗里的 `flex:1` 正文区：父级高 520、自己 732 → 触发，解除后弹性布局给它 362；
    /// - 整页那层 `<scroll-view class="page">`：父级本身就是内容高，自己 == 父级 → 不触发，
    ///   页面画布高与页面级滚动的现有行为完全不变。
    fn release_scroll_min_size(
        taffy: &mut Tree,
        node: &RenderNode,
        parent_inner: Option<(f32, f32)>,
        changed: &mut bool,
    ) {
        let Some((_, avail_h)) = parent_inner else { return };
        let Ok(layout) = taffy.layout(node.taffy_node) else { return };
        if avail_h <= 1.0 || layout.size.height <= avail_h + 0.5 {
            return;
        }
        let Ok(mut st) = taffy.style(node.taffy_node).cloned() else { return };
        // 只在「高度本来就该由父级分配」时解除；自己写了确定高度的滚动区不动
        if !dim_is_auto(st.size.height) {
            return;
        }
        if dim_length(st.min_size.height) == Some(0.0) {
            return; // 已经解除过，别重复触发 relayout
        }
        st.min_size.height = length(0.0_f32);
        taffy.set_style(node.taffy_node, st).ok();
        *changed = true;
    }
}
