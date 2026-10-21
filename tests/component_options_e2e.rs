//! 自定义组件的事件冒泡与组件选项（G-2b）：`triggerEvent` 的 `bubbles` / `composed` / `capturePhase`、
//! `options.virtualHost`、`options.dynamicSlots`。
//!
//! 依据官方 custom-component/events.html（「触发事件」的三个选项、`catch` 截断）、
//! glass-easel interaction/slot.md（动态 slot：`<slot>` 可重复、`slot:item` 接收数据）、
//! glass-easel styling/virtual_host.md（虚拟宿主：不出宿主节点，第一层节点直接进使用方的树）。
//! 独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn fixture(name: &str, files: &[(&str, &str)]) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("component-options-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let base: &[(&str, &str)] = &[
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
    ];
    for (rel, text) in base.iter().chain(files.iter()) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    Fixture(root)
}

fn engine(fx: &Fixture) -> MiniEngine {
    let data = fx.0.join("_data");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/index/index")).expect("打开页面");
    for t in 1..4 {
        e.pump(t * 16);
    }
    e
}

fn ev(e: &MiniEngine, code: &str) -> String {
    e.eval(code).unwrap_or_default().trim_matches('"').to_string()
}

const PAGE_JS: &str = r#"Page({ data: { log: [] },
  rec(tag) { var self = this; return function(e) { self.data.log.push(tag + ':' + (e.currentTarget.id || '-') + '/' + (e.target.id || '-')); }; },
  onLoad() { var self = this; ['onHost', 'onWrap', 'onOuter', 'onCap', 'onCatch'].forEach(function(n) { self[n] = self.rec(n); }); } });"#;

