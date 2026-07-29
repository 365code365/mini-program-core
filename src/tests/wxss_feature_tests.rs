//! WXSS 里「实现了但没测试」的那几项：CSS 变量、属性选择器、`:nth-child`、
//! `@import`、后代/子代组合器、祖先链匹配、`:active` 与结构性伪类的组合。
//!
//! 这些是层叠计算的边角，一旦回归就表现为「某个页面某个状态下样式突然不对」，
//! 靠整页快照很难定位到规则层面，所以在这一层单独钉住。

use crate::parser::wxss::{ElementDesc, StyleSheet, StyleValue, WxssParser};
use std::collections::HashMap;

fn parse(css: &str) -> StyleSheet {
    WxssParser::new(css).parse().unwrap_or_default()
}

fn no_attrs() -> HashMap<String, String> {
    HashMap::new()
}

fn color_of(styles: &HashMap<String, StyleValue>, key: &str) -> String {
    match styles.get(key) {
        Some(StyleValue::Color(c)) => format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
        Some(other) => format!("{other:?}"),
        None => String::from("<缺失>"),
    }
}

fn px_of(styles: &HashMap<String, StyleValue>, key: &str) -> Option<f32> {
    match styles.get(key) {
        Some(StyleValue::Length(v, _)) => Some(*v),
        _ => None,
    }
}

// ───────────────────────────── CSS 变量 ─────────────────────────────

