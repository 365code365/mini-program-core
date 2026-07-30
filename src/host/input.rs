//! 指针输入的**平台无关核心**：桌面窗体与移动端 SDK 共用这一份。
//!
//! 这里只放「拿到一个坐标之后该怎么办」的判定：触摸序列派发、手势归属仲裁、
//! 惯性收尾、滚动接管判断。窗口系统相关的东西（`window.request_redraw()`、
//! 剪贴板、softbuffer）留在各自的宿主里。
//!
//! ## 为什么必须共用
//! 这些逻辑曾经在两处各写一份：`src/bin/app_window/`（桌面）与
//! `src/host/engine_input.rs`（SDK）。结果不是「两份一样的代码」那么简单 ——
//! SDK 那份是**退化版**：没有方向锁定、没有嵌套滚动交接、没有 catchtouchmove、
//! 没有侧滑返回，`pointer_down` 直接 `scroll.begin_drag()` 把位移全给页面。
//! 于是横向卡片列表在手机上会吞掉整页纵滑，内层列表滚到底也不会交棒给页面。
//! 而 `tools/sdk-parity.sh` 只比静态页面像素，测不到这类差异，所以分叉能一直藏着。
//!
//! 判定逻辑本身在更下层的纯逻辑模块里，这里负责把它们接到宿主状态上：
//! - [`super::touch`]：触摸状态机与微信语义的事件对象（slop / longpress / cancel）
//! - [`super::gesture`]：拖动归属仲裁（方向锁定 / 嵌套传递 / catchtouchmove）
//! - [`super::edge_back`]：左边缘侧滑返回（跟手位移 / 阈值判定）

use super::edge_back::EdgeBack;
use super::gesture::{Axis, DragGesture, GestureAction};
use super::touch::TouchOut;
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::interaction::InteractionManager;
use crate::ui::ScrollController;

/// 手势仲裁这一步对宿主产生的副作用（各宿主用自己的方式落地）
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GestureEffect {
    /// 需要重绘（`scroll-view` 内滚动会改画布内容）
    pub needs_redraw: bool,
    /// 页面滚动位置变了（宿主据此更新「最近滚动时刻」，裁剪余量按它自适应）。
    ///
    /// 注意这里**不置** `needs_redraw`：整页内容已经画在长画布上，页面滚动只是换一条
    /// 切片上屏。置脏会让每帧都整页重绘（实测首页拖动 3.7ms → 6.5ms，等于把
    /// 「滚动不重绘」那条优化整个废掉）。
    pub page_scrolled: bool,
}

/// 把触摸状态机产出的事件派发给逻辑层。
///
/// 覆盖层与正常流是两套坐标，所以派发要分开问：覆盖层用**视口**坐标且命中即到此为止
/// （弹窗遮罩之上的触摸不许穿透到下层页面），正常流要加上页面滚动偏移。
///
/// `after_each` 在每次真的派发出去之后调用一次 —— 处理函数里可能 `setData` 或导航，
/// 宿主要在**这一刻**取脏标记/查导航请求（挪到循环外面的话，同一批事件里第二个
/// 处理函数的导航会盖掉第一个）。
///
/// 返回是否至少派发出去一次。
#[allow(clippy::too_many_arguments)]
pub fn dispatch_touch_sequence(
    app: &mut MiniApp,
    renderer: Option<&WxmlRenderer>,
    outs: &[TouchOut],
    (x, y): (f32, f32),
    scroll_pos: f32,
    identifier: u32,
    time_ms: u64,
    mut after_each: impl FnMut(&mut MiniApp),
) -> bool {
    if outs.is_empty() {
        return false;
    }
    let on_fixed = renderer.map(|r| r.fixed_layer_hit(x, y)).unwrap_or(false);
    let (hit_y, scope) = if on_fixed {
        (y, Some(true))
    } else {
        (y + scroll_pos, Some(false))
    };
    let mut any = false;
    for out in outs {
        // `tap` 不在这里发：它走点击链路（那边还要管按压态、picker、输入框）
        let names: &[&str] = match out {
            TouchOut::Start => &["touchstart"],
            TouchOut::Move => &["touchmove"],
            TouchOut::End => &["touchend"],
            TouchOut::Cancel => &["touchcancel"],
            // 微信同时派发新旧两个名字
            TouchOut::LongPress => &["longpress", "longtap"],
            TouchOut::Tap => &[],
        };
        for name in names {
            let detail = if *name == "longpress" || *name == "longtap" {
                serde_json::json!({ "x": x, "y": y })
            } else {
                serde_json::json!({})
            };
            let sent = match renderer {
                Some(r) => super::touch::dispatch_to_js(
                    app,
                    r,
                    name,
                    (x, hit_y),
                    (x, y),
                    scope,
                    identifier,
                    time_ms,
                    detail,
                ),
                None => false,
            };
            if sent {
                any = true;
                after_each(app);
            }
        }
    }
    any
}

