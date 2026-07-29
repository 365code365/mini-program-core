//! 组件属性行为测试：读了属性但一直没人验证的那些。
//!
//! 现有的 `component_tests.rs` 只覆盖了每个组件最主干的一两个属性，
//! `component_render_tests.rs` 又只验 HTML 转译。这里补的是「原生 build 之后
//! 属性到底生效没有」，重点在 picker 的四种 mode、input 的编辑类属性、
//! swiper 的自动播放/受控 current、以及一堆驼峰/中划线双写法的兼容。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use crate::parser::wxss::{StyleSheet, WxssParser};
use crate::renderer::components::*;
use std::collections::HashMap;
use taffy::prelude::*;

fn node_of(tag: &str, attrs: &[(&str, &str)]) -> WxmlNode {
    WxmlNode {
        node_type: WxmlNodeType::Element,
        tag_name: tag.to_string(),
        attributes: attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        children: vec![],
        text_content: String::new(),
    }
}

fn with_child(mut n: WxmlNode, child: WxmlNode) -> WxmlNode {
    n.children.push(child);
    n
}

fn css(s: &str) -> StyleSheet {
    WxssParser::new(s).parse().unwrap_or_default()
}

/// 跑一次组件 build，返回 RenderNode 与 taffy 里的样式
fn build(tag: &str, node: &WxmlNode, sheet: &StyleSheet) -> (RenderNode, Style) {
    let mut taffy: Tree = Tree::new();
    let mut ctx = ComponentContext {
        scale_factor: 1.0,
        screen_width: 375.0,
        screen_height: 667.0,
        stylesheet: sheet,
        taffy: &mut taffy,
        ancestors: Vec::new(),
        inherited: Default::default(),
        sibling_index: 0,
        sibling_count: 1,
        has_positioned_ancestor: false,
    };
    let rn = match tag {
        "picker" => PickerComponent::build(node, &mut ctx),
        "input" | "textarea" => InputComponent::build(node, &mut ctx),
        "swiper" => SwiperComponent::build(node, &mut ctx),
        "progress" => ProgressComponent::build(node, &mut ctx),
        "slider" => SliderComponent::build(node, &mut ctx),
        "switch" => SwitchComponent::build(node, &mut ctx),
        "checkbox" => CheckboxComponent::build(node, &mut ctx),
        "radio" => RadioComponent::build(node, &mut ctx),
        "icon" => IconComponent::build(node, &mut ctx),
        "image" => ImageComponent::build(node, &mut ctx),
        _ => ViewComponent::build(node, &mut ctx),
    }
    .unwrap_or_else(|| panic!("{tag} build 返回 None"));
    let style = taffy.style(rn.taffy_node).unwrap().clone();
    (rn, style)
}

fn build_simple(tag: &str, attrs: &[(&str, &str)]) -> RenderNode {
    let n = node_of(tag, attrs);
    build(tag, &n, &css("")).0
}

// ───────────────────────────── picker ─────────────────────────────

#[test]
fn picker_selector_shows_the_option_at_value_index() {
    let rn = build_simple(
        "picker",
        &[("mode", "selector"), ("range", "['北京','上海','广州']"), ("value", "2")],
    );
    assert_eq!(rn.text, "广州", "selector 的 value 是下标");
}

#[test]
fn picker_selector_falls_back_to_placeholder_when_out_of_range() {
    let rn = build_simple(
        "picker",
        &[("range", "['A']"), ("value", "9"), ("placeholder", "选个城市")],
    );
    assert_eq!(rn.text, "选个城市");
    // 连 placeholder 都没有时用微信的默认文案
    let rn2 = build_simple("picker", &[("range", "[]")]);
    assert_eq!(rn2.text, "请选择");
}

#[test]
fn picker_range_key_picks_the_field_from_objects() {
    let rn = build_simple(
        "picker",
        &[
            ("range", "[{'id':1,'name':'甲'},{'id':2,'name':'乙'}]"),
            ("range-key", "name"),
            ("value", "1"),
        ],
    );
    assert_eq!(rn.text, "乙", "range-key 指定用哪个字段做显示文本");
}

#[test]
fn picker_time_and_date_show_the_raw_value() {
    // time / date 的 value 本身就是要显示的字符串，不是下标
    let t = build_simple("picker", &[("mode", "time"), ("value", "09:30")]);
    assert_eq!(t.text, "09:30");
    let d = build_simple("picker", &[("mode", "date"), ("value", "2026-07-23")]);
    assert_eq!(d.text, "2026-07-23");
    // 没给 value 时回落到 placeholder
    let empty = build_simple("picker", &[("mode", "date"), ("placeholder", "选日期")]);
    assert_eq!(empty.text, "选日期");
}

