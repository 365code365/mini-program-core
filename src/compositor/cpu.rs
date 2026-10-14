//! CPU 合成器：参考实现，永远可用、永远不删。
//!
//! 合成顺序（从下到上）与原来四个宿主调用点里抄的那一串**完全相同**：
//!
//! 1. 页面 → tabBar → fixed 覆盖层（`present_to_buffer`）
//! 2. 下拉刷新指示器：画在页面之上、Toast/Modal 之下
//! 3. Toast / Loading / Modal
//! 4. picker 面板：在所有覆盖层之上
//! 5. 左边缘侧滑返回：整帧右移、左边露出上一页。放在最后 —— 被推走的是「整个页面」，
//!    页面自己的弹窗、Toast 跟着一起走才对，不然弹窗会诡异地钉在原处
//!
//! 每一步调用的函数本身一行没改，只是换了调用者。`Option` 为 `None` 的层跳过；
//! `render_ui_overlay` 在三个状态都是 `None` 时什么也不写，所以只要页面那一层的
//! 调用方（侧滑留图）照样传 [`OverlayRefs::none`](super::OverlayRefs::none)，结果与只调
//! `present_to_buffer` 逐字节相同。
//!
//! **不初始化输出缓冲**：tabBar 画布比 `tabbar_px` 矮时，那几行 `present_to_buffer`
//! 不写，保留缓冲原值。原来各调用点的初值各不相同（softbuffer 借出的缓冲、快照的 0、
//! 侧滑留图的页面底色），这里不能替它们决定。
use super::FrameLayers;
use crate::host::render::{present_to_buffer, render_pull_indicator};
use crate::host::{edge_back, picker_sheet, ui_overlay};

/// 把 `layers` 合成进 `buf`（0x00RRGGBB，行宽 = `layers.width`）
pub fn compose(buf: &mut [u32], layers: &FrameLayers<'_>) {
    let (w, h) = (layers.width, layers.height);
    present_to_buffer(
        buf,
        w,
        h,
        layers.page,
        layers.fixed,
        layers.tabbar,
        layers.scroll_px,
        layers.has_tabbar,
        layers.tabbar_px,
        layers.fixed_rows,
        layers.page_bg,
    );
    if let Some(p) = layers.pull {
        render_pull_indicator(
            buf,
            w,
            h,
            layers.scale,
            p.gap_logical,
            p.refreshing,
            p.progress,
            p.phase,
        );
    }
    let o = &layers.overlays;
    ui_overlay::render_ui_overlay(
        buf,
        w,
        h,
        layers.scale,
        o.last_frame,
        o.toast,
        o.loading,
        o.modal,
        o.text,
    );
    if let Some(sheet) = o.picker {
        picker_sheet::render(buf, w, h, layers.scale, sheet, o.text);
    }
    if let Some(eb) = &layers.edge_back {
        edge_back::compose(buf, w, h, eb.offset_px, eb.shot, eb.progress);
    }
}
