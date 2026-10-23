//! 应用运行时

mod app;
/// update() 里的 WebSocket / 音频事件（从 app.rs 拆出）
mod app_io;
/// 宿主事件与 hostCall（从 app.rs 拆出）
mod app_host;
mod window;
/// `{{m.f(x)}}` 的执行方：渲染层表达式引擎 → 逻辑层 QuickJS
mod wxs_host;

pub use app::{MiniApp, UiEvent};
pub use window::Window;
