//! 新示例（bank / restaurant / chat）撞到的指针层问题，走 SDK 的 `MiniEngine`：
//! - 竖向 scroll-view 被 `scroll-into-view` / 手指滚过之后，里面的 tap 全部失效；
//! - 长按触发后抬手仍派 tap；长按里弹 `showActionSheet`，抬手把菜单当遮罩点击关掉；
//! - 遮罩里套 `catchtap` 面板，点面板上的选项之后遮罩又收到一次 tap。
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
    let data = fx.0.join("_data");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/index/index")).expect("打开页面");
    let mut t = 1000;
    for _ in 0..3 {
        t += 16;
        e.pump(t);
    }
    (fx, e, t)
}
fn settle(e: &mut MiniEngine, t: &mut u64, frames: u32) {
    for _ in 0..frames {
        *t += 16;
        e.pump(*t);
    }
}
fn tap(e: &mut MiniEngine, t: &mut u64, x: f32, y: f32) {
    e.pointer_down(x, y);
    settle(e, t, 1);
    e.pointer_up(x, y);
    settle(e, t, 3);
}
fn data(e: &MiniEngine, expr: &str) -> String {
    e.eval(&format!("__currentPage.data.{expr}")).unwrap().trim_matches('"').to_string()
}
const LIST_WXML: &str = r#"<scroll-view id="sv" scroll-y style="height:400px" scroll-into-view="{{to}}" bindscroll="onScroll">
<view wx:for="{{items}}" wx:key="*this" id="it{{index}}" data-i="{{index}}" bindtap="onTap" style="height:80px">{{item}}</view>
</scroll-view>"#;
const LIST_JS: &str = r#"Page({ data: { to: '', items: Array.from({length: 30}, (_, i) => 'row' + i), hit: -1 },
  onTap(e) { this.setData({ hit: e.currentTarget.dataset.i }); }, onScroll(e) { this.__svTop = e.detail.scrollTop; } });"#;

