//! 渲染器单元测试
//! 测试 WXML 渲染器的各种功能

use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::ui::interaction::InteractionManager;
use crate::Canvas;
use serde_json::json;

/// 辅助函数：解析 CSS
fn parse_css(css: &str) -> crate::parser::wxss::StyleSheet {
    WxssParser::new(css).parse().unwrap_or_default()
}

/// 辅助函数：解析 WXML
fn parse_wxml(wxml: &str) -> Vec<crate::parser::wxml::WxmlNode> {
    let mut parser = WxmlParser::new(wxml);
    parser.parse().unwrap_or_default()
}

/// 创建测试用的渲染器
fn create_test_renderer(css: &str) -> WxmlRenderer {
    let stylesheet = parse_css(css);
    WxmlRenderer::new_with_scale(stylesheet, 375.0, 667.0, 2.0)
}

/// 创建测试用的 Canvas
fn create_test_canvas() -> Canvas {
    Canvas::new(750, 1334)
}

/// 测试基本渲染
#[test]
fn test_basic_render() {
    let css = r#"
        .container {
            width: 100%;
            padding: 20rpx;
            background-color: #FFFFFF;
        }
    "#;
    
    let wxml = r#"
        <view class="container">
            <text>Hello World</text>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    renderer.render(&mut canvas, &nodes, &data);
    
    // 模板里没有任何 bind*，就不该登记事件绑定
    // （原来写的是 `event_count() >= 0`，usize 恒真，等于没测）
    assert_eq!(renderer.event_count(), 0, "没有 bind* 的模板不该登记事件绑定");
}

/// 测试数据绑定渲染
#[test]
fn test_data_binding_render() {
    let css = r#"
        .price {
            color: #FF6B35;
            font-size: 32rpx;
        }
    "#;
    
    let wxml = r#"
        <view>
            <text class="price">¥{{price}}</text>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({
        "price": 99.99
    });
    
    renderer.render(&mut canvas, &nodes, &data);
}

/// 测试条件渲染 wx:if
#[test]
fn test_conditional_render() {
    let css = "";
    
    let wxml = r#"
        <view>
            <text wx:if="{{showText}}">Visible</text>
            <text wx:else>Hidden</text>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    
    // showText = true
    let data1 = json!({ "showText": true });
    renderer.render(&mut canvas, &nodes, &data1);
    
    // showText = false
    let data2 = json!({ "showText": false });
    renderer.render(&mut canvas, &nodes, &data2);
}

/// 测试列表渲染 wx:for
#[test]
fn test_list_render() {
    let css = r#"
        .item {
            padding: 20rpx;
            border-bottom: 1rpx solid #eee;
        }
    "#;
    
    let wxml = r#"
        <view>
            <view class="item" wx:for="{{items}}" wx:key="id">
                <text>{{item.name}}</text>
            </view>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({
        "items": [
            { "id": 1, "name": "Item 1" },
            { "id": 2, "name": "Item 2" },
            { "id": 3, "name": "Item 3" }
        ]
    });
    
    renderer.render(&mut canvas, &nodes, &data);
}

/// 测试事件绑定
#[test]
fn test_event_binding() {
    let css = "";
    
    let wxml = r#"
        <view bindtap="onTap" data-id="123">
            <text>Click me</text>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    renderer.render(&mut canvas, &nodes, &data);
    
    let bindings = renderer.get_event_bindings();
    assert!(!bindings.is_empty());
    
    let binding = &bindings[0];
    assert_eq!(binding.event_type, "tap");
    assert_eq!(binding.handler, "onTap");
    assert_eq!(binding.data.get("id"), Some(&"123".to_string()));
}

/// 测试交互式渲染
#[test]
fn test_interactive_render() {
    let css = r#"
        .form {
            padding: 20rpx;
        }
    "#;
    
    let wxml = r#"
        <view class="form">
            <input placeholder="Enter text" />
            <checkbox checked="true" />
            <switch />
            <slider value="50" />
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let mut interaction = InteractionManager::new();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    renderer.render_with_interaction(&mut canvas, &nodes, &data, &mut interaction);
}

/// 测试滚动渲染
#[test]
fn test_scroll_render() {
    let css = r#"
        .page {
            padding-bottom: 100rpx;
        }
        .action-bar {
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
            height: 100rpx;
            background-color: #fff;
        }
    "#;
    
    let wxml = r#"
        <view class="page">
            <text>Content</text>
            <view class="action-bar">
                <button>Submit</button>
            </view>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let mut interaction = InteractionManager::new();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    // 测试不同滚动偏移
    renderer.render_with_scroll(&mut canvas, &nodes, &data, &mut interaction, 0.0);
    renderer.render_with_scroll(&mut canvas, &nodes, &data, &mut interaction, 100.0);
    renderer.render_with_scroll(&mut canvas, &nodes, &data, &mut interaction, 200.0);
}

