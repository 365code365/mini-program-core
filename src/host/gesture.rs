//! 拖动手势的仲裁：**方向锁定**、滚动区域的选择、到边界后的**嵌套传递**。
//!
//! 之前是「按下的那一刻就决定谁滚」：命中 scroll-view 就把这次拖动交给它，
//! 否则交给页面。与微信的差距很明显：
//!
//! - **没有方向锁定**：横向 scroll-view（首页那一排卡片）会吃掉纵向的滑动，
//!   手指竖着划却什么也不动；反过来纵向容器里横着划也会带着页面上下跳；
//! - **没有嵌套传递**：内层列表滚到底之后继续上划，页面不会接着滚 —— 手感上就是「卡住了」；
//! - **`catchtouchmove` 不起作用**：弹窗遮罩上写它就是为了锁住下层页面的滚动，
//!   而引擎照样滚。
//!
//! 这里把决策抽成纯逻辑：按下时只记录候选（命中的最内层可滚区域、起点是否有
//! `catchtouchmove`、页面本身能不能滚），等**第一次明显位移**才按主方向定下由谁滚。
//! 与浏览器/Skyline 一致：一旦锁定，这次手势就不再换轴。

/// 定方向所需的最小位移（逻辑像素）。取小值让锁定尽早发生，
/// 又不至于把手指的抖动当成方向。
pub const DIRECTION_LOCK_SLOP: f32 = 4.0;

/// 手势主轴
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// 这次拖动归谁
#[derive(Clone, PartialEq, Debug)]
pub enum GestureTarget {
    /// 位移还不够，谁都别动
    Undecided,
    /// 明确不滚（`catchtouchmove`、或没有可滚的东西）
    Blocked,
    /// 页面滚动
    Page,
    /// 某个 scroll-view
    Area { id: String, axis: Axis },
    /// 左边缘侧滑返回（宿主级手势，不交给页面）
    EdgeBack,
}

/// 宿主该做的事
#[derive(Clone, PartialEq, Debug)]
pub enum GestureAction {
    None,
    /// 开始拖某个滚动区域（从**按下点**开始，避免锁定瞬间跳一下）
    BeginArea { id: String, axis: Axis },
    UpdateArea { id: String, axis: Axis },
    /// 开始拖页面
    BeginPage,
    UpdatePage,
    /// 开始/继续左边缘侧滑返回
    BeginEdgeBack,
    UpdateEdgeBack,
}

/// 一次拖动手势的状态
#[derive(Debug)]
pub struct DragGesture {
    start: (f32, f32),
    last: (f32, f32),
    prev: (f32, f32),
    target: GestureTarget,
    /// 命中的最内层可滚区域（id, 它的滚动轴）
    candidate: Option<(String, Axis)>,
    /// 起点处有 `catchtouchmove`（或 `capture-catch:touchmove`）
    catch_move: bool,
    /// 页面本身是否可滚
    page_scrollable: bool,
    /// 起点落在左边缘触发区，且当前页可以返回（栈里还有上一页）
    edge_back: bool,
}

impl DragGesture {
    pub fn new(
        start: (f32, f32),
        candidate: Option<(String, Axis)>,
        catch_move: bool,
        page_scrollable: bool,
    ) -> Self {
        Self {
            start,
            last: start,
            prev: start,
            target: GestureTarget::Undecided,
            candidate,
            catch_move,
            page_scrollable,
            edge_back: false,
        }
    }

    /// 允许这次手势升级成「左边缘侧滑返回」（起点在触发区且能返回时由宿主打开）
    pub fn allow_edge_back(mut self, allow: bool) -> Self {
        self.edge_back = allow;
        self
    }

    pub fn target(&self) -> &GestureTarget {
        &self.target
    }

    pub fn start(&self) -> (f32, f32) {
        self.start
    }

