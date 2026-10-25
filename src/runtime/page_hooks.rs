//! 宿主调页面生命周期（onLoad / onShow / onReady）的唯一入口。
//!
//! 从前各宿主各写一句 `eval("…onLoad(q)").ok()`：onLoad 里抛的异常连同调用栈一起被 `.ok()` 吞掉，
//! 页面一片空白、日志里一个字没有（travel 的 `T.get(q.id)` 写错就是这么藏住的）。
//! 微信里页面生命周期抛错会打到控制台、并走 `App.onError` / `wx.onError`，这里照做。
use super::app::MiniApp;

impl MiniApp {
    /// 调当前页面的生命周期 `name`（没定义就什么也不做），`arg` 是一段 JS 表达式（如 query 的 JSON）。
    /// 抛错时打 `console.error` 并派发 `App.onError`，返回 false
    pub fn call_page_hook(&self, name: &str, arg: &str) -> bool {
        let code = format!("if(__currentPage&&__currentPage.{name})__currentPage.{name}({arg})");
        let Err(e) = self.eval(&code) else { return true };
        let msg = format!("[Page.{name}] {e}");
        eprintln!("❌ {msg}");
        let lit = serde_json::Value::String(msg).to_string();
        self.eval(&format!("console.error({lit}); if (typeof __dispatchApp === 'function') __dispatchApp('onError', {lit});"))
            .ok();
        false
    }
}
