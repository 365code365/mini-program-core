//! swiper 的端到端用例：`MiniEngine`（SDK 链路：真指针 → 手势仲裁 → swiper 状态机 → 事件）。
//! 时钟由测试给（每帧 16ms），动画与自动播放可复现。
//!
//! 独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的；swiper 的状态表
//! 也是进程级的，所以每个用例都用一把锁串行。
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
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("swiper-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let js = r#"globalThis.__ev = [];
Page({
    data: { cur: 0, other: 0 },
    onChange: function (e) { __ev.push('change:' + e.detail.current + ':' + typeof e.detail.current + ':' + e.detail.source); },
    onFinish: function (e) { __ev.push('finish:' + e.detail.current); },
    onTransition: function (e) { this.dx = e.detail.dx; },
    onTapItem: function (e) { __ev.push('tap:' + e.currentTarget.dataset.i); }
});"#;
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", ".sw{width:375px;height:200px}.it{height:200px}"),
        ("pages/index/index.js", js),
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
        mini_render::renderer::components::reset_shared_component_state();
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
    fn drag_x(&mut self, y: f32, x0: f32, x1: f32, steps: u32) {
        self.e.pointer_down(x0, y);
        self.frames(1);
        for i in 1..=steps {
            self.e.pointer_move(x0 + (x1 - x0) * i as f32 / steps as f32, y);
            self.frames(1);
        }
        self.e.pointer_up(x1, y);
        self.frames(1);
    }
    fn ev(&self) -> String {
        self.e.eval("__ev.join(',')").unwrap_or_default().trim_matches('"').to_string()
    }
    fn clear(&mut self) {
        self.e.eval("__ev = []").unwrap();
    }
    /// (x, y) 处的像素（逻辑坐标）
    fn px(&mut self, x: u32, y: u32) -> (u8, u8, u8) {
        let (w, _) = self.e.pixel_size();
        let p = self.e.rgba();
        let i = (((y * 2) * w + x * 2) * 4) as usize;
        (p[i], p[i + 1], p[i + 2])
    }
}

