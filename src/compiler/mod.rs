//! 多端编译器框架
//!
//! 把小程序源码编译到不同目标平台。每种目标（HTML / 未来的 Android / iOS ...）
//! 实现 [`CompileTarget`] trait，放在各自的子目录（`compiler/html/`、`compiler/android/` ...），
//! 即可插入统一的编译流程：
//!
//! ```text
//! 小程序源码 ──load_app_source──▶ AppSource ──CompileTarget::compile──▶ Vec<EmittedFile> ──write_files──▶ 输出目录
//! ```
//!
//! 这样新增一个目标端只需：新建 `compiler/<target>/`，实现 `CompileTarget`，在 CLI 里注册。

pub mod html;

use crate::parser::{WxmlParser, wxml::WxmlNode};
use crate::runtime::MiniApp;
use serde_json::Value as JsonValue;
use std::path::Path;

/// 一个编译产物文件（相对输出根目录的路径 + 内容）
pub struct EmittedFile {
    pub path: String,
    pub contents: Vec<u8>,
}

impl EmittedFile {
    pub fn text(path: impl Into<String>, contents: impl Into<String>) -> Self {
        Self { path: path.into(), contents: contents.into().into_bytes() }
    }
    pub fn bytes(path: impl Into<String>, contents: Vec<u8>) -> Self {
        Self { path: path.into(), contents }
    }
}

/// 单个页面的源码（已解析 WXML + 初始数据快照）
pub struct PageSource {
    /// 页面路由，如 `pages/index/index`
    pub route: String,
    /// 解析后的 WXML 节点树
    pub wxml: Vec<WxmlNode>,
    /// 页面 WXSS 源码
    pub wxss: String,
    /// 页面 JS 源码
    pub js: String,
    /// 执行 `Page().data` + `onLoad` 后的初始数据快照
    pub data: JsonValue,
    /// 页面 json 的 `enablePullDownRefresh`（缺省继承 app.json 的 `window`）
    pub enable_pull_down_refresh: bool,
}

/// 自定义 tabBar 组件源码（`custom-tab-bar/`）。
///
/// 当 app.json 的 `tabBar.custom` 为真时，微信使用开发者提供的该组件渲染底部导航，
/// 而不是内置样式的导航条；两端都应遵循这一语义才能保持一致。
pub struct TabBarSource {
    /// 解析后的 WXML 节点树
    pub wxml: Vec<WxmlNode>,
    /// 组件 WXSS 源码
    pub wxss: String,
    /// 组件 JS 源码
    pub js: String,
    /// 组件实例化后的 data 快照（含 list / selected 等）
    pub data: JsonValue,
}

/// 整个小程序的源码
pub struct AppSource {
    /// 小程序根目录
    pub root: String,
    /// app.json 配置
    pub config: JsonValue,
    /// app.wxss 源码
    pub app_wxss: String,
    /// app.js 源码
    pub app_js: String,
    /// 所有页面
    pub pages: Vec<PageSource>,
    /// 自定义 tabBar（存在 `custom-tab-bar/` 时）
    pub custom_tab_bar: Option<TabBarSource>,
}

