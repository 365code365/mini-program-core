//! **引擎内置的官方组件**：用小程序自己的三件套（WXML + WXSS + `Component()`）实现的那一批官方标签 ——
//! `page-container`、`match-media`、`root-portal`、`page-meta` / `navigation-bar`、`draggable-sheet`、
//! 八个手势组件、`audio`、`editor`。
//!
//! ## 为什么用组件写、不进 Rust 渲染器
//! 它们的行为全是「逻辑层能做的事」：按属性显隐、按窗口尺寸判断、在触摸事件里算状态、调 `wx.*`。
//! 写成组件就直接复用整条自定义组件链路（实例、properties / observers、triggerEvent、slot、样式隔离），
//! 桌面窗体、移动端 SDK 一份实现（两边都从 `PageInfo.components` 拿组件），也不碰已经逐字节验过的绘制路径。
//! 需要渲染器才能做的（grid-view 网格、cover-image、原生组件占位与标签默认样式）在 `renderer/components`
//! （registry / grid_view / native_placeholder）。`movable-view` 也在这里：拖动是触摸事件里的事；它在微信里是
//! 一个节点，所以用引擎专用的组件选项 `__mergeHost` 把模板根节点并入宿主（`parser::template::component`）。
//!
//! ## 怎么注入
//! [`for_page`] 在加载页面时（`host::page_loader::load_page`）扫页面与它声明的组件的 WXML：
//! 用到了哪个内置标签、且页面没有自己声明同名组件，就追加一份 [`ComponentSource`]。
//! **按需**：没用到的页面一个都不加 —— 有组件的页面每次取数据都要多一趟对账（`component_mount::render_data`）。
//! 模块路径用 `__builtin__/<标签>`（不在小程序目录里，`require` 也解析不到它，正好）。
use crate::using_components::{scope_component_wxss, ComponentSource};

/// (标签, WXML, WXSS, JS)
const BUILTINS: &[(&str, &str, &str, &str)] = &[
    ("page-container", include_str!("page-container.wxml"), include_str!("page-container.wxss"), include_str!("page-container.js")),
    ("match-media", include_str!("match-media.wxml"), "", include_str!("match-media.js")),
    ("root-portal", include_str!("root-portal.wxml"), include_str!("root-portal.wxss"), include_str!("root-portal.js")),
    ("page-meta", include_str!("page-meta.wxml"), "", include_str!("page-meta.js")),
    ("navigation-bar", include_str!("navigation-bar.wxml"), "", include_str!("navigation-bar.js")),
    ("draggable-sheet", include_str!("draggable-sheet.wxml"), include_str!("draggable-sheet.wxss"), include_str!("draggable-sheet.js")),
    ("audio", include_str!("audio.wxml"), include_str!("audio.wxss"), include_str!("audio.js")),
    ("editor", include_str!("editor.wxml"), include_str!("editor.wxss"), include_str!("editor.js")),
    // 模板根节点并入宿主（`__mergeHost`）；movable-area 不是组件，只是带默认样式（相对定位、10px 见方）的容器
    ("movable-view", include_str!("movable-view.wxml"), "", include_str!("movable-view.js")),
];

/// 八个手势组件共用一份实现，按 `__KIND__` 区分
const GESTURES: &[&str] = &[
    "tap-gesture-handler",
    "double-tap-gesture-handler",
    "long-press-gesture-handler",
    "pan-gesture-handler",
    "horizontal-drag-gesture-handler",
    "vertical-drag-gesture-handler",
    "scale-gesture-handler",
    "force-press-gesture-handler",
];
const GESTURE_WXML: &str = include_str!("gesture.wxml");
const GESTURE_JS: &str = include_str!("gesture.js");

/// 引擎内置的全部标签名（canIUse、文档用）
pub fn tags() -> Vec<&'static str> {
    BUILTINS.iter().map(|b| b.0).chain(GESTURES.iter().copied()).collect()
}

