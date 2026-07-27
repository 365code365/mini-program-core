//! 页面内自定义组件（`usingComponents`）的展开与渲染。
//!
//! uni-app / Taro 编译产物普遍把底部导航做成页面内组件（`<tab-bar/>`）。
//! 引擎从前不认自定义标签，落到未知标签兜底当空 `view` —— 整条导航（图标 + 文字）
//! 在原生端凭空消失。这里锁住三件事：模板按标签展开、组件用自己的数据作用域、
//! 组件里的事件绑定能登记到宿主。

use crate::parser::template::{ComponentTemplates, TemplateEngine};
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::json;

fn parse(src: &str) -> Vec<crate::parser::wxml::WxmlNode> {
    WxmlParser::new(src).parse().unwrap_or_default()
}

fn templates(pairs: &[(&str, &str)]) -> ComponentTemplates {
    pairs
        .iter()
        .map(|(tag, wxml)| (tag.to_string(), parse(wxml)))
        .collect()
}

/// 组件标签展开成「宿主节点 + 组件模板」，模板里的 `{{}}` 用组件自己的 data 求值。
#[test]
fn component_tag_expands_with_its_own_data() {
    let comps = templates(&[(
        "tab-bar",
        r#"<view class="bar"><text class="label">{{a}}</text></view>"#,
    )]);
    let page = parse(r#"<view class="root"><tab-bar u-p="{{r}}"/></view>"#);
    // 页面自己的 `a` 与组件的 `a` 是两个不同的值，不能串味
    let data = json!({ "a": "页面的a", "$comp": { "tab-bar": { "a": "首页" } } });
    let out = TemplateEngine::render_with_components(&page, &data, &comps);

    assert_eq!(out.len(), 1);
    let host = &out[0].children[0];
    assert_eq!(host.tag_name, "tab-bar", "应保留以组件标签命名的宿主节点");
    let bar = &host.children[0];
    assert_eq!(bar.attributes.get("class").map(|s| s.as_str()), Some("bar"));
    let label_text = &bar.children[0].children[0].text_content;
    assert_eq!(label_text, "首页", "组件模板应使用组件实例的 data");
}

/// 没有组件数据时也不能崩，只是渲染成空内容。
#[test]
fn missing_component_data_renders_empty() {
    let comps = templates(&[("x-bar", r#"<view><text>{{v}}</text></view>"#)]);
    let page = parse(r#"<view><x-bar/></view>"#);
    let out = TemplateEngine::render_with_components(&page, &json!({}), &comps);
    let host = &out[0].children[0];
    assert_eq!(host.tag_name, "x-bar");
    // `{{v}}` 求值为空串后文本节点被丢弃，剩一个空 view
    assert_eq!(host.children.len(), 1);
}

/// 未登记的标签仍按普通节点处理（保持旧行为）
#[test]
fn unknown_tag_is_not_expanded() {
    let page = parse(r#"<view><unknown-thing><text>x</text></unknown-thing></view>"#);
    let out = TemplateEngine::render_with_components(&page, &json!({}), &ComponentTemplates::new());
    assert_eq!(out[0].children[0].tag_name, "unknown-thing");
    assert_eq!(out[0].children[0].children.len(), 1);
}

/// 端到端：组件模板 + 作用域化的组件样式 + 组件内 `bindtap`。
/// 组件里的可点区域必须登记进宿主的事件绑定，否则底部导航点不动。
#[test]
fn component_renders_and_registers_its_events() {
    let comp_wxss = crate::using_components::scope_component_wxss(
        ":host{display:flex}.bar{height:50px;background-color:#fff}.cell{flex:1;height:50px}",
        "tab-bar",
    );
    let css = format!(".root{{height:100px}}\n{comp_wxss}");
    let ss = WxssParser::new(&css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 1.0);
    r.set_component_templates(templates(&[(
        "tab-bar",
        r#"<view class="bar"><view class="cell" bindtap="{{h}}"><text>{{t}}</text></view></view>"#,
    )]));
    let page = parse(r#"<view class="root"><tab-bar/></view>"#);
    let data = json!({ "$comp": { "tab-bar": { "h": "e0_0", "t": "首页" } } });

    let mut canvas = Canvas::new(375, 667);
    r.render(&mut canvas, &page, &data);

    let bindings = r.get_event_bindings();
    let cell = bindings
        .iter()
        .find(|b| b.handler == "e0_0")
        .expect("组件内的 bindtap 应登记为事件绑定");
    assert!(cell.bounds.height > 40.0, "组件样式应生效（cell 高 50px），实际 {:?}", cell.bounds);
}

/// 作用域前缀让组件样式不会漏出去命中页面里的同名类。
#[test]
fn component_styles_do_not_leak_to_page() {
    let comp_wxss = crate::using_components::scope_component_wxss(".label{height:40px}", "tab-bar");
    let ss = WxssParser::new(&comp_wxss).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 667.0, 1.0);
    r.set_component_templates(templates(&[("tab-bar", r#"<view class="label"></view>"#)]));
    // 页面自己也有 .label，但不该被组件样式撑到 40px
    let page = parse(r#"<view><view class="label" bindtap="pageLabel"></view><tab-bar/></view>"#);
    let mut canvas = Canvas::new(375, 667);
    r.render(&mut canvas, &page, &json!({ "$comp": { "tab-bar": {} } }));
    let b = r
        .get_event_bindings()
        .iter()
        .find(|b| b.handler == "pageLabel")
        .cloned()
        .expect("页面节点应有绑定");
    assert!(b.bounds.height < 5.0, "页面的 .label 不该被组件样式命中，实际高 {}", b.bounds.height);
}

/// `class="{{['a', b]}}"`（uni-app 每个根节点的固定写法）要拼成 `a b`，
/// 不能是 JSON 字面量 —— 否则根节点上的类名一个都命中不了。
#[test]
fn class_bound_to_array_joins_with_space() {
    let page = parse(r#"<view class="{{['tabbar', d]}}"><view class="{{['root', e]}}"/></view>"#);
    let out = TemplateEngine::render_with_components(
        &page,
        &json!({ "d": "vh-1", "e": "" }),
        &ComponentTemplates::new(),
    );
    assert_eq!(out[0].attributes.get("class").map(|s| s.as_str()), Some("tabbar vh-1"));
    // 空串要被丢掉，不能留下多余空格之外的东西
    assert_eq!(out[0].children[0].attributes.get("class").map(|s| s.as_str()), Some("root"));
}

/// `style` 绑定数组按 `;` 拼接；对象按 `k:v` 拼接。
#[test]
fn style_bound_to_array_or_object() {
    let page = parse(r#"<view style="{{['color:red', s]}}"/>"#);
    let out = TemplateEngine::render_with_components(
        &page,
        &json!({ "s": "height:10px" }),
        &ComponentTemplates::new(),
    );
    assert_eq!(
        out[0].attributes.get("style").map(|s| s.as_str()),
        Some("color:red;height:10px")
    );
}

/// 普通字符串绑定与混合插值保持原样行为（不能被新逻辑改掉）
#[test]
fn plain_class_binding_unchanged() {
    let page = parse(r#"<view class="a {{b}}"/><view class="{{c}}"/>"#);
    let out = TemplateEngine::render_with_components(
        &page,
        &json!({ "b": "on", "c": "solo" }),
        &ComponentTemplates::new(),
    );
    assert_eq!(out[0].attributes.get("class").map(|s| s.as_str()), Some("a on"));
    assert_eq!(out[1].attributes.get("class").map(|s| s.as_str()), Some("solo"));
}

// ============ 逻辑层：组件实例的数据与事件 ============

/// 挂载页面内组件后：`__getRenderData()` 要带上组件自己的 data（放在 `$comp`），
/// 事件也要能派发到组件实例上 —— 底部导航的点击就靠这两条。
#[test]
fn mounted_component_exposes_data_and_receives_events() {
    let mut app = crate::runtime::MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init MiniApp");
    // 页面（处理函数只在组件上，页面上没有）
    app.load_script(r#"Page({ data: { fromPage: 1 } });"#).unwrap();
    // 组件定义按「组件路径」注册
    app.eval("__setPendingComponentPath('components/tab-bar/tab-bar')").unwrap();
    app.load_script(
        r#"Component({
            data: { hits: 0, label: '首页' },
            methods: { e0_0: function() { this.setData({ hits: this.data.hits + 1 }); } }
        });"#,
    )
    .unwrap();
    app.eval("__setPendingComponentPath('')").unwrap();

    let id = app
        .eval("__mountPageComponent('tab-bar', 'components/tab-bar/tab-bar', '{}')")
        .unwrap();
    assert!(!id.trim().is_empty() && id != "undefined", "应建出组件实例，实际 {id:?}");

    // 渲染数据里既有页面字段也有组件字段
    let render_data = app.eval("__getRenderData()").unwrap();
    assert!(render_data.contains("fromPage"), "{render_data}");
    assert!(render_data.contains("$comp"), "{render_data}");
    assert!(render_data.contains("首页"), "{render_data}");

    // 事件派发：页面上没有 e0_0，必须落到组件实例
    let dispatched = app.eval("__callPageMethod('e0_0', {})").unwrap();
    assert_eq!(dispatched, "true", "组件方法应被派发到");
    let hits = app
        .eval("__componentInstances[__pageComponents['tab-bar']].data.hits")
        .unwrap();
    assert_eq!(hits, "1", "组件的 setData 应生效");

    // 换页会清掉上一页的组件实例
    app.eval("__resetPageComponents()").unwrap();
    assert!(!app.eval("__getRenderData()").unwrap().contains("$comp"));
}
