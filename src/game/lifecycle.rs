//! 生命周期与包（game-doc/04 G6.1 / G6.2）：JS 侧在 game-lifecycle.js。
//! - 建实例时（前置代码之后、game.js 之前）把分包表交给 JS；从独立分包启动时返回它的入口模块键；
//! - 前后台切换时派发 onHide / onShow（frame.rs 的 set_background 调）。

use super::GameEngine;
use serde_json::json;

impl GameEngine {
    /// 分包表 → `__gameSubpackages` 等全局；`entry`：创建选项 entrySubpackage。返回要执行的独立分包入口（None = 执行 game.js）
    pub(super) fn install_packages(&mut self, entry: Option<&str>, plugin_dir: Option<&str>) -> Option<String> {
        let (plugins, plogs) = super::plugin::install(&self.app, &self.root, plugin_dir.map(std::path::Path::new), &self.config);
        for l in plogs {
            self.host_log(&l);
        }
        let (subs, warns) = super::package::describe(&self.root, &self.config);
        for w in warns {
            self.host_log(&format!("⚠ {w}"));
        }
        let main = super::package::main_bytes(&self.root, &self.config);
        let picked = match entry.map(|n| super::package::entry_subpackage(&self.config, n)) {
            Some(Ok((name, key))) => Some((name.to_string(), key)),
            Some(Err(e)) => {
                self.host_log(&format!("⚠ {e}；从主包启动"));
                None
            }
            None => None,
        };
        let entry_name = picked.as_ref().map(|p| json!(p.0)).unwrap_or(json!(null));
        let js = format!("globalThis.__gameSubpackages = {subs}; globalThis.__gameEntrySub = {entry_name}; globalThis.__gameMainBytes = {main}; globalThis.__gamePlugins = {plugins};");
        if let Err(e) = self.app.eval(&js) {
            self.report_error("script", &e);
        }
        if let Some((name, _)) = &picked {
            self.host_log(&format!("🎮 从独立分包 {name:?} 启动：主包要 loadSubpackage({{name:'__GAME__'}})"));
        }
        picked.map(|p| p.1)
    }

    /// 宿主推一个事件给 `wx.onXxx`（键盘输入 / 确认 / 完成、键盘高度、传感器、网络、主题……，与 `mr_app_dispatch_event` 同一套名字）。
    /// 返回收到它的监听者个数
    pub fn dispatch_event(&mut self, name: &str, payload_json: &str) -> u32 {
        self.guards.watchdog.arm();
        let r = self.app.eval(&format!("typeof __gameHostEvent === 'function' ? __gameHostEvent({}, {}) : 0",
            serde_json::json!(name), serde_json::json!(payload_json)));
        self.guards.watchdog.disarm();
        let n = match r {
            Ok(v) => v.parse::<f64>().unwrap_or(0.0) as u32,
            Err(e) => {
                self.report_error(super::error_kind(&e, "event"), &e);
                0
            }
        };
        self.after_js();
        n
    }

    /// 派发 wx.onShow / wx.onHide（`kind` = "show" / "hide"）
    pub(super) fn dispatch_lifecycle(&mut self, kind: &str) {
        self.guards.watchdog.arm();
        let r = self.app.eval(&format!("typeof __gameLifecycle === 'function' && __gameLifecycle('{kind}')"));
        self.guards.watchdog.disarm();
        if let Err(e) = r {
            self.report_error(super::error_kind(&e, "lifecycle"), &e);
        }
        self.after_js();
    }
}