/// WXML 里用到了 `<tag`（后面跟空白、`>` 或 `/`）
fn uses(wxml: &str, tag: &str) -> bool {
    let pat = format!("<{tag}");
    let mut rest = wxml;
    while let Some(i) = rest.find(&pat) {
        let after = rest[i + pat.len()..].chars().next();
        if matches!(after, Some(c) if c.is_whitespace() || c == '>' || c == '/') {
            return true;
        }
        rest = &rest[i + pat.len()..];
    }
    false
}

fn source(tag: &str, wxml: &str, wxss: &str, js: &str) -> ComponentSource {
    ComponentSource {
        tag: tag.to_string(),
        module_path: format!("__builtin__/{tag}"),
        wxml: wxml.to_string(),
        wxss: scope_component_wxss(wxss, tag),
        js: js.to_string(),
    }
}

/// 页面（`page_wxml`，已展开 import / include）与它已声明的组件用到的内置组件。
/// 页面自己声明了同名组件的以页面的为准（第三方组件库常自带一个 `navigation-bar`）
pub fn for_page(page_wxml: &str, declared: &[ComponentSource]) -> Vec<ComponentSource> {
    let mut text = String::from(page_wxml);
    for c in declared {
        text.push('\n');
        text.push_str(&c.wxml);
    }
    let taken = |tag: &str| declared.iter().any(|c| c.tag == tag);
    let mut out = Vec::new();
    // 内置组件的模板里也可能用别的内置组件（page-meta 里放 navigation-bar 是使用方写的，算在页面里）
    for (tag, wxml, wxss, js) in BUILTINS {
        if uses(&text, tag) && !taken(tag) {
            out.push(source(tag, wxml, wxss, js));
        }
    }
    for tag in GESTURES {
        if uses(&text, tag) && !taken(tag) {
            out.push(source(tag, &gesture_wxml(tag), "", &GESTURE_JS.replace("__KIND__", tag)));
        }
    }
    out
}

/// 拖动类手势（pan / vertical-drag）抓住这串触摸、不让页面跟着竖着滚（catchtouchmove）；
/// 其余的不拦：横向拖动本来就不滚页面（手势仲裁按轴锁定），点击 / 长按不该挡住滚动
fn gesture_wxml(tag: &str) -> String {
    let blocks = matches!(tag, "pan-gesture-handler" | "vertical-drag-gesture-handler");
    GESTURE_WXML.replace("__MOVE__", if blocks { "catchtouchmove" } else { "bindtouchmove" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_used_tags_are_injected_and_page_wins() {
        let wxml = r#"<view><page-container show="{{s}}"><text>x</text></page-container><pan-gesture-handler/></view>"#;
        let got = for_page(wxml, &[]);
        let tags: Vec<_> = got.iter().map(|c| c.tag.as_str()).collect();
        assert_eq!(tags, ["page-container", "pan-gesture-handler"]);
        assert!(got[1].js.contains("pan-gesture-handler") && !got[1].js.contains("__KIND__"));
        // `<audio-player>` 不是 `<audio>`
        assert!(for_page("<audio-player></audio-player>", &[]).is_empty());
        // 页面自己声明了同名组件
        let mine = ComponentSource { tag: "page-container".into(), module_path: "c/pc".into(), wxml: "<view/>".into(), wxss: String::new(), js: String::new() };
        assert!(for_page(wxml, std::slice::from_ref(&mine)).iter().all(|c| c.tag != "page-container"));
    }

    #[test]
    fn every_builtin_tag_is_in_caniuse() {
        let js = include_str!("../js/prelude/app-info.js");
        for tag in tags() {
            assert!(js.contains(&format!("'{tag}'")), "wx.canIUse 的组件表漏了 {tag}");
        }
    }

    #[test]
    fn every_builtin_template_parses() {
        for (tag, wxml, _, _) in BUILTINS {
            crate::parser::wxml::WxmlParser::new(wxml).parse().unwrap_or_else(|e| panic!("{tag}: {e}"));
        }
        for tag in GESTURES {
            let w = gesture_wxml(tag);
            assert!(!w.contains("__MOVE__"));
            crate::parser::wxml::WxmlParser::new(&w).parse().expect("gesture.wxml");
        }
    }
}