impl AppSource {
    /// tabBar 中的页面列表：(pagePath, text)
    pub fn tab_list(&self) -> Vec<(String, String)> {
        self.config
            .get("tabBar")
            .and_then(|t| t.get("list"))
            .and_then(|l| l.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let path = item.get("pagePath")?.as_str()?.to_string();
                        let text = item
                            .get("text")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some((path, text))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 是否使用自定义 tabBar（app.json 声明 custom 且组件存在）
    pub fn uses_custom_tab_bar(&self) -> bool {
        self.config
            .get("tabBar")
            .and_then(|t| t.get("custom"))
            .and_then(|c| c.as_bool())
            .unwrap_or(false)
            && self.custom_tab_bar.is_some()
    }

    /// 给定路由在 tabBar 列表中的下标
    pub fn tab_index_of(&self, route: &str) -> Option<usize> {
        self.tab_list().iter().position(|(path, _)| path == route)
    }
}

/// 编译目标：不同端实现该 trait 即可接入统一编译流程
pub trait CompileTarget {
    /// 目标名（CLI 选择用），如 `html`
    fn name(&self) -> &str;
    /// 把小程序源码编译为一组产物文件
    fn compile(&self, app: &AppSource) -> Result<Vec<EmittedFile>, String>;
}

const WIDTH: u32 = 375;
const HEIGHT: u32 = 667;

/// 从磁盘加载小程序源码，并对每个页面执行运行时得到初始数据快照。
pub fn load_app_source(root: &str) -> Result<AppSource, String> {
    let app_json = std::fs::read_to_string(format!("{}/app.json", root))
        .map_err(|e| format!("读取 app.json 失败: {}", e))?;
    let config: JsonValue = serde_json::from_str(&app_json).map_err(|e| format!("app.json: {}", e))?;
    let page_routes: Vec<String> = config.get("pages")
        .and_then(|p| p.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    if page_routes.is_empty() {
        return Err("app.json 未配置 pages".into());
    }

    let app_wxss = std::fs::read_to_string(format!("{}/app.wxss", root)).unwrap_or_default();
    let app_js = std::fs::read_to_string(format!("{}/app.js", root)).unwrap_or_default();

    let mut pages = Vec::new();
    for route in &page_routes {
        let prefix = format!("{}/{}", root, route);
        let wxml_src = match std::fs::read_to_string(format!("{}.wxml", prefix)) {
            Ok(s) => s,
            Err(_) => { eprintln!("  跳过 {}（无 wxml）", route); continue; }
        };
        let wxss = std::fs::read_to_string(format!("{}.wxss", prefix)).unwrap_or_default();
        let js = std::fs::read_to_string(format!("{}.js", prefix)).unwrap_or_default();

        // 独立运行时求初始数据快照，避免页面间相互污染
        let data = snapshot_page_data(&app_js, &js).unwrap_or_else(|| serde_json::json!({}));
        let wxml = WxmlParser::new(&wxml_src).parse().map_err(|e| format!("{} WXML: {}", route, e))?;

        // 页面 json 的 enablePullDownRefresh 优先，缺省回落到 app.json 的 window
        let page_json = std::fs::read_to_string(format!("{}.json", prefix)).unwrap_or_default();
        let enable_pull_down_refresh = serde_json::from_str::<JsonValue>(&page_json)
            .ok()
            .and_then(|v| v.get("enablePullDownRefresh").and_then(|b| b.as_bool()))
            .unwrap_or_else(|| {
                config
                    .get("window")
                    .and_then(|w| w.get("enablePullDownRefresh"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(false)
            });

        pages.push(PageSource { route: route.clone(), wxml, wxss, js, data, enable_pull_down_refresh });
    }

    let custom_tab_bar = load_custom_tab_bar(root, &app_js);

    Ok(AppSource { root: root.to_string(), config, app_wxss, app_js, pages, custom_tab_bar })
}

/// 加载 `custom-tab-bar/` 组件（不存在则返回 None）。
fn load_custom_tab_bar(root: &str, app_js: &str) -> Option<TabBarSource> {
    let dir = Path::new(root).join("custom-tab-bar");
    let wxml_src = std::fs::read_to_string(dir.join("index.wxml")).ok()?;
    let wxss = std::fs::read_to_string(dir.join("index.wxss")).unwrap_or_default();
    let js = std::fs::read_to_string(dir.join("index.js")).unwrap_or_default();
    let wxml = WxmlParser::new(&wxml_src).parse().ok()?;
    let data = snapshot_component_data(app_js, &js).unwrap_or_else(|| serde_json::json!({}));
    Some(TabBarSource { wxml, wxss, js, data })
}

/// 执行组件 JS 并实例化，取回其 data 快照（用于静态渲染自定义 tabBar）。
pub fn snapshot_component_data(app_js: &str, component_js: &str) -> Option<JsonValue> {
    if component_js.is_empty() {
        return None;
    }
    let mut app = MiniApp::new(WIDTH, HEIGHT).ok()?;
    app.init().ok()?;
    if !app_js.is_empty() {
        app.load_script(app_js).ok();
    }
    // 注册到固定路径后实例化，读取合并 properties/data 后的实例数据
    app.eval("__setPendingComponentPath('custom-tab-bar')").ok();
    app.load_script(component_js).ok();
    app.eval("__setPendingComponentPath('')").ok();
    let json = app
        .eval("JSON.stringify((__createComponentInstance('custom-tab-bar', {}) || {}).data || {})")
        .ok()?;
    serde_json::from_str(&json).ok()
}

/// 执行 app.js + page.js 的生命周期，取回 `__getPageData()`
fn snapshot_page_data(app_js: &str, page_js: &str) -> Option<JsonValue> {
    let mut app = MiniApp::new(WIDTH, HEIGHT).ok()?;
    app.init().ok()?;
    if !app_js.is_empty() {
        app.load_script(app_js).ok();
        app.eval("if (typeof __app!=='undefined' && __app && __app.onLaunch) __app.onLaunch({})").ok();
    }
    if !page_js.is_empty() {
        app.load_script(page_js).ok();
        app.eval("if (__currentPage && __currentPage.onLoad) __currentPage.onLoad({})").ok();
        app.eval("if (__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
        app.eval("if (__currentPage && __currentPage.onReady) __currentPage.onReady()").ok();
    }
    app.eval("__getPageData()").ok().and_then(|s| serde_json::from_str(&s).ok())
}

/// 把产物文件写入输出目录（自动创建子目录）
pub fn write_files(out_dir: &str, files: &[EmittedFile]) -> Result<(), String> {
    for f in files {
        let full = Path::new(out_dir).join(&f.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建 {:?} 失败: {}", parent, e))?;
        }
        std::fs::write(&full, &f.contents).map_err(|e| format!("写入 {:?} 失败: {}", full, e))?;
    }
    Ok(())
}
