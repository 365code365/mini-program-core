//! 页面 json 里 `usingComponents` 声明的自定义组件：加载三件套 + 样式作用域。
//!
//! 微信小程序把自定义组件写成「目录 + 同名 wxml/wxss/js/json」，页面 json 用
//! `usingComponents: { "tab-bar": "../../components/tab-bar/tab-bar" }` 声明后，
//! 页面 WXML 里就可以直接写 `<tab-bar/>`。
//!
//! 之前引擎完全不认这套：`<tab-bar/>` 落到未知标签的兜底分支被当成空 `view`，
//! 于是**整条底部导航（图标 + 文字）在原生端凭空消失**——而 uni-app / Taro
//! 编译出来的产物普遍把 tabBar 做成这样一个页面内组件（Skyline 下 tabBar
//! 也推荐自绘），所以这不是个别应用的问题。
//!
//! 样式隔离：微信组件默认 `styleIsolation: isolated`。这里做的是**单向隔离**——
//! 把组件 WXSS 的每条选择器前缀成 `<标签名> …`，于是组件样式不会漏出去命中页面元素
//! （`.icon` / `.label` 这种通名相当危险）。反向（页面样式渗进组件）暂未隔离。
//! `:host` 改写成标签名本身：展开时会保留一个以组件标签命名的宿主节点。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 一个自定义组件的源码三件套
#[derive(Debug, Clone)]
pub struct ComponentSource {
    /// WXML 里使用的标签名（页面 json 里的键）
    pub tag: String,
    /// 组件 js 的模块路径（不含扩展名，相对小程序根），同时作为 `Component()` 的注册键
    pub module_path: String,
    /// 组件 WXML 原文
    pub wxml: String,
    /// 组件 WXSS（已做作用域前缀与 `:host` 改写）
    pub wxss: String,
    /// 组件 js 原文
    pub js: String,
}

/// 读取某个页面声明的全部自定义组件（含组件自身再声明的组件，递归深度上限 4）。
///
/// `page_route` 形如 `pages/index/index`（不含扩展名）。
pub fn load_page_components(app_root: &Path, page_route: &str) -> Vec<ComponentSource> {
    let json_path = app_root.join(format!("{page_route}.json"));
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    collect(app_root, &json_path, &mut out, &mut seen, 0);
    out
}

fn collect(
    app_root: &Path,
    owner_json: &Path,
    out: &mut Vec<ComponentSource>,
    seen: &mut Vec<String>,
    depth: usize,
) {
    if depth > 4 {
        return;
    }
    let map = match read_using_components(owner_json) {
        Some(m) if !m.is_empty() => m,
        _ => return,
    };
    let owner_dir = owner_json.parent().unwrap_or(app_root).to_path_buf();
    for (tag, rel) in map {
        let base = resolve_component_path(app_root, &owner_dir, &rel);
        let module_path = match module_path_of(app_root, &base) {
            Some(p) => p,
            None => continue,
        };
        if seen.contains(&module_path) {
            continue;
        }
        seen.push(module_path.clone());

        let wxml = std::fs::read_to_string(base.with_extension("wxml")).unwrap_or_default();
        if wxml.trim().is_empty() {
            // 没有模板的「纯逻辑组件」没什么可渲染的，跳过（但仍算已见，避免重复尝试）
            continue;
        }
        let raw_wxss = std::fs::read_to_string(base.with_extension("wxss")).unwrap_or_default();
        let js = std::fs::read_to_string(base.with_extension("js")).unwrap_or_default();
        out.push(ComponentSource {
            wxss: scope_component_wxss(&raw_wxss, &tag),
            tag: tag.clone(),
            module_path,
            wxml,
            js,
        });
        // 组件自身也可以再用别的组件
        collect(app_root, &base.with_extension("json"), out, seen, depth + 1);
    }
}

