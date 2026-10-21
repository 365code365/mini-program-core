//! Modal 的命中与按钮回调（从 ui_overlay.rs 原样搬出）。
//!
//! 这一段最早从桌面窗体（`bin/app_window/click_handler.rs`）搬进 lib：它只依赖弹窗自身的
//! 尺寸算式与视口大小，和窗口系统无关。移动端 SDK 从前自己写了一版**粗糙的**判定
//! （「按下就算命中、抬手时按 x 是否过屏幕中线决定确定/取消」），于是单按钮弹窗在
//! 手机上点左半边会当成取消、点遮罩也会误触发回调。现在两端共用同一份。
//!
//! 命中与绘制用**同一份**几何（`geometry::modal_geom`）。从前这里自己算一遍（按总宽估行数、
//! 行高 1.6），绘制那边逐字换行、行高 1.5，多行内容时按钮的命中区与画出来的差几个像素。
use super::geometry::{modal_geom, MODAL_FOOTER_H};
use super::ModalState;
use crate::text::TextRenderer;

/// Modal 的命中判定用得到的几何量（逻辑像素）
pub struct ModalLayout {
    pub modal_x: f32,
    pub modal_width: f32,
    pub button_y: f32,
    pub button_height: f32,
}

/// 按弹窗内容算出布局。`(vw, vh)` 是视口逻辑尺寸 —— 桌面固定 375×667，
/// 移动端由宿主给（写死常量的话平板/折叠屏上弹窗按钮的命中区会整体错位）。
/// `_sf` 留着是为了不动调用方：几何本来就按逻辑像素算，与设备像素比无关。
pub fn modal_layout(
    modal: &ModalState,
    vp: (f32, f32),
    _sf: f32,
    text_renderer: Option<&TextRenderer>,
) -> ModalLayout {
    let g = modal_geom(modal, vp, text_renderer);
    ModalLayout { modal_x: g.x, modal_width: g.w, button_y: g.footer_y, button_height: MODAL_FOOTER_H }
}

/// 这个点落在哪个按钮上（`"cancel"` / `"confirm"`），没落在按钮上返回 `None`
pub fn modal_button_at(x: f32, y: f32, layout: &ModalLayout, show_cancel: bool) -> Option<String> {
    if y < layout.button_y || y > layout.button_y + layout.button_height {
        return None;
    }
    if x < layout.modal_x || x > layout.modal_x + layout.modal_width {
        return None;
    }
    if show_cancel && x < layout.modal_x + layout.modal_width / 2.0 {
        Some("cancel".to_string())
    } else {
        Some("confirm".to_string())
    }
}

/// 弹窗可见时的按下：命中按钮就记下按压态（给出反馈）。返回是否消费了这次按下。
pub fn modal_press(
    modal: &mut Option<ModalState>,
    (vw, vh): (f32, f32),
    sf: f32,
    text_renderer: Option<&TextRenderer>,
    x: f32,
    y: f32,
) -> bool {
    let Some(m) = modal.as_ref().filter(|m| m.visible) else {
        return false;
    };
    let layout = modal_layout(m, (vw, vh), sf, text_renderer);
    let show_cancel = m.show_cancel;
    match modal_button_at(x, y, &layout, show_cancel) {
        Some(btn) => {
            if let Some(m) = modal.as_mut() {
                m.pressed_button = Some(btn);
            }
            true
        }
        None => false,
    }
}

/// 弹窗可见时的抬手：**按下与抬起落在同一个按钮**才回调逻辑层并关闭
/// （与系统弹窗一致：按下后滑开再松手不算点击）。
pub fn modal_release(
    modal: &mut Option<ModalState>,
    app: &mut crate::runtime::MiniApp,
    (vw, vh): (f32, f32),
    sf: f32,
    text_renderer: Option<&TextRenderer>,
    x: f32,
    y: f32,
) {
    let pressed = modal.as_ref().and_then(|m| m.pressed_button.clone());
    if let Some(m) = modal.as_mut() {
        m.pressed_button = None;
    }
    let Some(m) = modal.as_ref().filter(|m| m.visible) else {
        return;
    };
    let layout = modal_layout(m, (vw, vh), sf, text_renderer);
    let show_cancel = m.show_cancel;
    if let Some(btn) = modal_button_at(x, y, &layout, show_cancel) {
        if pressed.as_deref() == Some(btn.as_str()) {
            // 必须走 JS 侧的 `__handleModalResult(confirm)`：它负责调 success/complete
            // 并清掉 `__modalVisible/__modalConfig/__modalCallback`。
            //
            // 桌面窗体从前写的是 `__modalCallback({confirm:...})` —— 而
            // `__modalCallback` 存的是 `wx.showModal` 的 **options 对象**，
            // 当函数调直接 TypeError，于是 success 回调一次都没执行过
            // （点「确定」只是弹窗消失，业务什么也没发生）。
            let confirm = if btn == "cancel" { "false" } else { "true" };
            app.eval(&format!("__handleModalResult({confirm})")).ok();
            *modal = None;
        }
    }
}
