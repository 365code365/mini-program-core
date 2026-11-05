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
    /// 下拉二级打开着（`refresher-two-level-enabled`）：内容被推到 scroll-view 底下，整个盒子露出刷新区（slot="refresher"）
    TwoLevel,
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
    /// 上次核对 `scroll-into-view` 绑定字段时逻辑层的 setData 序号（host/scroll_view.rs `rearm_into_view`）
    pub into_seq: Option<u64>,
    /// 上一次报的下拉距离（`refresherpulling` 只在变了时发）
    pub pulled: f32,
    /// 开了 `refresher-enabled`（渲染器登记时写）。手势仲裁据此决定：到顶后继续往下拉
    /// 是拉这个 scroll-view 的刷新区，而**不是**像普通 scroll-view 那样把手势交给页面
    pub refresher_enabled: bool,
    /// ScrollViewContext 写过的开关（None = 跟属性走；host/selector_query.rs 写，渲染器登记时生效）
    pub ctx: SvSwitches,
    /// `triggerRefresh()`（true）/ `closeRefresh()`（false）：宿主下一帧在刷新状态机里消费（host/scroll_view.rs）
    pub refresh_request: Option<bool>,
    /// 上一次报 `scroll` 的时刻（毫秒，宿主时钟）：`boundaryVelocity` 按两次之间的位移算速度
    pub last_scroll_ms: u64,
    /// Skyline `min-drag-distance`（逻辑 px，0 = 没写，按宿主默认的方向锁定距离）：手指至少移动这么远才开始滚这个 scroll-view
    pub min_drag: f32,
    /// Skyline `clip="{{false}}"`：盒子外也画着内容（主轴两侧各一个盒子长、交叉轴按内容溢出），
    /// 滚动时要重画的是这一块而不只是盒子（逻辑 px，页面内容坐标；渲染器登记时写，None = 裁到盒子）。
    /// [`paint_unbounded`](Self::paint_unbounded)：里面有 transform / 动画，画到哪算不准，滚动一律整帧
    pub paint: Option<crate::Rect>,
    pub paint_unbounded: bool,
    /// `triggerTwoLevel()`（true）/ `closeTwoLevel()`（false）：同上
    pub two_level_request: Option<bool>,
    /// 上一帧 `refresher-two-level-triggered` 的值（属性变了才开 / 关）
    pub last_two_triggered: Option<bool>,
    /// `bindrefresherstatuschange` 上一次报的 status（官方 RefreshStatus：0 Idle / 1 CanRefresh / 2 Refreshing / 3 Completed）
    pub refresh_status: u8,
    /// 滚动条：上次画时的位置、最后一次动的时刻（秒，渲染器动画时钟；wxml_renderer/scrollbar.rs）
    pub bar: (f32, f32),
    /// scroll-anchoring 的锚点：视口里第一项的身份与它在内容里的 y（物理 px；wxml_renderer/scroll_behavior.rs）
    pub anchor: Option<(String, f32)>,
    /// `reverse`：上一帧的可滚上限（内容变长时贴着底的接着贴底）
    pub last_max: f32,
    /// Skyline `type="nested"` 的外层（渲染器登记时写）：手势里外层衔接（host/nested_scroll.rs）
    pub nested_outer: bool,
    /// `scroll-x` 与 `scroll-y` 同时开着：横向另有一个控制器（键 `"{id}#x"`，ui/interaction/scroll_2d.rs）
    pub both_axes: bool,
    /// 双向的横向那一半：上次报的 scrollLeft、是否在左 / 右阈值区（host/scroll_view.rs）
    pub reported_x: f32,
    pub in_left: bool,
    pub in_right: bool,
}

/// ScrollViewContext 的可写属性（官方 api/ui/scroll/ScrollViewContext.html）
#[derive(Debug, Clone, Copy, Default)]
pub struct SvSwitches {
    pub scroll_enabled: Option<bool>,
    pub bounces: Option<bool>,
    pub show_scrollbar: Option<bool>,
    pub paging_enabled: Option<bool>,
    pub fast_deceleration: Option<bool>,
    pub deceleration_disabled: Option<bool>,
}

impl SvSwitches {
    /// 按 JS 属性名写一个开关，名字不认识返回 false
    pub fn set(&mut self, key: &str, v: bool) -> bool {
        let slot = match key {
            "scrollEnabled" => &mut self.scroll_enabled,
            "bounces" => &mut self.bounces,
            "showScrollbar" => &mut self.show_scrollbar,
            "pagingEnabled" => &mut self.paging_enabled,
            "fastDeceleration" => &mut self.fast_deceleration,
            "decelerationDisabled" => &mut self.deceleration_disabled,
            _ => return false,
        };
        *slot = Some(v);
        true
    }
}
