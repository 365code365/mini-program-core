//! 应用运行时

mod app;
/// update() 里的 WebSocket / 音频事件（从 app.rs 拆出）
mod app_io;
/// 宿主事件与 hostCall（从 app.rs 拆出）
mod app_host;
mod app_clock;
/// 页面生命周期的调用入口（异常不再被吞）
mod page_hooks;
mod window;
/// `{{m.f(x)}}` 的执行方：渲染层表达式引擎 → 逻辑层 QuickJS
mod wxs_host;
/// App 级生命周期事件（切前后台）
pub mod event;

pub use app::{MiniApp, UiEvent};
pub use window::Window;
/// 小游戏高性能模式（game/web）的宿主回包与 hostCall 走同一套文件翻译
pub(crate) use app_host::{annotate_host_call, import_reply_files};
