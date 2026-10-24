//! setData 之后**内容变高**时的失效范围：从最高的那处变化往下，整宽重画到内容底。
//!
//! 典型场景是列表 `onReachBottom` 追加几条：从前「内容高变了 → 整帧」，宿主换一块画布、整条带重画、
//! 整张重新上传 —— 一加 PLY110 上「节点与滚动」页每加载一次就是一个 14~38ms 的长帧，正好落在手指
//! 往上推的那一下（用户说的「列表往上滑顿一下」）。可变化其实只在旧内容的底边附近：新的几条接在
//! 后面，列表容器和它的祖先只是变高了，上面几屏一个像素没动。
//!
//! 规则（宁多画、不少画）：
//! - 孩子按位置配对，配到第一个标签对不上的地方为止；后面那段新旧两边各自整棵走一遍（新的要画、
//!   旧的要擦），取里面最高的节点顶边（这段里有 scroll-view / swiper / transform / 动画这种绘制
//!   坐标≠布局坐标的，整帧）。「节点与滚动」页的 `onReachBottom` 把新条目插在「底部提示」和悬浮
//!   按钮**前面**，按位置配对时后面几个全错位，就落在这一条里；
//! - 文本 / 属性变了、或者位置 / 宽度变了：取新旧两个框里靠上的顶边；
//! - **只有高度变了**的普通容器（view / block / 匿名宿主，纯色背景、四边同色、不透明）：它在旧底边
//!   往上「圆角 + 边框 + 阴影」那一截以上的像素与高度无关，取那一截的上沿；其余一律取顶边
//!   （渐变按整高铺、四边异色的边框按对角线分色，高度一变整块都变）。
//!
//! 出来的矩形是整宽、从切线到内容底；宿主再夹进它的裁剪窗（`engine_damage.rs`），太大就退回整条带。
//! 内容变**矮**不走这里（旧内容底下那截要擦，而且宿主要换更矮的画布），照旧整帧。
use super::invalidate::DiffCtx;
use super::*;

impl DiffCtx<'_> {
    /// 记一条切线：这一行往下都要重画。`node` 只用于诊断（`MINI_LAYOUT_LOG` 打出切线是谁定的）
    pub(super) fn cut_at(&mut self, y: f32, node: &RenderNode) {
        if self.cut.is_none_or(|c| y < c) {
            self.cut = Some(y);
            self.bail_why = format!("<{}{}>", node.tag, node.attrs.get("class").map(|c| format!(" class={c}")).unwrap_or_default());
        }
    }
}

/// 盒子外面还会画到多远：阴影加几个像素的余量（字形、抗锯齿出框）
fn overhang(style: &NodeStyle) -> f32 {
    shadow_reach(style) + 4.0
}

/// 阴影往框外画出去多远（四个方向取同一个上界；没有阴影为 0）。
///
/// 与 `components::shadow::draw_outer_shadow` 同一个单位（阴影参数解析出来就直接拿去画，两边都不乘
/// 像素比）：形状外扩 spread，掩膜四周再多 `3r + 2`（r ≈ 0.47·blur，三遍盒式模糊），再平移 offset。
/// 这里按 2·blur 算足，多画几行不要紧，少画一行就留残影
fn shadow_reach(style: &NodeStyle) -> f32 {
    style
        .box_shadow
        .filter(|s| !s.inset)
        .map(|s| s.blur.abs() * 2.0 + s.spread.abs() + s.offset_x.abs().max(s.offset_y.abs()) + 3.0)
        .unwrap_or(0.0)
}

/// 普通（内容高没变的）失效分析收的框：有阴影的盒子把阴影那圈也算上。没有阴影原样返回，
/// 与从前逐字节相同的重画范围
pub(super) fn inflate(r: GeoRect, style: &NodeStyle) -> GeoRect {
    let d = shadow_reach(style);
    if d <= 0.0 {
        return r;
    }
    GeoRect::new(r.x - d, r.y - d, r.width + 2.0 * d, r.height + 2.0 * d)
}

/// 只有高度变了的普通容器：从旧/新底边往上要让出多少（圆角、边框、阴影都在这一截里）
fn bottom_band(style: &NodeStyle) -> f32 {
    let r = style
        .border_radius
        .max(style.border_radius_br.unwrap_or(0.0))
        .max(style.border_radius_bl.unwrap_or(0.0));
    r + style.border_width + style.border_bottom_width + overhang(style)
}

