//! 输入框与布尔属性、dataset 的微信语义（2026-10-24 审计修掉的那几条），走 SDK 的 `MiniEngine`：
//! - `bindinput` 返回一个字符串就替换输入框内容；
//! - `data-row-index` 进事件对象是 `dataset.rowIndex`。
//! 独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::sync::Mutex;

/// 小程序根目录是进程级的：同一个二进制里的用例一个一个跑
static SERIAL: Mutex<()> = Mutex::new(());

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

const WXML: &str = r#"<input id="kw" style="height:40px" bindinput="onInput"/>
<view id="row" style="height:60px" data-row-index="3" data-Mixed="m" bindtap="onTap">row</view>
"#;

const JS: &str = r#"
Page({
  data: { seen: [], tap: '' },
  onInput(e) { this.data.seen.push(e.detail.value); return e.detail.value.toUpperCase(); },
  onTap(e) { this.setData({ tap: JSON.stringify(e.currentTarget.dataset) }); },
});
"#;

fn fixture() -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("input-semantics-e2e");
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", WXML),
        ("pages/index/index.wxss", ""),
        ("pages/index/index.js", JS),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    Fixture(root)
}

fn engine(fx: &Fixture) -> (MiniEngine, u64) {
    let data = fx.0.join("_data");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/index/index")).expect("打开页面");
    let mut t = 1000;
    for _ in 0..3 {
        t += 16;
        e.pump(t);
    }
    (e, t)
}

fn tap(e: &mut MiniEngine, t: &mut u64, x: f32, y: f32) {
    e.pointer_down(x, y);
    *t += 16;
    e.pump(*t);
    e.pointer_up(x, y);
    for _ in 0..2 {
        *t += 16;
        e.pump(*t);
    }
}

#[test]
fn bindinput_返回字符串替换内容() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture();
    let (mut e, mut t) = engine(&fx);
    tap(&mut e, &mut t, 100.0, 20.0);
    e.text_input("a");
    t += 16;
    e.pump(t);
    e.text_input("b");
    t += 16;
    e.pump(t);
    // 第二次输入时框里已经是被换成的 "A"
    assert_eq!(e.eval("__currentPage.data.seen.join(',')").unwrap().trim_matches('"'), "a,Ab");
}

#[test]
fn dataset_键名转驼峰() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture();
    let (mut e, mut t) = engine(&fx);
    tap(&mut e, &mut t, 100.0, 70.0);
    let ds = e.eval("__currentPage.data.tap").unwrap();
    assert!(ds.contains(r#"\"rowIndex\":3"#) || ds.contains(r#""rowIndex":3"#), "data-row-index → rowIndex：{ds}");
    assert!(ds.contains("mixed"), "data-Mixed → mixed（大写转小写）：{ds}");
}

fn engine_with(wxml: &str, js: &str, name: &str) -> (Fixture, MiniEngine, u64) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", ""),
        ("pages/index/index.js", js),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let fx = Fixture(root);
    let (e, t) = engine(&fx);
    (fx, e, t)
}

fn events(e: &mut MiniEngine) -> Vec<String> {
    std::iter::from_fn(|| e.poll_event()).collect()
}

/// 手机号这类「bindinput 里 setData 加空格」的输入框：光标保持在末尾，并发 `inputValue` 让输入法换内容。
/// 从前光标按下标夹住，每插一个空格就落后一位（「139 5901|48」）
#[test]
fn set_data_格式化之后光标还在末尾() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(
        r#"<input id="ph" style="height:40px" value="{{tx}}" bindinput="onIn"/>"#,
        r#"Page({ data: { tx: '' }, onIn(e) { const d = e.detail.value.replace(/\D/g, '');
            this.setData({ tx: d.replace(/^(\d{3})(\d{0,4})(\d{0,4}).*/, (m, a, b, c) => [a, b, c].filter(Boolean).join(' ')) }); } });"#,
        "input-format-e2e",
    );
    tap(&mut e, &mut t, 100.0, 20.0);
    events(&mut e);
    for ch in "1395901".chars() {
        e.text_input(&ch.to_string());
        t += 16;
        e.pump(t);
        t += 16;
        e.pump(t);
    }
    let evs = events(&mut e);
    // 打第 4 位时页面插了一个空格（「1395」→「139 5」）：光标要在末尾（5），从前停在 4
    let ev = evs.iter().find(|s| s.contains("\"inputValue\"")).expect("页面改了值要发 inputValue");
    assert!(ev.contains("\"value\":\"139 5\"") && ev.contains("\"cursor\":5"), "{ev}");
    // 之后接着打的字也落在末尾
    assert_eq!(e.eval("__currentPage.data.tx").unwrap().trim_matches('"'), "139 5901");
}

/// 输入框聚焦时点 picker：输入框失焦（`bindblur`、键盘收起），面板弹出
#[test]
fn 点_picker_输入框失焦() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(
        r#"<input id="a" style="height:40px;text-align:right" value="1000" bindblur="onBlur"/>
<picker range="{{r}}" value="0"><view style="height:60px">pick</view></picker>"#,
        r#"Page({ data: { r: ['a', 'b'], blurred: 0 }, onBlur() { this.setData({ blurred: this.data.blurred + 1 }); } });"#,
        "input-picker-blur-e2e",
    );
    tap(&mut e, &mut t, 100.0, 20.0);
    assert!(events(&mut e).iter().any(|s| s.contains("\"inputFocus\"")));
    tap(&mut e, &mut t, 100.0, 70.0);
    assert!(e.picker_sheet_open(), "点在 picker 上要弹面板");
    let evs = events(&mut e);
    assert!(evs.iter().any(|s| s.contains("\"inputBlur\"")), "要发 inputBlur 收键盘：{evs:?}");
    assert_eq!(e.eval("__currentPage.data.blurred").unwrap(), "1", "bindblur 要触发");
}
