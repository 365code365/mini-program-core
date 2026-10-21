//! WXSS `@media` 与 `@font-face`（G-3）。
//!
//! `@media` 在解析期按本线程的环境量求值（屏宽 / 屏高 / 深浅色，见 parser/wxss/media.rs），
//! 成立的块里的规则**就地**并进样式表（书写顺序决定层叠）；`@font-face` 登记字族
//! （src/text_faces.rs），`font-family` 写它的文字换成那个字体。
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::{set_env_context, EnvContext, StyleValue, WxssParser};
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::{Canvas, Color};
use serde_json::json;

fn color_of(css: &str, class: &str) -> Option<String> {
    let ss = WxssParser::new(css).parse().ok()?;
    let attrs = std::collections::HashMap::new();
    let chain = vec![crate::parser::wxss::ElementDesc::new("view", None, &[class], &attrs)];
    let styles = ss.get_styles_chain(&chain);
    styles.get("color").map(|v| match v {
        StyleValue::Color(c) => format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b),
        other => format!("{other:?}"),
    })
}

fn with_env<T>(w: f32, h: f32, dark: bool, f: impl FnOnce() -> T) -> T {
    set_env_context(EnvContext { screen_width: w, screen_height: h, dark, ..Default::default() });
    let r = f();
    set_env_context(EnvContext::default());
    r
}

#[test]
fn media_block_applies_only_when_condition_holds() {
    let css = ".a{color:#000000} @media (max-width: 400px) { .a{color:#ff0000} } @media (min-width: 800px) { .a{color:#00ff00} }";
    assert_eq!(with_env(375.0, 667.0, false, || color_of(css, "a")).as_deref(), Some("#ff0000"), "375 宽：max-width 400 成立");
    assert_eq!(with_env(1024.0, 768.0, false, || color_of(css, "a")).as_deref(), Some("#00ff00"), "1024 宽：min-width 800 成立");
}

#[test]
fn media_rules_keep_source_order_in_cascade() {
    // 媒体块在前、普通规则在后：同特异性时后写的赢（不能把媒体块挪到最后）
    let css = "@media screen { .a{color:#ff0000} } .a{color:#0000ff}";
    assert_eq!(with_env(375.0, 667.0, false, || color_of(css, "a")).as_deref(), Some("#0000ff"));
}

#[test]
fn prefers_color_scheme_dark() {
    let css = ".a{color:#111111} @media (prefers-color-scheme: dark) { .a{color:#eeeeee} }";
    assert_eq!(with_env(375.0, 667.0, false, || color_of(css, "a")).as_deref(), Some("#111111"));
    assert_eq!(with_env(375.0, 667.0, true, || color_of(css, "a")).as_deref(), Some("#eeeeee"));
}

#[test]
fn nested_media_and_keyframes_inside_media() {
    let css = "@media screen { @media (orientation: portrait) { .a{color:#123456} } @keyframes k { to { opacity: 0; } } }";
    let ss = with_env(375.0, 667.0, false, || WxssParser::new(css).parse().unwrap());
    assert_eq!(ss.keyframes.len(), 1, "媒体块里的 @keyframes 也要收进来");
    assert_eq!(with_env(375.0, 667.0, false, || color_of(css, "a")).as_deref(), Some("#123456"));
}

/// 节点宽度（物理像素）
fn width_of(css: &str, wxml: &str) -> f32 {
    let ss = WxssParser::new(css).parse().unwrap();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
    let mut c = Canvas::new(750, 300);
    c.clear(Color::WHITE);
    let nodes = WxmlParser::new(wxml).parse().unwrap();
    let mut im = crate::ui::interaction::InteractionManager::new();
    r.render_with_interaction(&mut c, &nodes, &json!({}), &mut im);
    r.node_size_by_class("t").map(|s| s.0).unwrap_or(0.0)
}

#[test]
fn font_face_registers_family_and_changes_metrics() {
    let mono = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/Monaco.ttf");
    // 横排容器：文字按内容宽度排（竖排时块级文字一律撑满整行，量不出差别）
    let wxml = r#"<view style="display:flex;flex-direction:row"><text class="t">iiiiiiii</text></view>"#;
    let base = width_of(".t{font-family:'E2E Mono Media';font-size:20px}", wxml);
    let css = format!("@font-face {{ font-family: 'E2E Mono Media'; src: url('{mono}') format('truetype'); }} .t{{font-family:'E2E Mono Media';font-size:20px}}");
    let ss = WxssParser::new(&css).parse().unwrap();
    assert_eq!(ss.font_faces.len(), 1);
    assert_eq!(ss.font_faces[0].family, "E2E Mono Media");
    assert!(crate::text_faces::face_path("e2e mono media").is_some(), "@font-face 解析时就登记了");
    let with_face = width_of(&css, wxml);
    // Monaco 是等宽字体：一串 i 比比例字体宽得多
    assert!(with_face > base * 1.3, "等宽字体下 i 变宽：默认 {base}，登记后 {with_face}");
}

#[test]
fn font_face_rejects_woff() {
    let dir = std::env::temp_dir().join("mini-render-woff-test");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("x.woff2");
    std::fs::write(&p, b"wOF2\0\0\0\0").unwrap();
    let r = crate::text_faces::register_file("E2E Woff", &p);
    assert!(r.unwrap_err().contains("woff"));
    assert!(crate::text_faces::face_path("e2e woff").is_none());
    std::fs::remove_dir_all(&dir).ok();
}

/// Skyline 不支持 `@supports` / `@container` / `@layer`（官方 WXSS 支持表的模块里没有，媒体查询只支持
/// DarkMode）：整块跳过、块里的规则一条都不生效，块后面的规则照常解析。H5 导出同样丢掉（transpile/css.rs）
#[test]
fn skyline_unsupported_at_rules_are_skipped_whole() {
    let css = ".a{color:#000000} @supports (display:flex) { .a{color:#ff0000} } \
               @container card (min-width: 1px) { .a{color:#00ff00} } @layer base, theme; \
               @layer theme { .a{color:#0000ff} } .b{color:#123456}";
    assert_eq!(color_of(css, "a").as_deref(), Some("#000000"));
    assert_eq!(color_of(css, "b").as_deref(), Some("#123456"), "跳过之后的规则照常解析");
}
