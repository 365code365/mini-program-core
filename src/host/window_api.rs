//! 窗口级的几个运行时 API：`wx.setBackgroundColor`、`wx.hideKeyboard`。
//!
//! - 背景色：微信里是**窗口**的背景（页面没铺满、下拉回弹时露出来的那块）。引擎的帧底色本来是
//!   「页面 CSS 的 page 背景，没有就 #F5F5F5」，这里在两者之间插一层：页面没写 page 背景时，
//!   用 setBackgroundColor 给的颜色。页面写了 page 背景时它盖住整帧，与从前一样。
//!   换页（`clear_page_state`）即失效 —— 微信里它也只作用于当前页面。
//!   取舍：微信在 page 背景**之外**的回弹区仍露窗口色；本引擎的回弹区画的是帧底色，
//!   页面写了 page 背景时回弹区也是 page 背景色（要分开画得改合成层次，暂不做）。
//! - 收键盘：等同于让聚焦的输入框失焦（`bindblur` 照发）。

use crate::color::Color;
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::{InteractionManager, InteractionResult};
use std::cell::Cell;

thread_local! {
    static BG: Cell<Option<Color>> = const { Cell::new(None) };
}

/// 帧底色：页面 CSS 背景 > setBackgroundColor > #F5F5F5
pub fn background(page_bg: Option<Color>) -> Color {
    page_bg.or_else(|| BG.with(|b| b.get())).unwrap_or(Color::from_hex(0xF5F5F5))
}

/// 同上，`0xRRGGBB` 形式（桌面窗体的 softbuffer 路径用）
pub fn background_rgb(page_bg: Option<Color>) -> u32 {
    let c = background(page_bg);
    ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32
}

/// 换页时清掉（只对当前页面生效）
pub fn clear() {
    BG.with(|b| b.set(None));
}

/// 每帧：取走 backgroundColor / hideKeyboard 请求。返回 (要重画, 有输入框因此失焦)
pub fn take_requests(app: &mut MiniApp, renderer: Option<&WxmlRenderer>, interaction: &mut InteractionManager) -> (bool, bool) {
    let mut redraw = false;
    for req in app.take_host_requests("backgroundColor") {
        let c = req
            .get("backgroundColor")
            .or_else(|| req.get("backgroundColorTop"))
            .and_then(|v| v.as_str())
            .and_then(super::tabbar::parse_color);
        if let Some(c) = c {
            BG.with(|b| b.set(Some(c)));
            redraw = true;
        }
    }
    let mut blurred = false;
    if !app.take_host_requests("hideKeyboard").is_empty() {
        if let Some(InteractionResult::InputBlur { id, value, .. }) = interaction.blur_input() {
            super::tap::dispatch_input_event(app, renderer, &id, "blur", &value);
            blurred = true;
            redraw = true;
        }
    }
    (redraw, blurred)
}
