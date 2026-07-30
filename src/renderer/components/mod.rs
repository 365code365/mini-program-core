    //! 微信小程序组件实现
//! 每个组件独立文件，便于维护

mod base;
mod color_parse;
mod gradient;
mod shadow;
mod style_parse;
mod view;
mod text;
mod button;
mod icon;
pub mod icon_data;
pub mod svg_path;
mod progress;
mod switch;
mod checkbox;
mod radio;
mod slider;
mod input;
pub mod image;
mod image_cache;
mod video;
pub mod video_audio;
mod canvas;
mod swiper;
mod rich_text;
mod picker;
/// 标签 → 组件行为的唯一登记点（build / draw / 叶子 / 绘制归因）
pub mod registry;
mod checkbox_group;

pub use base::*;
pub use view::ViewComponent;
pub use text::TextComponent;
pub use button::ButtonComponent;
pub use icon::IconComponent;
pub use progress::ProgressComponent;
pub use switch::SwitchComponent;
pub use checkbox::CheckboxComponent;
pub use radio::RadioComponent;
pub use slider::SliderComponent;
pub use input::{InputComponent, cursor_blink_visible, blink_visible_at, cursor_blink_interval_ms,
    reset_cursor_blink, last_caret_rect};
pub use image::{ImageComponent, is_animated, animation_total_ms, image_cache_report, clear_image_caches, image_cache_budget_mb};
mod image_state;
pub use image_state::{probe_load, ImageLoad};
pub mod image_net;
pub use video::VideoComponent;
pub use video::{
    has_playing_video, get_or_create_player, get_video_frame, get_video_progress,
    toggle_video_play, is_video_playing,
};
pub use canvas::{CanvasComponent, Canvas2DContext, CanvasContextManager, LinearGradient, RadialGradient, execute_canvas_draw, ensure_canvas_context};
pub use swiper::{SwiperComponent, SwiperItemComponent, SWIPER_MANAGER, swiper_needs_frame, has_autoplay_swiper, advance_due_swipers};
pub use rich_text::RichTextComponent;
pub use picker::{PickerComponent, PickerViewComponent, PickerViewColumnComponent, PickerMode, PICKER_MANAGER};
pub use checkbox_group::{CheckboxGroupComponent, RadioGroupComponent};
pub use registry::{spec_for, DrawCtx, TagSpec};


// 这里曾经有个 `ComponentRegistry::build_component`：**第二份**「标签 → 组件」分派表，
// 21 条 match 分支，与 `wxml_renderer::layout.rs` 里那份逐条重复。
//
// 它是死代码，而且是**会骗人**的死代码：新增组件的人很容易在这里加一条分支，
// 然后发现页面上什么都没变 —— 真正生效的是 layout.rs 那份。它建的 `ComponentContext`
// 里 `ancestors` / `inherited` / `sibling_index` 全是默认值，本来也没法参与
// 样式继承与 `:nth-child`，注定只能是个摆设。所以整块删掉，让分派只有一处。

