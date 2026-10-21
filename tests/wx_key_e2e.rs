//! `wx:key` 的端到端用例：列表重排后，项的内部状态（这里是没有绑定数据的 `<switch>` 的选中）
//! 跟着 key 走；没写 `wx:key` 时按位置（微信同样如此）。走 `MiniEngine` 真指针按下/抬起。
//!
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

fn fixture(name: &str, wxml: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wx-key-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        // flex-direction 必须显式写 row：Skyline 的 flex-direction 默认值是 column
        // （官方 skyline/wxss.html 属性支持表），不写的话开关和文字会竖着排
        ("pages/index/index.wxss", ".row{height:60px;display:flex;flex-direction:row;align-items:center}"),
        ("pages/index/index.js", "Page({ data: { list: [{ id: 'a' }, { id: 'b' }, { id: 'c' }] } });"),
    ];
    for (rel, text) in files {
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
    e.pump(1);
    e
}

fn tap(e: &mut MiniEngine, x: f32, y: f32) {
    e.pointer_down(x, y);
    e.pump(0);
    e.pointer_up(x, y);
    e.pump(0);
}

/// 第 `row` 行开关「开」着没有：开关左半段在选中时是绿色（逻辑坐标 x=12 处），关着是白底
fn switch_on(e: &mut MiniEngine, row: usize) -> bool {
    // 开关的切换有动画，多泵几帧让它落定
    for t in 1..=40 {
        e.pump(t * 16);
    }
    let (w, _) = e.pixel_size();
    let (x, y) = (12 * 2, (row * 60 + 30) * 2);
    let px = e.rgba();
    let i = (y * w as usize + x) * 4;
    let (r, g, b) = (px[i], px[i + 1], px[i + 2]);
    g > 150 && r < 120 && b < 120
}

fn run(name: &str, wxml: &str) -> (Vec<bool>, Vec<bool>) {
    let fx = fixture(name, wxml);
    let mut e = engine(&fx);
    // 点第二行（b）的开关
    tap(&mut e, 26.0, 90.0);
    let before = (0..3).map(|r| switch_on(&mut e, r)).collect();
    // 把 b 挪到第一行
    e.eval("__currentPage.setData({ list: [{ id: 'b' }, { id: 'a' }, { id: 'c' }] })").unwrap();
    e.pump(1);
    let after = (0..3).map(|r| switch_on(&mut e, r)).collect();
    (before, after)
}

#[test]
fn 带_wx_key_时项的内部状态跟着_key_走() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (before, after) = run(
        "keyed",
        r#"<view wx:for="{{list}}" wx:key="id" class="row"><switch/><text>{{item.id}}</text></view>"#,
    );
    assert_eq!(before, vec![false, true, false], "点了第二行的开关");
    assert_eq!(after, vec![true, false, false], "b 挪到第一行，开着的开关跟着 b 走");
}

#[test]
fn 没写_wx_key_时按位置() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (before, after) = run("unkeyed", r#"<view wx:for="{{list}}" class="row"><switch/><text>{{item.id}}</text></view>"#);
    assert_eq!(before, vec![false, true, false]);
    assert_eq!(after, vec![false, true, false], "没有 key：状态留在第二行（微信同样按下标复用）");
}
