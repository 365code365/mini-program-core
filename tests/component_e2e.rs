//! 自定义组件「每处使用一个实例」与 `<slot>` 的端到端用例：现场写一个最小小程序，
//! 用 `MiniEngine`（SDK 那条链路：真指针按下/抬起 → 命中 → 派发）跑起来断言。
//!
//! ## 为什么是集成测试（独立进程），不放进 `src/tests`
//! 小程序根目录是**进程级**的（`page_loader::set_app_path`）。`src/tests` 里的引擎用例全部用
//! `sample/sample-app`，并行跑时根目录始终是同一个值；这里要用自己的夹具目录，放进同一进程
//! 就会和它们互相改掉根目录 —— 实测全量 `cargo test` 时本用例必挂（读到的是 sample-app 的首页），
//! 反过来也会让 sample-app 的用例偶发读到夹具。独立二进制 + 一把锁串行，互不干扰。
//!
//! 模板层（实例键、slot 分配）与逻辑层（属性类型转换、observer）的细粒度用例在
//! `src/tests/component_instance_tests.rs` / `component_runtime_tests.rs`。
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

/// 临时小程序目录（drop 时删掉 —— 盘紧，不留垃圾）
struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// 写一个只有首页（`pages/index/index`）的小程序；`files` 是 (相对路径, 内容)
fn fixture(name: &str, files: &[(&str, &str)]) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("component-e2e").join(name);
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

/// 组件三件套 + json（`component: true`）
fn component<'a>(dir: &str, wxml: &'a str, wxss: &'a str, js: &'a str) -> Vec<(String, &'a str)> {
    vec![
        (format!("{dir}.json"), r#"{"component":true}"#),
        (format!("{dir}.wxml"), wxml),
        (format!("{dir}.wxss"), wxss),
        (format!("{dir}.js"), js),
    ]
}

fn engine(fx: &Fixture) -> MiniEngine {
    let data = fx.0.join("_data");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0)
        .expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/index/index")).expect("打开页面");
    e.pump(1);
    e
}

fn tap(e: &mut MiniEngine, x: f32, y: f32) {
    e.pointer_down(x, y);
    e.pump(0);
    e.pointer_up(x, y);
    e.pump(0);
}

fn ev(e: &MiniEngine, code: &str) -> String {
    e.eval(code).unwrap_or_default().trim_matches('"').to_string()
}

fn write_all(name: &str, page: &[(&str, &str)], comps: Vec<Vec<(String, &str)>>) -> Fixture {
    let owned: Vec<(String, &str)> = comps.into_iter().flatten().collect();
    let mut files: Vec<(&str, &str)> = page.to_vec();
    files.extend(owned.iter().map(|(p, t)| (p.as_str(), *t)));
    fixture(name, &files)
}

// ============ 列表里的组件、slot 点击、属性更新、detached ============

const CARD_JS: &str = r#"Component({
    properties: { n: { type: Number, observer: function (v, o) { __obs.push(this.data.n + '<-' + o); } } },
    data: { hits: 0 },
    lifetimes: { detached: function () { __detached.push(this.data.n); } },
    methods: {
        onTap: function () {
            this.setData({ hits: this.data.hits + 1 });
            this.triggerEvent('pick', { n: this.data.n });
        },
        onSlotTap: function () { __wrong.push('card'); }
    }
});"#;

const BOX_JS: &str = r#"Component({
    lifetimes: { detached: function () { __detached.push('box'); } },
    methods: { onSlotTap: function () { __wrong.push('box'); } }
});"#;

// 页面/组件 js 都按模块作用域执行，`var` 出不了模块 —— 断言用的记录挂在全局对象上
const PAGE_JS: &str = r#"globalThis.__obs = []; globalThis.__detached = []; globalThis.__wrong = [];
Page({
    data: { items: [10, 20], showBox: true, other: 0 },
    onPick: function (e) { this.picked = (this.picked || []).concat([e.detail.n]); this.pickType = typeof e.detail.n; },
    onSlotTap: function (e) { this.slotTaps = (this.slotTaps || 0) + 1; this.slotIdx = e.currentTarget.dataset.i; }
});"#;

