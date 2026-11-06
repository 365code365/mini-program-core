//! C ABI 导出（`mr_*`）与 Android JNI 入口。符号名由各函数的 `#[no_mangle]` / `Java_…` 名字定，
//! 与模块路径无关；全部 glob 再导出到 crate 根，`include/mini_render.h`（cbindgen）照旧从这里取。
//!
//! - `canvas`：画布级（`mr_canvas_*` / `mr_path_*`）
//! - `app`：**App 级**（`mr_app_*`）—— 移动端 SDK 用的就是这套
//! - `backend`：渲染后端（`mr_app_set_backend` / `attach_surface` / `pump_ex` …），有没有 `gpu` feature 都导出同一组符号
//! - `embed`：嵌入宿主（Flutter 插件 / 原生 View）：创建选项、改尺寸、安全区、像素直写、事件队列、hostCall 回包、
//!   原生组件事件。只增不改，老接口一个没动
//! - `game`：小游戏（`mr_game_*`、`mr_package_kind`、`mr_capabilities`），入口都有 panic 防护
//! - `jni` / `jni_embed` / `jni_game`：Android 的 JNI（只在 android 目标编译；`jni_embed` 是嵌入宿主那组 +
//!   Bitmap / ANativeWindow 两条像素出口，`jni_game` 是 Kotlin `dev.minirender.MiniGame`）
pub(crate) mod canvas;
pub use canvas::*;

pub(crate) mod app;
pub use app::*;

pub(crate) mod backend;
pub use backend::*;

pub(crate) mod embed;
pub use embed::*;

pub(crate) mod game;
pub use game::*;

#[cfg(target_os = "android")]
pub(crate) mod jni;
#[cfg(target_os = "android")]
pub(crate) mod jni_embed;
#[cfg(all(target_os = "android", feature = "game"))]
pub(crate) mod jni_game;
