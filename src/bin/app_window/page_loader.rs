//! 页面加载模块 - 支持动态加载小程序目录

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::parser::wxml::WxmlNode;
use mini_render::parser::wxss::StyleSheet;
use super::navigation::PageInfo;

/// 自定义 TabBar 数据
pub struct CustomTabBar {
    pub wxml_nodes: Vec<WxmlNode>,
    pub stylesheet: StyleSheet,
    pub js_code: String,
}

/// 小程序目录路径（全局状态）
static mut APP_PATH: Option<PathBuf> = None;

/// 设置小程序目录路径
pub fn set_app_path(path: PathBuf) {
    unsafe {
        APP_PATH = Some(path);
    }
}

/// 获取小程序目录路径
pub fn get_app_path() -> PathBuf {
    unsafe {
        APP_PATH.clone().unwrap_or_else(|| {
            // 默认使用 sample-app
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample-app")
        })
    }
}

/// 从文件系统加载所有页面
pub fn load_all_pages() -> HashMap<String, PageInfo> {
    let app_path = get_app_path();
    let app_json_path = app_path.join("app.json");
    
    let mut pages = HashMap::new();
    
    // 读取 app.json 获取页面列表
    if let Ok(content) = fs::read_to_string(&app_json_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(page_list) = json.get("pages").and_then(|p| p.as_array()) {
                for page_path in page_list {
                    if let Some(path_str) = page_path.as_str() {
                        if let Some(page_info) = load_page(&app_path, path_str) {
                            pages.insert(path_str.to_string(), page_info);
                        }
                    }
                }
            }
        }
    }
    
    // 如果没有加载到任何页面，使用内置的 sample-app
    if pages.is_empty() {
        println!("⚠️ 未找到页面，使用内置 sample-app");
        return load_builtin_pages();
    }
    
    pages
}

/// 加载单个页面
fn load_page(app_path: &Path, page_path: &str) -> Option<PageInfo> {
    let base_path = app_path.join(page_path);
    
    let wxml_path = base_path.with_extension("wxml");
    let wxss_path = base_path.with_extension("wxss");
    let js_path = base_path.with_extension("js");
    
    let wxml = fs::read_to_string(&wxml_path).unwrap_or_default();
    let wxss = fs::read_to_string(&wxss_path).unwrap_or_default();
    let js = fs::read_to_string(&js_path).unwrap_or_default();
    
    if wxml.is_empty() {
        println!("⚠️ 页面 {} 的 WXML 文件不存在或为空", page_path);
        return None;
    }
    
    Some(PageInfo {
        path: page_path.to_string(),
        wxml,
        wxss,
        js,
    })
}

/// 加载 app.js
pub fn load_app_js() -> String {
    let app_path = get_app_path();
    let app_js_path = app_path.join("app.js");
    
    fs::read_to_string(&app_js_path).unwrap_or_else(|_| {
        println!("⚠️ app.js 不存在，使用空脚本");
        "App({})".to_string()
    })
}

/// 加载 app.json
pub fn load_app_json() -> String {
    let app_path = get_app_path();
    let app_json_path = app_path.join("app.json");
    
    fs::read_to_string(&app_json_path).unwrap_or_else(|_| {
        println!("⚠️ app.json 不存在，使用默认配置");
        r#"{"pages":["pages/index/index"],"window":{"navigationBarTitleText":"Mini App"}}"#.to_string()
    })
}

/// 加载自定义 TabBar
pub fn load_custom_tabbar() -> Result<Option<CustomTabBar>, String> {
    let app_path = get_app_path();
    let tabbar_dir = app_path.join("custom-tab-bar");
    
    if !tabbar_dir.exists() {
        return Ok(None);
    }
    
    let wxml_path = tabbar_dir.join("index.wxml");
    let wxss_path = tabbar_dir.join("index.wxss");
    let js_path = tabbar_dir.join("index.js");
    
    let wxml = fs::read_to_string(&wxml_path)
        .map_err(|e| format!("读取 custom-tab-bar/index.wxml 失败: {}", e))?;
    let wxss = fs::read_to_string(&wxss_path).unwrap_or_default();
    let js = fs::read_to_string(&js_path).unwrap_or_default();
    
    let mut wxml_parser = WxmlParser::new(&wxml);
    let wxml_nodes = wxml_parser.parse().map_err(|e| format!("Custom TabBar WXML error: {}", e))?;
    
    let mut wxss_parser = WxssParser::new(&wxss);
    let stylesheet = wxss_parser.parse().map_err(|e| format!("Custom TabBar WXSS error: {}", e))?;
    
    Ok(Some(CustomTabBar {
        wxml_nodes,
        stylesheet,
        js_code: js,
    }))
}

/// 内置页面（fallback）
fn load_builtin_pages() -> HashMap<String, PageInfo> {
    let mut pages = HashMap::new();
    
    pages.insert("pages/index/index".to_string(), PageInfo {
        path: "pages/index/index".to_string(),
        wxml: include_str!("../../../sample-app/pages/index/index.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/index/index.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/index/index.js").to_string(),
    });
    
    pages.insert("pages/category/category".to_string(), PageInfo {
        path: "pages/category/category".to_string(),
        wxml: include_str!("../../../sample-app/pages/category/category.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/category/category.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/category/category.js").to_string(),
    });
    
    pages.insert("pages/cart/cart".to_string(), PageInfo {
        path: "pages/cart/cart".to_string(),
        wxml: include_str!("../../../sample-app/pages/cart/cart.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/cart/cart.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/cart/cart.js").to_string(),
    });
    
    pages.insert("pages/profile/profile".to_string(), PageInfo {
        path: "pages/profile/profile".to_string(),
        wxml: include_str!("../../../sample-app/pages/profile/profile.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/profile/profile.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/profile/profile.js").to_string(),
    });
    
    pages.insert("pages/detail/detail".to_string(), PageInfo {
        path: "pages/detail/detail".to_string(),
        wxml: include_str!("../../../sample-app/pages/detail/detail.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/detail/detail.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/detail/detail.js").to_string(),
    });
    
    pages.insert("pages/canvas/canvas".to_string(), PageInfo {
        path: "pages/canvas/canvas".to_string(),
        wxml: include_str!("../../../sample-app/pages/canvas/canvas.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/canvas/canvas.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/canvas/canvas.js").to_string(),
    });
    
    pages.insert("pages/components/components".to_string(), PageInfo {
        path: "pages/components/components".to_string(),
        wxml: include_str!("../../../sample-app/pages/components/components.wxml").to_string(),
        wxss: include_str!("../../../sample-app/pages/components/components.wxss").to_string(),
        js: include_str!("../../../sample-app/pages/components/components.js").to_string(),
    });
    
    pages
}