#[test]
fn 每处一个实例_点哪个是哪个_slot点击归页面_属性更新与detached() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = write_all(
        "list",
        &[
            ("pages/index/index.json", r#"{"usingComponents":{"my-card":"/components/card/card","my-box":"/components/box/box"}}"#),
            (
                "pages/index/index.wxml",
                r#"<view class="list"><my-card wx:for="{{items}}" class="card" n="{{item}}" bind:pick="onPick"/><my-box wx:if="{{showBox}}"><view class="slot-btn" wx:for="{{[7]}}" data-i="{{item}}" bindtap="onSlotTap">slot</view></my-box></view>"#,
            ),
            ("pages/index/index.wxss", ".list{display:flex;flex-direction:column}.slot-btn{height:80px}"),
            ("pages/index/index.js", PAGE_JS),
        ],
        vec![
            component("components/card/card", r#"<view class="c" bindtap="onTap">{{n}}-{{hits}}</view>"#, ":host{display:block}.c{height:100px}", CARD_JS),
            component("components/box/box", r#"<view class="b"><slot/></view>"#, ":host{display:block}", BOX_JS),
        ],
    );
    let mut e = engine(&fx);
    let keys = ev(&e, "Object.keys(__compState().map).sort().join(',')");
    assert_eq!(keys, "/my-box@0.1,/my-card@0.0[0],/my-card@0.0[1]", "首帧前就该按每处使用建好实例");
    assert_eq!(ev(&e, "__compState().map['/my-card@0.0[1]'].data.n"), "20");
    assert_eq!(ev(&e, "__obs.join(',')"), "10<-0,20<-0", "初始属性与默认值不同：observer 各触发一次");

    // 第二张卡片在 y∈[100,200)
    tap(&mut e, 100.0, 150.0);
    assert_eq!(ev(&e, "__compState().map['/my-card@0.0[0]'].data.hits"), "0", "第一张不该收到");
    assert_eq!(ev(&e, "__compState().map['/my-card@0.0[1]'].data.hits"), "1");
    assert_eq!(ev(&e, "JSON.stringify(__currentPage.picked)"), "[20]", "triggerEvent 到达页面的 bind:pick");
    assert_eq!(ev(&e, "__currentPage.pickType"), "number");

    // slot 里的按钮在卡片之后：y∈[200,280)。页面和两个组件都有 onSlotTap，只能落到页面
    tap(&mut e, 100.0, 240.0);
    assert_eq!(ev(&e, "__currentPage.slotTaps"), "1", "slot 内容的 bindtap 归页面");
    assert_eq!(ev(&e, "__currentPage.slotIdx"), "7", "slot 内容用页面作用域（含页面的 wx:for 变量）");
    assert_eq!(ev(&e, "__wrong.length"), "0", "不该派发到组件的同名方法");

    // 无关 setData：不触发属性 observer
    e.eval("__obs = []; __currentPage.setData({ other: 1 })").unwrap();
    e.pump(1);
    assert_eq!(ev(&e, "__obs.length"), "0");
    // 改第一项：只有第一张卡片的 observer 触发一次，hits 保留（同一个实例）
    e.eval("__currentPage.setData({ 'items[0]': 11 })").unwrap();
    e.pump(1);
    assert_eq!(ev(&e, "__obs.join(',')"), "11<-10");
    assert_eq!(ev(&e, "__compState().map['/my-card@0.0[1]'].data.hits"), "1");

    // wx:if 变假：box detached；列表变短：最后一张 detached
    let box_id = ev(&e, "__compState().map['/my-box@0.1'].id");
    e.eval("__currentPage.setData({ showBox: false, items: [11] })").unwrap();
    e.pump(1);
    let mut gone: Vec<String> = ev(&e, "__detached.join(',')").split(',').map(str::to_string).collect();
    gone.sort();
    assert_eq!(gone, vec!["20".to_string(), "box".to_string()]);
    assert_eq!(ev(&e, "Object.keys(__compState().map).join(',')"), "/my-card@0.0[0]");
    // 再显示：全新的 box 实例（同一个键，不同的实例）
    e.eval("__currentPage.setData({ showBox: true })").unwrap();
    e.pump(1);
    let new_id = ev(&e, "(__compState().map['/my-box@0.1'] || {}).id");
    assert!(!new_id.is_empty() && new_id != box_id, "应建出新实例：旧 {box_id}，新 {new_id}");
    // 再显示后 slot 照常可点
    tap(&mut e, 100.0, 140.0);
    assert_eq!(ev(&e, "__currentPage.slotTaps"), "2");
}

// ============ 组件里套组件：事件派发给外层实例，属性跟着外层 data 变 ============

const INNER_JS: &str = r#"Component({
    properties: { n: Number },
    methods: { onTap: function () { this.triggerEvent('pick', { n: this.data.n }); } }
});"#;

const OUTER_JS: &str = r#"Component({
    properties: { label: String },
    data: { count: 1 },
    methods: {
        onPick: function (e) {
            this.setData({ count: e.detail.n + 1 });
            this.triggerEvent('done', { label: this.data.label, n: e.detail.n });
        }
    }
});"#;

const NEST_PAGE_JS: &str = r#"globalThis.__wrong = [];
Page({
    data: {},
    onPick: function () { __wrong.push('page.onPick'); },
    onDone: function (e) { this.done = (this.done || []).concat([e.detail.label + ':' + e.detail.n + ':' + e.target.id]); },
    onCaught: function (e) { this.caught = e.detail.label; }
});"#;