fn swiper(attrs: &str) -> String {
    let colors = ["#ff0000", "#00ff00", "#0000ff"];
    let items: String = colors
        .iter()
        .enumerate()
        .map(|(i, c)| format!(r#"<swiper-item><view class="it" style="background-color:{c}" data-i="{i}" bindtap="onTapItem"></view></swiper-item>"#))
        .collect();
    format!(r#"<swiper id="sw" class="sw" {attrs} bindchange="onChange" bindanimationfinish="onFinish" bindtransition="onTransition">{items}</swiper><view style="height:1500px"></view>"#)
}

#[test]
fn 手指横划翻页_change_带_touch_与数字_current_竖划滚页面() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("drag", &swiper(""));
    let mut e = E::new(&fx);
    assert_eq!(e.px(100, 100), (255, 0, 0));
    e.drag_x(100.0, 300.0, 80.0, 8);
    assert!(e.ev().contains("change:1:number:touch"), "{}", e.ev());
    e.frames(40);
    assert!(e.ev().contains("finish:1"), "{}", e.ev());
    assert_eq!(e.px(100, 100), (0, 255, 0), "翻到第二页");
    // 拖到一半（不过半、慢）松手：回到第二页，不发 change
    e.clear();
    e.e.pointer_down(200.0, 100.0);
    e.frames(1);
    for i in 1..=20 {
        e.e.pointer_move(200.0 - 3.0 * i as f32, 100.0);
        e.frames(3);
    }
    e.e.pointer_up(140.0, 100.0);
    e.frames(40);
    assert!(!e.ev().contains("change"), "{}", e.ev());
    assert_eq!(e.px(100, 100), (0, 255, 0));
    // 竖着划：页面滚，swiper 不动
    e.e.pointer_down(100.0, 150.0);
    e.frames(1);
    for i in 1..=8 {
        e.e.pointer_move(101.0, 150.0 - 15.0 * i as f32);
        e.frames(1);
    }
    e.e.pointer_up(101.0, 30.0);
    e.frames(30);
    assert!(e.e.scroll_position() > 50.0, "竖划滚页面：{}", e.e.scroll_position());
    assert!(!e.ev().contains("change"));
}

#[test]
fn 点一下照常是点击_拖过去就不是() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("tap", &swiper(""));
    let mut e = E::new(&fx);
    e.e.pointer_down(100.0, 100.0);
    e.frames(1);
    e.e.pointer_up(100.0, 100.0);
    e.frames(1);
    assert_eq!(e.ev(), "tap:0");
    e.clear();
    e.drag_x(100.0, 300.0, 80.0, 8);
    assert!(!e.ev().contains("tap"), "拖动不算点击：{}", e.ev());
}

#[test]
fn 衔接_自动播放_受控_current() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("auto", &swiper(r#"circular autoplay interval="1000" duration="300" current="{{cur}}""#));
    let mut e = E::new(&fx);
    // 第一页往右划（衔接）：到末页
    e.drag_x(100.0, 80.0, 300.0, 8);
    e.frames(30);
    assert!(e.ev().contains("change:2:number:touch"), "{}", e.ev());
    assert_eq!(e.px(100, 100), (0, 0, 255));
    // 自动播放：1 秒后从末页往前到第一页（不倒着滑）
    e.clear();
    e.frames(66);
    assert!(e.ev().contains("change:0:number:autoplay"), "{}", e.ev());
    e.frames(30);
    assert_eq!(e.px(100, 100), (255, 0, 0));
    // 页面改 current：滑过去，change 的 source 是空串
    e.clear();
    e.e.eval("__currentPage.setData({ cur: 1 })").unwrap();
    e.frames(30);
    assert!(e.ev().contains("change:1:number:"), "{}", e.ev());
    assert_eq!(e.px(100, 100), (0, 255, 0));
}

/// 页面运行时改 `duration`：下一次翻页就按新时长走。`duration` 只是**滑动动画时长**，
/// 没写 `autoplay` 的 swiper 不会自己翻页（与微信一致；自动切换的间隔是 `interval`）。
#[test]
fn 运行时改_duration_下一次翻页按新时长_没有_autoplay_不自动翻() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("duration", &swiper(r#"current="{{cur}}" duration="{{dur}}""#));
    let mut e = E::new(&fx);
    e.e.eval("__currentPage.setData({ dur: 500 })").unwrap();
    e.frames(2);
    // 500ms：35 帧（560ms）内走完
    e.clear();
    e.e.eval("__currentPage.setData({ cur: 1 })").unwrap();
    e.frames(35);
    assert!(e.ev().contains("finish:1"), "500ms 应已走完：{}", e.ev());
    // 改成 1200ms 再翻：同样 35 帧还在半路，再 45 帧（共 ~1.3s）才走完
    e.e.eval("__currentPage.setData({ dur: 1200 })").unwrap();
    e.frames(2);
    e.clear();
    e.e.eval("__currentPage.setData({ cur: 2 })").unwrap();
    e.frames(35);
    assert!(e.ev().contains("change:2:number:"), "{}", e.ev());
    assert!(!e.ev().contains("finish:2"), "1200ms 不该 560ms 就走完：{}", e.ev());
    e.frames(45);
    assert!(e.ev().contains("finish:2"), "1200ms 应已走完：{}", e.ev());
    assert_eq!(e.px(100, 100), (0, 0, 255));
    // 没有 autoplay：等 8 秒也不翻
    e.clear();
    e.frames(500);
    assert!(e.ev().is_empty(), "没写 autoplay 不该自己翻页：{}", e.ev());
}

/// 运行时打开 `autoplay`：从打开那一刻起算 `interval`，不是一打开就立刻翻
/// （微信的 autoplay 观察者是重新排一次定时器）
#[test]
fn 运行时打开_autoplay_从打开时起算_interval() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("autoplay-on", &swiper(r#"autoplay="{{other === 1}}" interval="1000" duration="300""#));
    let mut e = E::new(&fx);
    e.frames(200);
    assert!(e.ev().is_empty(), "没开 autoplay：{}", e.ev());
    e.e.eval("__currentPage.setData({ other: 1 })").unwrap();
    e.frames(30);
    assert!(!e.ev().contains("autoplay"), "刚打开 480ms 不该翻：{}", e.ev());
    e.frames(40);
    assert!(e.ev().contains("change:1:number:autoplay"), "打开后 ~1.1s 应翻一页：{}", e.ev());
}