/// `scroll-into-view` 定位到第 10 项之后，点视口顶上那一行要命中第 10 项
#[test]
fn scroll_into_view_之后点击仍命中() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(LIST_WXML, LIST_JS, "sample-bugs-siv");
    e.eval("__currentPage.setData({ to: 'it10' })").unwrap();
    settle(&mut e, &mut t, 60);
    tap(&mut e, &mut t, 100.0, 40.0);
    assert_eq!(data(&e, "hit"), "10", "scroll-into-view 到 it10 后点视口顶部");
}
/// 手指往上拖 400px 并等惯性停下，再点视口顶上一行：命中的是滚动后在那里的那一项
#[test]
fn 手指滚动之后点击仍命中() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(LIST_WXML, LIST_JS, "sample-bugs-drag");
    e.pointer_down(100.0, 380.0);
    settle(&mut e, &mut t, 1);
    for i in 1..=20 {
        e.pointer_move(100.0, 380.0 - i as f32 * 16.0);
        settle(&mut e, &mut t, 1);
    }
    // 停住再抬手，不带惯性：落点刚好 320px
    settle(&mut e, &mut t, 10);
    e.pointer_up(100.0, 60.0);
    settle(&mut e, &mut t, 120);
    tap(&mut e, &mut t, 100.0, 40.0);
    let hit: i32 = data(&e, "hit").parse().unwrap();
    // 滚动距离含惯性、不精确：按 scroll-view 自己报的 scrollTop 算视口 y=40 处是第几项
    let top: f32 = e.eval("__currentPage.__svTop || 0").unwrap().parse().unwrap_or(0.0);
    eprintln!("drag: hit={hit} scrollTop={top}");
    assert_eq!(hit, ((top + 40.0) / 80.0) as i32, "滚过 {top}px 后视口 y=40 处那一项");
}
/// 微信：绑了 longpress 且触发了，抬手不再派 tap
#[test]
fn 长按触发后不再派_tap() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(
        r#"<view style="height:100px" bindtap="onTap" bindlongpress="onLong">x</view>"#,
        r#"Page({ data: { taps: 0, longs: 0 }, onTap() { this.setData({ taps: this.data.taps + 1 }); },
            onLong() { this.setData({ longs: this.data.longs + 1 }); } });"#,
        "sample-bugs-longpress",
    );
    e.pointer_down(100.0, 50.0);
    settle(&mut e, &mut t, 60); // ~1s，超过 350ms 的长按阈值
    e.pointer_up(100.0, 50.0);
    settle(&mut e, &mut t, 3);
    assert_eq!(data(&e, "longs"), "1");
    assert_eq!(data(&e, "taps"), "0", "longpress 触发后抬手不该再派 tap");
}
/// 长按里弹的操作菜单：抬起那根手指不能把菜单当成点遮罩关掉
#[test]
fn 长按弹出的_action_sheet_抬手不关() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(
        r#"<view style="height:100px" bindlongpress="onLong">x</view>"#,
        r#"Page({ data: { r: '' }, onLong() { wx.showActionSheet({ itemList: ['a', 'b'],
            success: (x) => this.setData({ r: 'ok' + x.tapIndex }), fail: (x) => this.setData({ r: 'fail' }) }); } });"#,
        "sample-bugs-longpress-sheet",
    );
    e.pointer_down(100.0, 50.0);
    settle(&mut e, &mut t, 60);
    e.pointer_up(100.0, 50.0);
    settle(&mut e, &mut t, 30);
    assert_eq!(data(&e, "r"), "", "抬手不该让菜单 fail cancel");
    // 菜单入场动画按墙上时钟走（action_sheet::hit），等它真的展开
    std::thread::sleep(std::time::Duration::from_millis(400));
    // 菜单还在：再点第 2 项（底部取消 56 + 间隔 8，第 2 项在 667-64-56..667-64）
    tap(&mut e, &mut t, 187.0, 667.0 - 64.0 - 28.0);
    assert_eq!(data(&e, "r"), "ok1", "弹出之后按下的那一下照常选中");
}
/// 没绑 longpress 的元素：按久了松手照样是 tap（微信语义）
#[test]
fn 没绑_longpress_按久了照样_tap() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(
        r#"<view style="height:100px" bindtap="onTap">x</view>"#,
        r#"Page({ data: { taps: 0 }, onTap() { this.setData({ taps: this.data.taps + 1 }); } });"#,
        "sample-bugs-hold-tap",
    );
    e.pointer_down(100.0, 50.0);
    settle(&mut e, &mut t, 60);
    e.pointer_up(100.0, 50.0);
    settle(&mut e, &mut t, 3);
    assert_eq!(data(&e, "taps"), "1");
}
/// 遮罩 bindtap 关弹层，面板 catchtap 截断：点面板里的选项只改选项，不关弹层
#[test]
fn 面板_catchtap_里点选项不穿透到遮罩() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(
        r#"<view wx:if="{{show}}" style="position:fixed;left:0;top:0;right:0;bottom:0;background:rgba(0,0,0,.5)" bindtap="close">
  <view catchtap="noop" style="position:absolute;left:0;right:0;bottom:0;height:300px;background:#fff">
    <view wx:for="{{opts}}" wx:key="*this" data-i="{{index}}" bindtap="pick" style="height:60px">{{item}}</view>
  </view>
</view>"#,
        r#"Page({ data: { show: true, sel: -1, opts: ['小', '中', '大'] }, close() { this.setData({ show: false }); },
            noop() {}, pick(e) { this.setData({ sel: e.currentTarget.dataset.i }); } });"#,
        "sample-bugs-catch-panel",
    );
    // 面板从 y=367 起，第 2 项在 427~487
    tap(&mut e, &mut t, 100.0, 450.0);
    assert_eq!(data(&e, "sel"), "1");
    assert_eq!(data(&e, "show"), "true", "点面板里的选项不能把弹层关掉");
}
/// 覆盖层（position:fixed 抽屉）里的 scroll-view：划过之后点列表项仍命中（restaurant 购物车抽屉）
#[test]
fn fixed_抽屉里的_scroll_view_划过之后点击仍命中() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, mut e, mut t) = engine_with(
        r#"<view style="position:fixed;left:0;right:0;bottom:0;height:400px;background:#fff">
<scroll-view scroll-y style="height:400px" bindscroll="onScroll">
<view wx:for="{{items}}" wx:key="*this" data-i="{{index}}" bindtap="onTap" style="height:80px">{{item}}</view>
</scroll-view></view>"#,
        r#"Page({ data: { items: Array.from({length: 20}, (_, i) => 'row' + i), hit: -1 },
            onTap(e) { this.setData({ hit: e.currentTarget.dataset.i }); }, onScroll(e) { this.__svTop = e.detail.scrollTop; } });"#,
        "sample-bugs-fixed-sv",
    );
    // 抽屉从 y=267 起；先不划，点第 1 行
    tap(&mut e, &mut t, 100.0, 307.0);
    assert_eq!(data(&e, "hit"), "0");
    e.pointer_down(100.0, 600.0);
    settle(&mut e, &mut t, 1);
    for i in 1..=15 {
        e.pointer_move(100.0, 600.0 - i as f32 * 16.0);
        settle(&mut e, &mut t, 1);
    }
    settle(&mut e, &mut t, 10);
    e.pointer_up(100.0, 360.0);
    settle(&mut e, &mut t, 120);
    let top: f32 = e.eval("__currentPage.__svTop || 0").unwrap().parse().unwrap_or(0.0);
    assert!(top > 100.0, "要真的滚过：scrollTop={top}");
    tap(&mut e, &mut t, 100.0, 307.0);
    assert_eq!(data(&e, "hit"), (((top + 40.0) / 80.0) as i32).to_string(), "滚过 {top}px 后点抽屉第一行");
}
/// onLoad 里抛的异常：不再被宿主吞掉，走 `wx.onError`（并打到控制台）
#[test]
fn onload_异常走_on_error() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let (_fx, e, _t) = engine_with(
        r#"<view>x</view>"#,
        r#"wx.onError(function (m) { globalThis.__lastErr = String(m); });
Page({ data: {}, onLoad() { const q = undefined; q.id; } });"#,
        "sample-bugs-onload-throw",
    );
    let err = e.eval("globalThis.__lastErr || ''").unwrap();
    assert!(err.contains("Page.onLoad"), "wx.onError 要收到 onLoad 的异常：{err:?}");
}
/// 自定义 tabBar：`getTabBar()` 拿得到实例、setData 改得到画面、组件里 `getApp()` 与 `require` 可用
#[test]
fn 自定义_tab_bar_是活实例() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sample-bugs-custom-tabbar");
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/a/a","pages/b/b"],"window":{"navigationStyle":"custom"},
            "tabBar":{"custom":true,"list":[{"pagePath":"pages/a/a","text":"A"},{"pagePath":"pages/b/b","text":"B"}]}}"#),
        ("app.js", "App({ globalData: { who: 'app' } });"),
        ("app.wxss", ""),
        ("utils/n.js", "module.exports = { start: 3 };"),
        ("custom-tab-bar/index.json", r#"{"component":true}"#),
        ("custom-tab-bar/index.wxml", r#"<view style="height:50px;display:flex;flex-direction:row">
<view wx:for="{{list}}" wx:key="path" data-index="{{index}}" data-path="{{item.path}}" style="flex:1;background:{{badge > 4 ? '#ff0000' : '#ffffff'}}">{{item.text}}</view></view>"#),
        ("custom-tab-bar/index.wxss", ""),
        ("custom-tab-bar/index.js", r#"const n = require('../utils/n.js');
Component({ data: { selected: 0, badge: n.start, who: '', list: [{ path: 'pages/a/a', text: 'A' }, { path: 'pages/b/b', text: 'B' }] },
  lifetimes: { attached() { const a = getApp(); this.setData({ who: a ? a.globalData.who : 'null' }); } } });"#),
        ("pages/a/a.json", "{}"), ("pages/a/a.wxml", "<view>a</view>"), ("pages/a/a.wxss", ""),
        ("pages/a/a.js", "Page({ onShow() { const t = this.getTabBar && this.getTabBar(); globalThis.__tb = t ? 'ok' : 'null'; if (t) t.setData({ badge: 9 }); } });"),
        ("pages/b/b.json", "{}"), ("pages/b/b.wxml", "<view>b</view>"), ("pages/b/b.wxss", ""), ("pages/b/b.js", "Page({});"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let fx = Fixture(root);
    let data = fx.0.join("_data");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/a/a")).expect("打开页面");
    let mut t = 1000;
    settle(&mut e, &mut t, 4);
    assert_eq!(e.eval("globalThis.__tb").unwrap().trim_matches('"'), "ok", "getTabBar() 要拿到实例");
    assert_eq!(e.eval("__customTabBarInstance.data.who").unwrap().trim_matches('"'), "app", "attached 里 getApp() 可用");
    assert_eq!(e.eval("__customTabBarInstance.data.badge").unwrap(), "9");
    // 画面：badge > 4 时 tab 底色变红。看合成后缓冲里 tabBar 那一条（底部 50px）
    let (w, h) = e.pixel_size();
    let (w, h) = (w as usize, h as usize);
    let px = e.rgba();
    let row = &px[(h - 50) * w * 4..(h - 49) * w * 4];
    let red = row.chunks_exact(4).filter(|p| p[0] > 200 && p[1] < 60 && p[2] < 60).count();
    assert!(red > w / 2, "tabBar 要按实例的新数据重画（红色像素 {red}/{w}）");
}
