//! `setData` 的增量失效分析：算出「这次数据变化只影响屏幕上哪一块」。
//!
//! 从前每次 `setData` 都整帧重绘。首页那种长页面一帧要 9~13ms，而秒杀倒计时
//! 每秒 setData 一次 —— 于是稳定的 145FPS 里每秒插进一个 17~25ms 的长帧，
//! 丢掉三四帧。平均帧率看着还很高，手上就是每秒一顿，这正是「滑动不够丝滑」
//! 里最刺眼的那一下。
//!
//! 做法：布局重建之后，把**新旧两棵 `RenderNode` 树连同各自的 taffy 布局**
//! 并行走一遍，只把「文本/属性变了」或「几何变了」的节点的包围盒收集起来
//! （几何变化要同时算上旧位置和新位置，否则旧像素擦不掉）。
//!
//! 判断依据只看 `text` 与 `attrs`：样式是由标签 + class/style 属性 + 祖先链 +
//! 兄弟序号推导出来的，而结构相等已经由这趟并行遍历本身保证，
//! 所以属性相等就意味着样式相等。
//!
//! **一切拿不准的情况都退回整帧重绘**（结构变了、跑到 scroll-view / swiper /
//! transform 子树里、影响面过大）。少画一块的代价是残留脏像素，
//! 比多画一次严重得多，所以这里的偏置是刻意保守的。

use super::*;

/// 这一帧该重绘多大范围
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FramePlan {
    /// 数据没变（滚动帧、动画帧）：布局树直接复用，重绘范围由调用方自行决定
    Unchanged,
    /// 数据变了，但只影响这个矩形（物理像素，画布坐标）
    Damage { rect: GeoRect, fixed_changed: bool },
    /// 数据变了且结构变化 / 影响面过大：整帧重绘
    Full,
}

/// 收集到的失效范围超过这个比例（相对视口面积）就不值得做局部重绘了
const DAMAGE_AREA_LIMIT: f32 = 0.34;

pub(super) struct DiffCtx {
    /// 失效矩形的并集（物理像素，画布坐标）
    pub union: Option<GeoRect>,
    /// 变化是否落在 `position: fixed` 覆盖层里（宿主据此决定是否重画覆盖层）
    pub fixed_changed: bool,
    /// 遇到无法安全局部重绘的情况，退回整帧
    pub bail: bool,
    /// 退回整帧的原因（诊断用）
    pub bail_why: String,
}

impl DiffCtx {
    fn add(&mut self, r: GeoRect) {
        if r.width <= 0.0 || r.height <= 0.0 {
            return;
        }
        self.union = Some(match self.union {
            None => r,
            Some(u) => {
                let x0 = u.x.min(r.x);
                let y0 = u.y.min(r.y);
                let x1 = (u.x + u.width).max(r.x + r.width);
                let y1 = (u.y + u.height).max(r.y + r.height);
                GeoRect::new(x0, y0, x1 - x0, y1 - y0)
            }
        });
    }
}

impl WxmlRenderer {
    /// 比较新旧布局树，算出这次数据变化的重绘范围
    pub(super) fn plan_from_diff(
        &self,
        old: &CachedLayout,
        new_nodes: &[RenderNode],
        new_taffy: &Tree,
        new_content_height: f32,
    ) -> FramePlan {
        let log = std::env::var("MINI_LAYOUT_LOG").is_ok();
        // 内容高变了，宿主可能要重建画布，整帧最稳
        if (old.content_height - new_content_height).abs() > 0.5 {
            if log {
                eprintln!(
                    "🩹 退回整帧：内容高 {:.1} -> {:.1}",
                    old.content_height, new_content_height
                );
            }
            return FramePlan::Full;
        }
        if old.render_nodes.len() != new_nodes.len() {
            if log {
                eprintln!("🩹 退回整帧：根节点数变化");
            }
            return FramePlan::Full;
        }
        let mut ctx = DiffCtx { union: None, fixed_changed: false, bail: false, bail_why: String::new() };
        for (o, n) in old.render_nodes.iter().zip(new_nodes.iter()) {
            Self::diff_node(&old.taffy, new_taffy, o, n, 0.0, 0.0, 0.0, 0.0, false, &mut ctx);
            if ctx.bail {
                if log {
                    eprintln!("🩹 退回整帧：{}", ctx.bail_why);
                }
                return FramePlan::Full;
            }
        }
        let Some(rect) = ctx.union else {
            // 数据指纹变了但渲染结果一模一样（比如改了不参与渲染的字段）：
            // 不用重绘，但覆盖层的判断照旧交给调用方
            return FramePlan::Damage {
                rect: GeoRect::new(0.0, 0.0, 0.0, 0.0),
                fixed_changed: ctx.fixed_changed,
            };
        };
        let sf = self.scale_factor;
        let viewport_area = (self.screen_width * sf) * (self.screen_height * sf);
        if rect.width * rect.height > viewport_area * DAMAGE_AREA_LIMIT {
            return FramePlan::Full;
        }
        FramePlan::Damage { rect, fixed_changed: ctx.fixed_changed }
    }

