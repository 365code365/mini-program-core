//! Mini App Engine - 小程序渲染引擎
//! 支持基本图形绑制、UI组件、事件处理、QuickJS 脚本

mod canvas;
mod color;
mod geometry;
mod paint;
mod path;
pub mod text;
/// 一块缓冲真正驻留了多少（mincore）
pub mod mem_probe;
/// `font-family` → 具体字体（按字体栈解析 + 每族一份渲染器）
pub mod text_family;
pub mod text_system;
/// `@font-face` / `wx.loadFontFace` 登记的字族（TTF / OTF）
pub mod text_faces;
// 彩色 Emoji 位图字形（Apple sbix）
pub mod emoji;
// 小程序资源根目录（图片等相对路径解析）
pub mod assets;

/// 示例小程序目录的定位（裸名字 / `sample/xxx` / 任意路径都能解析）
pub mod app_dir;

/// 包种类探测：小程序（app.json）还是小游戏（game.json），两者都有 / 都没有报错
pub mod app_profile;

/// 小游戏运行实例（game.json + game.js；与 `host::MiniEngine` 平级，见 game-doc/02 §2）
#[cfg(feature = "game")]
pub mod game;

/// `wx.request` 等网络 API 的原生实现（后台线程 + 按帧取回）
pub mod net;
/// wx.request 的流式部分：二进制正文、响应头 / 分块 / 下载进度事件
pub mod net_stream;
/// 长连接的二进制直通（不经 base64）
pub mod bin_stash;

/// 引擎的可写数据目录（storage 落盘、图片磁盘缓存），由宿主指定
pub mod data_dir;

/// `wx.setStorageSync` 的跨启动落盘
pub mod storage_file;

/// `wxfile://` 虚拟路径（downloadFile 的产物、USER_DATA_PATH）→ 数据目录下的沙盒
pub mod wxfile;

/// `wx.setClipboardData` / `getClipboardData`（引擎里一份，桌面窗体可同步系统剪贴板）
pub mod clipboard;

/// `wx.connectSocket`：每条连接一个线程，事件每帧取回
pub mod websocket;

/// `wx.createTCPSocket` / `wx.createUDPSocket` / `wx.getLocalIPAddress`：同 websocket 的线程 + 事件队列模式
pub mod net_socket;

/// 引擎内置的官方组件（page-container、match-media、root-portal、page-meta、手势组件……）：WXML + JS 实现，页面用到才注入
pub mod builtin_components;

/// `wx.getFileSystemManager()`：路径权限（USER_DATA_PATH 可写）与文件操作、unzip、图片信息
pub mod wxfs;

/// `wx.createInnerAudioContext()`：解码、按时钟走的播放位置、事件
pub mod audio_ctx;

/// 页面 json 的 `usingComponents`：自定义组件三件套加载与样式作用域
pub mod using_components;

pub use canvas::Canvas;
pub use color::Color;
pub use geometry::{pixel_rect, snap_out, IRect, Point, Rect, Size};
pub use paint::{Paint, PaintStyle};
pub use path::Path;
pub use text::TextRenderer;

// UI 组件系统
pub mod ui;

// 事件系统
pub mod event;

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

/// 「关于小程序」的信息（胶囊「•••」展示的内容；桌面、SDK、H5 共用）
pub mod app_about;

/// 宿主层：把小程序跑起来所需的、与平台无关的那部分（页面栈、覆盖层、
/// 触摸状态机、像素合成…）。桌面窗体与移动端 SDK 共用这一份。
pub mod host;
/// 合成层：本帧各图层（页面 / tabBar / fixed / 覆盖层）→ 输出缓冲。
/// CPU 合成器是参考实现；移动端构建（`--no-default-features`）同样编进来。
pub mod compositor;

/// 帧时钟：这一帧按哪个时刻画（SDK 按宿主 vsync 时间，见模块注释）
pub mod frame_clock;

// FFI 导出：画布级（mr_canvas_* / mr_path_*）
mod ffi;
pub use ffi::*;

/// FFI 导出：**App 级**（mr_app_*）—— 移动端 SDK 用的就是这套
mod ffi_app;
pub use ffi_app::*;

/// FFI 导出：渲染后端（mr_app_set_backend / attach_surface / pump_ex …）。
/// 有没有 `gpu` feature 都导出同一组符号
mod ffi_backend;
pub use ffi_backend::*;

/// FFI 导出：嵌入宿主（Flutter 插件 / 原生 View）用的 C ABI：创建选项、改尺寸、安全区、
/// 像素直写、事件队列、hostCall 回包、原生组件事件。只增不改，老接口一个没动
mod ffi_embed;
pub use ffi_embed::*;

/// 小游戏的 C ABI（`mr_game_*`、`mr_package_kind`、`mr_capabilities`），入口都有 panic 防护
mod ffi_game;
pub use ffi_game::*;

/// Android 的 JNI 入口（只在 android 目标编译）
#[cfg(target_os = "android")]
mod ffi_jni;
/// Android 的 JNI 入口：嵌入宿主那组新接口 + Bitmap / ANativeWindow 两条像素出口
#[cfg(target_os = "android")]
mod ffi_jni_embed;

/// Android：小游戏的 JNI（Kotlin `dev.minirender.MiniGame`）
#[cfg(all(target_os = "android", feature = "game"))]
mod ffi_jni_game;

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
    text_family::clear_caches();
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
        text_family::loaded_font_files().len(),
        text_family::loaded_font_files().join(", "),
        text::mem::report(),
    )
}
