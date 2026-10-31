//! 小游戏的「高性能模式」（对标微信 game.json 的 `iOSHighPerformance`，见 game-doc/12-高性能模式.md）：
//! 游戏 JS 不进 QuickJS，而是跑在系统 WebView（iOS 的 WKWebView）里 —— JavaScriptCore 带 JIT，
//! WebGL 与 Canvas 2D 都是 WebKit 自己的 GPU 实现。
//!
//! 为什么要它：iOS 不许第三方 App 自己 JIT，QuickJS 只能解释执行；真机上萌宠小岛稳态一半以上的 CPU 在执行 JS，
//! 2D 画布是 CPU 光栅再整张传纹理（doc/滑动顿挫避坑清单.md 2026-10-31）。微信官方给的数字是同一个游戏
//! 高性能模式下帧率明显提高、长时间运行 CPU 更低（不那么烫）。WKWebView 是 App Store 允许的唯一 JIT 通道
//! （审核指南 2.5.6 / 4.7：HTML5 与 JS 写的小游戏可以下发，要跑在 WebKit 里）。
//!
//! 分工（与 QuickJS 模式共用同一份「基础库」）：
//! - **页面里**：`js/prelude/` 里与平台无关的那几段原样装进去（storage / ui / device / network / host /
//!   fs / 生命周期 / 键盘 / 宿主代理……，[`boot::SEGMENTS`]），它们调的 `__native_*` 换成过桥的版本
//!   （`js/web_game/wg-env.js`）；画布、图片、WebGL、音频、rAF、触摸换成 DOM 的（`js/web_game/*.js`）。
//! - **这里（原生）**：系统信息 / 胶囊 / 启动参数与 QuickJS 模式同一套算法（engine_options、chrome.rs），
//!   storage、wxfile、网络、FileSystemManager 也走同一份 Rust 实现 —— 两种模式之间存档互通、行为一致。
//! - **宿主**（iOS 的 `MiniWebGameView.swift`，桌面的 `examples/web_game_serve.rs`）只是传话：
//!   把页面的请求交给 [`WebGame::call`] / [`WebGame::post`]，把 [`WebGame::take_js`] 的结果求值回页面，
//!   把 [`WebGame::take_event`] 的宿主事件（hostCall、键盘、振动、退出……）按 QuickJS 模式同样的语义处理。
//!
//! 页面与原生之间只有两种消息：同步（返回字符串；iOS 走 `prompt()`，桌面走同步 XHR）与异步（不等返回）。
//! 消息体一律是 `{"n": 名字, "a": [参数…]}`，参数与 `__native_*` 的原生签名一一对应（都是字符串）。

mod boot;
mod natives;
mod resource;
/// 桌面调试：HTTP 扮演宿主、headless Chrome 驱动（测试与 examples/web_game_check.rs 用；移动端不编）
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub mod cdp;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub mod devserver;

use super::config::GameConfig;
use super::GameOptions;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::PathBuf;

pub use resource::{unlocked as unlocked_resource, Resource};

pub struct WebGame {
    root: PathBuf,
    config: GameConfig,
    app_id: String,
    /// 系统信息（`wx.getSystemInfoSync()`，与 QuickJS 模式同一套口径）
    info: Value,
    launch: Value,
    host_apis: Vec<String>,
    opts_system_info: serde_json::Map<String, Value>,
    safe_area: Option<crate::host::SafeAreaInsets>,
    entry_subpackage: Option<String>,
    /// 网络请求的归属号（net.rs）
    net_owner: u64,
    /// 要在页面里执行的 JS（网络回包、宿主回包、生命周期）
    js: VecDeque<String>,
    /// 交给宿主的事件（JSON 串，与 `mr_game_poll_event` 同形）
    events: VecDeque<String>,
    /// storage（与 QuickJS 模式同一个文件：`<数据目录>/<appId>/storage.json`）
    storage: std::collections::HashMap<String, String>,
    storage_dirty: bool,
    /// 当前窗口（逻辑宽、高、dpr）与方向值（portrait / landscape / landscapeReverse）
    size: (u32, u32, f32),
    orientation: String,
}