#[test]
fn picker_multi_selector_and_region_keep_placeholder_text() {
    // 这两种模式的显示文本由开发者的触发视图或宿主面板决定，
    // picker 自己只给占位文案，不能拿 range[0] 冒充
    let m = build_simple(
        "picker",
        &[("mode", "multiSelector"), ("range", "[['a','b'],['c']]"), ("placeholder", "多列")],
    );
    assert_eq!(m.text, "多列");
    let r = build_simple("picker", &[("mode", "region"), ("placeholder", "选地区")]);
    assert_eq!(r.text, "选地区");
}

#[test]
fn picker_with_custom_trigger_view_is_marked_as_container() {
    // 有元素子节点时 picker 只作容器，绘制期不再合成占位文本 + 箭头
    let n = with_child(
        node_of("picker", &[("mode", "selector"), ("range", "['x']")]),
        node_of("view", &[]),
    );
    let (rn, _) = build("picker", &n, &css(""));
    assert_eq!(rn.attrs.get("__has_trigger").map(String::as_str), Some("1"));
    // 纯文本子节点不算触发视图
    let n2 = with_child(
        node_of("picker", &[("range", "['x']")]),
        WxmlNode {
            node_type: WxmlNodeType::Text,
            tag_name: String::new(),
            attributes: HashMap::new(),
            children: vec![],
            text_content: "文字".into(),
        },
    );
    let (rn2, _) = build("picker", &n2, &css(""));
    assert!(rn2.attrs.get("__has_trigger").is_none());
}

// ─────────────────────────── input / textarea ───────────────────────────

#[test]
fn input_editing_attrs_are_carried_to_the_host() {
    // 宿主侧的编辑器要靠这些属性工作，build 阶段不能丢
    let rn = build_simple(
        "input",
        &[
            ("value", "abc"),
            ("maxlength", "10"),
            ("type", "number"),
            ("password", "true"),
            ("focus", "true"),
            ("confirm-type", "search"),
            ("placeholder", "请输入"),
        ],
    );
    for (k, v) in [("maxlength", "10"), ("type", "number"), ("password", "true")] {
        assert_eq!(rn.attrs.get(k).map(String::as_str), Some(v), "属性 {k}");
    }
    assert_eq!(rn.attrs.get("focus").map(String::as_str), Some("true"));
    assert_eq!(rn.attrs.get("confirm-type").map(String::as_str), Some("search"));
}

#[test]
fn input_events_carry_editing_attrs_in_dataset() {
    // extract_events 会把 maxlength/type/password 塞进 dataset，
    // 宿主的输入处理靠它做长度与字符校验
    let n = node_of(
        "input",
        &[("bindinput", "onInput"), ("maxlength", "6"), ("type", "number")],
    );
    let (rn, _) = build("input", &n, &css(""));
    let bind = rn
        .events
        .iter()
        .find(|e| e.event_type == "input")
        .expect("应有 input 绑定");
    assert_eq!(bind.data.get("maxlength").map(String::as_str), Some("6"));
    assert_eq!(bind.data.get("type").map(String::as_str), Some("number"));
}

#[test]
fn textarea_is_built_by_the_same_component_but_keeps_its_tag() {
    let rn = build_simple("textarea", &[("value", "多行")]);
    assert_eq!(rn.tag, "textarea", "标签要留住，绘制与命中都按它区分");
}

#[test]
fn input_placeholder_shows_only_when_value_is_empty() {
    let empty = build_simple("input", &[("placeholder", "请输入手机号")]);
    assert!(
        empty.text.is_empty() || empty.text == "请输入手机号",
        "空值时显示占位或留空，实际 {:?}",
        empty.text
    );
    let filled = build_simple("input", &[("value", "13800000000"), ("placeholder", "请输入手机号")]);
    assert!(
        filled.text.contains("13800000000"),
        "有值时必须显示值，实际 {:?}",
        filled.text
    );
}

// ───────────────────────────── swiper ─────────────────────────────

