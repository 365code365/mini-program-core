//! 横向 scroll-view 与自定义 tabBar 的回归（2026-10-25，示例小程序里大量出现、真机上点出来的两个坑）：
//! - 顶层 `scroll-x` 的 scroll-view 横着一滑整排变空白（离屏缓存只会按纵向偏移贴，内容被贴出了视口）；
//! - 可滚范围不含结尾的内边距 / 外边距：最后一个标签滚到头也被切掉一截；
//! - 自定义 tabBar 只写 `data-index` + `wx.switchTab` 时点了没反应。
//! 走 `MiniEngine`（SDK 链路）。独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

static SERIAL: Mutex<()> = Mutex::new(());

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn fixture(name: &str, files: &[(&str, &str)]) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hscroll-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    Fixture(root)
}

fn engine(fx: &Fixture, route: &str) -> (MiniEngine, u64) {
    let data = fx.0.join("_data");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some(route)).expect("打开页面");
    let mut t = 1000;
    for _ in 0..3 {
        t += 16;
        e.pump(t);
    }
    (e, t)
}

fn px(e: &mut MiniEngine, x: f32, y: f32) -> (u8, u8, u8) {
    let (w, _) = e.pixel_size();
    let p = e.rgba();
    let i = ((y * 2.0) as usize * w as usize + (x * 2.0) as usize) * 4;
    (p[i], p[i + 1], p[i + 2])
}

const PAGE_WXML: &str = r#"<scroll-view scroll-x style="white-space:nowrap;height:60px;padding:0 20px;box-sizing:border-box">
  <view style="display:inline-block;width:150px;height:60px;margin-right:10px;background:#ff0000"></view>
  <view style="display:inline-block;width:150px;height:60px;margin-right:10px;background:#00ff00"></view>
  <view style="display:inline-block;width:150px;height:60px;margin-right:10px;background:#0000ff"></view>
</scroll-view>"#;

#[test]
fn 横向_scroll_view_滑动后照常画出_且能滚到结尾的内边距() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture(
        "drag",
        &[
            ("app.json", r#"{"pages":["pages/a/a"],"window":{"navigationStyle":"custom"}}"#),
            ("app.js", "App({});"),
            ("app.wxss", ""),
            ("pages/a/a.json", "{}"),
            ("pages/a/a.wxml", PAGE_WXML),
            ("pages/a/a.wxss", ""),
            ("pages/a/a.js", "Page({});"),
        ],
    );
    let (mut e, mut t) = engine(&fx, "pages/a/a");
    assert_eq!(px(&mut e, 30.0, 30.0), (255, 0, 0), "起始：第一块红色");
    // 往左甩到头（内容 20 + 3×160 + 20 = 520，视口 375：最多滚 145）
    e.pointer_down(350.0, 30.0);
    for i in 1..=12 {
        std::thread::sleep(Duration::from_millis(16));
        e.pointer_move(350.0 - 25.0 * i as f32, 30.0);
        t += 16;
        e.pump(t);
    }
    e.pointer_up(50.0, 30.0);
    for _ in 0..80 {
        std::thread::sleep(Duration::from_millis(16));
        t += 16;
        e.pump(t);
    }
    // 滚到头：蓝色块右边（520-145-20-10 = 345）之后是 10px 外边距 + 20px 内边距的空白，蓝块整块露出来
    assert_eq!(px(&mut e, 340.0, 30.0), (0, 0, 255), "滑动后蓝块要画出来（从前整排空白）");
    assert_ne!(px(&mut e, 365.0, 30.0), (0, 0, 255), "结尾的外边距 + 内边距也滚得出来");
}

#[test]
fn 自定义_tab_bar_只写_data_index_也能切页() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let page = |n: &str| format!("Page({{ onShow() {{ getApp().shown = '{n}'; }} }});");
    let pa = page("a");
    let pb = page("b");
    let fx = fixture(
        "tabbar",
        &[
            (
                "app.json",
                r#"{"pages":["pages/a/a","pages/b/b"],"window":{"navigationStyle":"custom"},
                "tabBar":{"custom":true,"list":[{"pagePath":"pages/a/a","text":"A"},{"pagePath":"pages/b/b","text":"B"}]}}"#,
            ),
            ("app.js", "App({});"),
            ("app.wxss", ""),
            ("custom-tab-bar/index.json", r#"{"component":true}"#),
            (
                "custom-tab-bar/index.wxml",
                r#"<view style="display:flex;flex-direction:row;height:50px;background:#fff">
  <view wx:for="{{list}}" wx:key="t" style="flex:1;height:50px" data-index="{{index}}" bindtap="go">{{item.t}}</view>
</view>"#,
            ),
            ("custom-tab-bar/index.wxss", ""),
            (
                "custom-tab-bar/index.js",
                "Component({ data: { list: [{ t: 'A', p: '/pages/a/a' }, { t: 'B', p: '/pages/b/b' }] }, methods: { go(e) { wx.switchTab({ url: this.data.list[e.currentTarget.dataset.index].p }); } } });",
            ),
            ("pages/a/a.json", "{}"),
            ("pages/a/a.wxml", "<view>A</view>"),
            ("pages/a/a.wxss", ""),
            ("pages/a/a.js", pa.as_str()),
            ("pages/b/b.json", "{}"),
            ("pages/b/b.wxml", "<view>B</view>"),
            ("pages/b/b.wxss", ""),
            ("pages/b/b.js", pb.as_str()),
        ],
    );
    let (mut e, mut t) = engine(&fx, "pages/a/a");
    e.pointer_down(280.0, 640.0);
    t += 16;
    e.pump(t);
    e.pointer_up(280.0, 640.0);
    for _ in 0..4 {
        t += 16;
        e.pump(t);
    }
    assert_eq!(e.current_route(), "pages/b/b", "点右半边的「B」应切到 b 页");
}
