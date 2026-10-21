//! `<scroll-view>` 属性与事件、`wx.pageScrollTo`、SDK 的 `onPageScroll` 的端到端用例：
//! 现场写一个最小小程序，用 `MiniEngine`（SDK 那条链路：真指针 → 手势仲裁 → 控制器 → 事件）跑。
//!
//! 依据官方 component/scroll-view.html 与 api/ui/scroll/wx.pageScrollTo.html。
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

fn fixture(name: &str, wxml: &str, wxss: &str, js: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("scroll-view-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", wxss),
        ("pages/index/index.js", js),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    Fixture(root)
}

/// 引擎 + 一个单调递增的时钟（每次 pump 前进 16ms，动画与惯性按它推进，结果可复现）
struct E {
    e: MiniEngine,
    t: u64,
}

impl E {
    fn new(fx: &Fixture) -> Self {
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
    /// 从 (x, y0) 竖着拖到 (x, y1)，`steps` 步、每步一帧；松手后再走 `settle` 帧
    fn drag(&mut self, x: f32, y0: f32, y1: f32, steps: u32, settle: u32) {
        self.e.pointer_down(x, y0);
        self.frames(1);
        for i in 1..=steps {
            let y = y0 + (y1 - y0) * i as f32 / steps as f32;
            self.e.pointer_move(x, y);
            self.frames(1);
        }
        self.e.pointer_up(x, y1);
        self.frames(settle);
    }
    fn js(&self, code: &str) -> String {
        self.e.eval(code).unwrap_or_default().trim_matches('"').to_string()
    }
    fn set(&mut self, data: &str) {
        self.e.eval(&format!("__currentPage.setData({data})")).unwrap();
        self.frames(1);
    }
    fn sv(&self) -> f32 {
        self.e.scroll_view_offset("sv").expect("scroll-view 应有滚动控制器")
    }
}

const LIST_WXSS: &str = ".sv{height:300px}.item{height:100px}";

fn list_wxml(attrs: &str) -> String {
    let items: String = (0..20).map(|i| format!(r#"<view class="item" id="item{i}">{i}</view>"#)).collect();
    format!(r#"<scroll-view id="sv" class="sv" scroll-y {attrs}>{items}</scroll-view><view style="height:1200px"></view>"#)
}

const LOG_JS: &str = r#"globalThis.__ev = [];
Page({
    data: { top: 0, toView: '', anim: false, triggered: false },
    onScroll: function (e) { __ev.push('scroll:' + typeof e.detail.scrollTop + ':' + e.detail.isDrag + ':' + (e.detail.deltaY < 0)); this.lastTop = e.detail.scrollTop; this.h = e.detail.scrollHeight; },
    onLower: function (e) { __ev.push('lower:' + e.detail.direction); },
    onUpper: function (e) { __ev.push('upper:' + e.detail.direction); },
    onStart: function (e) { __ev.push('start:' + e.detail.isDrag); },
    onEnd: function (e) { __ev.push('end:' + e.detail.isDrag); },
    onPulling: function (e) { __ev.push('pulling'); this.dy = e.detail.dy; },
    onWill: function () { __ev.push('will'); },
    onRefresh: function () { __ev.push('refresh'); },
    onRestore: function () { __ev.push('restore'); },
    onAbort: function () { __ev.push('abort'); },
    onPageScroll: function (e) { this.pageTop = e.scrollTop; }
});"#;

fn count(e: &E, prefix: &str) -> usize {
    e.js(&format!("__ev.filter(function (x) {{ return x.indexOf('{prefix}') === 0; }}).length")).parse().unwrap_or(0)
}

#[test]
fn 拖动派发_scroll_与触底_一次且带类型() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture(
        "drag",
        &list_wxml(r#"scroll-top="{{top}}" lower-threshold="50" bindscroll="onScroll" bindscrolltolower="onLower" bindscrolltoupper="onUpper" bindscrollstart="onStart" bindscrollend="onEnd""#),
        LIST_WXSS,
        LOG_JS,
    );
    let mut e = E::new(&fx);
    assert_eq!(e.js("__ev.length"), "0", "刚打开不该有事件（停在顶部不算进入 upper 区）");
    e.drag(100.0, 280.0, 80.0, 10, 2);
    assert!(e.sv() > 150.0, "scroll-view 应被拖动：{}", e.sv());
    assert!(count(&e, "scroll:number:true:true") > 0, "拖动中的 scroll：scrollTop 是数字、isDrag 为真、往下滚 deltaY 为负：{}", e.js("__ev.join(',')"));
    assert_eq!(e.js("__ev[0]"), "start:true", "第一条是 scrollstart，isDrag 为真");
    assert_eq!(e.js("__currentPage.h"), "2000", "scrollHeight 是内容高");
    // 一路拖到底：scrolltolower 只发一次
    for _ in 0..6 {
        e.drag(100.0, 280.0, 20.0, 8, 2);
    }
    e.frames(90);
    assert!((e.sv() - 1700.0).abs() < 1.0, "应停在底部（2000 - 300）：{}", e.sv());
    assert_eq!(count(&e, "lower:bottom"), 1, "进入底部阈值区只发一次：{}", e.js("__ev.join(',')"));
    assert!(count(&e, "end:") >= 1, "滚动停下有 scrollend");
    // 拖到底后继续推，手势交给了页面，页面滚走了 —— 先把页面拉回来，scroll-view 才在绘制范围里
    // （受控属性在 scroll-view 被画到时生效，见 doc/引擎测试说明.md T14）
    e.e.scroll_to(0.0);
    e.frames(2);
    // 回到顶部：scrolltoupper 一次
    e.set("{ top: 0 }");
    e.set("{ top: 1 }");
    e.set("{ top: 0 }");
    assert!(e.sv() < 1.0, "{}", e.sv());
    assert_eq!(count(&e, "upper:top"), 1);
}

#[test]
fn scroll_top_首帧就落位_改值才动_拖动后不被拽回() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture(
        "top",
        &list_wxml(r#"scroll-top="{{top}}" bindscroll="onScroll""#),
        LIST_WXSS,
        &LOG_JS.replace("top: 0,", "top: 250,"),
    );
    let mut e = E::new(&fx);
    assert!((e.sv() - 250.0).abs() < 0.5, "页面打开就停在 scroll-top：{}", e.sv());
    e.drag(100.0, 280.0, 180.0, 5, 60);
    let after_drag = e.sv();
    assert!(after_drag > 300.0, "拖动生效：{after_drag}");
    e.set("{ other: 1 }");
    assert!((e.sv() - after_drag).abs() < 0.5, "无关 setData 不该把位置拽回 scroll-top");
    e.set("{ top: 120 }");
    assert!((e.sv() - 120.0).abs() < 0.5, "改了 scroll-top 才动（没开动画：直接到）：{}", e.sv());
    // rpx 单位：375 宽的屏 600rpx = 300px
    e.set("{ top: '600rpx' }");
    assert!((e.sv() - 300.0).abs() < 0.5, "rpx：{}", e.sv());
}

#[test]
fn scroll_into_view_优先_带动画时滑过去() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture(
        "into",
        &list_wxml(r#"scroll-top="{{top}}" scroll-into-view="{{toView}}" scroll-with-animation="{{anim}}""#),
        LIST_WXSS,
        LOG_JS,
    );
    let mut e = E::new(&fx);
    e.set("{ toView: 'item5', top: 900 }");
    assert!((e.sv() - 500.0).abs() < 0.5, "scroll-into-view 优先于 scroll-top：{}", e.sv());
    e.set("{ anim: true }");
    e.set("{ toView: 'item12' }");
    // 数据到达的那一帧开始动画，下一帧才推进
    e.frames(2);
    let mid = e.sv();
    assert!(mid > 500.0 && mid < 1200.0, "开了 scroll-with-animation：第一帧在半路：{mid}");
    e.frames(30);
    assert!((e.sv() - 1200.0).abs() < 0.5, "300ms 后到位：{}", e.sv());
    e.set("{ toView: 'nope' }");
    assert!((e.sv() - 1200.0).abs() < 0.5, "找不到的 id 不动");
}

#[test]
fn 没写_scroll_y_不能滚_手势交给页面() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let items: String = (0..20).map(|i| format!(r#"<view class="item">{i}</view>"#)).collect();
    let wxml = format!(r#"<scroll-view id="sv" class="sv">{items}</scroll-view><view style="height:1200px"></view>"#);
    let fx = fixture("noaxis", &wxml, LIST_WXSS, LOG_JS);
    let mut e = E::new(&fx);
    assert!(e.e.scroll_view_offset("sv").is_none(), "没有 scroll-x / scroll-y：没有滚动控制器");
    e.drag(100.0, 280.0, 80.0, 10, 60);
    assert!(e.e.scroll_position() > 100.0, "手势交给了页面：页面位置 {}", e.e.scroll_position());
}

#[test]
fn 自定义下拉刷新_拉过阈值触发_设回_false_复位() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture(
        "refresher",
        &list_wxml(r##"refresher-enabled refresher-threshold="45" refresher-background="#ff0000" refresher-triggered="{{triggered}}" bindrefresherpulling="onPulling" bindrefresherwillrefresh="onWill" bindrefresherrefresh="onRefresh" bindrefresherrestore="onRestore" bindrefresherabort="onAbort""##),
        LIST_WXSS,
        LOG_JS,
    );
    let mut e = E::new(&fx);
    // 拉一小段就松手：中止
    e.drag(100.0, 100.0, 140.0, 4, 60);
    assert!(count(&e, "pulling") > 0, "下拉中有 refresherpulling：{}", e.js("__ev.join(',')"));
    assert_eq!(count(&e, "abort"), 1);
    assert_eq!(count(&e, "refresh"), 0);
    assert!(e.sv().abs() < 0.5, "没触发：弹回原位");
    // 拉过阈值（橡皮筋衰减：手指走 300px 内容走得少，但足够过 45）
    e.e.eval("__ev = []").unwrap();
    e.drag(100.0, 60.0, 400.0, 12, 1);
    assert_eq!(count(&e, "will"), 1, "过阈值那一刻发一次 willrefresh：{}", e.js("__ev.join(',')"));
    assert_eq!(count(&e, "refresh"), 1);
    e.frames(60);
    assert!((e.sv() + 45.0).abs() < 1.0, "刷新中指示器停在阈值处：{}", e.sv());
    // 刷新区背景画出来了（红）：scroll-view 顶部那一条
    let (w, _) = e.e.pixel_size();
    let px = e.e.rgba();
    let i = ((20 * 2) * w as usize + 100 * 2) * 4;
    assert!(px[i] > 200 && px[i + 1] < 60 && px[i + 2] < 60, "刷新区应铺 refresher-background：{:?}", &px[i..i + 4]);
    // 无关 setData 不复位
    e.set("{ other: 1 }");
    e.frames(5);
    assert!((e.sv() + 45.0).abs() < 1.0, "无关 setData 不该收起刷新");
    // triggered 一直是 false，再 setData 一次 false：复位（微信的属性设置语义）
    e.set("{ triggered: false }");
    e.frames(60);
    assert_eq!(count(&e, "restore"), 1, "{}", e.js("__ev.join(',')"));
    assert!(e.sv().abs() < 0.5, "复位后回到顶部：{}", e.sv());
    // 页面主动 true → 出现，false → 复位
    e.set("{ triggered: true }");
    e.frames(60);
    assert!((e.sv() + 45.0).abs() < 1.0, "triggered=true 时指示器出现：{}", e.sv());
    e.set("{ triggered: false }");
    e.frames(60);
    assert_eq!(count(&e, "restore"), 2);
    assert!(e.sv().abs() < 0.5);
}

#[test]
fn page_scroll_to_与_sdk_的_on_page_scroll() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let wxml = r#"<view style="height:900px"></view><view id="target" style="height:50px">T</view><view style="height:1500px"></view>"#;
    let fx = fixture("pagescroll", wxml, "", LOG_JS);
    let mut e = E::new(&fx);
    e.e.eval("wx.pageScrollTo({ scrollTop: 300, duration: 0 })").unwrap();
    e.frames(2);
    assert!((e.e.scroll_position() - 300.0).abs() < 0.5, "duration 0：直接到：{}", e.e.scroll_position());
    assert_eq!(e.js("__currentPage.pageTop"), "300", "SDK 也派发 onPageScroll");
    e.e.eval("wx.pageScrollTo({ selector: '#target', offsetTop: -20, duration: 300 })").unwrap();
    e.frames(2);
    let mid = e.e.scroll_position();
    assert!(mid > 300.0 && mid < 880.0, "带 duration：半路 {mid}");
    e.frames(30);
    assert!((e.e.scroll_position() - 880.0).abs() < 0.5, "selector 顶边 900 + offsetTop -20：{}", e.e.scroll_position());
    let r = e.js("(function(){ var r = 'none'; wx.pageScrollTo({ fail: function (x) { r = x.errMsg; } }); return r; })()");
    assert!(r.contains("fail"), "两个都没给：fail（{r}）");
}
