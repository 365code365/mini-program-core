//! 行内文本在 flex 行里的收缩行为。
//!
//! 背景：行内 `<text>` 被建成**定宽叶子**（宽度 = 单行内容宽），于是它的
//! min-content 等于整行宽。taffy 0.12 开始严格执行 flex 项的自动最小宽度
//! （`min-width:auto` = min-content），「一行里几段文本加起来略超容器宽」时
//! 收缩量就会落到这些文本上：文字被压窄 → 绘制期折行 → 溢出容器。
//!
//! 现在这些文本 `flex-shrink: 0`，宁可溢出也不被压 —— 理由见 text.rs 里的注释
//! （让它们真正可压需要文字度量与 Chrome 完全一致，1px 误差就会让标题误折行，
//! 实测首页双端差异 4.1% → 13.0%）。
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::json;

/// 优惠券行：左侧定宽金额 + 中间 `flex:1` 文案 + 右侧胶囊按钮。
///
/// 三者宽度加起来略超容器（中间那栏的 min-content 被里面的定宽文本抬高了），
/// 按钮**不能**因此被压窄 —— 压窄之后「立即领取」会折成两行并溢出卡片。
/// 按钮宽度取自 Chrome 实测（同款 CSS 下 4×13px 字 + 左右 12px 内边距 = 76 逻辑 px，
/// 本例 scale=2 故 152 物理 px）。
#[test]
fn inline_text_pill_is_not_squeezed_in_overconstrained_row() {
    let wxml = r##"<view class="cp row">
        <view class="cp-l col"><text class="amt">20</text></view>
        <view class="cp-m col grow"><text class="cp-name">全场通用券</text><text class="cp-time">有效期至 07-31</text></view>
        <text class="cp-btn">立即领取</text>
      </view>"##;
    let wxss = r##"
    .row{ display:flex; flex-direction:row; align-items:center; }
    .col{ display:flex; flex-direction:column; }
    .grow{ flex:1; }
    .cp{ display:flex; flex-direction:row; align-items:center; width:540rpx; padding:24rpx; }
    .cp-l{ display:flex; flex-direction:column; align-items:center; width:170rpx; }
    .amt{ font-size:64rpx; font-weight:bold; }
    .cp-m{ display:flex; flex-direction:column; flex:1; padding-left:24rpx; }
    .cp-name{ font-size:30rpx; font-weight:bold; }
    .cp-time{ font-size:22rpx; margin-top:10rpx; }
    .cp-btn{ font-size:26rpx; padding:14rpx 24rpx; }
    "##;
    let ss = WxssParser::new(wxss).parse().unwrap();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0);
    let mut c = Canvas::new(750, 1334);
    let nodes = WxmlParser::new(wxml).parse().unwrap();
    let mut im = crate::ui::interaction::InteractionManager::new();
    r.render_with_interaction(&mut c, &nodes, &json!({}), &mut im);

    let (w, h) = r.node_size_by_class("cp-btn").expect("按钮节点应该在布局树里");
    assert!(
        (w - 152.0).abs() < 1.5,
        "胶囊按钮被压窄了：宽 {w}（应为 152 物理 px，Chrome 同款 CSS 实测 76 逻辑 px）"
    );
    // 单行高度 = 26rpx 字号的自然行高 + 上下 14rpx 内边距，折成两行会明显更高
    assert!(h < 80.0, "按钮文字折行了：高 {h}（单行应在 80 物理 px 以内）");
}

/// `box-sizing` 必须落地：`content-box` 的盒子要在声明宽度之外再撑出内边距。
///
/// 这条属性从前是被整条忽略的（旧版布局引擎没有这个概念），于是写
/// `box-sizing: content-box` 的元素比浏览器窄一圈内边距。两端缺省都是
/// `border-box`，所以只有显式写 content-box 的地方会变。
/// 期望值与 Chrome 一致：120 + 左右各 10 = 140。
#[test]
fn box_sizing_content_box_adds_padding_to_width() {
    let wxml = r##"<view class="wrap">
        <view class="border-box">a</view>
        <view class="content-box">b</view>
      </view>"##;
    let wxss = r##"
    .wrap{ display:flex; flex-direction:column; width:375px; }
    .border-box{ width:120px; height:40px; padding:10px; box-sizing:border-box; }
    .content-box{ width:120px; height:40px; padding:10px; box-sizing:content-box; }
    "##;
    let ss = WxssParser::new(wxss).parse().unwrap();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 1.0);
    let mut c = Canvas::new(375, 667);
    let nodes = WxmlParser::new(wxml).parse().unwrap();
    let mut im = crate::ui::interaction::InteractionManager::new();
    r.render_with_interaction(&mut c, &nodes, &json!({}), &mut im);

    let (bw, bh) = r.node_size_by_class("border-box").expect("border-box 节点");
    let (cw, ch) = r.node_size_by_class("content-box").expect("content-box 节点");
    assert!((bw - 120.0).abs() < 0.6 && (bh - 40.0).abs() < 0.6, "border-box 应就是 120x40，实为 {bw}x{bh}");
    assert!((cw - 140.0).abs() < 0.6 && (ch - 60.0).abs() < 0.6, "content-box 应是 140x60，实为 {cw}x{ch}");
}

