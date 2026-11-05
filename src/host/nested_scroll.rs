//! Skyline `<scroll-view type="nested">` 的里外层衔接（官方 component/scroll-view.html「嵌套模式」）：
//! 外层的子节点是 `nested-scroll-header`（滚走的头）与一个 `nested-scroll-body`（里面放里层 scroll-view）。
//! 嵌套滚动策略按官方原话：**向下滚动（手指上推、内容往上走）先滚外层再滚里层；向上滚动先滚里层再滚外层**。
//!
//! 做法：一次纵向拖动归一个 scroll-view（host::gesture），这里在每一步移动时看要不要换给另一个 ——
//! - 拖着里层、手指上推、外层还没到底（头还没滚完）→ 换给外层；手指下拉、里层已在顶、外层没到顶 → 换给外层；
//! - 拖着外层、手指上推、外层到底了、里层没到底 → 换给里层；手指下拉、里层不在顶 → 换给里层。
//! 换手时旧的那个停住（不带惯性），新的从当前点接着拖（内容不跳）。外层到顶再下拉照旧交给页面 / 下拉刷新。
//!
//! 只认「手指落在里层上」的那串：起点下的滚动区域链（由内到外）里有 nested 外层、它里面有一个纵向里层。
//! 手指落在头上（起点下只有外层）时只滚外层，头滚完不接着滚里层（Skyline 会接，没做）。松手之后的惯性不跨层传递（没做）。
//! 每帧代价：只在拖着 scroll-view 的移动里，按起点查一次滚动区域链（几个盒子比较）；页面上没有 nested 外层时链里一个都不命中。
use crate::ui::interaction::InteractionManager;

/// 起点下的 (里层, nested 外层)。`y` 是内容坐标。
/// 链按面积由小到大排，里外层一样大（里层撑满外层的视口，最常见）时先后不定 —— 所以不按先后认，
/// 外层取链里的 nested 那个，里层取链里第一个不是 nested 的纵向区域
fn pair_at(im: &InteractionManager, x: f32, y: f32) -> Option<(String, String)> {
    let (mut inner, mut outer): (Option<&str>, Option<&str>) = (None, None);
    for e in im.scroll_areas_at(x, y, false) {
        if im.scroll_views.get(&e.id).is_some_and(|s| s.nested_outer) {
            outer = outer.or(Some(&e.id));
        } else if !e.is_horizontal {
            inner = inner.or(Some(&e.id));
        }
    }
    Some((inner?.to_string(), outer?.to_string()))
}

/// 拖着 `current`、这一步手指纵向位移 `dy`（正 = 下拉）：该换给谁（None = 不换）
pub(super) fn switch_to(im: &InteractionManager, start: (f32, f32), page_scroll: f32, current: &str, dy: f32) -> Option<String> {
    if dy == 0.0 || im.scroll_views.values().all(|s| !s.nested_outer) {
        return None;
    }
    let (inner, outer) = pair_at(im, start.0, start.1 + page_scroll)?;
    let at = |id: &str| im.get_scroll_controller(id).map(|c| (c.is_at_top(), c.is_at_bottom() || c.get_max_scroll() <= 0.5));
    let (o_top, o_bottom) = at(&outer)?;
    let (i_top, i_bottom) = at(&inner)?;
    if current == inner {
        if (dy < 0.0 && !o_bottom) || (dy > 0.0 && i_top && !o_top) {
            return Some(outer);
        }
    } else if current == outer && ((dy < 0.0 && o_bottom && !i_bottom) || (dy > 0.0 && !i_top)) {
        return Some(inner);
    }
    None
}

/// 换手：旧的停住（不带惯性，越界的照常弹回），新的从当前点 `y` 接着拖
pub(super) fn hand_over(im: &mut InteractionManager, from: &str, to: &str, y: f32, clock_ms: u64) {
    if let Some(c) = im.get_scroll_controller_mut(from) {
        c.end_drag();
        if c.inertia_velocity().is_some() {
            c.stop();
        }
    }
    if let Some(c) = im.get_scroll_controller_mut(to) {
        c.begin_drag(y, clock_ms);
    }
    im.dragging_scroll_area = Some(to.to_string());
}
