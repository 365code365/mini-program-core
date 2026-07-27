//! 点击处理模块

use mini_render::runtime::MiniApp;
use mini_render::text::TextRenderer;
use mini_render::ui::interaction::InteractionManager;
use mini_render::ui::ScrollController;
use mini_render::renderer::WxmlRenderer;
use std::sync::Arc;
use winit::window::Window;

use super::{
    LOGICAL_WIDTH, LOGICAL_HEIGHT,
    NavigationRequest, TabBarConfig,
    handle_native_tabbar_click,
    ui_overlay::ModalState,
};
use super::events::mouse;
use super::interaction_handler::{handle_interaction_result, check_navigation, print_js_output};

/// Modal 点击处理结果
pub enum ModalClickResult {
    /// 按下了按钮
    ButtonPressed(String),
    /// 释放了按钮，需要执行回调
    ButtonReleased { button: String, should_callback: bool },
    /// 没有点击到按钮
    None,
}

/// 计算 Modal 布局参数
pub struct ModalLayout {
    pub modal_x: f32,
    pub modal_y: f32,
    pub modal_width: f32,
    pub modal_height: f32,
    pub button_y: f32,
    pub button_height: f32,
}

/// 计算 Modal 布局
pub fn calculate_modal_layout(modal: &ModalState, sf: f32, text_renderer: Option<&TextRenderer>) -> ModalLayout {
    let modal_width = 280.0 * sf;
    let modal_padding = 24.0 * sf;
    let title_font_size = 17.0 * sf;
    let content_font_size = 14.0 * sf;
    let button_height = 50.0 * sf;
    let gap = 16.0 * sf;
    
    let title_line_height = title_font_size * 1.4;
    let content_max_width = modal_width - modal_padding * 2.0;
    let content_lines = if let Some(tr) = text_renderer {
        let text_width = tr.measure_text(&modal.content, content_font_size);
        ((text_width / content_max_width).ceil() as i32).max(1)
    } else { 1 };
    let content_line_height = content_font_size * 1.6 * content_lines as f32;
    
    let modal_height = modal_padding + title_line_height + gap + content_line_height + gap + button_height;
    
    let modal_x = (LOGICAL_WIDTH as f32 * sf - modal_width) / 2.0 / sf;
    let modal_y = (LOGICAL_HEIGHT as f32 * sf - modal_height) / 2.0 / sf;
    let button_y = modal_y + (modal_height - button_height) / sf;
    
    ModalLayout {
        modal_x,
        modal_y,
        modal_width: modal_width / sf,
        modal_height: modal_height / sf,
        button_y,
        button_height: button_height / sf,
    }
}

/// 检测 Modal 按钮点击
pub fn detect_modal_button(x: f32, y: f32, layout: &ModalLayout, show_cancel: bool) -> Option<String> {
    if y >= layout.button_y && y <= layout.button_y + layout.button_height {
        if x >= layout.modal_x && x <= layout.modal_x + layout.modal_width {
            if show_cancel {
                let button_width = layout.modal_width / 2.0;
                if x < layout.modal_x + button_width {
                    return Some("cancel".to_string());
                } else {
                    return Some("confirm".to_string());
                }
            } else {
                return Some("confirm".to_string());
            }
        }
    }
    None
}

/// 处理内容区域点击
#[allow(clippy::too_many_arguments)]
pub fn handle_content_click(
    x: f32, y: f32,
    scroll: &ScrollController,
    has_tabbar: bool,
    interaction: &mut InteractionManager,
    renderer: Option<&WxmlRenderer>,
    app: &mut MiniApp,
    scale_factor: f64,
    text_renderer: Option<&TextRenderer>,
    window: Option<&Arc<Window>>,
    clipboard: &mut Option<arboard::Clipboard>,
    // 本次 tap 所属触点的标识与页面时间戳（写进事件对象）
    tap_ctx: (u32, u64),
) -> Option<NavigationRequest> {
    let scroll_pos = scroll.get_position();
    
    if let Some(result) = mouse::handle_content_click(
        x, y, scroll_pos, has_tabbar,
        interaction,
        renderer,
        app,
        scale_factor,
        text_renderer,
        tap_ctx,
    ) {
        handle_interaction_result(
            &result,
            window,
            renderer,
            app,
            clipboard,
            scroll_pos,
            scale_factor,
        );
    }
    
    let nav = check_navigation(app);
    print_js_output(app);
    nav
}

/// 处理自定义 TabBar 点击
pub fn handle_custom_tabbar_click(
    x: f32, y: f32,
    tabbar_renderer: Option<&WxmlRenderer>,
    current_path: &str,
) -> Option<NavigationRequest> {
    if let Some(renderer) = tabbar_renderer {
        if let Some(binding) = renderer.hit_test(x, y) {
            if let (Some(index_str), Some(path)) = (binding.data.get("index"), binding.data.get("path")) {
                if let Ok(_index) = index_str.parse::<usize>() {
                    if path != current_path {
                        println!("👆 TabBar -> {}", path);
                        return Some(NavigationRequest::SwitchTab { url: path.clone() });
                    }
                }
            }
        }
    }
    None
}

/// 处理原生 TabBar 点击
pub fn handle_native_tabbar_click_wrapper(
    x: f32,
    tab_bar: &TabBarConfig,
    current_path: &str,
) -> Option<NavigationRequest> {
    if let Some(target_path) = handle_native_tabbar_click(tab_bar, x, current_path) {
        Some(NavigationRequest::SwitchTab { url: target_path })
    } else {
        None
    }
}