impl WebGame {
    /// `dir`：解好的小游戏包；`data_dir`：存档目录（与 QuickJS 模式给的同一个，存档互通）
    pub fn new(dir: &str, data_dir: Option<&str>, width: u32, height: u32, dpr: f32, opts: &GameOptions) -> Result<Self, String> {
        let root = PathBuf::from(dir);
        let text = std::fs::read_to_string(root.join("game.json")).map_err(|e| format!("读不出 game.json：{e}"))?;
        let config = GameConfig::parse(&text)?;
        if !root.join("game.js").is_file() {
            return Err("包里没有 game.js".into());
        }
        if let Some(d) = data_dir {
            crate::data_dir::set_data_dir(d);
        }
        let app_id = opts.app_id.clone().unwrap_or_else(|| root.file_name().and_then(|s| s.to_str()).unwrap_or("game").to_string());
        crate::data_dir::set_app_scope(Some(&app_id));
        crate::host::page_loader::set_app_path(root.clone());
        let storage = crate::storage_file::load();
        let mut g = Self {
            root,
            config,
            app_id,
            info: Value::Null,
            launch: opts.launch_options.clone().unwrap_or(Value::Null),
            host_apis: opts.host_apis.clone(),
            opts_system_info: opts.system_info.clone(),
            safe_area: opts.safe_area,
            entry_subpackage: opts.entry_subpackage.clone(),
            net_owner: crate::net::new_owner(),
            js: VecDeque::new(),
            events: VecDeque::new(),
            storage,
            storage_dirty: false,
            size: (width.max(1), height.max(1), if dpr > 0.1 { dpr } else { 1.0 }),
            orientation: if width > height { "landscape".into() } else { "portrait".into() },
        };
        g.info = g.system_info(width, height, dpr);
        Ok(g)
    }

    /// 与 `GameEngine::new` 同一套：宿主给的覆盖项、platform / SDKVersion 不许改、安全区与胶囊（chrome.rs）
    fn system_info(&self, width: u32, height: u32, dpr: f32) -> Value {
        let eo = crate::host::EngineOptions { system_info: self.opts_system_info.clone(), ..Default::default() };
        let (w, h) = (width.max(1), height.max(1));
        let dpr = if dpr > 0.1 { dpr } else { 1.0 };
        let mut info = crate::host::engine_options::system_info(w, h, dpr, &eo);
        info["deviceOrientation"] = json!(if w > h { "landscape" } else { "portrait" });
        info["platform"] = json!(super::target_platform());
        info["SDKVersion"] = json!(super::GAME_SDK_VERSION);
        super::chrome::apply(&mut info, &self.opts_system_info, self.safe_area, w > h);
        info
    }

    pub fn config(&self) -> &GameConfig {
        &self.config
    }
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
    pub fn system_info_json(&self) -> &Value {
        &self.info
    }

    /// 胶囊（右上角）的位置：宿主按它摆原生的胶囊按钮（逻辑像素 x, y, w, h）
    pub fn menu_button(&self) -> (f32, f32, f32, f32) {
        let m = &self.info["menuButton"];
        if m.is_object() {
            let f = |k: &str| m[k].as_f64().unwrap_or(0.0) as f32;
            return (f("left"), f("top"), f("width"), f("height"));
        }
        // 宿主没报安全区：与 QuickJS 模式的老口径一致（状态栏 20）
        let w = self.info["windowWidth"].as_f64().unwrap_or(0.0) as f32;
        let h = self.info["windowHeight"].as_f64().unwrap_or(0.0) as f32;
        let insets = crate::host::SafeAreaInsets { top: 20.0, ..Default::default() };
        super::chrome::menu_button(w, insets, w > h, self.info["platform"].as_str().unwrap_or(""))
    }

