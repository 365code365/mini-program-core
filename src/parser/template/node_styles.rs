//! 逻辑层写给**某些节点**的样式：`this.animate` / `clearAnimation`（官方 api/ui/animation/animate）
//! 与 worklet 的 `applyAnimatedStyle`（Skyline）。它们按选择器改节点样式，而不是改模板里的数据绑定。
//!
//! 走数据：页面 / 组件实例 data 里的 `__nodeStyles`（选择器 → 样式文本，见 `js/prelude/animate.js`）。
//! 模板展开每个元素时，按**所在作用域的根 data**（页面的，或组件实例的）找命中的选择器，把样式拼在
//! `style` 后面 —— 内联样式优先级最高，与微信「animate 新增的样式覆盖原有样式」一致。
//! 这样逐帧改的只是一次 setData：增量失效、局部重画、两种宿主都不用另接一条路。
//!
//! 选择器只认复合段（`#id`、`.a.b`、`tag`、`tag.a`）；写了后代关系（`.list .item`）时按最后一段匹配。
use crate::parser::wxml::WxmlNode;
use serde_json::{Map, Value as JsonValue};

fn matches(node: &WxmlNode, sel: &str) -> bool {
    let last = sel.split(|c: char| c.is_whitespace() || c == '>').filter(|s| !s.is_empty()).last().unwrap_or("");
    if last.is_empty() {
        return false;
    }
    let (mut tag, mut id, mut classes) = (String::new(), String::new(), Vec::new());
    let mut cur = String::new();
    let mut kind = 't';
    let mut flush = |kind: char, cur: &mut String| {
        let v = std::mem::take(cur);
        if v.is_empty() {
            return;
        }
        match kind {
            '#' => id = v,
            '.' => classes.push(v),
            _ => tag = v,
        }
    };
    for ch in last.chars() {
        if ch == '#' || ch == '.' {
            flush(kind, &mut cur);
            kind = ch;
        } else {
            cur.push(ch);
        }
    }
    flush(kind, &mut cur);
    if !tag.is_empty() && tag != node.tag_name {
        return false;
    }
    if !id.is_empty() && node.attributes.get("id").map(String::as_str) != Some(id.as_str()) {
        return false;
    }
    let have = node.attributes.get("class").map(String::as_str).unwrap_or("");
    classes.iter().all(|c| have.split_whitespace().any(|h| h == c))
}

/// 把命中的样式拼到节点的 style 后面
pub(super) fn apply(node: &mut WxmlNode, styles: &Map<String, JsonValue>) {
    for (sel, css) in styles {
        let Some(css) = css.as_str().filter(|s| !s.trim().is_empty()) else { continue };
        if !matches(node, sel) {
            continue;
        }
        let st = node.attributes.entry("style".to_string()).or_default();
        if !st.trim().is_empty() && !st.trim_end().ends_with(';') {
            st.push(';');
        }
        st.push_str(css);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_selectors_append_to_style() {
        let mut n = WxmlNode::new_element("view");
        n.attributes.insert("id".into(), "box".into());
        n.attributes.insert("class".into(), "a b".into());
        n.attributes.insert("style".into(), "color: red".into());
        let m: Map<String, JsonValue> = serde_json::from_str(r##"{"#box":"opacity:0.5;","view.a.b":"left:3px;","#other":"top:1px;",".list .b":"width:2px;"}"##).unwrap();
        apply(&mut n, &m);
        let st = &n.attributes["style"];
        assert!(st.starts_with("color: red;"), "{st}");
        assert!(st.contains("opacity:0.5;") && st.contains("left:3px;") && st.contains("width:2px;"), "{st}");
        assert!(!st.contains("top:1px"), "没命中的不拼：{st}");
    }
}