/// 按下点处**最内层**的可滚区域（id + 它的滚动轴），供方向锁定决策。
///
/// 要的是最内层的可滚区域，不是最上层的元素：卡片/按钮盖在 `scroll-view` 上面时
/// 普通命中测试返回的是卡片，于是真正装内容的容器永远得不到手势。
pub fn scroll_candidate_at(
    interaction: &InteractionManager,
    scroll_pos: f32,
    x: f32,
    y: f32,
    on_fixed_layer: bool,
) -> Option<(String, Axis)> {
    use crate::ui::scroll_controller::ScrollDirection;
    let actual_y = y + scroll_pos;
    let el = if on_fixed_layer {
        interaction.hit_test_scroll_area(x, y, true).cloned()
    } else {
        interaction
            .hit_test_scroll_area(x, actual_y, false)
            .or_else(|| interaction.hit_test_scroll_area(x, y, false))
            .cloned()
    }?;
    let axis = match interaction
        .get_scroll_controller(&el.id)
        .map(|c| c.get_direction())
    {
        Some(ScrollDirection::Horizontal) => Axis::Horizontal,
        Some(ScrollDirection::Vertical) => Axis::Vertical,
        // 控制器还没建出来时按元素声明的方向
        None if el.is_horizontal => Axis::Horizontal,
        None => Axis::Vertical,
    };
    Some((el.id, axis))
}

/// 停掉正在跑的惯性/回弹（页面与所有 `scroll-view`）。返回是否真的停了什么。
///
/// 惯性滚动中按下：那一下只是**停住**，不算点击（iOS/微信一致）。
pub fn stop_running_flings(
    scroll: &mut ScrollController,
    interaction: &mut InteractionManager,
) -> bool {
    let mut stopped = false;
    if scroll.is_animating() {
        scroll.stop();
        stopped = true;
    }
    for c in interaction.scroll_controllers.values_mut() {
        if c.is_animating() {
            c.stop();
            stopped = true;
        }
    }
    stopped
}

/// 滚动是否已经接管这次触摸（页面滚动位置变了，或某个 `scroll-view` 正在被拖）。
///
/// 接管方要给触摸序列补一次 `touchcancel` —— 与浏览器/Skyline 的手势竞争一致。
pub fn scroll_took_over(
    scroll: &ScrollController,
    interaction: &InteractionManager,
    scroll_pos_at_press: f32,
) -> bool {
    if (scroll.get_position() - scroll_pos_at_press).abs() > 0.5 {
        return true;
    }
    interaction
        .dragging_scroll_area
        .as_ref()
        .and_then(|id| interaction.get_scroll_controller(id))
        .map(|c| c.is_dragging && c.get_position() > 0.0 || c.is_animating())
        .unwrap_or(false)
}

/// 建立本次触摸的手势候选：**归属不在按下时决定**，等第一次明显位移再按主方向锁定。
///
/// 微信/浏览器都是这个语义：横向 `scroll-view` 里竖着划应该滚页面，纵向列表里横着划
/// 则谁也不动。按下就把手势交给命中的容器（旧实现）会让横滑卡片吃掉整页的纵向滑动。
#[allow(clippy::too_many_arguments)]
pub fn begin_gesture(
    interaction: &InteractionManager,
    scroll: &ScrollController,
    renderer: Option<&WxmlRenderer>,
    x: f32,
    y: f32,
    on_fixed_layer: bool,
    stack_depth: usize,
    edge_back_busy: bool,
) -> DragGesture {
    let scroll_pos = scroll.get_position();
    let candidate = scroll_candidate_at(interaction, scroll_pos, x, y, on_fixed_layer);
    let catch_move = renderer
        .map(|r| {
            let hy = if on_fixed_layer { y } else { y + scroll_pos };
            r.has_catch_for(x, hy, "touchmove")
        })
        .unwrap_or(false);
    let page_scrollable =
        !on_fixed_layer && !interaction.is_dragging_slider() && scroll.get_max_scroll() > 0.5;
    // 左边缘侧滑返回：起点在触发区、栈里还有上一页、且没盖着覆盖层。
    // 栈底（tab 首页）没有这个手势 —— 微信里在首页往右划什么也不会发生。
    let allow_edge_back =
        EdgeBack::at_edge(x) && stack_depth > 1 && !on_fixed_layer && !edge_back_busy;
    DragGesture::new((x, y), candidate, catch_move, page_scrollable)
        .allow_edge_back(allow_edge_back)
}

