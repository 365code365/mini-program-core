//! Mini App Engine - 小程序渲染引擎
//! 支持基本图形绑制、UI组件、事件处理、QuickJS 脚本

// 按职责分的几组（各目录 mod.rs 的模块注释里写了每组管什么）：
// - 图形：`canvas`（画布）+ `gfx`（颜色 / 几何 / 画笔 / 路径）
// - 文字：`text`（字体加载、排版、font-family、@font-face、emoji）
// - 小程序包：`app`（目录定位、包种类、资源根、usingComponents、「关于」）
// - 机器环境：`sys`（数据目录、storage 落盘、剪贴板、帧时钟、内存驻留）
// - 网络：`net`（request / 流式正文 / WebSocket / TCP·UDP）；文件：`wxfs`（含 wxfile:// 沙盒）
// - 解析 → 运行时 → 渲染 → 合成 → 宿主：`parser` / `js` + `runtime` / `renderer` + `ui` / `compositor` / `host`
// - 对外接口：`ffi`（C ABI 与 Android JNI，全部再导出到根上）
mod canvas;
mod gfx;
pub mod text;
pub mod app;
pub mod sys;

/// 小游戏运行实例（game.json + game.js；与 `host::MiniEngine` 平级，见 game-doc/02 §2）
#[cfg(feature = "game")]
pub mod game;

/// `wx.request` 等网络 API 的原生实现（后台线程 + 按帧取回），以及流式正文、WebSocket、TCP / UDP
pub mod net;

/// 引擎内置的官方组件（page-container、match-media、root-portal、page-meta、手势组件……）：WXML + JS 实现，页面用到才注入
pub mod builtin_components;

/// `wx.getFileSystemManager()`：路径权限（USER_DATA_PATH 可写）与文件操作、unzip、图片信息；`wxfile://` 沙盒
pub mod wxfs;

/// `wx.createInnerAudioContext()`：解码、按时钟走的播放位置、事件
pub mod audio_ctx;

pub use canvas::Canvas;
pub use gfx::color::Color;
pub use gfx::geometry::{pixel_rect, snap_out, IRect, Point, Rect};
pub use gfx::paint::{Paint, PaintStyle};
pub use gfx::path::Path;
pub use text::TextRenderer;

// UI 组件系统
pub mod ui;

// JS 引擎绑定
pub mod js;

// 应用运行时
pub mod runtime;

// WXML/WXSS 解析器
pub mod parser;

// UI 渲染器
pub mod renderer;

// 多端编译器框架（HTML / 未来 Android、iOS ...）
pub mod compiler;

/// 宿主层：把小程序跑起来所需的、与平台无关的那部分（页面栈、覆盖层、
/// 触摸状态机、像素合成…）。桌面窗体与移动端 SDK 共用这一份。
pub mod host;
/// 合成层：本帧各图层（页面 / tabBar / fixed / 覆盖层）→ 输出缓冲。
/// CPU 合成器是参考实现；移动端构建（`--no-default-features`）同样编进来。
pub mod compositor;

/// C ABI（`mr_*`）与 Android JNI：画布级、App 级、渲染后端、嵌入宿主、小游戏（见 ffi/mod.rs）
mod ffi;
pub use ffi::*;

// 单元测试
#[cfg(test)]
mod tests;

/// 内存回收：宿主收到系统内存告警（iOS `didReceiveMemoryWarning`、
/// Android `onTrimMemory`、鸿蒙 `onMemoryLevel`）或退到后台时调用。
///
/// 放掉的是「可以重建的东西」：
/// - 解码后的图片与动图帧（下次要用时重新解码）；
/// - 自定义 `font-family` 的字体（下次要用时重新解析）。
///
/// **不动**系统主字体：它是所有文字的兜底，放掉下一帧就要重新解析几百 MB，
/// 反而会在内存最紧张的时候制造一次尖峰。
///
/// 正在使用中的资源不会被打断：调用方手上的 `Arc` 仍然有效，当前帧照常画完。
pub fn trim_memory() {
    renderer::components::clear_image_caches();
    canvas::clear_scaled_image_caches();
    text::family::clear_caches();
    // 大块渐变的行缓存（本线程的）与系统字体的字形位图：都是可以重算的
    renderer::components::gradient::row_cache::clear();
    if let Some(t) = text::shared_fonts() {
        t.clear_glyph_bitmaps();
    }
}

/// 各缓存的占用现状（一行文本，接内存监控/日志用）
pub fn memory_report() -> String {
    let (scaled, mip) = canvas::scaled_image_cache_bytes();
    format!(
        "{}；缩放缓存 {:.1}MB + mip {:.1}MB；自定义字体 {} 个（{}）；{}",
        renderer::components::image_cache_report(),
        scaled as f32 / 1048576.0,
        mip as f32 / 1048576.0,
        text::family::loaded_font_files().len(),
        text::family::loaded_font_files().join(", "),
        text::mem::report(),
    )
}
