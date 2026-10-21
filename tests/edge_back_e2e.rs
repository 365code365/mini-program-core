//! SDK 引擎的左边缘侧滑返回（从前 SDK 没有这个手势，`begin_gesture` 恒传「侧滑忙」把它关掉）。
//!
//! 判定与合成与桌面窗体同一份（`host::edge_back`），这里守 SDK 那几处接线：
//! 跟手时整帧右移、左边露出上一页留下的图；不够阈值松手滑回、页面不变；过阈值退栈；栈底没有这个手势。
//! 走 `MiniEngine`，独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::time::Duration;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn fixture() -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("edge-back-e2e");
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/a/a","pages/b/b"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/a/a.json", "{}"),
        ("pages/a/a.wxml", "<view>A</view>"),
        ("pages/a/a.wxss", "page{background:#ff0000}"),
        ("pages/a/a.js", "Page({});"),
        ("pages/b/b.json", "{}"),
        ("pages/b/b.wxml", "<view>B</view>"),
        ("pages/b/b.wxss", "page{background:#0000ff}"),
        ("pages/b/b.js", "Page({});"),
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
    fn frames(&mut self, n: u32) {
        for _ in 0..n {
            self.t += 16;
            self.e.pump(self.t);
        }
    }
    /// 收尾动画按墙上时钟走（0.22s）
    fn settle(&mut self) {
        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(16));
            self.frames(1);
        }
    }
    fn px(&mut self, x: usize, y: usize) -> (u8, u8, u8) {
        let (w, _) = self.e.pixel_size();
        let p = self.e.rgba();
        let i = (y * 2 * w as usize + x * 2) * 4;
        (p[i], p[i + 1], p[i + 2])
    }
    /// 从左边缘按下、慢慢右划到 `to_x`，`release` 为假时手指不抬
    fn drag_from_edge(&mut self, to_x: f32, release: bool) {
        let y = 300.0;
        self.e.pointer_down(4.0, y);
        self.frames(1);
        for i in 1..=10 {
            std::thread::sleep(Duration::from_millis(30));
            self.e.pointer_move(4.0 + (to_x - 4.0) * i as f32 / 10.0, y);
            self.frames(1);
        }
        if release {
            std::thread::sleep(Duration::from_millis(120));
            self.e.pointer_up(to_x, y);
            self.frames(1);
        }
    }
}

#[test]
fn edge_swipe_follows_finger_cancels_and_commits() {
    let fx = fixture();
    let data = fx.0.join("_data");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/a/a")).expect("打开页面");
    let mut e = E { e, t: 1000 };
    e.frames(3);
    assert_eq!(e.px(100, 300), (255, 0, 0));

    // 栈底：没有侧滑返回，页面不动
    e.drag_from_edge(200.0, true);
    e.settle();
    assert_eq!(e.e.current_route(), "pages/a/a");
    assert_eq!(e.px(100, 300), (255, 0, 0), "栈底右划不该推页面");

    e.e.navigate("pages/b/b").unwrap();
    e.frames(3);
    assert_eq!(e.px(100, 300), (0, 0, 255));

    // 跟手：手指在 200，当前页（蓝）右移 196，左边露出上一页（红，带压暗）
    e.drag_from_edge(200.0, false);
    let left = e.px(60, 300);
    assert!(left.0 > 150 && left.2 < 40, "左边露出上一页留下的图：{left:?}");
    assert_eq!(e.px(300, 300), (0, 0, 255), "右边还是当前页");
    // 慢慢退回到 60 再松手：不够 35% 屏宽，滑回去，页面不变
    for i in 1..=6 {
        std::thread::sleep(Duration::from_millis(30));
        e.e.pointer_move(200.0 - 140.0 * i as f32 / 6.0, 300.0);
        e.frames(1);
    }
    std::thread::sleep(Duration::from_millis(120));
    e.e.pointer_up(60.0, 300.0);
    e.frames(1);
    e.settle();
    assert_eq!(e.e.current_route(), "pages/b/b", "没过阈值：取消");
    assert_eq!(e.px(30, 300), (0, 0, 255), "滑回原位");

    // 过阈值松手：退栈
    e.drag_from_edge(250.0, true);
    e.settle();
    e.frames(3);
    assert_eq!(e.e.current_route(), "pages/a/a", "过阈值：返回上一页");
    assert_eq!(e.px(100, 300), (255, 0, 0));
}
