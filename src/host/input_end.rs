//! `host::input` 的一片：停掉惯性、滚动是否接管了这次触摸、抬手收尾（从 `input.rs` 原样搬出）
use super::super::edge_back::EdgeBack;
use super::super::gesture::DragGesture;
use crate::ui::interaction::{scroll_x_key, InteractionManager};
use crate::ui::ScrollController;

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
    // swiper 接手了这次拖动：与滚动接管一样补 touchcancel、不再算点击
    if crate::renderer::components::swiper_is_dragging() {
        return true;
    }
    let took = |c: &ScrollController| c.is_dragging && c.get_position() > 0.0 || c.is_animating();
    let Some(id) = interaction.dragging_scroll_area.as_ref() else { return false };
    if interaction.get_scroll_controller(id).is_some_and(took) {
        return true;
    }
    // 横竖都能滚的横向那一半（scroll_2d.rs）
    interaction.scroll_is_2d(id) && interaction.get_scroll_controller(&scroll_x_key(id)).is_some_and(took)
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
    match gesture.as_ref().map(|g| g.target().clone()) {
        Some(crate::host::gesture::GestureTarget::Swiper { id, .. }) => crate::renderer::components::swiper_drag_end(&id),
        // 拖过视频的进度条：跳到松手的位置（一次）
        Some(crate::host::gesture::GestureTarget::VideoSeek { .. }) => {
            crate::renderer::components::video_ui::seek_end();
        }
        _ => crate::renderer::components::video_ui::disarm_seek(),
    }
    interaction.end_area_drag();
    let animating = scroll.end_drag();
    // 侧滑返回：松手进收尾动画（推到底真返回 / 不够阈值滑回去），由宿主逐帧推进
    if let Some(eb) = edge_back {
        eb.release();
    }
    *gesture = None;
    animating
}
