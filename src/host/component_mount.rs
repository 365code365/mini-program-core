//! 页面内自定义组件的挂载：执行组件 js → 建实例 → 组件 data 进入渲染作用域。
//!
//! 光把组件模板展开是不够的：模板里的 `{{a}}` 来自**组件实例自己的 data**，
//! 而框架型产物（uni-app / Taro）是在组件的 `attached` 里挂载自己的组件树并触发
//! 首次 `setData` 的。不建实例的话组件模板里全是空值 ——
//! 表现就是「底部导航整条空白，图标和文字都没有」。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::runtime::MiniApp;
use crate::using_components::ComponentSource;

/// 挂载页面声明的全部自定义组件。
pub fn mount_page_components(app: &mut MiniApp, comps: &[ComponentSource], page_nodes: &[WxmlNode]) {
    for c in comps {
        // 1) 以「组件路径」为注册键执行组件 js：`Component({...})` 会登记到该路径。
        //    这个标记同时告诉 `Component()`「这是组件不是页面」，
        //    否则组件 js 会被当成页面注册，把当前页面顶掉。
        app.eval(&format!("__setPendingComponentPath({})", js_str(&c.module_path))).ok();
        let loaded = app.load_module_script(&c.module_path, &c.js);
        app.eval("__setPendingComponentPath('')").ok();
        if let Err(e) = loaded {
            eprintln!("⚠️  组件 {} 的 js 执行失败: {}", c.tag, e);
            continue;
        }
        // 2) 从页面 WXML 里取这个标签第一处使用上的属性，作为 props
        let props_expr = props_expr_for(page_nodes, &c.tag);
        let code = format!(
            "__mountPageComponent({}, {}, {})",
            js_str(&c.tag),
            js_str(&c.module_path),
            js_str(&props_expr)
        );
        match app.eval(&code) {
            Ok(id) if id.trim_matches('"').is_empty() => {
                eprintln!("⚠️  组件 {} 没有创建出实例（Component() 未注册到 {}）", c.tag, c.module_path);
            }
            Ok(_) => {}
            Err(e) => eprintln!("⚠️  组件 {} 挂载失败: {}", c.tag, e),
        }
    }
}

/// 把使用处的属性拼成一段「在页面数据作用域里求值」的对象字面量。
///
/// 微信的属性名规则是 `foo-bar` 属性 ↔ `fooBar` 属性名；uni-app 用 `u-p` 传整包 props、
/// `u-i` 传组件 id，都走这条规则。`{{expr}}` 原样作为表达式，纯文本作为字符串常量。
fn props_expr_for(nodes: &[WxmlNode], tag: &str) -> String {
    let node = match find_first(nodes, tag) {
        Some(n) => n,
        None => return "{}".to_string(),
    };
    let mut parts: Vec<String> = Vec::new();
    // 排序遍历：这里拼出的是给 JS 的属性字面量，顺序进了输出就该稳定
    for (key, value) in node.attrs_sorted() {
        if !is_property_attr(key) {
            continue;
        }
        let name = camel_case(key);
        let expr = match strip_braces(value) {
            Some(inner) => format!("({inner})"),
            None => js_str(value),
        };
        parts.push(format!("{}: {}", js_str(&name), expr));
    }
    parts.sort(); // 稳定输出，便于对比与测试
    format!("{{{}}}", parts.join(", "))
}

/// 哪些属性算「传给组件的属性」：排除 class/style/id、事件绑定、指令与 wxs 观察器。
fn is_property_attr(key: &str) -> bool {
    if key.contains(':') {
        return false; // wx:if / change:eS / bind:tap
    }
    if key.starts_with("bind") || key.starts_with("catch") || key.starts_with("data-") {
        return false;
    }
    !matches!(key, "class" | "style" | "id" | "hidden" | "eS" | "eA")
}

fn camel_case(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut upper = false;
    for ch in key.chars() {
        if ch == '-' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn strip_braces(value: &str) -> Option<&str> {
    let v = value.trim();
    let inner = v.strip_prefix("{{")?.strip_suffix("}}")?;
    Some(inner.trim())
}

fn find_first<'a>(nodes: &'a [WxmlNode], tag: &str) -> Option<&'a WxmlNode> {
    for n in nodes {
        if n.node_type == WxmlNodeType::Element {
            if n.tag_name == tag {
                return Some(n);
            }
            if let Some(found) = find_first(&n.children, tag) {
                return Some(found);
            }
        }
    }
    None
}

fn js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "''".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::wxml::WxmlParser;

    fn nodes(src: &str) -> Vec<WxmlNode> {
        WxmlParser::new(src).parse().unwrap_or_default()
    }

    #[test]
    fn builds_props_from_usage_attributes() {
        // uni-app 的写法：u-p 传整包 props、u-i 传组件 id，还有一堆要忽略的属性
        let src = r#"<view><tab-bar class="x" u-i="1f-0" bind:__l="__l" u-p="{{r||''}}"></tab-bar></view>"#;
        let expr = props_expr_for(&nodes(src), "tab-bar");
        assert!(expr.contains("\"uP\": (r||'')"), "{expr}");
        assert!(expr.contains("\"uI\": \"1f-0\""), "{expr}");
        assert!(!expr.contains("class"), "class 不是组件属性: {expr}");
        assert!(!expr.contains("__l"), "事件绑定不是组件属性: {expr}");
    }

    #[test]
    fn no_usage_yields_empty_props() {
        assert_eq!(props_expr_for(&nodes("<view/>"), "tab-bar"), "{}");
    }

    #[test]
    fn dashed_attrs_become_camel_case() {
        assert_eq!(camel_case("u-p"), "uP");
        assert_eq!(camel_case("my-long-name"), "myLongName");
        assert_eq!(camel_case("plain"), "plain");
    }
}