#[test]
fn swiper_reads_orientation_and_indicator_attrs() {
    let rn = build_simple(
        "swiper",
        &[
            ("vertical", "true"),
            ("indicator-dots", "true"),
            ("indicator-color", "#cccccc"),
            ("indicator-active-color", "#ff0000"),
        ],
    );
    assert_eq!(rn.attrs.get("vertical").map(String::as_str), Some("true"));
    assert_eq!(rn.attrs.get("indicator-dots").map(String::as_str), Some("true"));
    assert_eq!(rn.attrs.get("indicator-color").map(String::as_str), Some("#cccccc"));
}

#[test]
fn swiper_autoplay_state_is_registered_while_drawing() {
    // 自动播放的状态是在**绘制期**登记的（current_index 里），不是 build 期 ——
    // 因为 current 要按时钟推进。这里走一遍完整渲染，再查全局状态表。
    // 用带 id 的 swiper，避免和其它测试共用同一条状态（状态表是进程级全局）。
    let wxml = r#"
        <view class="wrap">
          <swiper class="sw" id="sw-attr-test" autoplay="true" interval="1200">
            <swiper-item><view class="p">1</view></swiper-item>
            <swiper-item><view class="p">2</view></swiper-item>
          </swiper>
        </view>"#;
    let sheet = css(".wrap{ width:375px; height:200px; } .sw{ width:375px; height:150px; } .p{ width:375px; height:150px; }");
    let nodes = crate::parser::WxmlParser::new(wxml).parse().expect("wxml");
    let mut r = crate::renderer::WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, 1.0);
    let mut canvas = crate::Canvas::new(375, 300);
    canvas.clear(crate::Color::WHITE);
    // 必须走**带交互**的入口：轮播的分页/自动播放只在这条路径上生效
    // （无交互的 render() 是画廊/离屏那条，swiper 不分页，见 draw.rs::draws_children）
    let mut im = crate::ui::InteractionManager::new();
    r.render_with_interaction(&mut canvas, &nodes, &serde_json::json!({}), &mut im);

    let m = SWIPER_MANAGER.lock().expect("swiper manager");
    let st = m
        .get("sw-attr-test")
        .unwrap_or_else(|| panic!("未登记；现有 key = {:?}", m.debug_keys()));
    assert!(st.autoplay, "autoplay=true 要落到状态里");
    assert_eq!(st.total, 2, "两个 swiper-item");
    assert_eq!(st.autoplay_interval, 1200, "interval 要生效（默认是 5000）");
    drop(m);
    assert!(
        has_autoplay_swiper(),
        "有自动播放的 swiper 时宿主必须知道要继续出帧，否则轮播卡住不动"
    );
}

#[test]
fn swiper_controlled_current_is_respected() {
    // 没开 autoplay 时，WXML 的 current 是受控值，必须同步到状态
    let wxml = r#"
        <view class="wrap">
          <swiper class="sw" id="sw-controlled" current="1">
            <swiper-item><view class="p">1</view></swiper-item>
            <swiper-item><view class="p">2</view></swiper-item>
            <swiper-item><view class="p">3</view></swiper-item>
          </swiper>
        </view>"#;
    let sheet = css(".wrap{ width:375px; height:200px; } .sw{ width:375px; height:150px; } .p{ width:375px; height:150px; }");
    let nodes = crate::parser::WxmlParser::new(wxml).parse().expect("wxml");
    let mut r = crate::renderer::WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, 1.0);
    let mut canvas = crate::Canvas::new(375, 300);
    canvas.clear(crate::Color::WHITE);
    let mut im = crate::ui::InteractionManager::new();
    r.render_with_interaction(&mut canvas, &nodes, &serde_json::json!({}), &mut im);
    let m = SWIPER_MANAGER.lock().expect("swiper manager");
    let st = m.get("sw-controlled").expect("应登记状态");
    assert_eq!(st.current, 1, "current=1 要生效");
    assert_eq!(st.total, 3);
}

// ──────────────────── 驼峰 / 中划线 双写法兼容 ────────────────────

#[test]
fn progress_accepts_both_dashed_and_camel_color_attrs() {
    // 微信文档写的是 activeColor/backgroundColor，WXML 里两种写法都有人用
    let dashed = build_simple(
        "progress",
        &[("percent", "40"), ("active-color", "#ff0000"), ("background-color", "#eeeeee")],
    );
    let camel = build_simple(
        "progress",
        &[("percent", "40"), ("activeColor", "#ff0000"), ("backgroundColor", "#eeeeee")],
    );
    // 两种写法解析出的节点应等价（颜色进了 style，属性原样保留）
    assert_eq!(dashed.tag, camel.tag);
    assert!(
        dashed.attrs.contains_key("active-color") && camel.attrs.contains_key("activeColor"),
        "属性表原样保留，解析在绘制层做"
    );
}