fn read_using_components(json_path: &Path) -> Option<HashMap<String, String>> {
    let text = std::fs::read_to_string(json_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let obj = value.get("usingComponents")?.as_object()?;
    Some(
        obj.iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect(),
    )
}

/// `/abs/path` 相对小程序根，其余相对声明它的文件所在目录。
fn resolve_component_path(app_root: &Path, owner_dir: &Path, rel: &str) -> PathBuf {
    let rel = rel.trim();
    let joined = if let Some(abs) = rel.strip_prefix('/') {
        app_root.join(abs)
    } else {
        owner_dir.join(rel)
    };
    normalize(&joined)
}

/// 去掉路径里的 `.` 与 `..`（组件路径普遍写成 `../../components/x/x`）
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// 组件在小程序内的模块路径（相对根、不含扩展名）
fn module_path_of(app_root: &Path, base: &Path) -> Option<String> {
    let root = normalize(app_root);
    let rel = base.strip_prefix(&root).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// 给组件 WXSS 的每条选择器加上组件标签前缀，并把 `:host` 改写成标签本身。
///
/// 只处理顶层规则：`@keyframes` / `@media` / `@import` 整块原样保留
/// （改写它们的内容需要递归解析，而组件里少见；后续需要再补）。
pub fn scope_component_wxss(wxss: &str, tag: &str) -> String {
    let mut out = String::with_capacity(wxss.len() + 64);
    let bytes: Vec<char> = wxss.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        // 跳过注释
        if bytes[i] == '/' && i + 1 < bytes.len() && bytes[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == '*' && bytes[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            out.extend(&bytes[start..i]);
            continue;
        }
        if bytes[i].is_whitespace() {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        // 取到下一个 `{` 之前的选择器（或分号结束的语句，如 @import）
        let sel_start = i;
        while i < bytes.len() && bytes[i] != '{' && bytes[i] != ';' {
            i += 1;
        }
        let selector: String = bytes[sel_start..i].iter().collect();
        if i >= bytes.len() {
            out.push_str(&selector);
            break;
        }
        if bytes[i] == ';' {
            // `@import "…";` 这类没有块的语句
            out.push_str(&selector);
            out.push(';');
            i += 1;
            continue;
        }
        // 找到与之匹配的 `}`
        let block_start = i;
        let mut depth = 0i32;
        while i < bytes.len() {
            match bytes[i] {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        let block: String = bytes[block_start..i.min(bytes.len())].iter().collect();
        let trimmed = selector.trim();
        if trimmed.starts_with('@') {
            out.push_str(&selector);
            out.push_str(&block);
        } else {
            out.push_str(&scope_selector_list(trimmed, tag));
            out.push_str(&block);
        }
    }
    out
}

fn scope_selector_list(list: &str, tag: &str) -> String {
    list.split(',')
        .map(|sel| scope_one_selector(sel.trim(), tag))
        .collect::<Vec<_>>()
        .join(", ")
}

fn scope_one_selector(sel: &str, tag: &str) -> String {
    if sel.is_empty() {
        return String::new();
    }
    // `:host` / `:host(...)` 指组件根节点，展开后就是那个以标签命名的宿主节点
    if let Some(rest) = sel.strip_prefix(":host") {
        let rest = rest.trim_start_matches(|c| c == '(' || c == ')').trim();
        if rest.is_empty() {
            return tag.to_string();
        }
        return format!("{tag}{rest}");
    }
    // `page` 是页面根，组件里写它也是想影响整页，不加前缀
    if sel == "page" {
        return sel.to_string();
    }
    format!("{tag} {sel}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_selectors_and_host() {
        let css = ":host{display:flex}\n.bar{height:10px}\n.cell,.icon{width:2px}";
        let out = scope_component_wxss(css, "tab-bar");
        assert!(out.contains("tab-bar{display:flex}"), "\n{out}");
        assert!(out.contains("tab-bar .bar{height:10px}"), "\n{out}");
        assert!(out.contains("tab-bar .cell, tab-bar .icon{width:2px}"), "\n{out}");
    }

    /// `@keyframes` 里的 `0%` / `from` 不是选择器，不能被加前缀（否则动画整条失效）
    #[test]
    fn leaves_at_rules_alone() {
        let css = "@keyframes spin{from{opacity:0}to{opacity:1}}\n.x{color:red}";
        let out = scope_component_wxss(css, "my-comp");
        assert!(out.contains("@keyframes spin{from{opacity:0}to{opacity:1}}"), "\n{out}");
        assert!(out.contains("my-comp .x{color:red}"), "\n{out}");
    }

    #[test]
    fn keeps_comments_and_page_selector() {
        let css = "/* c */ page{background:#fff} .y{color:#000}";
        let out = scope_component_wxss(css, "c-a");
        assert!(out.contains("/* c */"), "\n{out}");
        assert!(out.contains("page{background:#fff}"), "\n{out}");
        assert!(out.contains("c-a .y{color:#000}"), "\n{out}");
    }

    #[test]
    fn resolves_relative_and_root_paths() {
        let root = Path::new("/app");
        let owner = Path::new("/app/pages/index");
        assert_eq!(
            resolve_component_path(root, owner, "../../components/tab-bar/tab-bar"),
            PathBuf::from("/app/components/tab-bar/tab-bar")
        );
        assert_eq!(
            resolve_component_path(root, owner, "/components/x/x"),
            PathBuf::from("/app/components/x/x")
        );
        assert_eq!(
            module_path_of(root, Path::new("/app/components/tab-bar/tab-bar")).as_deref(),
            Some("components/tab-bar/tab-bar")
        );
    }
}
