//! 逻辑层：QuickJS 运行时、`wx.*` 的实现、以及 JS ↔ Rust 的桥。
//!
//! ## 三层结构（加一个 `wx.*` API 要动哪些地方）
//!
//! | 层 | 位置 | 职责 |
//! |---|---|---|
//! | ① JS 前置代码 | [`api`] 的注册表 → `src/js/prelude/*.js` | 实现 `wx.xxx`：参数缺省值、回调登记、返回值形状。**微信语义在这一层对齐** |
//! | ② 原生入口 | [`bridge`] | `register_function("__native_xxx", …)`：把请求塞进事件队列，或直接干活（storage 写盘、提交网络请求） |
//! | ③ 宿主消费 | [`crate::runtime::MiniApp::update`] → `crate::host` | 每帧取走事件队列，真正执行（弹 Toast、起定时器、改页面栈） |
//!
//! 这三层的内容各不相同，**不该也无法合并**。真正的风险是层与层之间对不上：
//! JS 调了一个没人注册的 `__native_xxx`，QuickJS 只抛
//! `TypeError: not a function` —— 没有名字也没有栈，跑第三方编译产物时极难定位。
//! 所以有一组契约测试盯着它，不用维护任何清单
//! （见 `src/tests/native_bridge_contract_tests.rs`）：
//!
//! - prelude 引用到的每个 `__native_*` 都必须在 `bridge.rs` 注册过；
//! - `bridge.rs` 注册的每个函数都必须真有 JS 在调（否则是改名时漏掉的残留）；
//! - 引导完成后，每个名字在运行时**确实是** `function`（防止注册被错误吞掉）。
//!
//! ## 加一个 API 的顺序
//! 1. 在对应域的 `prelude/*.js` 里写 `wx.xxx`（能纯 JS 实现的就别下到原生）；
//! 2. 需要原生能力时，在 `bridge.rs` 对应的 `register_*_functions` 里加入口，
//!    push 一个 [`BridgeEvent`]；
//! 3. 在 `runtime/app.rs` 的事件循环里消费它，必要时转成 `UiEvent` 交给宿主；
//! 4. 跑 `cargo test --release --lib native_bridge` —— 契约测试会告诉你哪一层漏了。

/// QuickJS 运行时封装（求值、注册原生函数、模块作用域）
mod runtime;
/// `wx.*` 的 JS 前置代码注册表（真正的实现在 `src/js/prelude/*.js`）
mod api;
/// JS ↔ Rust 桥：原生函数入口与事件队列
mod bridge;

pub use runtime::JsRuntime;
pub use api::MiniAppApi;
pub use bridge::{JsBridge, BridgeEvent};