/// 这个节点只有高度变了时，旧底边以上的像素是不是与高度无关
fn bottom_only(node: &RenderNode) -> bool {
    let s = &node.style;
    let uniform_border = [s.border_top_color, s.border_right_color, s.border_bottom_color, s.border_left_color]
        .iter()
        .all(|c| c.is_none() || *c == s.border_color);
    matches!(node.tag.as_str(), "view" | "block" | "")
        && s.background_gradient.is_none()
        && s.opacity >= 1.0
        && uniform_border
}

impl WxmlRenderer {
    /// 一个变了的节点（不在覆盖层、不是绘制坐标≠布局坐标的子树）从哪一行往下要重画
    pub(super) fn grow_cut(node: &RenderNode, old: &GeoRect, new: &GeoRect, painted_differs: bool) -> f32 {
        // 框往上还画出去的那截（阴影）也算：盒子挪了，旧阴影要擦
        let top = old.y.min(new.y) - overhang(&node.style);
        let height_only = (old.x - new.x).abs() <= 0.01 && (old.y - new.y).abs() <= 0.01 && (old.width - new.width).abs() <= 0.01;
        if painted_differs || !height_only || !bottom_only(node) {
            return top;
        }
        let bottom = (old.y + old.height).min(new.y + new.height);
        (bottom - bottom_band(&node.style)).max(top)
    }

    /// 配不上对的那段子树（新树里多出来的 = 要画，旧树里剩下的 = 要擦）：整棵里最高的顶边。
    /// `t` 是这棵子树所在的那棵布局树，`(px, py)` 是父节点在其中的累计原点
    pub(super) fn tail_cut(t: &dyn super::layout_snapshot::Layouts, node: &RenderNode, px: f32, py: f32, in_fixed: bool, ctx: &mut DiffCtx) {
        if ctx.bail {
            return;
        }
        // 增量重建留下来的子树：旧树里的占位换回真节点（旧布局照样从快照取）
        let moved = ctx.moved;
        let node = super::layout_incremental::resolve(node, moved);
        let Ok(l) = t.layout(node.taffy_node) else {
            ctx.bail = true;
            ctx.bail_why = format!("配不上对的 <{}> 取不到布局", node.tag);
            return;
        };
        let (x, y) = (px + l.location.x, py + l.location.y);
        let in_fixed = in_fixed || node.style.is_fixed;
        if in_fixed {
            // 画在覆盖层里，不在页面画布上
            ctx.fixed_changed = true;
        } else if let Some((top, reach)) = Self::subtree_box(t, node, x, y)
            // 变换原点是节点自己的框心，不是外包框的中心：再加半条对角线，转到哪个角度都盖得住
            .and_then(|(top, w, h)| Self::tail_reach(node, w, h, ctx).map(|r| (top, r + (w * w + h * h).sqrt() / 2.0)))
        {
            // 带 transform / CSS 动画的（列表底下那个在转的加载圈）：整棵子树跟着它一起变换，
            // 按子树的外包框算它最多画到框外多远，切线取「外包框顶 - 这一圈」—— 切线以下整宽重画，
            // 它画到哪都盖得住。子树里还嵌着变换 / 动画的算不准，`subtree_box` 给 None，照旧整帧
            ctx.cut_at(top - reach - overhang(&node.style), node);
            return;
        } else if matches!(node.tag.as_str(), "scroll-view" | "swiper") || node.style.transform.is_some() || node.style.animation.is_some() {
            ctx.bail = true;
            let what = if node.style.animation.is_some() { "动画" } else if node.style.transform.is_some() { "transform" } else { "子画布" };
            ctx.bail_why = format!(
                "配不上对的 <{}{}>（{what}）子树绘制坐标≠布局坐标",
                node.tag,
                node.attrs.get("class").map(|c| format!(" class={c}")).unwrap_or_default()
            );
            return;
        } else {
            ctx.cut_at(y - overhang(&node.style), node);
        }
        for c in &node.children {
            Self::tail_cut(t, c, x, y, in_fixed, ctx);
        }
    }

