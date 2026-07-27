//! `font-family` 端到端：样式解析 → 继承 → 度量/绘制都换成对应字族。
//!
//! 之前 `font-family` 整条被忽略，全局只有一个系统黑体，于是所有声明宋体/衬线的
//! 文字都画成黑体。这几个用例锁住「声明了就真的换字体」，以及「度量与绘制用同一族」
//! （两边不一致会让盒子宽度和文字对不上，进而误换行）。
//!
//! 本机没装对应字体时（非 macOS / 精简系统）用例自动跳过 —— 判据是「本机有没有」，
//! 与浏览器的字体栈解析一致。

use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::{Canvas, Color};
use serde_json::json;

fn render(css: &str, wxml: &str) -> Canvas {
    let ss = WxssParser::new(css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
    let mut c = Canvas::new(750, 300);
    c.clear(Color::WHITE);
    let nodes = WxmlParser::new(wxml).parse().unwrap_or_default();
    r.render(&mut c, &nodes, &json!({}));
    c
}

/// 墨量（越黑越大），用来判断「是不是换了字形」
fn ink(c: &Canvas) -> u64 {
    c.pixels().iter().map(|p| (255 - p.r) as u64).sum()
}

fn has_songti() -> bool {
    crate::text_family::family_font_path("Songti SC").is_some()
}

/// 声明宋体后，同一段文字的字形必须和默认黑体不同。
#[test]
fn declared_serif_family_changes_glyphs() {
    if !has_songti() {
        return;
    }
    let wxml = r#"<view><text class="t">凤凰云岫の茶</text></view>"#;
    let base = render(".t{font-size:40px;color:#000}", wxml);
    let serif = render(
        ".t{font-size:40px;color:#000;font-family:\"Songti SC\",\"STSong\",serif}",
        wxml,
    );
    assert!(ink(&base) > 0 && ink(&serif) > 0, "两边都应该画出文字");
    assert_ne!(ink(&base), ink(&serif), "声明宋体后字形应当变化");
}

/// `font-family` 是继承属性：父级声明、子级 `<text>` 未声明时也要用父级的字族。
#[test]
fn font_family_is_inherited() {
    if !has_songti() {
        return;
    }
    let wxml = r#"<view class="wrap"><text class="t">凤凰云岫</text></view>"#;
    let base = render(".wrap{}.t{font-size:40px;color:#000}", wxml);
    let inherited = render(
        ".wrap{font-family:\"Songti SC\",serif}.t{font-size:40px;color:#000}",
        wxml,
    );
    assert_ne!(ink(&base), ink(&inherited), "父级的 font-family 应继承给子级文本");
}

/// `page { font-family }` 作为整页基线，裸文本（不是 `<text>`）也要跟着换。
#[test]
fn page_font_family_applies_to_bare_text() {
    if !has_songti() {
        return;
    }
    let wxml = r#"<view class="t">凤凰云岫</view>"#;
    let base = render(".t{font-size:40px;color:#000}", wxml);
    let paged = render(
        "page{font-family:\"Songti SC\",serif}.t{font-size:40px;color:#000}",
        wxml,
    );
    assert_ne!(ink(&base), ink(&paged), "page 上的 font-family 应作用于裸文本");
}

/// 认不出的字族（本机没装）要按浏览器行为跳到栈里的下一个，最终能落到系统字体，
/// 而不是画不出字。
#[test]
fn unknown_family_falls_back_and_still_draws() {
    let wxml = r#"<view><text class="t">凤凰云岫 ABC</text></view>"#;
    let c = render(
        ".t{font-size:40px;color:#000;font-family:\"No Such Font 12345\",sans-serif}",
        wxml,
    );
    assert!(ink(&c) > 0, "字族认不出时也必须照常画出文字");
}

/// 西文衬线（Times New Roman，没有汉字）+ 中文：汉字要按回退链画出来，不能是豆腐块。
#[test]
fn latin_only_family_falls_back_for_cjk() {
    if crate::text_family::family_font_path("Times New Roman").is_none() {
        return;
    }
    let wxml = r#"<view><text class="t">凤凰</text></view>"#;
    let c = render(
        ".t{font-size:40px;color:#000;font-family:\"Times New Roman\",serif}",
        wxml,
    );
    // 豆腐块/空白的墨量与真实汉字差一个量级，这里只要求「画出了足够的墨」
    assert!(ink(&c) > 20_000, "汉字应通过回退链画出，实际墨量 {}", ink(&c));
}
