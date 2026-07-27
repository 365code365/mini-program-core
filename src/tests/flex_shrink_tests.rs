//! flex 溢出行为测试：**定高容器装不下的内容不应被压扁**。
//!
//! CSS flexbox 的「自动最小尺寸」（min-height: auto，见 css-flexbox-1 §4.5）规定
//! flex item 不能被压到内容尺寸以下：装不下就溢出，而不是把文字挤没。
//! Skyline 与浏览器都是这个行为。
//!
//! tea-app 首页导航栏正中这样写：
//! ```text
//! .nav { height: 92rpx; padding-top: <状态栏+8>px; align-items:center; justify-content:center }
//! .brand > text.brand-cn (46rpx 行高) + text.brand-en (20rpx 字号)
//! ```
//! 定高 46px 扣掉 28px 内边距只剩 18px，两行文字需要 38px。我们以前会把两个
//! 文本节点按 flex-shrink 一路压到几像素高，第二行（`PHOENIX YUNXIU`）直接消失。

use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::json;

/// scale=1.0，直接用逻辑像素断言
fn rects(css: &str, wxml: &str) -> Vec<(String, f32, f32)> {
    let ss = WxssParser::new(css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 1.0);
    let mut c = Canvas::new(375, 667);
    let ns = WxmlParser::new(wxml).parse().unwrap();
    r.render(&mut c, &ns, &json!({}));
    r.get_event_bindings()
        .iter()
        .map(|b| (b.handler.clone(), b.bounds.y, b.bounds.height))
        .collect()
}

/// 定高 + padding 的 flex 列容器里，两段文字都要保住各自的行高。
#[test]
fn text_in_overflowing_fixed_height_column_keeps_its_line_height() {
    let css = "
        .nav { height: 46px; padding-top: 28px; align-items: center; justify-content: center; }
        .brand { align-items: center; justify-content: center; }
        .cn { font-size: 20px; line-height: 23px; }
        .en { font-size: 10px; margin-top: 3px; }
    ";
    let wxml = r#"<view class="nav"><view class="brand">
        <text class="cn" bindtap="cn">凤 凰 云 岫</text>
        <text class="en" bindtap="en">PHOENIX YUNXIU</text>
    </view></view>"#;
    let got = rects(css, wxml);
    let cn = got.iter().find(|(h, _, _)| h == "cn").expect("cn 节点应存在");
    let en = got.iter().find(|(h, _, _)| h == "en").expect("en 节点应存在");
    assert!(
        (cn.2 - 23.0).abs() < 1.0,
        "定高容器装不下时首行也不该被压缩：期望 23px，实际 {:.1}px",
        cn.2
    );
    // 10px 字号的自然行高约 12px；被压扁的话这里会是 4~6px，文字整行不见
    assert!(
        en.2 >= 11.0,
        "第二行文字被压扁了（`PHOENIX YUNXIU` 会整行消失）：高度 {:.1}px",
        en.2
    );
    // 第二行必须排在第一行下面，而不是重叠
    assert!(
        en.1 >= cn.1 + cn.2 - 0.5,
        "第二行应紧随第一行：cn(y={:.1},h={:.1}) en(y={:.1})",
        cn.1,
        cn.2,
        en.1
    );
}

/// 同一段内容在不定高容器里的排布是「正确答案」，定高溢出时应与它一致。
#[test]
fn overflowing_layout_matches_auto_height_layout() {
    let css_auto = "
        .nav { align-items: center; justify-content: center; }
        .brand { align-items: center; justify-content: center; }
        .cn { font-size: 20px; line-height: 23px; }
        .en { font-size: 10px; margin-top: 3px; }
    ";
    let css_fixed = css_auto.replace(
        ".nav { align-items",
        ".nav { height: 46px; padding-top: 28px; align-items",
    );
    let wxml = r#"<view class="nav"><view class="brand">
        <text class="cn" bindtap="cn">凤 凰 云 岫</text>
        <text class="en" bindtap="en">PHOENIX YUNXIU</text>
    </view></view>"#;
    let a = rects(css_auto, wxml);
    let b = rects(&css_fixed, wxml);
    for name in ["cn", "en"] {
        let ha = a.iter().find(|(h, _, _)| h == name).unwrap().2;
        let hb = b.iter().find(|(h, _, _)| h == name).unwrap().2;
        assert!(
            (ha - hb).abs() < 1.0,
            "{name} 的高度在定高容器里变了：不定高 {ha:.1}px vs 定高 {hb:.1}px"
        );
    }
}

/// `letter-spacing` 不该让文本盒凭空多一行。
///
/// 盒子宽度按内容宽定（含字间距），断行算法必须用同一把尺子，否则「刚好装下」
/// 会被判成「装不下」，最后一个字掉到第二行、盒子高一倍。这是上面那个
/// 定高导航栏 bug 的真正起因：`.brand-cn` 变两行，把 `.brand-en` 挤出容器。
#[test]
fn letter_spacing_does_not_add_a_phantom_line() {
    let base = ".t { font-size: 20px; line-height: 23px; }";
    let spaced = ".t { font-size: 20px; line-height: 23px; letter-spacing: 3px; }";
    let wxml = r#"<view><text class="t" bindtap="t">凤 凰 云 岫</text></view>"#;
    let h_plain = rects(base, wxml)
        .into_iter()
        .find(|(h, _, _)| h == "t")
        .unwrap()
        .2;
    let h_spaced = rects(spaced, wxml)
        .into_iter()
        .find(|(h, _, _)| h == "t")
        .unwrap()
        .2;
    assert!((h_plain - 23.0).abs() < 1.0, "无字间距时应单行 23px，实际 {h_plain:.1}px");
    assert!(
        (h_spaced - h_plain).abs() < 1.0,
        "加了 letter-spacing 后行数变了：{h_plain:.1}px -> {h_spaced:.1}px"
    );
}

/// 字间距的宽度口径与 Chrome 一致：**每个字符后各加一份，末字符也算**。
/// （Chrome 实测：`letter-spacing:10px` 的 4 字符 inline-block 正好宽 40px。）
#[test]
fn letter_spacing_width_includes_trailing_gap() {
    let tr = match crate::text::shared_fonts() {
        Some(t) => t,
        None => return,
    };
    let w0 = tr.measure_text_weighted("abcd", 20.0, 0.0, false);
    let w10 = tr.measure_text_weighted("abcd", 20.0, 10.0, false);
    assert!(
        ((w10 - w0) - 40.0).abs() < 0.01,
        "4 个字符加 10px 字间距应宽 40px，实际 {:.2}px",
        w10 - w0
    );
}