    /// 手指移动。第一次超过阈值时定方向并选定目标，之后只更新。
    pub fn on_move(&mut self, x: f32, y: f32) -> GestureAction {
        self.prev = self.last;
        self.last = (x, y);
        match &self.target {
            GestureTarget::Blocked => GestureAction::None,
            GestureTarget::Page => GestureAction::UpdatePage,
            GestureTarget::EdgeBack => GestureAction::UpdateEdgeBack,
            GestureTarget::Area { id, axis } => GestureAction::UpdateArea {
                id: id.clone(),
                axis: *axis,
            },
            GestureTarget::Undecided => {
                let (dx, dy) = (x - self.start.0, y - self.start.1);
                if dx.abs().max(dy.abs()) < DIRECTION_LOCK_SLOP {
                    return GestureAction::None;
                }
                let axis = if dx.abs() > dy.abs() {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                };
                // 左边缘往右划：宿主级的侧滑返回优先于页面里的任何横向滚动。
                // 真机上这一条是系统手势，页面拦不住它（`catchtouchmove` 也不行），
                // 所以判定放在最前面 —— 弹窗/picker 弹着时宿主根本不会开这个开关。
                if self.edge_back && axis == Axis::Horizontal && dx > 0.0 {
                    self.target = GestureTarget::EdgeBack;
                    return GestureAction::BeginEdgeBack;
                }
                // `catchtouchmove`：这次手势不许滚任何东西（遮罩锁滚动的标准写法）
                if self.catch_move {
                    self.target = GestureTarget::Blocked;
                    return GestureAction::None;
                }
                // 命中的滚动区域**方向一致**才由它接手；不一致就往外找页面
                if let Some((id, area_axis)) = &self.candidate {
                    if *area_axis == axis {
                        self.target = GestureTarget::Area {
                            id: id.clone(),
                            axis,
                        };
                        return GestureAction::BeginArea {
                            id: id.clone(),
                            axis,
                        };
                    }
                }
                if axis == Axis::Vertical && self.page_scrollable {
                    self.target = GestureTarget::Page;
                    return GestureAction::BeginPage;
                }
                self.target = GestureTarget::Blocked;
                GestureAction::None
            }
        }
    }

    /// 内层滚动区域到边界了：把这次手势交给页面继续（嵌套滚动传递）。
    /// 只在纵向、且页面可滚时成立。
    pub fn handoff_to_page(&mut self) -> GestureAction {
        if !self.page_scrollable {
            return GestureAction::None;
        }
        if let GestureTarget::Area { axis, .. } = &self.target {
            if *axis != Axis::Vertical {
                return GestureAction::None;
            }
        } else {
            return GestureAction::None;
        }
        self.target = GestureTarget::Page;
        GestureAction::BeginPage
    }

    /// 交接给页面时，页面拖动应从**当前点**开始（内容不许跳）
    pub fn last(&self) -> (f32, f32) {
        self.last
    }

    /// 本次移动的位移（判断「到边界后是否还在往同一方向推」用）
    pub fn delta(&self) -> (f32, f32) {
        (self.last.0 - self.prev.0, self.last.1 - self.prev.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertical_area() -> Option<(String, Axis)> {
        Some(("list".to_string(), Axis::Vertical))
    }
    fn horizontal_area() -> Option<(String, Axis)> {
        Some(("row".to_string(), Axis::Horizontal))
    }

    /// 抖动不该定方向
    #[test]
    fn tiny_move_stays_undecided() {
        let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true);
        assert_eq!(g.on_move(101.0, 102.0), GestureAction::None);
        assert_eq!(*g.target(), GestureTarget::Undecided);
    }

    /// 纵向划 + 纵向 scroll-view → 交给它，并且是从按下点开始拖
    #[test]
    fn vertical_drag_goes_to_vertical_area() {
        let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true);
        assert_eq!(
            g.on_move(101.0, 130.0),
            GestureAction::BeginArea { id: "list".into(), axis: Axis::Vertical }
        );
        assert_eq!(
            g.on_move(101.0, 160.0),
            GestureAction::UpdateArea { id: "list".into(), axis: Axis::Vertical }
        );
    }