/// 把手势仲裁的结果落到滚动控制器/侧滑返回上（方向锁定 + 嵌套传递都在这里生效）。
#[allow(clippy::too_many_arguments)]
pub fn apply_gesture_move(
    gesture: &mut Option<DragGesture>,
    interaction: &mut InteractionManager,
    scroll: &mut ScrollController,
    edge_back: &mut Option<EdgeBack>,
    viewport_width: f32,
    x: f32,
    y: f32,
    clock_ms: u64,
) -> GestureEffect {
    let mut eff = GestureEffect::default();
    let Some(mut g) = gesture.take() else {
        return eff;
    };
    let log = std::env::var("MINI_SCROLL_LOG").is_ok();
    let before_target = g.target().clone();
    let mut action = g.on_move(x, y);
    if before_target != *g.target() && log {
        eprintln!("🖐 手势归属：{:?} -> {:?}", before_target, g.target());
    }
    loop {
        match action {
            GestureAction::None => break,
            GestureAction::BeginArea { ref id, axis } => {
                let (sx, sy) = g.start();
                // 这个 scroll-view 根本没有滚动控制器（内容不足一屏，压根不用滚）：
                // 直接把手势交给页面，否则手指划在它上面时整页都不动 ——
                // 「短列表挡住整页滚动」正是这么来的。
                let cannot_scroll = interaction
                    .get_scroll_controller(id)
                    .map(|c| c.get_max_scroll() <= 0.5)
                    .unwrap_or(true);
                if cannot_scroll {
                    if log {
                        let max = interaction.get_scroll_controller(id).map(|c| c.get_max_scroll());
                        eprintln!("🖐 {} 不可滚（max_scroll={:?}）→ 交给页面", id, max);
                    }
                    action = g.handoff_to_page();
                    continue;
                }
                if let Some(c) = interaction.get_scroll_controller_mut(id) {
                    // 从**按下点**开始拖：锁定那一刻内容不该跳一下
                    c.begin_drag(if axis == Axis::Horizontal { sx } else { sy }, clock_ms);
                    c.update_drag(if axis == Axis::Horizontal { x } else { y }, clock_ms);
                }
                interaction.dragging_scroll_area = Some(id.clone());
                eff.needs_redraw = true;
                break;
            }
            GestureAction::UpdateArea { ref id, axis } => {
                if let Some(c) = interaction.get_scroll_controller_mut(id) {
                    c.update_drag(if axis == Axis::Horizontal { x } else { y }, clock_ms);
                }
                // 内层到边界后还在往同一方向推 → 把这次手势交给页面继续
                // （手指往下推时内容已经到顶、往上推时已经到底）。
                // 不能用「位置没变」判断：控制器有橡皮筋越界，到边界后位置照样在动。
                let (_, dy) = g.delta();
                let keep_pushing = interaction
                    .get_scroll_controller(id)
                    .map(|c| {
                        c.get_max_scroll() <= 0.5
                            || (c.is_at_top() && dy > 0.0)
                            || (c.is_at_bottom() && dy < 0.0)
                    })
                    .unwrap_or(true);
                eff.needs_redraw = true;
                if axis == Axis::Vertical && keep_pushing {
                    if let Some(id) = interaction.dragging_scroll_area.take() {
                        if let Some(c) = interaction.get_scroll_controller_mut(&id) {
                            c.end_drag();
                        }
                    }
                    action = g.handoff_to_page();
                    continue;
                }
                break;
            }
            GestureAction::BeginPage => {
                let (_, ly) = g.last();
                scroll.begin_drag(ly, clock_ms);
                scroll.update_drag(y, clock_ms);
                eff.page_scrolled = true;
                break;
            }
            GestureAction::UpdatePage => {
                scroll.update_drag(y, clock_ms);
                eff.page_scrolled = true;
                break;
            }
            GestureAction::BeginEdgeBack => {
                // 手势升级成侧滑返回：先把可能已经开始的滚动收掉，
                // 否则页面会一边被推出去一边继续上下滚。
                if let Some(id) = interaction.dragging_scroll_area.take() {
                    if let Some(c) = interaction.get_scroll_controller_mut(&id) {
                        c.end_drag();
                    }
                }
                scroll.end_drag();
                let (sx, _) = g.start();
                let mut eb = EdgeBack::new(sx, clock_ms, viewport_width);
                eb.on_move(x, clock_ms);
                *edge_back = Some(eb);
                // 这里也**不置** needs_redraw：页面内容一点没变，变的只是上屏时整帧
                // 往右挪多少（present 每帧都跑），置脏等于每帧白重画一次整页。
                break;
            }
            GestureAction::UpdateEdgeBack => {
                if let Some(eb) = edge_back {
                    eb.on_move(x, clock_ms);
                }
                break;
            }
        }
    }
    *gesture = Some(g);
    eff
}

/// 抬手时收尾正在进行的拖动：清按压态、结束 `scroll-view` 拖拽、结束页面拖拽。
///
/// 返回页面滚动是否进入了惯性/回弹动画（宿主据此决定要不要继续出帧）。
pub fn end_drags(
    interaction: &mut InteractionManager,
    scroll: &mut ScrollController,
    edge_back: &mut Option<EdgeBack>,
    gesture: &mut Option<DragGesture>,
) -> bool {
    interaction.clear_button_pressed();
    if let Some(id) = interaction.dragging_scroll_area.take() {
        if let Some(c) = interaction.get_scroll_controller_mut(&id) {
            c.end_drag();
        }
    }
    let animating = scroll.end_drag();
    // 侧滑返回：松手进收尾动画（推到底真返回 / 不够阈值滑回去），由宿主逐帧推进
    if let Some(eb) = edge_back {
        eb.release();
    }
    *gesture = None;
    animating
}
