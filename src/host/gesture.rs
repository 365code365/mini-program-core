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
    /// 某个 swiper（跟手拖动，松手吸附 / 翻页）
    Swiper { id: String, axis: Axis },
    /// 某个 slider（横向跟手，松手发 change）
    Slider { id: String },
    /// 某个 `<video>` 的进度条（横向跟手，松手 seek；`key` 是 `host_feed::video_key`）
    VideoSeek { key: String },
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
    /// 开始/继续拖 swiper（从按下点开始）
    BeginSwiper { id: String, axis: Axis },
    UpdateSwiper { id: String, axis: Axis },
    /// 开始/继续拖 slider（开始时从按下点判断是不是抓在圆点上）
    BeginSlider { id: String },
    UpdateSlider { id: String },
    /// 开始/继续拖视频的进度条
    BeginVideoSeek { key: String },
    UpdateVideoSeek { key: String },
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
    /// 命中的最内层 swiper（id, 它的滑动轴, 是否比 `candidate` 更内层）
    swiper: Option<(String, Axis, bool)>,
    /// 起点下最上层的元素是一个 slider
    slider: Option<String>,
    /// 起点落在某个 `<video>` 显示着的进度条上
    video_seek: Option<String>,
    /// 起点落在 `<draggable-sheet>` 的面板里
    sheet: Option<SheetHit>,
    /// 命中的 scroll-view 写了 `min-drag-distance`（逻辑 px，0 = 没写）：移动不到它就还不定方向
    min_drag: f32,
    /// 命中的 scroll-view 横竖都能滚：哪个方向先动都归它（宿主两个轴一起推，host/input.rs）
    free_area: bool,
}

/// 起点下的 `<draggable-sheet>` 面板（`builtin_components/draggable-sheet.js`）。面板自己在逻辑层的 touchmove 里拖，
/// 这里只决定这串触摸归面板（谁也不滚）还是归面板里的列表
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SheetHit {
    /// 面板已到 max-child-size
    pub at_max: bool,
    /// 命中的纵向滚动区域在面板里面
    pub area_inside: bool,
    /// 那个滚动区域在顶（scrollTop ≈ 0）
    pub area_at_top: bool,
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
            swiper: None,
            slider: None,
            video_seek: None,
            sheet: None,
            min_drag: 0.0,
            free_area: false,
        }
    }

    /// 命中的 scroll-view 横竖都能滚（`scroll-x` + `scroll-y`）：不按轴挑，哪个方向先动都由它接手
    pub fn with_free_area(mut self, free: bool) -> Self {
        self.free_area = free;
        self
    }

    /// 起点落在 draggable-sheet 的面板里（`host::gesture_native::sheet_at`）
    pub fn with_sheet(mut self, sheet: Option<SheetHit>) -> Self {
        self.sheet = sheet;
        self
    }

    /// 起点下的 scroll-view 写了 `min-drag-distance`：手指移动到这么远才定方向、开始滚（Skyline）
    pub fn with_min_drag(mut self, px: f32) -> Self {
        self.min_drag = px.max(0.0);
        self
    }

    /// 起点落在视频的进度条上：横向锁定时由它接手（与 slider 同理：竖着划照样滚页面，点一下走 tap）
    pub fn with_video_seek(mut self, key: Option<String>) -> Self {
        self.video_seek = key;
        self
    }

    /// 起点落在 slider 上：横向锁定时由它接手（竖着划照样滚页面，点一下走 tap）。
    ///
    /// 优先级高于 `catchtouchmove`：弹窗遮罩上写 `catchtouchmove` 锁页面滚动是标准写法，
    /// 弹窗里的滑块不能因此拖不动（微信里 slider 的拖动是组件自己的，与冒泡无关）。
    pub fn with_slider(mut self, slider: Option<String>) -> Self {
        self.slider = slider;
        self
    }

    /// 起点下有 swiper：方向与它一致时由它接手。`inner` 表示它比命中的 scroll-view 更内层
    /// （swiper 在纵向列表里、列表横划交给 swiper；横向 scroll-view 里的 swiper 也是 swiper 先拿）
    pub fn with_swiper(mut self, swiper: Option<(String, Axis)>, inner: bool) -> Self {
        self.swiper = swiper.map(|(id, axis)| (id, axis, inner));
        self
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
            GestureTarget::Swiper { id, axis } => GestureAction::UpdateSwiper {
                id: id.clone(),
                axis: *axis,
            },
            GestureTarget::Slider { id } => GestureAction::UpdateSlider { id: id.clone() },
            GestureTarget::VideoSeek { key } => GestureAction::UpdateVideoSeek { key: key.clone() },
            GestureTarget::Undecided => {
                let (dx, dy) = (x - self.start.0, y - self.start.1);
                if dx.abs().max(dy.abs()) < DIRECTION_LOCK_SLOP.max(self.min_drag) {
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
                // slider 横着拖：跟手（它是最内层、也是最上层的那个元素）
                if let (Some(id), Axis::Horizontal) = (&self.slider, axis) {
                    self.target = GestureTarget::Slider { id: id.clone() };
                    return GestureAction::BeginSlider { id: id.clone() };
                }
                // 视频的进度条横着拖：同上（它画在视频框里，比外面任何滚动区域都内层）
                if let (Some(key), Axis::Horizontal) = (&self.video_seek, axis) {
                    self.target = GestureTarget::VideoSeek { key: key.clone() };
                    return GestureAction::BeginVideoSeek { key: key.clone() };
                }
                // draggable-sheet 的面板里竖着划：面板没到最大时上推、列表在顶时下拉（或面板里没有列表）归面板 ——
                // 谁也不滚，逻辑层的 touchmove 拖面板；其余交给列表（滚动接管时面板收到 touchcancel、退回去）
                if let (Some(s), Axis::Vertical) = (&self.sheet, axis) {
                    let list = s.area_inside && matches!(&self.candidate, Some((_, Axis::Vertical)));
                    if !list || (dy < 0.0 && !s.at_max) || (dy > 0.0 && s.area_at_top) {
                        self.target = GestureTarget::Blocked;
                        return GestureAction::None;
                    }
                }
                // `catchtouchmove`：这次手势不许滚任何东西（遮罩锁滚动的标准写法）
                if self.catch_move {
                    self.target = GestureTarget::Blocked;
                    return GestureAction::None;
                }
                // swiper 方向一致、且它在更内层（或没有同方向的滚动区域）：由它接手
                if let Some((id, sw_axis, inner)) = &self.swiper {
                    let area_same_axis = self.candidate.as_ref().map(|(_, a)| *a == axis || self.free_area).unwrap_or(false);
                    if *sw_axis == axis && (*inner || !area_same_axis) {
                        self.target = GestureTarget::Swiper { id: id.clone(), axis };
                        return GestureAction::BeginSwiper { id: id.clone(), axis };
                    }
                }
                // 命中的滚动区域**方向一致**才由它接手（横竖都能滚的哪个方向都算）；不一致就往外找页面
                if let Some((id, area_axis)) = &self.candidate {
                    if *area_axis == axis || self.free_area {
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

    /// `type="nested"` 的里外层交接（host/nested_scroll.rs）：这次拖动改归另一个纵向 scroll-view，宿主从当前点接着拖
    pub fn retarget_area(&mut self, id: String) {
        self.target = GestureTarget::Area { id, axis: Axis::Vertical };
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
#[path = "gesture_tests.rs"]
mod tests;
