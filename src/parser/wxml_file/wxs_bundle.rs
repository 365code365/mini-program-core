//! `<wxs src="./m.wxs">` 与 wxs 代码里的 `require('./x.wxs')`：在读 WXML 的这一刻把用到的
//! `.wxs` 文件全部读进来，打成一个包放进 `<wxs>` 节点的子文本，渲染期与 H5 导出都只认这个包
//! （渲染期拿不到文件路径，逻辑层也不该去读包里的文件）。
//!
//! 包的形状：`{"entry": "/utils/m.wxs", "files": {"/utils/m.wxs": "…", "/utils/x.wxs": "…"}}`，
//! 路径相对小程序根、以 `/` 开头；内联模块的入口是 `<wxml 路径>#<模块名>`（它的 require
//! 相对 WXML 所在目录）。节点打上 `__wxs_bundle` 标记，见 `parser::template::WXS_BUNDLE_ATTR`。
//!
//! 规则（官方 wxs/01wxs-module.html）：`require` 只能引 `.wxs`、必须是相对路径；
//! 模块只在第一次被 require 时执行一次（单例）—— 后者由逻辑层的模块表保证。
//! 读不到、越出小程序目录的文件：打警告、包里缺那一项（运行时 require 它抛错，被调用方吞掉）。

use crate::parser::template::WXS_BUNDLE_ATTR;
use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::using_components::normalize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 一个包里最多读多少个文件（防止 require 链写出环或极端大的依赖树）
const MAX_FILES: usize = 64;

/// 处理一个 `<wxs>` 节点。没有 `module` 的不管；内联且不 require 别人的原样留着（零改动）。
pub(super) fn bundle(root: &Path, wxml_file: &Path, node: &mut WxmlNode) {
    let Some(module) = node.get_attr("module").map(|s| s.trim().to_string()) else { return };
    let (entry, code) = match node.get_attr("src").map(str::trim).filter(|s| !s.is_empty()) {
        Some(src) => {
            let dir = wxml_file.parent().unwrap_or(root);
            let Some(path) = resolve(root, dir, src) else {
                eprintln!("⚠️ <wxs module=\"{module}\" src=\"{src}\">：路径无效或越出小程序目录（来自 {}）", wxml_file.display());
                return;
            };
            match std::fs::read_to_string(&path) {
                Ok(code) => (rel_key(root, &path), code),
                Err(_) => {
                    eprintln!("⚠️ <wxs module=\"{module}\" src=\"{src}\">：找不到文件 {}", path.display());
                    return;
                }
            }
        }
        None => {
            let code: String = node
                .children
                .iter()
                .filter(|c| c.node_type == WxmlNodeType::Text)
                .map(|c| c.text_content.as_str())
                .collect();
            if requires(&code).is_empty() {
                return;
            }
            (format!("{}#{module}", rel_key(root, wxml_file)), code)
        }
    };
    let mut files = BTreeMap::new();
    files.insert(entry.clone(), code);
    let mut queue = vec![entry.clone()];
    while let Some(cur) = queue.pop() {
        let base_dir = dir_of_key(root, &cur);
        let deps: Vec<String> = requires(files.get(&cur).map(String::as_str).unwrap_or(""));
        for dep in deps {
            let Some(path) = resolve(root, &base_dir, &dep).filter(|p| p.extension().is_some_and(|e| e == "wxs")) else {
                eprintln!("⚠️ wxs require('{dep}')：只能相对引用小程序目录里的 .wxs（来自 {cur}）");
                continue;
            };
            let key = rel_key(root, &path);
            if files.contains_key(&key) {
                continue;
            }
            if files.len() >= MAX_FILES {
                eprintln!("⚠️ wxs 模块 {module} 的依赖超过 {MAX_FILES} 个文件，后面的不再读");
                break;
            }
            match std::fs::read_to_string(&path) {
                Ok(code) => {
                    files.insert(key.clone(), code);
                    queue.push(key);
                }
                Err(_) => eprintln!("⚠️ wxs require('{dep}')：找不到文件 {}（来自 {cur}）", path.display()),
            }
        }
    }
    let json = serde_json::json!({ "entry": entry, "files": files }).to_string();
    // 包放在 `<wxs>` 的原始文本里，而文本读到 `</wxs` 为止 —— 把 `</` 写成 JSON 合法的 `<\/`
    let json = json.replace("</", "<\\/");
    node.attributes.insert(WXS_BUNDLE_ATTR.to_string(), String::new());
    node.children = vec![WxmlNode::new_text(&json)];
}

/// 代码里所有 `require('…')` / `require("…")` 的字符串参数（按出现顺序，不去重）
pub fn requires(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(i) = code[pos..].find("require") {
        let at = pos + i;
        pos = at + "require".len();
        // `myrequire(` / `x.require(` 不是模块引用
        let prev = code[..at].chars().last();
        if prev.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$' || c == '.') {
            continue;
        }
        let t = code[pos..].trim_start();
        if !t.starts_with('(') {
            continue;
        }
        let t = t[1..].trim_start();
        let Some(q) = t.chars().next().filter(|c| *c == '\'' || *c == '"') else { continue };
        if let Some(end) = t[1..].find(q) {
            out.push(t[1..1 + end].to_string());
        }
    }
    out
}

/// 相对路径 → 绝对路径（必须留在小程序目录里；`require` 不允许 `/` 开头，`src` 允许）
fn resolve(root: &Path, dir: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() || rel.contains("{{") {
        return None;
    }
    let p = if let Some(abs) = rel.strip_prefix('/') { root.join(abs) } else { dir.join(rel) };
    let p = normalize(&p);
    p.starts_with(normalize(root)).then_some(p)
}

/// 绝对路径 → 包里的键（`/` 开头、相对小程序根、正斜杠）
fn rel_key(root: &Path, p: &Path) -> String {
    let root = normalize(root);
    let rel = normalize(p);
    let rel = rel.strip_prefix(&root).unwrap_or(&rel);
    let s: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    format!("/{}", s.join("/"))
}

/// 包里的键 → 它所在的目录（绝对路径）；内联模块的键是 `<wxml>#<模块名>`
fn dir_of_key(root: &Path, key: &str) -> PathBuf {
    let file = key.split('#').next().unwrap_or(key);
    let p = normalize(&root.join(file.trim_start_matches('/')));
    p.parent().map(Path::to_path_buf).unwrap_or_else(|| normalize(root))
}
