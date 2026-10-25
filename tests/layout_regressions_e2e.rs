//! 真机上撞出来的布局问题（用户截图），每条一个用例，按 SelectorQuery 量出来的盒子判。
//!
//! 走 `MiniEngine`（SDK 链路）。独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
static SERIAL: Mutex<()> = Mutex::new(());

fn fixture(name: &str, wxml: &str, wxss: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("layout-regress").join(name);
    std::fs::remove_dir_all(&root).ok();
    let js = "Page({ data: { out: null }, q(sel) { wx.createSelectorQuery().selectAll(sel).boundingClientRect(r => this.setData({ out: r })).exec(); } });";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", wxss),
        ("pages/index/index.js", js),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn rects(root: &PathBuf, sel: &str) -> Vec<(f64, f64, f64, f64)> {
    rects_at(root, sel, 375, 667, 2.0)
}

fn rects_at(root: &PathBuf, sel: &str, w: u32, h: u32, dpr: f32) -> Vec<(f64, f64, f64, f64)> {
    let data = root.join("_data");
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), w, h, dpr).expect("创建引擎");
    e.launch(Some("pages/index/index")).expect("打开页面");
    let mut t = 1000;
    for _ in 0..3 {
        t += 16;
        e.pump(t);
    }
    e.eval(&format!("__currentPage.q({sel:?})")).unwrap();
    for _ in 0..4 {
        t += 16;
        e.pump(t);
    }
    let s = e.eval("JSON.stringify(__currentPage.data.out)").unwrap_or_default();
    let s: String = serde_json::from_str(&s).unwrap_or(s);
    let v: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
    v.as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|r| {
            let f = |k: &str| r[k].as_f64().unwrap_or(-1.0);
            (f("left"), f("top"), f("width"), f("height"))
        })
        .collect()
}

/// 生活缴费首页「待缴提醒」：名称后面的小标签 `我家` 在一个 column flex 里的 row 里，横向空间足够，
/// 却被挤成两行（每行一个字）。浏览器 / 微信里它是一行：flex 项的收缩只在放不下时发生
#[test]
fn 纵向_flex_里的横排_文字标签不该被挤成竖排() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let wxml = r#"<view class="item row">
  <view class="ic"><text>电</text></view>
  <view class="main">
    <view class="row"><text class="name">电费</text><text class="tag">我家</text></view>
    <text class="muted">上月电费 · 户号 3300 8812 04</text>
  </view>
  <view class="right"><text class="owe">¥213.65</text><view class="btn"><text>去缴费</text></view></view>
</view>"#;
    let wxss = ".row { display: flex; flex-direction: row; align-items: center; }
.item { padding: 13px 14px 2px; }
.ic { width: 44px; height: 44px; border-radius: 12px; background: #ffb340; }
.main { flex: 1; margin-left: 10px; display: flex; flex-direction: column; }
.name { font-size: 14px; font-weight: bold; margin-bottom: 3px; }
.tag { font-size: 10px; color: #14a06f; background: #e6f7f0; padding: 1px 5px; border-radius: 3px; margin: 0 0 3px 5px; }
.muted { font-size: 12px; color: #999; }
.right { display: flex; flex-direction: column; align-items: flex-end; }
.owe { font-size: 15px; }
.btn { margin-top: 4px; height: 24px; padding: 0 10px; border-radius: 12px; background: #2fc488; font-size: 11px; display: flex; flex-direction: row; align-items: center; }";
    let root = fixture("tag-in-column", wxml, wxss);
    let tag = rects(&root, ".tag");
    let name = rects(&root, ".name");
    assert_eq!(tag.len(), 1, "{tag:?}");
    let (_, _, w, h) = tag[0];
    // 两个 10px 的字 + 左右 5px 内边距 ≈ 30px 宽、一行 ≈ 15px 高
    assert!(w >= 28.0, "标签被挤窄了：{tag:?}");
    assert!(h < 20.0, "标签被挤成了两行：{tag:?}（名称 {name:?}）");
}

#[test]
fn 探查_标签各种像素比() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    if std::env::var_os("PROBE").is_none() { return; }
    let root = fixture("tag-probe", r#"<view class="row"><text class="name">电费</text><text class="tag">我家</text></view>"#,
        ".row{display:flex;flex-direction:row;align-items:center}.name{font-size:14px;font-weight:bold}.tag{font-size:10px;padding:1px 5px;margin-left:5px;background:#e6f7f0}");
    for (w, dpr) in [(375u32, 2.0f32), (424, 3.0), (442, 2.875), (393, 2.75), (411, 2.625), (360, 3.0), (414, 3.0)] {
        eprintln!("{w}@{dpr}: {:?}", rects_at(&root, ".tag", w, 800, dpr));
    }
}

#[test]
fn 探查_真实页面() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (Some(app), Ok(route), Ok(sel)) = (std::env::var_os("PROBE_APP"), std::env::var("PROBE_ROUTE"), std::env::var("PROBE_SEL")) else { return };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample").join(app);
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("probe-real");
    std::fs::remove_dir_all(&data).ok();
    for (w, dpr) in [(375u32, 2.0f32), (427, 2.975), (360, 3.0)] {
        let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), w, 900, dpr).expect("创建引擎");
        e.launch(Some(&route)).expect("打开");
        let mut t = 1000;
        for _ in 0..5 { t += 16; e.pump(t); }
        e.eval(&format!("wx.createSelectorQuery().selectAll({sel:?}).boundingClientRect(r => {{ globalThis.__probe = JSON.stringify(r.map(x => [x.left, x.top, x.width, x.height].map(v => Math.round(v*10)/10))); }}).exec()")).unwrap();
        for _ in 0..4 { t += 16; e.pump(t); }
        eprintln!("{w}@{dpr}: {}", e.eval("globalThis.__probe").unwrap_or_default());
    }
}