    /// 并行走新旧两棵树。`(oox, ooy)` / `(nox, noy)` 分别是旧/新树里父节点的累计原点。
    #[allow(clippy::too_many_arguments)]
    fn diff_node(
        old_t: &Tree,
        new_t: &Tree,
        old: &RenderNode,
        new: &RenderNode,
        oox: f32,
        ooy: f32,
        nox: f32,
        noy: f32,
        in_fixed: bool,
        ctx: &mut DiffCtx,
    ) {
        if ctx.bail {
            return;
        }
        if old.tag != new.tag || old.children.len() != new.children.len() {
            ctx.bail = true;
            ctx.bail_why = format!(
                "结构变化 <{}>({} 子) -> <{}>({} 子)",
                old.tag, old.children.len(), new.tag, new.children.len()
            );
            return;
        }
        let (Ok(ol), Ok(nl)) = (old_t.layout(old.taffy_node), new_t.layout(new.taffy_node)) else {
            ctx.bail = true;
            ctx.bail_why = format!("<{}> 取不到布局", new.tag);
            return;
        };
        let (ox, oy) = (oox + ol.location.x, ooy + ol.location.y);
        let (nx, ny) = (nox + nl.location.x, noy + nl.location.y);
        let old_rect = GeoRect::new(ox, oy, ol.size.width, ol.size.height);
        let new_rect = GeoRect::new(nx, ny, nl.size.width, nl.size.height);

        let in_fixed = in_fixed || new.style.is_fixed;
        // 这些子树的绘制坐标不等于布局坐标（子画布 / 横向偏移 / 离屏仿射），
        // 用布局包围盒当失效范围会擦错地方 —— 里面一有变化就整帧重绘。
        let opaque_subtree = matches!(new.tag.as_str(), "scroll-view" | "swiper")
            || new.style.transform.is_some()
            || new.style.animation.is_some();

        let painted_differs = old.text != new.text || old.attrs != new.attrs;
        let moved = (old_rect.x - new_rect.x).abs() > 0.01
            || (old_rect.y - new_rect.y).abs() > 0.01
            || (old_rect.width - new_rect.width).abs() > 0.01
            || (old_rect.height - new_rect.height).abs() > 0.01;

        if painted_differs || moved {
            if in_fixed {
                // 覆盖层是整张单独重画的，所以这里只需要打个标记。
                // 注意这一步必须排在 `opaque_subtree` 之前：覆盖层里的弹窗普遍带
                // 入场动画（`animation: popIn`，含 transform），要是先判 opaque
                // 就会因为「弹窗里有个带动画的 view」把**整页**也拖去重绘。
                ctx.fixed_changed = true;
            } else if opaque_subtree {
                ctx.bail = true;
                ctx.bail_why = format!(
                    "<{}{}> 子树绘制坐标≠布局坐标，无法安全局部重绘",
                    new.tag,
                    new.attrs.get("class").map(|c| format!(" class={}", c)).unwrap_or_default()
                );
                return;
            } else {
                ctx.add(old_rect);
                ctx.add(new_rect);
            }
        }

        for (o, n) in old.children.iter().zip(new.children.iter()) {
            Self::diff_node(old_t, new_t, o, n, ox, oy, nx, ny, in_fixed, ctx);
            if ctx.bail {
                return;
            }
        }
    }
}
