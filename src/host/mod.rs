//! 宿主层：把「一个小程序包」跑起来所需要的、**与平台无关**的那部分。
//!
//! 这些代码原来住在 `src/bin/app_window/` 里（桌面窗体的私有模块），移进 lib 的
//! 原因很直接：**移动端 SDK 需要同一套东西**。放在 bin 里的代码没法被 `.so`/`.a`
//! 里的 FFI 调用，只能另写一份 —— 那就会变成两套渲染宿主，行为迟早分叉
//! （这个仓库已经踩过一次：无交互的 `render()` 不做 swiper 分页，见
//! `doc/引擎测试说明.md` 的 G-13）。
//!
//! 划分标准是**要不要 winit**：
//! - 这里（lib）：页面加载与页面栈、自定义 tabBar、覆盖层（Toast/Loading/Modal）、
//!   picker 面板、触摸状态机与手势仲裁、侧滑返回、像素合成、组件挂载。
//! - 留在 bin：winit 事件 → 引擎调用的适配层（鼠标/键盘/输入法/滚轮）、softbuffer 上屏。
//!
//! 桌面窗体继续 `use mini_render::host::*`，所以只有一份实现。

pub mod component_mount;
/// 平台无关的引擎实例（移动端 SDK 的核心）
pub mod engine;
/// 后端选择与带标志位的出帧（`pump_ex`）
pub mod engine_backend;
/// 出帧：pump → 按需重画 → 合成（从 engine.rs 拆出）
mod engine_frame;
/// 页面画布那一趟前后的记账（换画布、动画包围盒、scroll-view 位置）
mod engine_page_pass;
mod engine_press;
/// 图片预览盖满整屏时只合成预览（页面的重画留到它露出来）
mod engine_preview;
/// 快要滑进屏幕的图先在后台解码、降采样
mod engine_prefetch;
/// swiper 换页只重画它自己的盒子
mod engine_swiper;
mod engine_resample;
mod engine_wants;
/// 同层渲染的视频与宿主播放器的约定（`videos` / `videoCommand` 事件、回传事件）
pub mod engine_video_feed;
/// 同层渲染的通用组件（`<map>` 这类）与宿主的约定（`sameLayerViews` / `nativeViewTap` 事件、回传事件）
mod engine_host_views;
mod frame_laps;
mod engine_damage_rects;
/// 这一帧重画多少（只合成 / 局部 / 补条带 / 整帧）与整帧归因
pub mod engine_damage;
/// setData 之后的失效分析入口（取数据 + 布局 + 比对）
mod engine_damage_data;
/// 布局树诊断：增量重建留了几棵子树、树里有没有漏拿掉的节点
mod engine_layout_diag;
/// 内存分项：每块大缓冲分配 / 驻留多少
pub mod engine_memory;
/// 页面截图的无损压缩（栈顶下面那张以外的都压着）
pub mod shot_pack;
mod engine_input;
/// 路由：返回（与逻辑层页面栈按层数对齐）、逻辑层发起的跳转、宿主深链
mod engine_nav;
/// 进入页面：解析、样式、页面 js 与生命周期、渲染器
mod engine_page;
/// 被盖住的页留视图、返回时放回、tab 页切走时留实例（SDK 一侧）
mod engine_keep;
/// 页面视图的留存（SDK 与桌面共用）：滚动、渲染器、交互状态
pub mod page_keep;
/// 页面转场（navigateTo 滑入 / 返回滑出）
pub mod route_anim;
/// 默认导航栏：几何、画法、命中（引擎画，`navigationBar` 选项）
pub mod navbar;
/// 导航栏的显隐、重画与指针（`MiniEngine` 的一片）
mod engine_navbar;
/// 宿主浮层先吃指针（picker / 全屏视频 / Modal / Loading …）
mod engine_overlay_input;
/// 右上角胶囊：几何、画法、命中（`navigationStyle: custom` 的页面上由引擎画）
pub mod capsule;
/// 胶囊的显隐、配色与指针（`MiniEngine` 的一片）
mod engine_capsule;
/// 左边缘侧滑返回：留图、收尾动画、合成参数
mod engine_edge_back;
/// SDK 的 `MINI_FPS` 统计（FrameStats）
pub mod engine_stats;
/// 创建选项（系统信息、安全区、宿主 API、原生组件）
pub mod engine_options;
/// 引擎 → 宿主的事件队列（页面、导航栏、输入框、原生组件、hostCall、日志）
pub mod engine_events;
/// 宿主原生组件被引擎自己画的层（picker / 弹窗 / fixed / tabBar）挡住多少
mod native_occlusion;
/// nativeViews 带的滚动取样（宿主按自己那一帧的 vsync 推算控件位置）
mod native_motion;
/// 像素出口：RGBA / BGRA、带行跨度直写平台缓冲
pub mod engine_pixels;
/// 尺寸 / 像素比 / 安全区变化
mod engine_resize;
/// 生命周期（前后台 / 内存告警）、动画时钟与引擎时钟
mod engine_life;
/// SDK 的 GPU 上屏（原生 surface → GpuCompositor）
#[cfg(feature = "gpu")]
mod engine_gpu;
/// surface 来之前在后台线程上先开 GPU 合成器（第一次打开快 0.3s）
#[cfg(feature = "gpu")]
mod engine_gpu_warm;
#[cfg(feature = "gpu")]
pub use engine_gpu_warm::warmup;
/// 覆盖层状态：Toast/Loading/Modal 指令落地与查询（从 engine.rs 拆出）
mod engine_ui;
pub mod config;
/// Android：引擎线程绑大核（`MINI_CPU_PIN=0` 关掉，理由见文件头）
pub mod cpu_affinity;
pub mod edge_back;
pub mod gesture;
/// 指针输入的平台无关核心（触摸派发 / 手势仲裁 / 惯性收尾），窗体与 SDK 共用
pub mod input;
pub mod navigation;
pub mod page_loader;
pub mod picker_sheet;
pub mod region_data;
pub mod render;
/// 键盘顶起画面时的合成（导航栏以下整体上移，render.rs `present_to_buffer` 转进来）
mod render_shift;
/// 键盘顶起画面（`adjust-position`）：宿主给目标上移量，引擎按帧推进、合成时整体上移、触摸坐标加回去
mod engine_keyboard;
/// 输入法连着送来的几下（上屏 + 发送键）：页面 setData 过就等它画完一帧再送下一下
mod engine_input_queue;
/// 覆盖层只重画点击态 / 光标那一块（支付密码键盘按键）
mod engine_fixed_partial;
/// 动了的 scroll-view 要重画哪（页面 / 覆盖层分开）
mod engine_damage_area;
pub mod tabbar;
/// 原生 tabBar 的运行时 API（setTabBarBadge / RedDot / Item / Style / show / hide）
pub mod tabbar_api;
/// 点击命中与事件派发（tap / change 的链路）
pub mod tap;
pub mod touch;
/// scroll-view 事件、`wx.pageScrollTo`、`onPageScroll`（SDK 与桌面共用）
pub mod scroll_view;
/// swiper 的 change / transition / animationfinish 派发（SDK 与桌面共用）
pub mod swiper;
/// slider 拖动的 changing / change 派发（SDK 与桌面共用）
pub mod slider_input;
/// 组件自己的事件（不冒泡）派发给逻辑层
mod own_events;
/// `<form>` 的提交与重置（form-type 按钮）
pub mod form;
/// 点击之后内置组件的行为：form 提交 / 重置、navigator 跳转、label 转发
mod builtin_tap;
/// `<video>` 的 VideoContext 命令、组件事件与点击播放（SDK 与桌面共用）
pub mod video;
/// `wx.createSelectorQuery` 与 ScrollViewContext（SDK 与桌面共用）
pub mod selector_query;
/// `wx.createIntersectionObserver`（SDK 与桌面共用）
pub mod observers;
pub mod ui_overlay;
/// wx.setBackgroundColor（帧底色）与 wx.hideKeyboard（输入框失焦）
pub mod window_api;
/// 自定义 tabBar 的活实例（getTabBar、组件 setData 改角标），SDK 与桌面共用
pub mod custom_tabbar_live;

pub use config::*;
pub use navigation::*;
pub use page_loader::{load_all_pages, load_custom_tabbar, load_custom_tabbar_with_app_wxss, CustomTabBar};
pub use render::*;
pub use tabbar::*;
pub use engine::MiniEngine;
pub use engine_options::{EngineOptions, SafeAreaInsets};
pub use engine_pixels::PixelFormat;
pub use ui_overlay::{render_ui_overlay, LoadingState, ModalState, ToastState};

/// 逻辑视口宽（pt/dp）。小程序的设计基准是 750rpx 对应这个宽度。
pub const LOGICAL_WIDTH: u32 = 375;
/// 逻辑视口高（pt/dp）
pub const LOGICAL_HEIGHT: u32 = 667;
/// 页面画布的初始高度（内容更高时按内容扩）
pub const CONTENT_HEIGHT: u32 = 1500;
