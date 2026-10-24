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

use super::layout_snapshot::Layouts;
use super::*;

/// 失效分析要的旧树：节点、它们上一次的布局（旧 taffy 树或增量重建前拍下的那份）、内容高
pub(super) struct OldLayout<'a> {
    pub nodes: &'a [RenderNode],
    pub layouts: &'a dyn Layouts,
    pub content_height: f32,
    /// 增量重建留下来的子树：旧树里的占位 → 新树里的真节点（整棵重建是空表）
    pub moved: &'a HashMap<NodeId, &'a RenderNode>,
}

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

pub(super) struct DiffCtx<'a> {
    /// 见 [`OldLayout::moved`]
    pub moved: &'a HashMap<NodeId, &'a RenderNode>,
    /// 失效矩形的并集（物理像素，画布坐标）
    pub union: Option<GeoRect>,
    /// 变化是否落在 `position: fixed` 覆盖层里（宿主据此决定是否重画覆盖层）
    pub fixed_changed: bool,
    /// 遇到无法安全局部重绘的情况，退回整帧
    pub bail: bool,
    /// 退回整帧的原因（诊断用）
    pub bail_why: String,
    /// 内容变高了：不收集矩形，只记「最高的变化在哪一行」（invalidate_grow.rs）
    pub grow: bool,
    /// 内容变高时，这一行（物理像素，画布坐标）往下整宽都要重画
    pub cut: Option<f32>,
    /// 会被按兄弟位置选的规则选中的类名 / 标签（决定 fixed 的孩子能不能单独配，见 invalidate_pair.rs）
    pub positional: crate::parser::wxss::PositionalSubjects,
    /// 查 `@keyframes` 用（列表追加时末尾那段里有 CSS 动画的节点，按整段动画画到多远取切线，见 invalidate_grow.rs）
    pub sheet: &'a StyleSheet,
    /// rpx 换算：(屏宽逻辑像素, 像素比)
    pub units: (f32, f32),
}

