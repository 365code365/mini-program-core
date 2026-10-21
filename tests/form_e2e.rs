//! `<form>` 的 `bindsubmit` / `bindreset`（G-3 的 form 部分）。
//!
//! 依据官方 component/form.html：点 `form-type="submit"` 的按钮，把表单里带 `name` 的控件按
//! `{name: value}` 提交，`e.detail = {value, formId: ''}`；值的类型跟控件走（switch 布尔、slider
//! 数字、checkbox-group 数组、radio-group 字符串、picker 与 bindchange 同型）。
//!
//! 走 `MiniEngine`（SDK 链路：真指针 → 触摸状态机 → tap 派发），独立集成测试二进制的原因同
//! `component_e2e.rs`：小程序根目录是进程级的。
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

/// 每个控件独占一行 50px，点哪一行就是点哪个控件（控件都贴着行的左上角）
const ROW: f32 = 50.0;

const WXML: &str = r#"<form bindsubmit="onSubmit" bindreset="onReset">
  <view class="row"><input name="nick" value="init" style="height:40px;width:300px"/></view>
  <view class="row"><switch name="agree" checked="{{true}}"/></view>
  <view class="row"><slider name="vol" value="30" min="0" max="100"/></view>
  <checkbox-group name="tags">
    <view class="row"><checkbox value="a" checked="{{true}}"/></view>
    <view class="row"><checkbox value="b"/></view>
  </checkbox-group>
  <radio-group name="sex">
    <view class="row"><radio value="m"/></view>
    <view class="row"><radio value="f" checked="{{true}}"/></view>
  </radio-group>
  <view class="row"><picker name="pk" range="{{opts}}" value="2"><view>pick</view></picker></view>
  <view class="row"><input value="no-name" style="height:40px;width:300px"/></view>
  <view class="row"><button form-type="submit" bindtap="onBtn" style="width:200px;height:44px;margin:0">提交</button></view>
  <view class="row"><button form-type="reset" style="width:200px;height:44px;margin:0">重置</button></view>
</form>
<view class="row"><button form-type="submit" style="width:200px;height:44px;margin:0">表单外</button></view>"#;

const WXSS: &str = ".row{height:50px;overflow:hidden}";

const JS: &str = r#"
Page({
  data: { opts: ['x', 'y', 'z'], log: [], submitted: null, resetDetail: null },
  onBtn() { this.data.log.push('tap'); this.setData({ log: this.data.log }); },
  onSubmit(e) {
    this.data.log.push('submit');
    this.setData({ log: this.data.log, submitted: JSON.stringify(e.detail) });
  },
  onReset(e) { this.setData({ resetDetail: JSON.stringify(e.detail) }); },
});
"#;

fn fixture(name: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("form-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", WXML),
        ("pages/index/index.wxss", WXSS),
        ("pages/index/index.js", JS),
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
        let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 700, 2.0)
            .expect("创建引擎");
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
    /// 点第 `row` 行里靠左上的位置（控件都贴着行的左上角）
    fn tap_row(&mut self, row: usize) {
        let (x, y) = (10.0, row as f32 * ROW + 10.0);
        self.e.pointer_down(x, y);
        self.frames(1);
        self.e.pointer_up(x, y);
        self.frames(2);
    }
    fn js(&self, code: &str) -> String {
        self.e.eval(code).unwrap_or_default().trim_matches('"').to_string()
    }
    fn submitted(&self) -> serde_json::Value {
        let s = self.js("__currentPage.data.submitted");
        serde_json::from_str(&s.replace("\\\"", "\"")).unwrap_or_else(|_| panic!("没收到 submit：{s}"))
    }
}

const SUBMIT: usize = 9;
const RESET: usize = 10;

#[test]
fn submit_collects_named_fields_with_wechat_types() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("initial");
    let mut e = E::new(&fx);
    e.tap_row(SUBMIT);
    assert_eq!(e.js("JSON.stringify(__currentPage.data.log)"), r#"["tap","submit"]"#, "按钮的 bindtap 先发，再提交");
    let d = e.submitted();
    assert_eq!(d["formId"], "");
    assert_eq!(
        d["value"],
        serde_json::json!({ "nick": "init", "agree": true, "vol": 30, "tags": ["a"], "sex": "f", "pk": 2 }),
        "没写 name 的输入框不提交；类型：switch 布尔、slider 数字、checkbox-group 数组、radio-group 字符串、selector 下标数字"
    );
}

#[test]
fn submit_reflects_user_edits_then_reset_clears_them() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("edits");
    let mut e = E::new(&fx);
    // 拨开关、勾 b、选 m、往输入框里补一个字
    e.tap_row(1);
    e.tap_row(4);
    e.tap_row(5);
    e.tap_row(0);
    e.e.text_input("X");
    e.frames(2);
    e.tap_row(SUBMIT);
    assert_eq!(
        e.submitted()["value"],
        // 点在输入框最左边，光标落在开头，补的字在最前
        serde_json::json!({ "nick": "Xinit", "agree": false, "vol": 30, "tags": ["a", "b"], "sex": "m", "pk": 2 })
    );

    e.tap_row(RESET);
    assert_eq!(e.js("__currentPage.data.resetDetail"), "{}", "bindreset 不带数据");
    e.tap_row(SUBMIT);
    assert_eq!(
        e.submitted()["value"],
        serde_json::json!({ "nick": "", "agree": false, "vol": 0, "tags": [], "sex": "", "pk": 2 }),
        "重置：输入框清空、开关与勾选取消、滑块回到 min；picker 不留状态、不动"
    );
}

#[test]
fn button_outside_form_does_not_submit() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("outside");
    let mut e = E::new(&fx);
    e.tap_row(11);
    assert_eq!(e.js("__currentPage.data.submitted"), "null");
}