// ============ min-content 不得大于 max-content ============

fn probe_measure(text: &str, font_px: f32, max_line: f32, min_unit: f32) -> f32 {
    use crate::renderer::components::{measure_text_node, TextMeasure};
    use taffy::style::AvailableSpace;
    let tm = TextMeasure {
        text: text.to_string(),
        font_px,
        letter_spacing_px: 0.0,
        line_height_px: font_px * 1.2,
        pad_l: 0.0,
        pad_r: 0.0,
        pad_t: 0.0,
        pad_b: 0.0,
        nowrap: true, // 只关心宽度，别让行数估算掺进来
        bold: false,
        min_lines: 1,
        max_line_width: max_line,
        min_unit_width: min_unit,
        font_family: None,
    };
    measure_text_node(
        taffy::geometry::Size { width: None, height: None },
        taffy::geometry::Size {
            width: AvailableSpace::Definite(400.0),
            height: AvailableSpace::MaxContent,
        },
        &tm,
        None,
    )
    .width
}

/// 窄字符（`+` `−` `:` `¥`）的文本盒不能被「最小内容宽」的 1em 底顶宽。
///
/// `min_unit_width` 末尾垫了 `max(font_px)`（断行按整字切，可用宽再小也要放得下一个字），
/// 但窄字符的 advance 比 1em 小得多 —— 宋体的 `+` 在 36px 下只有 24。取用方不夹一次，
/// 单个 `+` 的盒子就被顶成 36：盒子被 `justify-content:center` 居中了，
/// 里面的字形却偏左 3px，圆形按钮里的加号看着没对准圆心。
#[test]
fn narrow_glyph_box_is_not_inflated_by_one_em_floor() {
    // 宋体 `+` 在 36px 下的真实数值：advance 24.01（取整 25），而 1em = 36
    let w = probe_measure("+", 36.0, 25.0, 36.0);
    assert!(
        (w - 25.0).abs() < 0.01,
        "单个 `+` 的盒宽应等于它的 max-content 25，不该被 1em 底顶到 36，实际 {w}"
    );
}

/// 一般文本（max-content 大于 1em）不受影响：底仍然生效
#[test]
fn wide_text_still_keeps_min_content_floor() {
    // 可用宽 400 足够，取 max-content
    let w = probe_measure("一段比较长的文字", 28.0, 224.0, 28.0);
    assert!((w - 224.0).abs() < 0.01, "放得下时应取 max-content，实际 {w}");
}

/// MinContent 询问下也不能超过 max-content（祖先的自动最小尺寸靠它）
#[test]
fn min_content_query_never_exceeds_max_content() {
    use crate::renderer::components::{measure_text_node, TextMeasure};
    use taffy::style::AvailableSpace;
    let tm = TextMeasure {
        text: "+".into(),
        font_px: 36.0,
        letter_spacing_px: 0.0,
        line_height_px: 43.0,
        pad_l: 0.0,
        pad_r: 0.0,
        pad_t: 0.0,
        pad_b: 0.0,
        nowrap: true,
        bold: false,
        min_lines: 1,
        max_line_width: 25.0,
        min_unit_width: 36.0,
        font_family: None,
    };
    let w = measure_text_node(
        taffy::geometry::Size { width: None, height: None },
        taffy::geometry::Size {
            width: AvailableSpace::MinContent,
            height: AvailableSpace::MaxContent,
        },
        &tm,
        None,
    )
    .width;
    assert!(w <= 25.0 + 0.01, "min-content 不能大于 max-content（25），实际 {w}");
}