    /// 子树的外包框（布局坐标，不算变换）：(顶, 宽, 高)。子树里还有带变换 / 动画 / 子画布的节点给 None
    pub(super) fn subtree_box(t: &dyn super::layout_snapshot::Layouts, node: &RenderNode, x: f32, y: f32) -> Option<(f32, f32, f32)> {
        let l = t.layout(node.taffy_node).ok()?;
        let (mut x0, mut y0, mut x1, mut y1) = (x, y, x + l.size.width, y + l.size.height);
        fn walk(t: &dyn super::layout_snapshot::Layouts, n: &RenderNode, px: f32, py: f32, b: &mut (f32, f32, f32, f32)) -> Option<()> {
            let s = &n.style;
            if s.transform.is_some() || s.animation.is_some() || matches!(n.tag.as_str(), "scroll-view" | "swiper") {
                return None;
            }
            let l = t.layout(n.taffy_node).ok()?;
            let (x, y) = (px + l.location.x, py + l.location.y);
            *b = (b.0.min(x), b.1.min(y), b.2.max(x + l.size.width), b.3.max(y + l.size.height));
            n.children.iter().try_for_each(|c| walk(t, c, x, y, b))
        }
        let mut b = (x0, y0, x1, y1);
        node.children.iter().try_for_each(|c| walk(t, c, x, y, &mut b))?;
        (x0, y0, x1, y1) = b;
        let _ = x0;
        Some((y0, x1 - b.0, y1 - y0))
    }

    /// 内容变高时，一个配上了对、带 transform / CSS 动画的子树变了（挪了位置 / 内容变了）：
    /// 新旧两次各自最多画到的顶边取上面那个。算不准给 None（调用方照旧整帧）
    pub(super) fn moving_anim_cut(
        old_t: &dyn super::layout_snapshot::Layouts,
        old: &RenderNode,
        (ox, oy): (f32, f32),
        new_t: &dyn super::layout_snapshot::Layouts,
        new: &RenderNode,
        (nx, ny): (f32, f32),
        ctx: &DiffCtx,
    ) -> Option<f32> {
        let top = |t: &dyn super::layout_snapshot::Layouts, n: &RenderNode, x: f32, y: f32| -> Option<f32> {
            let (top, w, h) = Self::subtree_box(t, n, x, y)?;
            let reach = Self::tail_reach(n, w, h, ctx)? + (w * w + h * h).sqrt() / 2.0;
            Some(top - reach - overhang(&n.style))
        };
        Some(top(old_t, old, ox, oy)?.min(top(new_t, new, nx, ny)?))
    }

    /// 配不上对的那段里一个带 transform / CSS 动画的节点：最多画到布局框外多远（物理像素）。
    /// 算不出来（`wx.createAnimation` 下发的动画、找不到 `@keyframes`、scroll-view / swiper）给 None，照旧整帧
    pub(super) fn tail_reach(node: &RenderNode, w: f32, h: f32, ctx: &DiffCtx) -> Option<f32> {
        let s = &node.style;
        if matches!(node.tag.as_str(), "scroll-view" | "swiper") || node.attrs.contains_key("animation") {
            return None;
        }
        let pad = |t: &crate::renderer::components::Transform| {
            t.translate_x.abs().max(t.translate_y.abs()) + crate::renderer::compose::transform_padding(w, h, t)
        };
        match (&s.animation, &s.transform) {
            (None, None) => None,
            (None, Some(t)) => Some(pad(t)),
            (Some(spec), base) => {
                let rule = ctx.sheet.keyframes_named(&spec.name)?;
                let tl = crate::renderer::anim::Timeline::from_rule(rule, ctx.units.0, ctx.units.1);
                Some(tl.paint_reach(w, h, *base))
            }
        }
    }

    /// 内容变高这一趟比对的结论：切线往下整宽、到新内容底
    pub(super) fn grow_plan(&self, old_h: f32, new_h: f32, ctx: &DiffCtx, log: bool) -> FramePlan {
        let sf = self.scale_factor;
        // 没有记到任何变化（理论上不会：内容高总是某个节点撑出来的）：从旧内容底开始
        let cut = ctx.cut.unwrap_or(old_h * sf).max(0.0).floor();
        let bottom = (new_h * sf).ceil();
        if log {
            // 没退回整帧时 bail_why 记的是定下切线的那个节点（cut_at）
            eprintln!("🩹 内容变高 {old_h:.1} -> {new_h:.1}：从第 {cut:.0} 行往下重画（切线由 {} 定）", ctx.bail_why);
        }
        FramePlan::Damage {
            rect: GeoRect::new(0.0, cut, self.screen_width * sf, (bottom - cut).max(0.0)),
            fixed_changed: ctx.fixed_changed,
        }
    }
}