    /// **横向 scroll-view 里纵向划要滚页面** —— 旧实现会被横向容器吃掉，手指竖划没反应
    #[test]
    fn vertical_drag_inside_horizontal_area_scrolls_page() {
        let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), false, true);
        assert_eq!(g.on_move(102.0, 140.0), GestureAction::BeginPage);
        assert_eq!(*g.target(), GestureTarget::Page);
        assert_eq!(g.on_move(102.0, 180.0), GestureAction::UpdatePage);
    }

    /// 横向划 + 横向 scroll-view → 交给它；页面不该跟着上下动
    #[test]
    fn horizontal_drag_goes_to_horizontal_area() {
        let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), false, true);
        assert_eq!(
            g.on_move(60.0, 101.0),
            GestureAction::BeginArea { id: "row".into(), axis: Axis::Horizontal }
        );
    }

    /// 纵向容器里横向划：谁都不滚（页面只能纵向滚）
    #[test]
    fn horizontal_drag_with_only_vertical_scrollables_does_nothing() {
        let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true);
        assert_eq!(g.on_move(40.0, 101.0), GestureAction::None);
        assert_eq!(*g.target(), GestureTarget::Blocked);
    }

    /// `catchtouchmove`：一律不滚（遮罩锁住下层页面）
    #[test]
    fn catch_touchmove_blocks_scrolling() {
        let mut g = DragGesture::new((100.0, 100.0), vertical_area(), true, true);
        assert_eq!(g.on_move(100.0, 200.0), GestureAction::None);
        assert_eq!(*g.target(), GestureTarget::Blocked);
    }

    /// 页面不可滚且没有可滚区域时，纵向划也不该动
    #[test]
    fn nothing_scrollable_means_blocked() {
        let mut g = DragGesture::new((100.0, 100.0), None, false, false);
        assert_eq!(g.on_move(100.0, 200.0), GestureAction::None);
        assert_eq!(*g.target(), GestureTarget::Blocked);
    }

    /// 嵌套传递：内层到边界 → 页面接手，且从当前点开始
    #[test]
    fn handoff_switches_to_page_from_current_point() {
        let mut g = DragGesture::new((100.0, 100.0), vertical_area(), false, true);
        g.on_move(100.0, 140.0);
        assert_eq!(g.handoff_to_page(), GestureAction::BeginPage);
        assert_eq!(g.last(), (100.0, 140.0));
        assert_eq!(g.on_move(100.0, 180.0), GestureAction::UpdatePage);
    }

    /// 左边缘往右划：即使命中了横向 scroll-view，也要走侧滑返回（系统手势优先）
    #[test]
    fn edge_swipe_right_wins_over_horizontal_area() {
        let mut g = DragGesture::new((6.0, 300.0), horizontal_area(), false, true).allow_edge_back(true);
        assert_eq!(g.on_move(40.0, 301.0), GestureAction::BeginEdgeBack);
        assert_eq!(*g.target(), GestureTarget::EdgeBack);
        assert_eq!(g.on_move(80.0, 302.0), GestureAction::UpdateEdgeBack);
        // 已经锁成侧滑返回，就不再交给页面
        assert_eq!(g.handoff_to_page(), GestureAction::None);
    }

    /// `catchtouchmove` 拦不住系统级的侧滑返回
    #[test]
    fn edge_swipe_ignores_catch_touchmove() {
        let mut g = DragGesture::new((2.0, 300.0), None, true, true).allow_edge_back(true);
        assert_eq!(g.on_move(40.0, 300.0), GestureAction::BeginEdgeBack);
    }

    /// 边缘往**左**划、或纵向划，都不是侧滑返回
    #[test]
    fn edge_swipe_needs_rightward_horizontal_move() {
        let mut g = DragGesture::new((6.0, 300.0), vertical_area(), false, true).allow_edge_back(true);
        assert_eq!(
            g.on_move(6.0, 350.0),
            GestureAction::BeginArea { id: "list".into(), axis: Axis::Vertical }
        );

        let mut g2 = DragGesture::new((18.0, 300.0), horizontal_area(), false, true).allow_edge_back(true);
        assert_eq!(
            g2.on_move(2.0, 300.0),
            GestureAction::BeginArea { id: "row".into(), axis: Axis::Horizontal }
        );
    }

    /// 宿主没开这个开关（栈底页 / 弹窗弹着）时，边缘右划退回普通仲裁
    #[test]
    fn edge_swipe_disabled_falls_back_to_normal_arbitration() {
        let mut g = DragGesture::new((4.0, 300.0), horizontal_area(), false, true);
        assert_eq!(
            g.on_move(40.0, 300.0),
            GestureAction::BeginArea { id: "row".into(), axis: Axis::Horizontal }
        );
    }

    /// 横向区域不参与纵向嵌套传递；页面不可滚时也不传递
    #[test]
    fn handoff_requires_vertical_area_and_scrollable_page() {
        let mut g = DragGesture::new((100.0, 100.0), horizontal_area(), false, true);
        g.on_move(60.0, 100.0); // 锁成横向区域
        assert_eq!(g.handoff_to_page(), GestureAction::None);

        let mut g2 = DragGesture::new((100.0, 100.0), vertical_area(), false, false);
        g2.on_move(100.0, 140.0);
        assert_eq!(g2.handoff_to_page(), GestureAction::None);
    }
}