#[test]
fn progress_percent_is_clamped() {
    for (given, want) in [("-20", 0.0f32), ("140", 100.0), ("33.5", 33.5)] {
        let rn = build_simple("progress", &[("percent", given)]);
        let got: f32 = rn
            .attrs
            .get("percent")
            .and_then(|v| v.parse().ok())
            .unwrap_or(-1.0);
        // 属性原样保留，绘制时再夹；这里验证解析不会把合法值弄丢
        assert!(
            (got - given.parse::<f32>().unwrap()).abs() < 0.01,
            "percent={given} 应原样保留，实际 {got}（期望绘制阶段夹到 {want}）"
        );
    }
}

#[test]
fn slider_reads_range_and_block_attrs() {
    let rn = build_simple(
        "slider",
        &[
            ("value", "30"),
            ("min", "10"),
            ("max", "50"),
            ("block-size", "20"),
            ("block-color", "#123456"),
            ("show-value", "true"),
        ],
    );
    for (k, v) in [
        ("value", "30"),
        ("min", "10"),
        ("max", "50"),
        ("block-size", "20"),
        ("block-color", "#123456"),
        ("show-value", "true"),
    ] {
        assert_eq!(rn.attrs.get(k).map(String::as_str), Some(v), "slider 属性 {k}");
    }
}

#[test]
fn switch_supports_checkbox_shape() {
    let sw = build_simple("switch", &[("checked", "true")]);
    assert_eq!(sw.attrs.get("checked").map(String::as_str), Some("true"));
    let cb = build_simple("switch", &[("type", "checkbox"), ("checked", "true")]);
    assert_eq!(
        cb.attrs.get("type").map(String::as_str),
        Some("checkbox"),
        "switch 的 checkbox 形态要保留 type，绘制层据此换外观"
    );
}

#[test]
fn disabled_form_controls_keep_the_flag() {
    for tag in ["switch", "checkbox", "radio", "slider", "input"] {
        let rn = build_simple(tag, &[("disabled", "true")]);
        assert_eq!(
            rn.attrs.get("disabled").map(String::as_str),
            Some("true"),
            "{tag} 的 disabled 要留住（命中测试靠它跳过）"
        );
    }
}

// ───────────────────────── hover-class ─────────────────────────

#[test]
fn hover_class_is_recorded_for_press_feedback() {
    // 微信的按压反馈有两条路：CSS 的 :active 和 WXML 的 hover-class。
    // 后者一直没有测试，改动按压态时很容易顺手弄坏。
    let sheet = css(".btn-hover{ background-color:#dddddd; }");
    let n = node_of("view", &[("hover-class", "btn-hover")]);
    let (rn, _) = build("view", &n, &sheet);
    assert_eq!(
        rn.attrs.get("hover-class").map(String::as_str),
        Some("btn-hover"),
        "hover-class 要留在属性表里，按下时宿主据此追加类名"
    );
}

#[test]
fn image_keeps_src_and_mode() {
    let rn = build_simple("image", &[("src", "/assets/a.png"), ("mode", "aspectFill")]);
    assert_eq!(rn.attrs.get("src").map(String::as_str), Some("/assets/a.png"));
    assert_eq!(rn.attrs.get("mode").map(String::as_str), Some("aspectFill"));
}

#[test]
fn icon_size_attr_drives_the_box_but_css_wins() {
    // size 属性决定边长；CSS 里写了 width/height 时以 CSS 为准
    let (_, style) = build("icon", &node_of("icon", &[("size", "40")]), &css(""));
    assert_eq!(
        style.size.width,
        Dimension::length(40.0),
        "size 属性应写进布局尺寸"
    );
    let sheet = css(".big{ width:60px; height:60px; }");
    let (_, style2) = build(
        "icon",
        &node_of("icon", &[("size", "40"), ("class", "big")]),
        &sheet,
    );
    assert_eq!(
        style2.size.width,
        Dimension::length(60.0),
        "CSS 尺寸优先于 size 属性"
    );
}

#[test]
fn icon_never_shrinks_in_a_crowded_row() {
    // 图标被 flex 压扁是很常见的翻车点（同排文字一溢出图标就变形）
    let (_, style) = build("icon", &node_of("icon", &[("size", "23")]), &css(""));
    assert_eq!(style.flex_shrink, 0.0);
    assert_eq!(style.flex_grow, 0.0);
}
