//! 从一页进下一页再回来：滚动位置要停回原处；中途宿主改了尺寸（Flutter 里 AppBar 显隐），
//! 侧滑返回露出来的上一页也不能是一片黑。两条都是 2026-10-22 在 Android 模拟器上的 Flutter 例子里发现的。
//!
//! 夹具写在 `CARGO_TARGET_TMPDIR` 下，串行跑（小程序根目录是进程级的，见 embed_host.rs）。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

/// p：2000 高的绿色长页，导航栏蓝色；q：custom 导航栏的白页
fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("page-return").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r##"{"pages":["pages/p/p","pages/q/q"],"window":{"navigationBarBackgroundColor":"#0000ff"}}"##),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", r#"<view class="long">长页</view>"#),
        ("pages/p/p.wxss", "page{background-color:#00ff00;} .long{height:2000px;}"),
        ("pages/p/p.js", "Page({});"),
        ("pages/p/p.json", "{}"),
        ("pages/q/q.wxml", "<view>第二页</view>"),
        ("pages/q/q.wxss", "page{background-color:#ffffff;}"),
        ("pages/q/q.js", "Page({});"),
        ("pages/q/q.json", r#"{"navigationStyle":"custom"}"#),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, h: u32) -> MiniEngine {
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, h, 1.0, HostEnv::default(), EngineOptions::default())
        .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e.pump(1);
    e
}

/// 滚到 `target` 附近（滚轮一步步推，等它停下）
fn scroll_to(e: &mut MiniEngine, target: f32, t: &mut u64) {
    for _ in 0..200 {
        if e.scroll_position() >= target {
            break;
        }
        e.wheel(40.0, true);
        *t += 16;
        e.pump(*t);
    }
    for _ in 0..120 {
        *t += 16;
        e.pump(*t);
    }
}

#[test]
fn 返回上一页时停回离开时的滚动位置() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("scroll");
    let mut e = engine(&root, 667);
    let mut t = 10;
    scroll_to(&mut e, 600.0, &mut t);
    let before = e.scroll_position();
    assert!(before > 500.0, "先滚下去：{before}");
    e.navigate("/pages/q/q").unwrap();
    t += 16;
    e.pump(t);
    assert_eq!(e.scroll_position(), 0.0, "新页从顶上开始");
    assert!(e.back());
    t += 16;
    e.pump(t);
    assert!((e.scroll_position() - before).abs() < 0.5, "回来停在 {before}，实际 {}", e.scroll_position());
}

#[test]
fn 改了高度之后侧滑露出的上一页不是一片黑() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("shot");
    // 普通页在 AppBar 下面（矮 80），custom 页铺满（宿主去掉 AppBar 后改尺寸）
    let mut e = engine(&root, 587);
    e.navigate("/pages/q/q").unwrap();
    e.pump(20);
    assert!(e.resize(375, 667, 1.0));
    e.pump(40);
    // 从左边缘拖到 200：左边露出上一页
    e.pointer_down(2.0, 400.0);
    let mut t = 40;
    for x in [20.0, 60.0, 120.0, 200.0] {
        e.pointer_move(x, 400.0);
        t += 16;
        e.pump(t);
    }
    let (w, _) = e.pixel_size();
    let px = |e: &mut MiniEngine, x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        let p = e.rgba();
        (p[i], p[i + 1], p[i + 2])
    };
    // 露出来的是压暗的绿色页面（不是黑）；顶上补的 80 是那一页的导航栏色（蓝）
    let mid = px(&mut e, 60, 400);
    assert!(mid.1 > 120 && mid.0 < 60 && mid.2 < 60, "上一页的绿底（压暗）：{mid:?}");
    let top = px(&mut e, 60, 40);
    assert!(top.2 > 120 && top.0 < 60 && top.1 < 60, "顶上补的是导航栏色：{top:?}");
    e.pointer_cancel();
}
