//! 输入框与布尔属性、dataset 的微信语义（2026-10-24 审计修掉的那几条），走 SDK 的 `MiniEngine`：
//! - `bindinput` 返回一个字符串就替换输入框内容；
//! - `data-row-index` 进事件对象是 `dataset.rowIndex`。
//! 独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use std::path::PathBuf;

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
    let fx = fixture();
    let (mut e, mut t) = engine(&fx);
    tap(&mut e, &mut t, 100.0, 70.0);
    let ds = e.eval("__currentPage.data.tap").unwrap();
    assert!(ds.contains(r#"\"rowIndex\":3"#) || ds.contains(r#""rowIndex":3"#), "data-row-index → rowIndex：{ds}");
    assert!(ds.contains("mixed"), "data-Mixed → mixed（大写转小写）：{ds}");
}
