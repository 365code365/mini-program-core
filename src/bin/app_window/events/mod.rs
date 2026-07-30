//! winit 事件 → 引擎调用的适配层。
//!
//! 点击命中与事件派发（原 `mouse.rs`）与平台无关，已搬到
//! `mini_render::host::tap`，移动端 SDK 用同一份。
pub mod keyboard;
pub mod ime;
pub mod wheel;

pub use mini_render::host::tap as mouse;