/// 测试复杂布局渲染
#[test]
fn test_complex_layout_render() {
    let css = r#"
        .header {
            display: flex;
            flex-direction: row;
            justify-content: space-between;
            align-items: center;
            padding: 20rpx;
            background-color: #fff;
        }
        .title {
            font-size: 36rpx;
            font-weight: bold;
        }
        .grid {
            display: flex;
            flex-direction: row;
            flex-wrap: wrap;
        }
        .grid-item {
            width: 50%;
            padding: 10rpx;
        }
    "#;
    
    let wxml = r#"
        <view>
            <view class="header">
                <text class="title">Title</text>
                <icon type="search" size="24" />
            </view>
            <view class="grid">
                <view class="grid-item" wx:for="{{items}}" wx:key="id">
                    <text>{{item.name}}</text>
                </view>
            </view>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({
        "items": [
            { "id": 1, "name": "Item 1" },
            { "id": 2, "name": "Item 2" },
            { "id": 3, "name": "Item 3" },
            { "id": 4, "name": "Item 4" }
        ]
    });
    
    renderer.render(&mut canvas, &nodes, &data);
}

/// 测试嵌套组件渲染
#[test]
fn test_nested_components_render() {
    let css = r#"
        .card {
            background-color: #fff;
            border-radius: 16rpx;
            padding: 20rpx;
            margin: 20rpx;
        }
        .card-header {
            display: flex;
            flex-direction: row;
            align-items: center;
        }
        .avatar {
            width: 80rpx;
            height: 80rpx;
            border-radius: 40rpx;
        }
        .info {
            margin-left: 20rpx;
        }
    "#;
    
    let wxml = r#"
        <view class="card">
            <view class="card-header">
                <image class="avatar" src="avatar.png" />
                <view class="info">
                    <text>Username</text>
                    <text>Description</text>
                </view>
            </view>
            <view class="card-body">
                <text>Card content goes here</text>
            </view>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    renderer.render(&mut canvas, &nodes, &data);
}

/// 测试表单组件渲染
#[test]
fn test_form_components_render() {
    let css = r#"
        .form-item {
            display: flex;
            flex-direction: row;
            align-items: center;
            padding: 20rpx;
            border-bottom: 1rpx solid #eee;
        }
        .label {
            width: 160rpx;
        }
    "#;
    
    let wxml = r#"
        <view>
            <view class="form-item">
                <text class="label">Name</text>
                <input placeholder="Enter name" />
            </view>
            <view class="form-item">
                <text class="label">Gender</text>
                <radio value="male" />
                <radio value="female" />
            </view>
            <view class="form-item">
                <text class="label">Agree</text>
                <checkbox />
            </view>
            <view class="form-item">
                <text class="label">Notify</text>
                <switch />
            </view>
            <view class="form-item">
                <text class="label">Volume</text>
                <slider value="50" show-value="true" />
            </view>
            <view class="form-item">
                <progress percent="75" show-info="true" />
            </view>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    renderer.render(&mut canvas, &nodes, &data);
}

/// 测试空节点处理
#[test]
fn test_empty_nodes() {
    let css = "";
    let wxml = "";
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    renderer.render(&mut canvas, &nodes, &data);
    assert_eq!(renderer.event_count(), 0);
}

/// 测试特殊字符处理
#[test]
fn test_special_characters() {
    let css = "";
    
    let wxml = r#"
        <view>
            <text>Price: ¥99.99</text>
            <text>中文测试</text>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    renderer.render(&mut canvas, &nodes, &data);
}

/// 测试多个事件绑定
#[test]
fn test_multiple_event_bindings() {
    let css = "";
    
    let wxml = r#"
        <view>
            <view bindtap="onTap1" data-id="1">Item 1</view>
            <view bindtap="onTap2" data-id="2">Item 2</view>
            <view bindtap="onTap3" data-id="3">Item 3</view>
        </view>
    "#;
    
    let mut renderer = create_test_renderer(css);
    let mut canvas = create_test_canvas();
    let nodes = parse_wxml(wxml);
    let data = json!({});
    
    renderer.render(&mut canvas, &nodes, &data);
    
    let bindings = renderer.get_event_bindings();
    assert_eq!(bindings.len(), 3);
}
