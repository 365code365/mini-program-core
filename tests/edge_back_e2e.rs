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
    fixture_in("edge-back-e2e")
}

/// 每个用例一个目录：并行跑时别互相删
fn fixture_in(name: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/a/a","pages/b/b","pages/c/c"],"window":{"navigationStyle":"custom"}}"#),
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
        ("pages/c/c.json", "{}"),
        ("pages/c/c.wxml", "<view>C</view>"),
        ("pages/c/c.wxss", "page{background:#00ff00}"),
        ("pages/c/c.js", "Page({});"),
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

/// 栈顶下面那张以外的截图压着（host/shot_pack.rs）：进三层之后截图只占一张原图多一点；
/// 退一层之后新的最后一张解开，侧滑露出来的还是它（逐像素无损，红就是红）
#[test]
fn 深一点的栈_截图压着_退回来照样露出上一页() {
    let fx = fixture_in("edge-back-e2e-deep");
    let data = fx.0.join("_data2");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/a/a")).expect("打开页面");
    let mut e = E { e, t: 1000 };
    e.frames(3);
    e.e.navigate("pages/b/b").unwrap();
    e.settle();
    e.e.navigate("pages/c/c").unwrap();
    e.settle();
    assert_eq!(e.px(100, 300), (0, 255, 0));
    let shots = e.e.memory_breakdown().into_iter().find(|i| i.name == "侧滑返回截图").expect("有这一项");
    let one = 750 * 1334 * 4;
    assert!(shots.alloc < one + one / 10, "两张截图占了 {} 字节（一张原图 {one}）：底下那张没压", shots.alloc);
    // 侧滑回 b：露出的是 b 的图（蓝）
    e.drag_from_edge(250.0, true);
    e.settle();
    e.frames(3);
    assert_eq!(e.e.current_route(), "pages/b/b");
    assert_eq!(e.px(100, 300), (0, 0, 255));
    // 在 b 上再侧滑：露出 a 的图（红，压过又解开的那张）
    e.drag_from_edge(200.0, false);
    let left = e.px(60, 300);
    assert!(left.0 > 150 && left.1 < 40 && left.2 < 40, "左边该露出 a（红，带压暗）：{left:?}");
}

/// 进后台（engine_life.rs `on_hide`）：合成缓冲、RGBA 读出缓冲放掉，最后一张截图也压起来；
/// 回前台（`on_show`）重新合成出同一帧，侧滑露出来的还是上一页（截图解开后无损）
#[test]
fn 进后台放掉合成缓冲与截图_回前台画面与侧滑照旧() {
    let fx = fixture_in("edge-back-e2e-hide");
    let data = fx.0.join("_data3");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/a/a")).expect("打开页面");
    let mut e = E { e, t: 1000 };
    e.frames(3);
    e.e.navigate("pages/b/b").unwrap();
    e.settle();
    let before = e.e.rgba().to_vec();
    e.e.on_hide();
    let item = |e: &E, name: &str| e.e.memory_breakdown().into_iter().find(|i| i.name == name).map_or(0, |i| i.alloc);
    assert_eq!(item(&e, "合成缓冲"), 0, "后台还留着合成缓冲");
    assert_eq!(item(&e, "RGBA 读出缓冲"), 0, "后台还留着 RGBA 读出缓冲");
    let one = 750 * 1334 * 4;
    assert!(item(&e, "侧滑返回截图") < one / 10, "后台截图没压：{} 字节", item(&e, "侧滑返回截图"));
    e.e.on_show();
    e.frames(2);
    assert!(e.e.rgba() == &before[..], "回前台重新合成的一帧与进后台前不一样");
    e.drag_from_edge(200.0, false);
    let left = e.px(60, 300);
    assert!(left.0 > 150 && left.1 < 40 && left.2 < 40, "左边该露出 a（红，带压暗）：{left:?}");
}