/// bubbles：宿主 → 写了绑定的祖先（catch 截断）；不带 bubbles 只有宿主；capturePhase 先由外向内走捕获
#[test]
fn trigger_event_bubbles_through_owner_template() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture(
        "bubbles",
        &[
            ("pages/index/index.json", r#"{"usingComponents":{"my-btn":"/comp/btn"}}"#),
            (
                "pages/index/index.wxml",
                r#"<view id="outer" bind:hit="onOuter" capture-bind:hit="onCap">
                     <view id="wrap" bind:hit="onWrap"><my-btn id="b" bind:hit="onHost"/></view>
                   </view>
                   <view id="stop" catch:hit="onCatch"><view><my-btn id="b2"/></view></view>"#,
            ),
            ("pages/index/index.wxss", ""),
            ("pages/index/index.js", PAGE_JS),
            ("comp/btn.json", r#"{"component":true}"#),
            ("comp/btn.wxml", "<view>btn</view>"),
            ("comp/btn.wxss", ""),
            ("comp/btn.js", "Component({ methods: { fire(o) { this.triggerEvent('hit', {}, o); } } });"),
        ],
    );
    let e = engine(&fx);
    let fire = |sel: &str, opts: &str| {
        e.eval(&format!("getCurrentPages()[0].data.log = []; getCurrentPages()[0].selectComponent('{sel}').fire({opts})")).unwrap();
        ev(&e, "getCurrentPages()[0].data.log.join(' ')")
    };
    assert_eq!(fire("#b", "{}"), "onHost:b/b", "不冒泡：只有宿主上的绑定");
    assert_eq!(fire("#b", "{bubbles:true}"), "onHost:b/b onWrap:wrap/b onOuter:outer/b", "冒泡：由内向外，target 是宿主");
    assert_eq!(
        fire("#b", "{bubbles:true,capturePhase:true}"),
        "onCap:outer/b onHost:b/b onWrap:wrap/b onOuter:outer/b",
        "捕获阶段先由外向内"
    );
    assert_eq!(fire("#b2", "{bubbles:true}"), "onCatch:stop/b2", "catch 截断（它自己照样触发）");
}

/// composed：使用方是组件时，穿过它的边界继续往外（target 重定向成外层宿主）
#[test]
fn trigger_event_composed_crosses_component_boundary() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture(
        "composed",
        &[
            ("pages/index/index.json", r#"{"usingComponents":{"my-panel":"/comp/panel"}}"#),
            ("pages/index/index.wxml", r#"<view id="pg" bind:hit="onOuter"><my-panel id="p" bind:hit="onHost"/></view>"#),
            ("pages/index/index.wxss", ""),
            ("pages/index/index.js", PAGE_JS),
            ("comp/panel.json", r#"{"component":true,"usingComponents":{"my-btn":"/comp/btn"}}"#),
            ("comp/panel.wxml", r#"<view id="inpanel" bind:hit="onIn"><my-btn id="b"/></view>"#),
            ("comp/panel.wxss", ""),
            ("comp/panel.js", "Component({ methods: { onIn(e) { getCurrentPages()[0].data.log.push('onIn:' + e.currentTarget.id + '/' + e.target.id); } } });"),
            ("comp/btn.json", r#"{"component":true}"#),
            ("comp/btn.wxml", "<view>btn</view>"),
            ("comp/btn.wxss", ""),
            ("comp/btn.js", "Component({ methods: { fire(o) { this.triggerEvent('hit', {}, o); } } });"),
        ],
    );
    let e = engine(&fx);
    let fire = |opts: &str| {
        e.eval(&format!(
            "getCurrentPages()[0].data.log = []; getCurrentPages()[0].selectComponent('#p').selectComponent('#b').fire({opts})"
        ))
        .unwrap();
        ev(&e, "getCurrentPages()[0].data.log.join(' ')")
    };
    assert_eq!(fire("{bubbles:true}"), "onIn:inpanel/b", "不 composed：停在 panel 的模板里");
    assert_eq!(
        fire("{bubbles:true,composed:true}"),
        "onIn:inpanel/b onHost:p/p onOuter:pg/p",
        "composed：穿过 panel 的边界，页面看到的 target 是 panel 的宿主"
    );
}

/// virtualHost：宿主节点不进布局（宿主上的 class 不生效），组件的作用域样式照样命中模板节点
#[test]
fn virtual_host_drops_host_node_but_keeps_scoped_styles() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture(
        "vhost",
        &[
            ("pages/index/index.json", r#"{"usingComponents":{"v-cell":"/comp/cell","n-cell":"/comp/ncell"}}"#),
            (
                "pages/index/index.wxml",
                r#"<view style="display:flex;flex-direction:row"><v-cell class="pad"/><v-cell class="pad"/></view>
                   <view style="display:flex;flex-direction:row"><n-cell class="pad"/></view>"#,
            ),
            ("pages/index/index.wxss", ".pad{padding:20px}"),
            ("pages/index/index.js", "Page({});"),
            ("comp/cell.json", r#"{"component":true}"#),
            ("comp/cell.wxml", r#"<view class="c">x</view>"#),
            ("comp/cell.wxss", ".c{width:50px;height:30px;background:#ff0000}"),
            ("comp/cell.js", "Component({ options: { virtualHost: true } });"),
            ("comp/ncell.json", r#"{"component":true}"#),
            ("comp/ncell.wxml", r#"<view class="c">x</view>"#),
            ("comp/ncell.wxss", ".c{width:50px;height:30px;background:#0000ff}"),
            ("comp/ncell.js", "Component({});"),
        ],
    );
    let mut e = engine(&fx);
    let (w, _) = e.pixel_size();
    let px = |e: &mut MiniEngine, x: usize, y: usize| {
        let p = e.rgba();
        let i = (y * 2 * w as usize + x * 2) * 4;
        (p[i], p[i + 1], p[i + 2])
    };
    // 虚拟宿主：第一个红块从 x=0、y=0 开始（宿主的 padding:20px 没生效），第二个紧挨着在 x=50
    assert_eq!(px(&mut e, 5, 5), (255, 0, 0), "作用域样式命中、没有宿主的 padding");
    assert_eq!(px(&mut e, 55, 5), (255, 0, 0), "两个虚拟宿主的内容紧挨着排");
    // 普通组件：宿主的 padding:20px 生效，蓝块从 (20, 30+20) 开始
    assert_ne!(px(&mut e, 5, 35), (0, 0, 255));
    assert_eq!(px(&mut e, 25, 55), (0, 0, 255), "普通宿主节点照常吃使用方的 class");
}

/// 内置表单 behavior：wx://form-field 的组件按 name 交出自己的 value；普通组件内部的控件 form 看不见；
/// wx://form-field-group 的看得见；wx://form-field-button 组件里的 submit 按钮能提交外层 form
#[test]
fn form_field_behaviors() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let comp = |dir: &'static str, wxml: &'static str, js: &'static str| -> Vec<(String, &'static str)> {
        vec![
            (format!("comp/{dir}.json"), r#"{"component":true}"#),
            (format!("comp/{dir}.wxml"), wxml),
            (format!("comp/{dir}.wxss"), ""),
            (format!("comp/{dir}.js"), js),
        ]
    };
    let mut files: Vec<(String, &str)> = vec![
        (
            "pages/index/index.json".into(),
            r#"{"usingComponents":{"my-field":"/comp/field","my-plain":"/comp/plain","my-group":"/comp/group","my-btn":"/comp/btn"}}"#,
        ),
        (
            "pages/index/index.wxml".into(),
            r#"<form bindsubmit="onSubmit"><my-field name="rate" value="{{5}}"/><my-plain/><my-group/><my-btn/></form>"#,
        ),
        ("pages/index/index.wxss".into(), ""),
        ("pages/index/index.js".into(), "Page({ data: { got: '' }, onSubmit(e) { this.data.got = JSON.stringify(e.detail.value); } });"),
    ];
    files.extend(comp("field", r#"<view style="height:40px"></view>"#, "Component({ behaviors: ['wx://form-field'] });"));
    files.extend(comp("plain", r#"<input name="hidden" value="x" style="height:40px"/>"#, "Component({});"));
    files.extend(comp("group", r#"<input name="g" value="gv" style="height:40px"/>"#, "Component({ behaviors: ['wx://form-field-group'] });"));
    files.extend(comp("btn", r#"<button form-type="submit" style="height:40px">提交</button>"#, "Component({ behaviors: ['wx://form-field-button'] });"));
    let refs: Vec<(&str, &str)> = files.iter().map(|(p, t)| (p.as_str(), *t)).collect();
    let fx = fixture("formfield", &refs);
    let mut e = engine(&fx);
    e.pointer_down(100.0, 140.0);
    e.pump(80);
    e.pointer_up(100.0, 140.0);
    for t in 6..10 {
        e.pump(t * 16);
    }
    let got = ev(&e, "getCurrentPages()[0].data.got").replace("\\\"", "\"");
    let v: serde_json::Value = serde_json::from_str(&got).unwrap_or(serde_json::Value::Null);
    assert_eq!(v, serde_json::json!({ "rate": 5, "g": "gv" }), "提交值：{got}");
}

/// dynamicSlots：slot 写在 wx:for 里会重复，内容用 slot:item / slot:listIndex="i" 接收数据
#[test]
fn dynamic_slots_repeat_and_pass_data() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture(
        "dynslot",
        &[
            ("pages/index/index.json", r#"{"usingComponents":{"my-list":"/comp/list","my-static":"/comp/static"}}"#),
            (
                "pages/index/index.wxml",
                r#"<my-list><text class="t" slot:item slot:listIndex="i">{{i}}={{item}}</text></my-list>
                   <my-static><text class="s">once</text></my-static>"#,
            ),
            ("pages/index/index.wxss", ""),
            ("pages/index/index.js", "Page({});"),
            ("comp/list.json", r#"{"component":true}"#),
            ("comp/list.wxml", r#"<block wx:for="{{list}}"><slot list-index="{{index}}" item="{{item}}"/></block>"#),
            ("comp/list.wxss", ""),
            ("comp/list.js", "Component({ options: { dynamicSlots: true }, data: { list: ['A', 'B', 'C'] } });"),
            ("comp/static.json", r#"{"component":true}"#),
            ("comp/static.wxml", r#"<block wx:for="{{[1,2,3]}}"><slot/></block>"#),
            ("comp/static.wxss", ""),
            ("comp/static.js", "Component({});"),
        ],
    );
    let mut e = engine(&fx);
    let q = |e: &MiniEngine, sel: &str| {
        e.eval(&format!(
            "globalThis.__qr = null; wx.createSelectorQuery().selectAll('{sel}').fields({{dataset: true, rect: true}}).exec(function(r) {{ globalThis.__qr = r[0].length; }})"
        ))
        .unwrap();
    };
    q(&e, ".t");
    for t in 4..8 {
        e.pump(t * 16);
    }
    assert_eq!(ev(&e, "String(__qr)"), "3", "动态 slot：内容随 <slot> 重复 3 次");
    q(&e, ".s");
    for t in 8..12 {
        e.pump(t * 16);
    }
    assert_eq!(ev(&e, "String(__qr)"), "1", "静态 slot：重复的 <slot> 只有第一个生效");
}
