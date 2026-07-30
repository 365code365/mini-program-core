//! 点击处理模块

use mini_render::runtime::MiniApp;
use mini_render::text::TextRenderer;
use mini_render::ui::interaction::InteractionManager;
use mini_render::ui::ScrollController;
use mini_render::renderer::WxmlRenderer;
use std::sync::Arc;
use winit::window::Window;

use super::NavigationRequest;
use super::events::mouse;
use super::interaction_handler::{handle_interaction_result, check_navigation, print_js_output};


// Modal 的布局与按钮命中在 `mini_render::host::ui_overlay`（与移动端 SDK 共用），
// tabBar 命中在 `mini_render::host::tabbar::nav_at` —— 这里只剩内容区点击的胶水。

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

// tabBar 命中（自定义 + 原生）已经搬进 `mini_render::host::tabbar::nav_at`
