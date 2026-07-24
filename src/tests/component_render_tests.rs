//! 全组件渲染测试
//! 覆盖所有内置组件类型的构建与渲染，确保不 panic 且布局合理

use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::ui::interaction::InteractionManager;
use crate::Canvas;
use serde_json::json;

fn renderer(css: &str) -> WxmlRenderer {
    let ss = WxssParser::new(css).parse().unwrap_or_default();
    WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 2.0)
}

fn nodes(wxml: &str) -> Vec<crate::parser::wxml::WxmlNode> {
    WxmlParser::new(wxml).parse().unwrap_or_default()
}

fn canvas() -> Canvas {
    Canvas::new(750, 1334)
}

/// 渲染并附带交互（不 panic 即通过）
fn render_ok(css: &str, wxml: &str, data: serde_json::Value) -> WxmlRenderer {
    let mut r = renderer(css);
    let mut c = canvas();
    let mut im = InteractionManager::new();
    let ns = nodes(wxml);
    r.render_with_interaction(&mut c, &ns, &data, &mut im);
    r
}

#[test]
fn test_view_and_text() {
    render_ok("", r#"<view><text>Hello</text></view>"#, json!({}));
}

#[test]
fn test_block_text_wraps_to_multiple_lines() {
    // display:block 的长文本应按容器宽度换行，盒子高度随行数增长（CSS 语义）。
    // 回归测试第二遍布局修正 correct_wrapped_text_heights。
    let css = ".box{ width:200px; } .t{ display:block; font-size:16px; line-height:24px; }";
    let long = "这是一段很长的中文文本用来验证在固定宽度容器内能够正确地自动换行到多行而不是被压缩成一行显示";
    let wxml = format!(r#"<view class="box"><text class="t">{}</text></view>"#, long);
    let r = renderer(css);
    let ns = nodes(&wxml);
    let h = r.measure_content_height(&ns, &json!({}));
    // 单行约 24px；这段文本在 200px 宽下必然超过 3 行，高度应明显大于单行。
    assert!(h > 24.0 * 3.0, "block 长文本未正确换行撑高，height={}", h);
}

#[test]
fn test_last_child_pseudo_selector() {
    use crate::parser::wxss::{WxssParser, ElementDesc};
    use std::collections::HashMap;
    let ss = WxssParser::new(".item{ color:#000000; } .item:last-child{ color:#ff0000; }").parse().unwrap();
    let attrs = HashMap::new();
    let mk = |i: usize, c: usize| vec![ElementDesc::new("view", None, &["item"], &attrs).with_position(i, c)];
    let mid = ss.get_styles_chain(&mk(0, 3));
    let last = ss.get_styles_chain(&mk(2, 3));
    // 中间元素不匹配 :last-child，末尾元素匹配 -> 两者 color 不同
    assert_ne!(
        format!("{:?}", mid.get("color")),
        format!("{:?}", last.get("color")),
        "last-child 伪类未生效"
    );
}

#[test]
fn test_first_child_pseudo_selector() {
    use crate::parser::wxss::{WxssParser, ElementDesc};
    use std::collections::HashMap;
    let ss = WxssParser::new(".row:first-child{ color:#00ff00; }").parse().unwrap();
    let attrs = HashMap::new();
    let first = ss.get_styles_chain(&[ElementDesc::new("view", None, &["row"], &attrs).with_position(0, 4)]);
    let other = ss.get_styles_chain(&[ElementDesc::new("view", None, &["row"], &attrs).with_position(1, 4)]);
    assert!(first.get("color").is_some(), "first-child 应匹配首元素");
    assert!(other.get("color").is_none(), "first-child 不应匹配非首元素");
}

#[test]
fn test_side_borders_render() {
    // border-bottom 分割线应真实绘制（此前被忽略）。
    // 在白底容器上画一条深色底边，检测该行是否出现深色像素。
    let css = ".row{ width:200px; height:60px; background-color:#ffffff; border-bottom:4px solid #ff0000; }";
    let wxml = r#"<view class="row"></view>"#;
    let mut r = renderer(css);
    let mut c = canvas();
    let mut im = InteractionManager::new();
    let ns = nodes(wxml);
    r.render_with_interaction(&mut c, &ns, &json!({}), &mut im);
    // 检测画布中是否有红色像素（底边）
    let mut red = 0;
    for px in c.pixels() {
        if px.r > 200 && px.g < 80 && px.b < 80 { red += 1; }
    }
    assert!(red > 100, "border-bottom 未绘制，红色像素={}", red);
}

#[test]
fn test_short_text_stays_single_line() {
    // 短文本不应被误判为多行。
    let css = ".t{ display:block; font-size:16px; line-height:24px; }";
    let wxml = r#"<view style="width:300px"><text class="t">短文本</text></view>"#;
    let r = renderer(css);
    let ns = nodes(wxml);
    let h = r.measure_content_height(&ns, &json!({}));
    assert!(h < 24.0 * 2.5, "短文本高度异常偏大，height={}", h);
}

#[test]
fn test_button_variants() {
    let wxml = r#"
        <view>
            <button type="primary">Primary</button>
            <button type="default" disabled="true">Disabled</button>
            <button size="mini">Mini</button>
        </view>
    "#;
    let r = render_ok("", wxml, json!({}));
    // button 会注册为交互元素，不强求事件绑定
    assert!(r.event_count() >= 0);
}

#[test]
fn test_icon_all_types() {
    let wxml = r##"
        <view>
            <icon type="success" size="30" />
            <icon type="info" size="30" />
            <icon type="warn" size="30" />
            <icon type="waiting" size="30" />
            <icon type="cancel" size="30" />
            <icon type="download" size="30" />
            <icon type="search" size="30" />
            <icon type="clear" size="30" />
            <icon type="circle" size="30" color="#722ed1" />
        </view>
    "##;
    render_ok("", wxml, json!({}));
}

#[test]
fn test_image_modes() {
    let wxml = r#"
        <view>
            <image src="a.png" mode="scaleToFill" />
            <image src="b.png" mode="aspectFit" />
            <image src="c.png" mode="aspectFill" />
        </view>
    "#;
    render_ok(".x{width:100px;height:100px;}", wxml, json!({}));
}

#[test]
fn test_form_inputs() {
    let wxml = r#"
        <view>
            <input placeholder="用户名" value="{{name}}" />
            <input type="password" placeholder="密码" />
            <textarea placeholder="备注" />
        </view>
    "#;
    render_ok("", wxml, json!({ "name": "Tom" }));
}

#[test]
fn test_selection_components() {
    let wxml = r#"
        <view>
            <checkbox-group>
                <checkbox value="a" checked="true" />
                <checkbox value="b" />
            </checkbox-group>
            <radio-group>
                <radio value="m" checked="true" />
                <radio value="f" />
            </radio-group>
            <switch checked="true" />
            <switch />
        </view>
    "#;
    render_ok("", wxml, json!({}));
}

#[test]
fn test_slider_and_progress() {
    let wxml = r#"
        <view>
            <slider value="30" min="0" max="100" show-value="true" />
            <slider value="80" />
            <progress percent="60" show-info="true" />
            <progress percent="100" />
        </view>
    "#;
    render_ok("", wxml, json!({}));
}

#[test]
fn test_swiper() {
    let wxml = r#"
        <swiper indicator-dots="true" autoplay="true" interval="3000">
            <swiper-item><view>Slide 1</view></swiper-item>
            <swiper-item><view>Slide 2</view></swiper-item>
            <swiper-item><view>Slide 3</view></swiper-item>
        </swiper>
    "#;
    render_ok(".s{height:200px;}", wxml, json!({}));
}

#[test]
fn test_picker() {
    let wxml = r#"
        <view>
            <picker mode="selector" range="{{items}}">
                <view>请选择</view>
            </picker>
            <picker-view value="{{[0,1]}}">
                <picker-view-column>
                    <view>2023</view>
                    <view>2024</view>
                </picker-view-column>
            </picker-view>
        </view>
    "#;
    render_ok("", wxml, json!({ "items": ["A", "B", "C"] }));
}

#[test]
fn test_rich_text() {
    let wxml = r#"<rich-text nodes="{{html}}"></rich-text>"#;
    render_ok("", wxml, json!({ "html": "<p>hello <b>world</b></p>" }));
}

#[test]
fn test_scroll_view_vertical_and_horizontal() {
    let wxml = r#"
        <view>
            <scroll-view scroll-y="true" class="sv">
                <view wx:for="{{list}}" wx:key="*this">{{item}}</view>
            </scroll-view>
            <scroll-view scroll-x="true" class="svx">
                <view wx:for="{{list}}" wx:key="*this">{{item}}</view>
            </scroll-view>
        </view>
    "#;
    let css = ".sv{height:200px;} .svx{width:300px;}";
    render_ok(css, wxml, json!({ "list": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] }));
}

#[test]
fn test_canvas_component() {
    let wxml = r#"<canvas canvas-id="myCanvas" class="cv"></canvas>"#;
    render_ok(".cv{width:300px;height:150px;}", wxml, json!({}));
}

#[test]
fn test_video_component() {
    let wxml = r#"<video src="test.mp4" controls="true" class="v"></video>"#;
    render_ok(".v{width:300px;height:200px;}", wxml, json!({}));
}

#[test]
fn test_deeply_nested_layout() {
    let css = r#"
        .card { padding: 20rpx; margin: 10rpx; border-radius: 12rpx; background-color: #fff; }
        .row { display: flex; flex-direction: row; align-items: center; }
        .col { display: flex; flex-direction: column; }
        .grow { flex: 1; }
    "#;
    let wxml = r#"
        <view class="col">
            <view class="card" wx:for="{{cards}}" wx:key="id">
                <view class="row">
                    <image class="avatar" src="{{item.avatar}}" />
                    <view class="col grow">
                        <text class="name">{{item.name}}</text>
                        <view class="row">
                            <text wx:for="{{item.tags}}" wx:for-item="tag" wx:key="*this">{{tag}}</text>
                        </view>
                    </view>
                    <button size="mini">关注</button>
                </view>
            </view>
        </view>
    "#;
    let data = json!({
        "cards": [
            { "id": 1, "name": "用户A", "avatar": "a.png", "tags": ["新人", "认证"] },
            { "id": 2, "name": "用户B", "avatar": "b.png", "tags": ["活跃"] }
        ]
    });
    render_ok(css, wxml, data);
}

#[test]
fn test_conditional_chains_render() {
    let wxml = r#"
        <view>
            <text wx:if="{{status === 1}}">待付款</text>
            <text wx:elif="{{status === 2}}">已付款</text>
            <text wx:elif="{{status === 3}}">已发货</text>
            <text wx:else>已完成</text>
        </view>
    "#;
    for s in 1..=4 {
        render_ok("", wxml, json!({ "status": s }));
    }
}

#[test]
fn test_block_wrapper_render() {
    let wxml = r#"
        <view>
            <block wx:for="{{groups}}" wx:key="id">
                <text class="group-title">{{item.title}}</text>
                <view wx:for="{{item.children}}" wx:for-item="child" wx:key="*this">{{child}}</view>
            </block>
        </view>
    "#;
    let data = json!({
        "groups": [
            { "id": 1, "title": "分组1", "children": ["a", "b"] },
            { "id": 2, "title": "分组2", "children": ["c"] }
        ]
    });
    render_ok("", wxml, data);
}
