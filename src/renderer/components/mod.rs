    //! 微信小程序组件实现
//! 每个组件独立文件，便于维护

mod base;
mod color_parse;
pub(crate) mod gradient;
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
pub(crate) mod image_cache;
mod video;
pub mod video_audio;
/// 同层渲染的通用原生组件（`sameLayer` 里除 video 以外的标签，如 `<map>`）：宿主画好帧交给引擎画进页面
pub mod host_view;
/// 宿主没接管的原生组件（map / camera / web-view ...）的占位与几类标签的默认样式
pub mod native_placeholder;
/// `<grid-view>`：对齐网格（taffy Grid）与瀑布流（布局后按最矮列放置）
pub mod grid_view;
mod canvas;
mod swiper;
mod rich_text;
mod picker;
/// `<picker-view>` 内嵌滚轮：每列的位置、跟手 / 吸附、change / pickstart / pickend
pub mod picker_view;
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
pub use slider::{SliderComponent, SliderAttrs, slider_track_span, SLIDER_VALUE_WIDTH};
pub use input::{InputComponent, is_password_input, masked_value, cursor_blink_visible, blink_visible_at, cursor_blink_interval_ms,
    reset_cursor_blink, last_caret_rect, layout_textarea_lines};
pub use image::{ImageComponent, is_animated, animated_frame, animation_total_ms, image_cache_report, image_cache_bytes, clear_image_caches, image_cache_budget_mb, image_rgba};
mod image_state;
pub use image_state::{probe_load, ImageLoad};
pub mod image_net;
pub use video::VideoComponent;
pub use video::host_feed;
pub use video::{
    set_host_owns_video,
    has_playing_video, get_or_create_player, get_video_frame, get_video_progress,
    toggle_video_play, is_video_playing,
    clear_fullscreen, draw_fullscreen, fullscreen_video, video_command, video_events_pending,
    video_poll_events, video_position, VideoCommand, clear_danmu, progress_track, send_danmu,
};
/// 控制条的状态 / 几何、全屏那一层（两种播放器共用，见 video/ui.rs、chrome.rs、fullscreen.rs）
pub use video::{chrome as video_chrome, fullscreen as video_fullscreen, ui as video_ui};
pub use canvas::{CanvasComponent, Canvas2DContext, CanvasContextManager, LinearGradient, RadialGradient, execute_canvas_draw, ensure_canvas_context, take_canvas_dirty, canvas_key, canvas_measure_text};
// 单测直接看画布后备缓冲：页面合成结果依赖进程级的 CANVAS_DIRTY，并行的别的用例 pump 时
// 可能先把它取走，拿页面像素断言会偶发失败
// 读像素 / 导出的同步桥（js::bridge_io）与单测要直接摸管理器
pub(crate) use canvas::CANVAS_MANAGER;
pub use swiper::{SwiperComponent, SwiperItemComponent, SwiperConfig, SWIPER_MANAGER, swiper_needs_frame, has_autoplay_swiper, advance_due_swipers, take_moved_swipers,
    swiper_is_dragging, take_swiper_events, swiper_drag_begin, swiper_drag_update, swiper_drag_end, swiper_set_clock, swiper_clock_ms, swiper_pages,
    swiper_layout_params, Easing as SwiperEasing, SwiperStateManager};
pub use swiper::effects as swiper_effects;
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

/// 清空**进程级**的组件状态（swiper 当前页与自动播放时钟、picker 选中项、canvas 上下文）。
///
/// ## 为什么会有进程级状态
/// 这三样东西的生命周期跟着「页面里的那个组件」，而组件是由渲染器按 id 找回来的，
/// 所以最初就做成了进程内的 `Lazy<Mutex<..Manager>>`。单个宿主跑单个小程序时没问题，
/// **同一进程里开第二个引擎实例就会串**：两个实例的组件 id 生成规则相同，
/// 于是 A 的 swiper 翻到第 2 页，B 里同名的 swiper 也从第 2 页开始。
///
/// 这不是假想：`tools/sdk-parity.sh` 每页新建一个 `MiniEngine`，曾偶发 72% 的差异 ——
/// 详情页的图片轮播继承了上一页轮播的页码与自动播放时钟（时间相关，所以偶发）。
///
/// 彻底解决要把这三份状态挪进引擎实例（改动面覆盖建树、绘制、宿主三处）。在那之前，
/// **新建实例前调一次这个函数**就能拿到确定的起点。
pub fn reset_shared_component_state() {
    if let Ok(mut m) = swiper::SWIPER_MANAGER.lock() {
        *m = swiper::SwiperStateManager::new();
    }
    if let Ok(mut m) = picker::PICKER_MANAGER.lock() {
        *m = picker::PickerStateManager::new();
    }
    if let Ok(mut m) = canvas::CANVAS_MANAGER.lock() {
        *m = canvas::CanvasContextManager::new();
    }
}