#[test]
fn 嵌套组件_内层事件派发给外层实例_外层再派发给页面() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = write_all(
        "nested",
        &[
            ("pages/index/index.json", r#"{"usingComponents":{"my-outer":"/components/outer/outer"}}"#),
            // 两种写法：`bind:done` 与 `catch:done`
            (
                "pages/index/index.wxml",
                r#"<view><my-outer id="o1" label="A" bind:done="onDone"/><my-outer id="o2" label="B" catch:done="onCaught" binddone="onDone"/></view>"#,
            ),
            ("pages/index/index.wxss", ""),
            ("pages/index/index.js", NEST_PAGE_JS),
        ],
        vec![
            // 外层自己声明内层（组件 json 里的 usingComponents）；内层宿主上用不带冒号的 `bindpick`
            vec![
                ("components/outer/outer.json".to_string(), r#"{"component":true,"usingComponents":{"my-inner":"/components/inner/inner"}}"#),
                ("components/outer/outer.wxml".to_string(), r#"<view class="ow"><my-inner n="{{count}}" bindpick="onPick"/><view class="ob">{{label}}</view></view>"#),
                ("components/outer/outer.wxss".to_string(), ":host{display:block}.ob{height:50px}"),
                ("components/outer/outer.js".to_string(), OUTER_JS),
            ],
            component("components/inner/inner", r#"<view class="iv" bindtap="onTap">{{n}}</view>"#, ":host{display:block}.iv{height:50px}", INNER_JS),
        ],
    );
    let mut e = engine(&fx);
    let inner2 = "/my-outer@0.1/my-inner@0.0";
    assert_eq!(
        ev(&e, "Object.keys(__compState().map).sort().join(',')"),
        format!("/my-outer@0.0,/my-outer@0.0/my-inner@0.0,/my-outer@0.1,{inner2}"),
        "外层模板里的组件也是每处一个实例"
    );
    assert_eq!(ev(&e, &format!("__compState().map['{inner2}'].data.n")), "1", "内层属性来自外层 data");
    assert_eq!(ev(&e, &format!("typeof __compState().map['{inner2}'].data.n")), "number");

    // 第二个外层的内层在 y∈[100,150)
    tap(&mut e, 100.0, 125.0);
    assert_eq!(ev(&e, "__wrong.join(',')"), "", "内层的 pick 只派发给外层实例，不能打到页面的同名方法");
    assert_eq!(ev(&e, "__compState().map['/my-outer@0.1'].data.count"), "2");
    assert_eq!(ev(&e, "__compState().map['/my-outer@0.0'].data.count"), "1", "第一个外层不受影响");
    assert_eq!(ev(&e, "JSON.stringify(__currentPage.done)"), r#"["B:1:o2"]"#, "外层的 done 派发给页面（target.id 是宿主 id）");
    assert_eq!(ev(&e, "__currentPage.caught"), "B", "catch:done 也是绑定");
    // 外层 setData 改了 count：内层属性跟着变
    assert_eq!(ev(&e, &format!("__compState().map['{inner2}'].data.n")), "2");
}

// ============ multipleSlots：组件 js 的 options 一路带到模板展开 ============

const LAYOUT_JS: &str = r#"Component({
    options: { multipleSlots: true },
    methods: { onFoot: function () { __wrong.push('layout'); } }
});"#;

#[test]
fn 具名slot_由组件js的multiple_slots打开_内容各进各的位置且归页面() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = write_all(
        "slots",
        &[
            ("pages/index/index.json", r#"{"usingComponents":{"my-layout":"/components/layout/layout"}}"#),
            (
                "pages/index/index.wxml",
                r#"<my-layout><view slot="footer" class="f" bindtap="onFoot">F</view><view class="d" bindtap="onDefault">D</view><view slot="header" class="h" bindtap="onHead">H</view></my-layout>"#,
            ),
            ("pages/index/index.wxss", ".f,.d,.h{height:40px}"),
            (
                "pages/index/index.js",
                r#"globalThis.__wrong = []; Page({ data: {}, onFoot: function () { this.hit = 'foot'; }, onDefault: function () { this.hit = 'default'; }, onHead: function () { this.hit = 'head'; } });"#,
            ),
        ],
        vec![component(
            "components/layout/layout",
            r#"<view><view class="hd"><slot name="header"/></view><slot/><view class="ft"><slot name="footer"/></view></view>"#,
            ":host{display:block}.hd{height:60px}",
            LAYOUT_JS,
        )],
    );
    let mut e = engine(&fx);
    // 期望排布：header 区 [0,60)（.h 在 [0,40)）→ 默认 slot 的 .d [60,100) → footer 的 .f [100,140)。
    // 单一 slot 模式下三个全挤进第一个 slot（header 区），y=120 什么都点不到
    for (y, want) in [(20.0, "head"), (80.0, "default"), (120.0, "foot")] {
        e.eval("__currentPage.hit = ''").unwrap();
        tap(&mut e, 100.0, y);
        assert_eq!(ev(&e, "__currentPage.hit"), want, "y={y}");
    }
    assert_eq!(ev(&e, "__wrong.join(',')"), "", "slot 内容的 onFoot 归页面，不是组件的同名方法");
}
