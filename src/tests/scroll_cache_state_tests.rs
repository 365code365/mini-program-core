//! `scroll-view` 离屏缓存里的**有状态组件**：与正常流走同一份状态落地。
//!
//! 为什么单独测：滚动容器的内容是画进离屏画布再上屏的（`draw_child_to_cache`），
//! 那条路径曾经带着一份**退化的**状态落地实现 —— 三条绘制路径各写一份，其中两份逐字节
//! 相同、第三份（缓存路径）少两个分支：
//!
//! 1. **不落 `input` / `textarea` 的当前值** —— 用户在滚动容器里输入的字不显示。
//!    占位符照旧能看见（那是组件自己按 attrs 画的），所以现象是「敲了字但框里还是灰提示」，
//!    很容易被当成输入法没生效；
//! 2. **把 `switch` 和 `checkbox` 合成一条分支** —— switch 因此被套上勾选框的选中配色，
//!    并且丢掉拨动进度（只剩 0/1，拖到一半不跟手）。
//!
//! 示例里唯一命中这种结构的是 tea-app（uni-app 产物，搜索框套在 `scroll-view` 里），
//! 而它不在逐页快照基线里 —— 像素回归发现不了。这里用最小页面钉住第 1 条
//! （最不容易误判的那条：文字画出来了没有）。

use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::ui::interaction::{ComponentState, InteractionManager};
use crate::Canvas;
use serde_json::json;

const CSS: &str = r#"
    .box { height: 200px; width: 320px; }
    input { height: 40px; width: 280px; font-size: 16px; }
"#;

/// 画一遍并数「明显偏离白底的像素」。
///
/// 状态在**第一次渲染之前**就塞好：组件 id 取的是 WXML 上的 `id` 属性，所以不必先渲染
/// 一遍去发现它。这一点很重要 —— `scroll-view` 的离屏缓存只在内容版本变化时重画，
/// 渲染之后再改交互状态是看不到的。
fn ink(wxml: &str, states: &[(&str, ComponentState)]) -> u32 {
    let sheet = WxssParser::new(CSS).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, 2.0);
    let nodes = WxmlParser::new(wxml).parse().unwrap_or_default();
    let mut canvas = Canvas::new(750, 800);
    canvas.clear(crate::Color::WHITE);
    let mut im = InteractionManager::new();
    for (id, st) in states {
        im.set_state((*id).to_string(), st.clone());
    }
    r.render_with_interaction(&mut canvas, &nodes, &json!({}), &mut im);
    canvas
        .pixels()
        .iter()
        .filter(|p| {
            (p.r as i32 - 255).abs() > 12
                || (p.g as i32 - 255).abs() > 12
                || (p.b as i32 - 255).abs() > 12
        })
        .count() as u32
}

fn typed(value: &str) -> Vec<(&'static str, ComponentState)> {
    vec![(
        "ipt",
        ComponentState {
            checked: false,
            value: value.to_string(),
        },
    )]
}

#[test]
fn 滚动容器里输入的文字要真的画出来() {
    let wxml =
        r#"<scroll-view class="box" scroll-y="true"><input id="ipt" placeholder="搜" /></scroll-view>"#;
    let empty = ink(wxml, &[]);
    let filled = ink(wxml, &typed("已经输入的一长串内容"));
    assert!(
        filled > empty + 300,
        "滚动容器里输入的文字没画出来：只有占位符时墨迹 {empty}，输入后 {filled}\n\
         —— 离屏缓存那条路径的状态落地又退化了（见 renderer/wxml_renderer/interactive.rs）"
    );
}

#[test]
fn 容器内外的输入框显示同一份内容() {
    // 同一个值、同样的输入框，只是一个套在 scroll-view 里 —— 画出来的墨量应当接近。
    // 退化实现下容器内根本不画那串文字，两者会差出整段文本的量。
    let inside = ink(
        r#"<scroll-view class="box" scroll-y="true"><input id="ipt" placeholder="搜" /></scroll-view>"#,
        &typed("同一段文字"),
    );
    let outside = ink(
        r#"<view class="box"><input id="ipt" placeholder="搜" /></view>"#,
        &typed("同一段文字"),
    );
    let delta = inside.abs_diff(outside);
    assert!(
        delta * 10 <= outside.max(1),
        "容器内外画得差太多（内 {inside} / 外 {outside}，差 {delta}）—— 缓存路径又漏了状态"
    );
}
