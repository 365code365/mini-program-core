//! 每个 `<scroll-view>` 跨帧要记住的东西（按与滚动控制器相同的组件 id 存）。
//!
//! 两拨人用它：
//! - **渲染器**（登记 scroll-view 时）：受控属性 `scroll-top` / `scroll-left` /
//!   `scroll-into-view` / `refresher-triggered` 上一次看到的值 —— 只有值变了才去动滚动位置，
//!   否则用户一拖，下一帧又被属性拽回去；
//! - **宿主**（`host::scroll_view`，每帧）：上一次报给逻辑层的位置、是否在上 / 下阈值区内、
//!   是否在滚 / 在拖、下拉刷新走到了哪一步 —— 事件只在状态**变化**时发。
//!
//! 与滚动控制器同生共死：换页 `clear_page_state` 一起清。

/// 自定义下拉刷新的阶段（scroll-view 的 `refresher-*`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefresherPhase {
    /// 没拉
    #[default]
    Idle,
    /// 手指正往下拉（每帧发 `refresherpulling`）
    Pulling,
    /// 已触发刷新，指示器停在阈值处，等 `refresher-triggered` 被设回 false
    Refreshing,
}

#[derive(Debug, Clone, Default)]
pub struct ScrollViewState {
    /// 渲染器见过它没有（第一次见时受控属性直接落位、不做动画，也不发事件）
    pub seen: bool,
    pub last_scroll_top: Option<String>,
    pub last_scroll_left: Option<String>,
    pub last_into_view: Option<String>,
    pub last_triggered: Option<bool>,

    /// 宿主那边初始化过没有（第一帧只记状态不发事件）
    pub tracked: bool,
    /// 上一次报给逻辑层的位置（逻辑 px）
    pub reported_pos: f32,
    pub in_upper: bool,
    pub in_lower: bool,
    /// 上一帧是否在动（`scrollstart` / `scrollend`）
    pub moving: bool,
    /// 上一帧手指是否按着拖（`dragstart` / `dragend`、`isDrag`）
    pub dragging: bool,
    /// 这一次滚动是否由拖动开始（`scrollend` 的 `isDrag`）
    pub drag_started: bool,
    pub refresher: RefresherPhase,
    /// 进入刷新态时逻辑层的 setData 序号：之后再 setData 了 `refresher-triggered` 绑定的字段
    /// （哪怕值没变，比如一直是 false）才算「设回 false」—— 微信的属性设置就是这个语义
    pub refresh_seq: u64,
    /// 上一次报的下拉距离（`refresherpulling` 只在变了时发）
    pub pulled: f32,
    /// 开了 `refresher-enabled`（渲染器登记时写）。手势仲裁据此决定：到顶后继续往下拉
    /// 是拉这个 scroll-view 的刷新区，而**不是**像普通 scroll-view 那样把手势交给页面
    pub refresher_enabled: bool,
}
