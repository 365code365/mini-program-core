//! 内置组件自己的点击行为（G-12 里列出的「没单独登记的官方组件」）：`<navigator>` 跳转、`<label>` 转发，
//! 以及路由 API 的相对路径（`../other/other?id=3`，按当前页面所在目录解析）。
//!
//! 依据官方 component/navigator.html、component/label.html、api/route/wx.navigateTo（url 可以是相对路径）。
//! 走 `MiniEngine`（SDK 链路）。
use mini_render::host::MiniEngine;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn fixture(name: &str, wxml: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("builtin-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index","pages/other/other"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", ".row{height:60px}"),
        ("pages/index/index.js", "Page({ data: { log: [] }, rec(x) { this.data.log.push(x); }, onChange(e) { this.rec('change:' + JSON.stringify(e.detail.value)); }, onSw(e) { this.rec('switch:' + e.detail.value); }, onInner() { this.rec('inner'); } });"),
        ("pages/other/other.json", "{}"),
        ("pages/other/other.wxml", "<view>other</view>"),
        ("pages/other/other.wxss", ""),
        ("pages/other/other.js", "Page({ onLoad(q) { getApp().__q = q; } });"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    Fixture(root)
}

struct E {
    e: MiniEngine,
    t: u64,
}

impl E {
    fn new(fx: &Fixture) -> Self {
        let data = fx.0.join("_data");
        let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
        e.set_animation_time(2.0);
        e.launch(Some("pages/index/index")).expect("打开页面");
        let mut me = E { e, t: 1000 };
        me.frames(3);
        me
    }
    fn frames(&mut self, n: u32) {
        for _ in 0..n {
            self.t += 16;
            self.e.pump(self.t);
        }
    }
    fn tap(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.frames(1);
        self.e.pointer_up(x, y);
        self.frames(6);
    }
    fn js(&self, code: &str) -> Value {
        let s = self.e.eval(&format!("JSON.stringify({code})")).unwrap_or_default();
        let s: String = serde_json::from_str(&s).unwrap_or(s);
        serde_json::from_str(&s).unwrap_or(Value::Null)
    }
    /// 字符串结果直接取（`js()` 走 JSON，字符串会被多解一层）
    fn text(&self, code: &str) -> String {
        self.e.eval(&format!("String({code})")).unwrap_or_default().trim_matches('"').to_string()
    }
    fn route(&self) -> String {
        self.text("(getCurrentPages().slice(-1)[0] || {}).route")
    }
    fn log(&self) -> Value {
        self.js("getCurrentPages()[0].data.log")
    }
}

fn has(log: &Value, s: &str) -> bool {
    log.as_array().map(|a| a.iter().any(|x| x.as_str() == Some(s))).unwrap_or(false)
}

/// 点 navigator 里的文字：相对地址按当前页目录解析，查询参数带到 onLoad
#[test]
fn navigator_navigates_with_relative_url() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture("nav", r#"<navigator class="row" url="../other/other?id=3"><text>去详情</text></navigator>"#);
    let mut e = E::new(&fx);
    assert_eq!(e.route(), "pages/index/index");
    e.tap(40.0, 30.0);
    assert_eq!(e.route(), "pages/other/other", "点 navigator 应跳到 other 页");
    assert_eq!(e.text("getApp().__q.id"), "3", "查询参数带到 onLoad");
}

/// catchtap 截住的子节点里点击不跳转；路由 API 的相对路径同样按当前页目录解析
#[test]
fn navigator_catch_stops_it_and_api_resolves_relative() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture(
        "nav-back",
        r#"<navigator class="row" url="/pages/other/other"><view class="row" catchtap="onInner"><text>截住</text></view></navigator>"#,
    );
    let mut e = E::new(&fx);
    e.tap(40.0, 30.0);
    assert_eq!(e.route(), "pages/index/index", "catchtap 截住了，不跳");
    assert!(has(&e.log(), "inner"), "{}", e.log());
    e.e.eval("wx.navigateTo({ url: '../other/other?id=9' })").unwrap();
    e.frames(6);
    assert_eq!(e.route(), "pages/other/other");
    assert_eq!(e.text("getApp().__q.id"), "9");
}

/// label：点 label 里的文字 → 里面的 checkbox 被勾选（checkbox-group 的 change 带上它）；
/// for 指向页面别处的 switch → 点 label 拨动它
#[test]
fn label_forwards_tap_to_its_control() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture(
        "label",
        r#"<checkbox-group bindchange="onChange">
             <label class="row" style="display:flex;flex-direction:row"><checkbox value="a"/><text style="margin-left:100px">选项 A</text></label>
           </checkbox-group>
           <label class="row" for="sw"><text>开关说明</text></label>
           <switch id="sw" bindchange="onSw"/>"#,
    );
    let mut e = E::new(&fx);
    // 「选项 A」的文字在 checkbox 右边 100px 开外
    e.tap(200.0, 30.0);
    let log = e.log();
    assert!(has(&log, "change:[\"a\"]"), "点 label 文字应勾上 checkbox：{log}");
    e.tap(40.0, 90.0);
    let log = e.log();
    assert!(has(&log, "switch:true"), "label for=sw 应拨动 switch：{log}");
}