    /// 视图尺寸 / 方向变了（与 GameEngine::resize 同一语义，window.rs）：重算系统信息，尺寸变了派发
    /// `wx.onWindowResize({windowWidth, windowHeight})`，方向值变了派发 `wx.onDeviceOrientationChange({value})`。
    /// 上屏画布的 CSS 尺寸跟着视图走（铺满），绘图缓冲尺寸不动 —— 与微信一样由游戏自己在 onWindowResize 里改
    pub fn resize(&mut self, width: u32, height: u32, dpr: f32, orientation: Option<&str>) {
        let (width, height) = (width.max(1), height.max(1));
        let dpr = if dpr > 0.1 { dpr } else { self.size.2 };
        let value = match orientation {
            Some(o @ ("portrait" | "landscape" | "landscapeReverse")) => o.to_string(),
            _ => if width > height { "landscape".into() } else { "portrait".into() },
        };
        let size_changed = (width, height) != (self.size.0, self.size.1) || (dpr - self.size.2).abs() > 1e-6;
        let orient_changed = value != self.orientation;
        if !size_changed && !orient_changed {
            return;
        }
        self.size = (width, height, dpr);
        self.orientation = value.clone();
        self.info = self.system_info(width, height, dpr);
        let mut js = format!("globalThis.__hostSystemInfo = {};", self.info);
        if size_changed {
            js.push_str(&format!("__gameWindowResize({});", json!({ "windowWidth": width, "windowHeight": height })));
        }
        if orient_changed {
            js.push_str(&format!("__gameOrientationChange({});", json!({ "value": value })));
        }
        self.js.push_back(js);
    }

    /// 宿主报新的安全区（转屏之后）：重算安全区与胶囊，不派发 resize（尺寸没变）
    pub fn set_safe_area(&mut self, insets: crate::host::SafeAreaInsets) {
        self.safe_area = Some(insets);
        self.info = self.system_info(self.size.0, self.size.1, self.size.2);
        self.js.push_back(format!("globalThis.__hostSystemInfo = {};", self.info));
    }

    /// 前后台（宿主调）：派发 onShow / onHide，暂停 / 恢复音频与 rAF（`js/web_game/wg-frame.js`）
    pub fn set_background(&mut self, bg: bool) {
        let kind = if bg { "hide" } else { "show" };
        self.js.push_back(format!("typeof __wgLifecycle === 'function' && __wgLifecycle('{kind}');"));
    }

    /// 宿主推事件给 `wx.onXxx`（键盘输入、传感器……，与 `mr_game_dispatch_event` 同一套名字）
    pub fn dispatch_event(&mut self, name: &str, payload_json: &str) {
        self.js.push_back(format!(
            "typeof __gameHostEvent === 'function' && __gameHostEvent({}, {});",
            json!(name),
            json!(payload_json)
        ));
    }

    /// 宿主回 hostCall（回包里的 `file://` 先导入沙盒，与 QuickJS 模式同一条路：runtime/app_host.rs）
    pub fn host_reply(&mut self, id: u32, ok: bool, result_json: &str) {
        let res = crate::runtime::import_reply_files(result_json);
        self.js.push_back(format!("__resolveHostCall({id}, {ok}, {});", Value::String(res)));
    }

    /// 取一段要在页面里执行的 JS（宿主 `evaluateJavaScript`）；网络回包也在这里变成 JS
    pub fn take_js(&mut self) -> Option<String> {
        self.collect_network();
        self.flush_storage();
        if self.js.is_empty() {
            return None;
        }
        // 一次交一整批：每段 evaluateJavaScript 都是一次进程间往返
        let all: Vec<String> = self.js.drain(..).collect();
        Some(all.join("\n"))
    }

    /// 取一条宿主事件（JSON，type 与 QuickJS 模式相同：hostCall / exit / restart / showKeyboard / vibrate …）
    pub fn take_event(&mut self) -> Option<String> {
        self.events.pop_front()
    }

    /// 还有没有在等的异步活（网络请求在飞、有待执行的 JS）：宿主据此决定要不要继续轮询
    pub fn pending(&self) -> bool {
        !self.js.is_empty() || crate::net::has_pending(self.net_owner) || crate::net_stream::has_pending(self.net_owner)
    }

    /// storage 攒着改、轮询时落一次盘（页面每帧 setStorageSync 也只写一次文件）
    fn flush_storage(&mut self) {
        if std::mem::take(&mut self.storage_dirty) {
            crate::storage_file::save(&self.storage);
        }
    }

    fn log(&mut self, text: &str) {
        self.events.push_back(json!({ "type": "log", "text": text }).to_string());
    }
}

impl Drop for WebGame {
    fn drop(&mut self) {
        self.flush_storage();
        crate::net::forget_owner(self.net_owner);
        crate::net_stream::forget_owner(self.net_owner);
    }
}

#[cfg(test)]
mod tests;