impl DiffCtx<'_> {
    pub(super) fn add(&mut self, r: GeoRect) {
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
        old: OldLayout<'_>,
        new_nodes: &[RenderNode],
        new_taffy: &Tree,
        new_content_height: f32,
    ) -> FramePlan {
        let log = std::env::var("MINI_LAYOUT_LOG").is_ok();
        // 内容变矮：宿主要换一块更矮的画布，整帧最稳。
        // 内容变高（列表 onReachBottom 追加几条）走「从最高的变化往下整宽重画」（invalidate_grow.rs）
        let grow = new_content_height > old.content_height + 0.5;
        if !grow && (old.content_height - new_content_height).abs() > 0.5 {
            return self.full_because(|| format!("内容变矮 {:.1} -> {:.1}", old.content_height, new_content_height));
        }
        let mut ctx = DiffCtx {
            moved: old.moved,
            union: None,
            fixed_changed: false,
            bail: false,
            bail_why: String::new(),
            grow,
            cut: None,
            positional: self.stylesheet.positional_subjects(),
            sheet: &self.stylesheet,
            units: (self.screen_width, self.scale_factor),
        };
        // 根节点这一层与每个节点的孩子同一套配对规则（invalidate_pair.rs）
        Self::diff_children(old.layouts, new_taffy, old.nodes, new_nodes, (0.0, 0.0), (0.0, 0.0), false, false, &mut ctx);
        if ctx.bail {
            return self.full_because(|| ctx.bail_why.clone());
        }
        if grow {
            return self.grow_plan(old.content_height, new_content_height, &ctx, log);
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
            return self.full_because(|| {
                format!("失效范围 {:.0}x{:.0}@({:.0},{:.0}) 超过视口的 {DAMAGE_AREA_LIMIT}", rect.width, rect.height, rect.x, rect.y)
            });
        }
        FramePlan::Damage { rect, fixed_changed: ctx.fixed_changed }
    }

    /// 并行走新旧两棵树。`(oox, ooy)` / `(nox, noy)` 分别是旧/新树里父节点的累计原点。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn diff_node(
        old_t: &dyn Layouts,
        new_t: &dyn Layouts,
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
        // 增量重建留下来的子树：旧树里只剩个占位，旧的就是新的这一份（布局各取各的）
        let moved = ctx.moved;
        let old = super::layout_incremental::resolve(old, moved);
        // 孩子个数对不对得上由 diff_children 判（fixed 的孩子单独数；内容变高时可以不同）
        if old.tag != new.tag {
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

        // 一边是 fixed、一边不是（换了 class 变成 / 不再是 fixed，或内容变高时配错了对）。
        // fixed 那一边的布局位置不是它画的位置（覆盖层按 fixed 的偏移另摆，布局里常常就在 (0,0)），
        // 不能拿来当页面上的失效范围：内容高没变就整帧；变高了就两边各当「配不上的那段」走一遍 ——
        // fixed 那边只记「覆盖层要重画」，另一边按它在页面上的位置取切线
        if !in_fixed && old.style.is_fixed != new.style.is_fixed {
            ctx.fixed_changed = true;
            if !ctx.grow {
                ctx.bail = true;
                ctx.bail_why = format!("<{}> 一边是 fixed 一边不是", new.tag);
                return;
            }
            Self::tail_cut(old_t, old, oox, ooy, old.style.is_fixed, ctx);
            Self::tail_cut(new_t, new, nox, noy, new.style.is_fixed, ctx);
            return;
        }
        let in_fixed = in_fixed || new.style.is_fixed;
        // 带静态 transform 的子树（手势 / worklet 每帧改 style 里的 transform）：按绘制几何
        // 算旧、新两次画到哪，不再一律整帧（invalidate_transform.rs）
        if !in_fixed && !ctx.grow && Self::transform_diffable(old, new) {
            Self::diff_transform_subtree(old_t, new_t, old, new, (oox, ooy), (nox, noy), ctx);
            return;
        }
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
            } else if opaque_subtree && ctx.grow && !matches!(new.tag.as_str(), "scroll-view" | "swiper") {
                // 内容变高、它在切线下面整宽重画的那截里：按整段变换 / 动画最多画到哪取切线（invalidate_grow.rs）
                let (ob, nb) = ((oox, ooy), (nox, noy));
                match Self::moving_anim_cut(old_t, old, ob, new_t, new, nb, ctx) {
                    Some(cut) => ctx.cut_at(cut, new),
                    None => {
                        ctx.bail = true;
                        ctx.bail_why = format!("<{}> 变换 / 动画画到哪算不准", new.tag);
                    }
                }
                return;
            } else if opaque_subtree {
                ctx.bail = true;
                ctx.bail_why = format!(
                    "<{}{}> 子树绘制坐标≠布局坐标，无法安全局部重绘",
                    new.tag,
                    new.attrs.get("class").map(|c| format!(" class={}", c)).unwrap_or_default()
                );
                return;
            } else if ctx.grow {
                ctx.cut_at(Self::grow_cut(new, &old_rect, &new_rect, painted_differs), new);
            } else {
                // 框外还画出去的那圈（阴影）也要算进来：从前只收布局框，盒子一挪 / 一变高，
                // 框外的旧阴影没人擦、新阴影只画了框里那半（tests/sdk_damage_grow.rs 抓到的）
                ctx.add(super::invalidate_grow::inflate(old_rect, &old.style));
                ctx.add(super::invalidate_grow::inflate(new_rect, &new.style));
            }
        }

        // 孩子怎么配对（fixed 的单独配、内容变高时配不上的两段各走一遍）见 invalidate_pair.rs
        let old_fixed = in_fixed || old.style.is_fixed;
        Self::diff_children(old_t, new_t, &old.children, &new.children, (ox, oy), (nx, ny), in_fixed, old_fixed, ctx);
    }
}

/// 缓存整体作废的两个入口（从 animation.rs 搬来：那个文件超了 500 行）
impl WxmlRenderer {
    /// scroll-view 的离屏缓存全部作废（布局不动）。远程图片下完、canvas 画过时用：
    /// 数据没变，布局报 Unchanged，缓存也就一直「干净」—— 宿主照常重画，scroll-view 却只是
    /// 把缓存里那张还画着占位图的旧内容贴回去，滑一下（缓存被别的原因作废）才出图（G-1）
    pub fn invalidate_offscreen_caches(&mut self) {
        self.scroll_cache.mark_all_dirty();
    }

    /// 离屏缓存整个丢掉（不只是标脏）：页面被盖住 / tab 页切走时宿主留着这个渲染器，
    /// scroll-view 的整段内容位图（宽 × 内容高 × 4 字节）在那段时间里只是白占内存，回来时重画
    pub fn drop_offscreen_caches(&mut self) {
        self.scroll_cache.clear();
    }

    /// scroll-view 离屏缓存的像素缓冲（内存分项用）
    pub fn offscreen_buffers(&self) -> Vec<&[crate::Color]> {
        self.scroll_cache.pixel_buffers().collect()
    }

    /// 布局缓存作废，下一帧整棵树重建重排。字体换了（`@font-face` / `wx.loadFontFace` 下完）时用：
    /// 数据没变，失效分析会以为什么都不用动，可文字宽度已经变了
    pub fn invalidate_layout(&mut self) {
        self.cache = None;
    }
}