#[test]
fn css_variables_resolve_from_the_global_pool() {
    // 小程序里 `page{--brand:...}` + `var(--brand)` 是主题色的常见写法
    let ss = parse(
        r#"
        page { --brand: #07c160; --gap: 16px; }
        .btn { color: var(--brand); padding: var(--gap); }
        "#,
    );
    let s = ss.get_styles_el("view", None, &["btn"], &no_attrs());
    assert_eq!(color_of(&s, "color"), "#07c160", "var(--brand) 应解析为定义值");
    assert_eq!(px_of(&s, "padding"), Some(16.0), "长度类变量也要能解析");
}

#[test]
fn css_variable_falls_back_when_undefined() {
    let ss = parse(".btn { color: var(--nope, #ff0000); }");
    let s = ss.get_styles_el("view", None, &["btn"], &no_attrs());
    assert_eq!(color_of(&s, "color"), "#ff0000", "未定义时用 fallback");
}

#[test]
fn later_variable_definition_wins() {
    let ss = parse(
        r#"
        page { --brand: #111111; }
        page { --brand: #222222; }
        .t { color: var(--brand); }
        "#,
    );
    let s = ss.get_styles_el("view", None, &["t"], &no_attrs());
    assert_eq!(color_of(&s, "color"), "#222222");
}

// ─────────────────────────── 属性选择器 ───────────────────────────

#[test]
fn attribute_selector_matches_exact_value() {
    let ss = parse(
        r#"
        .cell[data-state="on"] { color: #00ff00; }
        .cell { color: #999999; }
        "#,
    );
    assert!(ss.has_attr_selectors(), "样式表应报告存在属性选择器");
    let mut on = no_attrs();
    on.insert("data-state".into(), "on".into());
    let hit = ss.get_styles_el("view", None, &["cell"], &on);
    assert_eq!(color_of(&hit, "color"), "#00ff00", "属性匹配时特异性更高");

    let mut off = no_attrs();
    off.insert("data-state".into(), "off".into());
    let miss = ss.get_styles_el("view", None, &["cell"], &off);
    assert_eq!(color_of(&miss, "color"), "#999999", "值不同则不匹配");

    let none = ss.get_styles_el("view", None, &["cell"], &no_attrs());
    assert_eq!(color_of(&none, "color"), "#999999", "属性不存在则不匹配");
}

#[test]
fn attribute_presence_selector_matches_any_value() {
    let ss = parse("[disabled] { color: #cccccc; }");
    let mut attrs = no_attrs();
    attrs.insert("disabled".into(), "true".into());
    let s = ss.get_styles_el("button", None, &[], &attrs);
    assert_eq!(color_of(&s, "color"), "#cccccc");
    let s2 = ss.get_styles_el("button", None, &[], &no_attrs());
    assert_eq!(color_of(&s2, "color"), "<缺失>");
}

// ────────────────────────── 结构性伪类 ──────────────────────────

#[test]
fn nth_child_matches_by_position() {
    let ss = parse(
        r#"
        .row:nth-child(1) { color: #ff0000; }
        .row:nth-child(2) { color: #00ff00; }
        .row:nth-child(3) { color: #0000ff; }
        "#,
    );
    let colors: Vec<String> = (0..3)
        .map(|i| {
            let d = ElementDesc::new("view", None, &["row"], &no_attrs()).with_position(i, 3);
            color_of(&ss.get_styles_chain(&[d]), "color")
        })
        .collect();
    assert_eq!(colors, vec!["#ff0000", "#00ff00", "#0000ff"], "nth-child 从 1 开始计数");
}

#[test]
fn nth_child_supports_an_plus_b_form() {
    // 斑马纹列表的写法
    let ss = parse(".row:nth-child(2n) { color: #eeeeee; }");
    for (i, want_hit) in [(0usize, false), (1, true), (2, false), (3, true)] {
        let d = ElementDesc::new("view", None, &["row"], &no_attrs()).with_position(i, 4);
        let got = color_of(&ss.get_styles_chain(&[d]), "color");
        if want_hit {
            assert_eq!(got, "#eeeeee", "第 {} 个（1-based {}）应命中 2n", i, i + 1);
        } else {
            assert_eq!(got, "<缺失>", "第 {} 个（1-based {}）不该命中 2n", i, i + 1);
        }
    }
}

#[test]
fn first_and_last_child_are_position_aware() {
    let ss = parse(
        r#"
        .row:first-child { color: #ff0000; }
        .row:last-child { color: #0000ff; }
        "#,
    );
    let first = ElementDesc::new("view", None, &["row"], &no_attrs()).with_position(0, 3);
    let mid = ElementDesc::new("view", None, &["row"], &no_attrs()).with_position(1, 3);
    let last = ElementDesc::new("view", None, &["row"], &no_attrs()).with_position(2, 3);
    assert_eq!(color_of(&ss.get_styles_chain(&[first]), "color"), "#ff0000");
    assert_eq!(color_of(&ss.get_styles_chain(&[mid]), "color"), "<缺失>");
    assert_eq!(color_of(&ss.get_styles_chain(&[last]), "color"), "#0000ff");
    // 只有一个孩子时 first 和 last 同时命中，后写的赢
    let only = ElementDesc::new("view", None, &["row"], &no_attrs()).with_position(0, 1);
    assert_eq!(color_of(&ss.get_styles_chain(&[only]), "color"), "#0000ff");
}

#[test]
fn hover_and_focus_pseudo_classes_never_match() {
    // 手机上没有 hover；focus 由宿主的输入态处理，不走样式表。
    // 这两条要**明确不匹配**，否则会莫名其妙地把样式套上去。
    let ss = parse(".a:hover { color: #ff0000; } .b:focus { color: #00ff00; }");
    assert_eq!(
        color_of(&ss.get_styles_el("view", None, &["a"], &no_attrs()), "color"),
        "<缺失>"
    );
    assert_eq!(
        color_of(&ss.get_styles_el("input", None, &["b"], &no_attrs()), "color"),
        "<缺失>"
    );
}

// ─────────────────────── 组合器与祖先链 ───────────────────────

#[test]
fn descendant_and_child_combinators_differ() {
    let ss = parse(
        r#"
        .page .txt { color: #ff0000; }
        .card > .txt { color: #00ff00; }
        "#,
    );
    let page = ElementDesc::new("view", None, &["page"], &no_attrs());
    let card = ElementDesc::new("view", None, &["card"], &no_attrs());
    let txt = ElementDesc::new("text", None, &["txt"], &no_attrs());

    // page > card > txt：后代选择器命中，子选择器也命中（card 是直接父）
    let both = ss.get_styles_chain(&[page.clone(), card.clone(), txt.clone()]);
    assert_eq!(color_of(&both, "color"), "#00ff00", "同特异性时后写的赢");

    // page > txt：只有后代选择器命中
    let only_desc = ss.get_styles_chain(&[page.clone(), txt.clone()]);
    assert_eq!(color_of(&only_desc, "color"), "#ff0000");

    // card > view > txt：`>` 要求直接父，隔了一层就不该命中
    let spacer = ElementDesc::new("view", None, &[], &no_attrs());
    let indirect = ss.get_styles_chain(&[card, spacer, txt]);
    assert_eq!(color_of(&indirect, "color"), "<缺失>", "子选择器不能跨层匹配");
}

#[test]
fn selector_groups_share_one_declaration_block() {
    let ss = parse(".a, .b, #c { color: #123456; }");
    for (tag, id, class) in [("view", None, "a"), ("view", None, "b")] {
        let s = ss.get_styles_el(tag, id, &[class], &no_attrs());
        assert_eq!(color_of(&s, "color"), "#123456", ".{class} 应命中");
    }
    let s = ss.get_styles_el("view", Some("c"), &[], &no_attrs());
    assert_eq!(color_of(&s, "color"), "#123456", "#c 应命中");
}

#[test]
fn id_beats_class_beats_tag() {
    let ss = parse(
        r#"
        view { color: #111111; }
        .t { color: #222222; }
        #x { color: #333333; }
        "#,
    );
    let s = ss.get_styles_el("view", Some("x"), &["t"], &no_attrs());
    assert_eq!(color_of(&s, "color"), "#333333", "id 特异性最高");
    let s2 = ss.get_styles_el("view", None, &["t"], &no_attrs());
    assert_eq!(color_of(&s2, "color"), "#222222", "class 高于 tag");
}

// ───────────────────────────── @import ─────────────────────────────

#[test]
fn imported_rules_come_before_local_ones() {
    // `@import` 的规则相当于写在文件开头：同特异性时本文件覆盖被导入的
    let imported = parse(".btn { color: #111111; padding: 8px; }");
    let mut local = parse(".btn { color: #222222; }");
    local.prepend_rules(imported);
    let s = local.get_styles_el("view", None, &["btn"], &no_attrs());
    assert_eq!(color_of(&s, "color"), "#222222", "本文件的声明胜出");
    assert_eq!(px_of(&s, "padding"), Some(8.0), "被导入文件里独有的声明要保留");
}

#[test]
fn import_paths_are_collected_for_the_host_to_resolve() {
    let ss = parse("@import \"common/base.wxss\";\n.a{color:#000000;}");
    assert!(
        ss.imports.iter().any(|p| p.contains("common/base.wxss")),
        "@import 的路径要收集出来交给宿主加载，实际 {:?}",
        ss.imports
    );
}

// ───────────────────────── @keyframes ─────────────────────────

#[test]
fn keyframes_are_indexed_by_name_with_later_wins() {
    let ss = parse(
        r#"
        @keyframes spin { from { opacity: 0; } to { opacity: 1; } }
        @keyframes spin { from { opacity: 0.5; } to { opacity: 1; } }
        "#,
    );
    let k = ss.keyframes_named("spin").expect("应找到 spin");
    assert_eq!(k.name, "spin");
    assert!(k.steps.len() >= 2, "至少两个时间点");
    // 同名后定义覆盖先定义
    let from = k
        .steps
        .iter()
        .find(|s| s.offset <= 0.01)
        .expect("应有 0% 的关键帧");
    match from.properties.get("opacity") {
        Some(StyleValue::Number(v)) => assert!((*v - 0.5).abs() < 0.001, "后定义的 from 应为 0.5，实际 {v}"),
        other => panic!("opacity 解析异常: {other:?}"),
    }
}

#[test]
fn webkit_prefixed_keyframes_are_accepted() {
    let ss = parse("@-webkit-keyframes fade { from { opacity: 0; } to { opacity: 1; } }");
    assert!(
        ss.keyframes_named("fade").is_some(),
        "带 -webkit- 前缀的 @keyframes 也要收（编译产物里很常见）"
    );
}

// ───────────────────────── :active 按压态 ─────────────────────────

#[test]
fn active_rules_only_apply_when_pressed() {
    let ss = parse(".b { color: #111111; } .b:active { color: #ff0000; }");
    assert!(ss.has_active_rules(), "应报告存在 :active 规则");
    let idle = ElementDesc::new("view", None, &["b"], &no_attrs());
    let down = ElementDesc::new("view", None, &["b"], &no_attrs()).with_pressed(true);
    assert_eq!(color_of(&ss.get_styles_chain(&[idle]), "color"), "#111111");
    assert_eq!(color_of(&ss.get_styles_chain(&[down]), "color"), "#ff0000");
}

#[test]
fn stylesheet_without_active_rules_reports_false() {
    // 没有 :active 时建树期就不必多算一份按压态样式，这个判断要准
    let ss = parse(".b { color: #111111; }");
    assert!(!ss.has_active_rules());
    assert!(!ss.has_attr_selectors());
}
