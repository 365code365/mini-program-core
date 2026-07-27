//! 「子节点的样式必须算它自己的」这一类回归。
//!
//! 三个曾经失效的场景，都是拿 tea-app 登录页 / sample-app 优惠券页对照微信与 Chrome 才发现的：
//!
//! 1. `page { font-size: 28rpx }` 继承下去被乘了一次 scale_factor，@2x 上所有靠继承的
//!    文字整整大一倍；
//! 2. `<button>` 被当成叶子，里面 `<text class="…">` 的颜色/字号被整棵吞掉；
//! 3. `<text>` 里嵌套 `<text>` 时整段被拍平成一个字符串，内层的字号/颜色全丢。
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::json;

fn render(wxss: &str, wxml: &str, sf: f32) -> WxmlRenderer {
    let ss = WxssParser::new(wxss).parse().unwrap();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, sf);
    let mut c = Canvas::new((375.0 * sf) as u32, (667.0 * sf) as u32);
    let nodes = WxmlParser::new(wxml).parse().unwrap();
    let mut im = crate::ui::interaction::InteractionManager::new();
    r.render_with_interaction(&mut c, &nodes, &json!({}), &mut im);
    r
}

/// `page{font-size}` 继承到没写字号的节点上：@2x 也必须是 14 逻辑 px，不是 28。
#[test]
fn page_font_size_inherits_without_double_scaling() {
    let r = render(
        "page { font-size: 28rpx; color: #2d251e; } .plain { padding: 10px; }",
        r#"<view class="plain">继承字号的裸文字</view>"#,
        2.0,
    );
    let (fs, color, _) = r.node_style_by_class("plain").expect("节点应在树里");
    assert!((fs - 14.0).abs() < 0.51, "28rpx 应继承成 14 逻辑 px，实际 {fs}");
    let c = color.expect("page 的文字色要继承下来");
    assert_eq!((c.r, c.g, c.b), (0x2d, 0x25, 0x1e));
}

/// `<button>` 里的 `<text class="…">` 用自己的颜色和字号（微信里 button 是普通容器）。
#[test]
fn button_child_text_keeps_its_own_style() {
    let r = render(
        "page { font-size: 28rpx; color: #2d251e; }
         .wx-btn { background-color: #07c160; height: 88rpx; }
         .wx-btn-t { color: #ffffff; font-size: 30rpx; font-weight: bold; }",
        r#"<button class="wx-btn"><text class="wx-btn-t">微信一键登录</text></button>"#,
        2.0,
    );
    let (btn_fs, _, btn_text) = r.node_style_by_class("wx-btn").expect("按钮应在树里");
    let (fs, color, text) = r.node_style_by_class("wx-btn-t").expect("按钮里的 text 不该被吞掉");
    assert_eq!(text, "微信一键登录");
    assert!((fs - 15.0).abs() < 0.51, "30rpx 应是 15 逻辑 px，实际 {fs}");
    let c = color.expect("子 text 的颜色要生效");
    assert_eq!((c.r, c.g, c.b), (0xff, 0xff, 0xff), "子 text 写了 #fff，不该退回按钮的深色");
    assert!(btn_text.is_empty(), "文字交给子节点画，按钮自己不该再画一遍（实为 {btn_text:?}）");
    assert!((btn_fs - 14.0).abs() < 0.51, "按钮自己继承 page 的 14 逻辑 px");
}

/// `<text>` 里嵌套 `<text>`：外层「¥」小、内层数字大，各按自己的 CSS 出。
#[test]
fn nested_text_runs_keep_per_run_font_size() {
    let r = render(
        ".c-amount { font-size: 32rpx; color: #ffffff; }
         .c-num { font-size: 64rpx; color: #ffffff; }",
        r#"<text class="c-amount">¥<text class="c-num">10</text></text>"#,
        2.0,
    );
    // 内层那一段单独成 run，字号是它自己的 64rpx = 32 逻辑 px
    let (inner_fs, _, inner_text) = r.node_style_by_class("c-num").expect("内层 text 应在树里");
    assert_eq!(inner_text, "10");
    assert!((inner_fs - 32.0).abs() < 0.51, "64rpx 应是 32 逻辑 px，实际 {inner_fs}");
    // 外层容器自己不画文字（交给 run），高度按大号数字那一行算
    let (_, _, outer_text) = r.node_style_by_class("c-amount").expect("外层 text 应在树里");
    assert!(outer_text.is_empty(), "外层容器不该再拍平画一遍（实为 {outer_text:?}）");
    let (_, h) = r.node_size_by_class("c-amount").expect("外层应有尺寸");
    assert!(h >= 60.0, "行高要按 32 逻辑 px 的那一段算（物理 ≥60），实际 {h}");
}

/// 混排里出现非 `<text>` 元素时退回原来的拍平逻辑 —— 本引擎没有真正的行内流，
/// 硬拆会把图文混排拆成上下两行。
#[test]
fn text_with_non_text_child_falls_back_to_flattening() {
    let r = render(
        ".mix { font-size: 28rpx; }",
        r#"<text class="mix">前<image src="/a.png"></image>后</text>"#,
        2.0,
    );
    let (_, _, text) = r.node_style_by_class("mix").expect("节点应在树里");
    assert_eq!(text, "前后", "应保持拍平成一段文字");
}
