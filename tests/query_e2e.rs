//! SelectorQuery、ScrollViewContext、IntersectionObserver、nextTick、`<canvas type="2d">`（G-9 / G-10 的这几项）。
//!
//! 依据官方 api/wxml/SelectorQuery.html、NodesRef.html、IntersectionObserver.html、ScrollViewContext.html、
//! api/ui/custom-component/wx.nextTick.html、api/canvas/Canvas.html。
//! 走 `MiniEngine`（SDK 链路），独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use serde_json::Value;
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
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("query-e2e").join(name);
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
    fn run(&mut self, code: &str) {
        self.e.eval(code).unwrap();
        self.frames(3);
    }
    fn js(&self, code: &str) -> Value {
        let s = self.e.eval(&format!("JSON.stringify({code})")).unwrap_or_default();
        let s: String = serde_json::from_str(&s).unwrap_or(s);
        serde_json::from_str(&s).unwrap_or(Value::Null)
    }
    fn out(&self) -> Value {
        self.js("__currentPage.data.out")
    }
}

const PAGE_WXSS: &str = "page{background:#ffffff}.box{height:100px;margin-top:50px}.item{height:40px}.tall{height:2000px}";

#[test]
fn bounding_rect_scroll_offset_fields_and_query_in_onload() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let wxml = r#"<view id="a" class="box" data-k="v">A</view>
<view class="item" data-i="0">0</view><view class="item" data-i="1">1</view><view class="item" data-i="2">2</view>
<image id="img" src="/nope.png" style="width:30px;height:20px"></image>
<view class="tall"></view>"#;
    // onLoad 里就查（页面还没布局）：应留到布局之后再答，而不是回一串 null
    let js = r#"Page({ data: { out: null, early: null },
  onLoad() { wx.createSelectorQuery().select('#a').boundingClientRect().exec(r => { this.data.early = r; }); },
  q() { const q = this.createSelectorQuery();
    q.select('#a').boundingClientRect();
    q.selectAll('.item').fields({ dataset: true, size: true });
    q.selectViewport().scrollOffset();
    q.select('#img').fields({ properties: ['src'], computedStyle: ['width'] });
    q.select('#none').boundingClientRect();
    q.exec(res => { this.data.out = res; }); } });"#;
    let fx = fixture("rect", wxml, PAGE_WXSS, js);
    let mut e = E::new(&fx);
    let early = e.js("__currentPage.data.early");
    assert_eq!(early[0]["top"], 50.0, "onLoad 里发起的查询在布局后答：{early}");

    e.run("__currentPage.q()");
    let r = e.out();
    assert_eq!(r[0]["id"], "a");
    assert_eq!(r[0]["dataset"]["k"], "v");
    assert_eq!((r[0]["left"].as_f64(), r[0]["top"].as_f64(), r[0]["width"].as_f64(), r[0]["height"].as_f64()), (Some(0.0), Some(50.0), Some(375.0), Some(100.0)));
    assert_eq!(r[0]["bottom"], 150.0);
    let items = r[1].as_array().expect("selectAll 给数组");
    assert_eq!(items.len(), 3);
    assert_eq!(items[2]["dataset"]["i"], "2");
    assert_eq!(items[2]["height"], 40.0);
    assert_eq!(r[2]["scrollTop"], 0.0);
    assert!(r[2]["scrollHeight"].as_f64().unwrap() > 2000.0, "页面内容高：{}", r[2]);
    assert_eq!(r[3]["src"], "/nope.png");
    assert_eq!(r[3]["width"], "30px");
    assert!(r[4].is_null(), "找不到给 null");

    // 页面滚下去 100：boundingClientRect 是视口坐标，跟着上移
    e.run("wx.pageScrollTo({ scrollTop: 100, duration: 0 })");
    e.run("__currentPage.q()");
    let r = e.out();
    assert_eq!(r[0]["top"], -50.0);
    assert_eq!(r[2]["scrollTop"], 100.0);
}

#[test]
fn scroll_view_context_scroll_to_and_into_view() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let items: String = (0..20).map(|i| format!(r#"<view class="item" id="it{i}">{i}</view>"#)).collect();
    let wxml = format!(r#"<scroll-view id="sv" scroll-y enhanced style="height:200px">{items}</scroll-view>"#);
    let js = r#"Page({ data: { out: null },
  node(f) { this.createSelectorQuery().select('#sv').fields({ node: true, scrollOffset: true }).exec(r => { this.data.out = r[0].scrollTop; f(r[0].node); }); } });"#;
    let fx = fixture("sv", &wxml, PAGE_WXSS, js);
    let mut e = E::new(&fx);
    e.run("__currentPage.node(n => n.scrollTo({ top: 120, animated: false }))");
    e.frames(2);
    assert_eq!(e.e.scroll_view_offset("sv"), Some(120.0));
    e.run("__currentPage.node(n => n.scrollIntoView('#it10', { animated: false }))");
    e.frames(2);
    assert_eq!(e.e.scroll_view_offset("sv"), Some(400.0), "第 10 项（每项 40px）顶到 scroll-view 顶边");
    e.run("__currentPage.node(n => {})");
    assert_eq!(e.out(), 400.0, "fields 的 scrollOffset 读到当前位置");
}

#[test]
fn intersection_observer_fires_on_threshold_crossing() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let wxml = r#"<view id="top" style="height:100px">top</view><view style="height:900px"></view><view id="far" style="height:100px">far</view><view style="height:1000px"></view>"#;
    let js = r#"Page({ data: { ev: [] },
  onReady() {
    this.io = this.createIntersectionObserver({ thresholds: [0, 1] });
    this.io.relativeToViewport().observe('#far', r => this.data.ev.push(['far', Math.round(r.intersectionRatio * 100), r.id]));
    wx.createIntersectionObserver(this).relativeToViewport({ bottom: 0 }).observe('#top', r => this.data.ev.push(['top', Math.round(r.intersectionRatio * 100), r.boundingClientRect.top]));
  } });"#;
    let fx = fixture("io", wxml, PAGE_WXSS, js);
    let mut e = E::new(&fx);
    e.frames(2);
    // 一开始就在视口里的 #top 立刻回调一次（initialRatio 默认 0）；#far 在视口外，不回调
    assert_eq!(e.js("__currentPage.data.ev"), serde_json::json!([["top", 100, 0]]));

    // 滚到 #far 露出一半（视口高 667，#far 在 1000~1100）：跨过阈值 0
    e.run("wx.pageScrollTo({ scrollTop: 383, duration: 0 })");
    let ev = e.js("__currentPage.data.ev");
    assert_eq!(ev[1], serde_json::json!(["far", 50, "far"]), "{ev}");
    // 再往下，整个露出来：跨过阈值 1
    e.run("wx.pageScrollTo({ scrollTop: 600, duration: 0 })");
    let ev = e.js("__currentPage.data.ev");
    assert!(ev.as_array().unwrap().iter().any(|x| x[0] == "far" && x[1] == 100), "{ev}");
    assert!(ev.as_array().unwrap().iter().any(|x| x[0] == "top" && x[1] == 0), "#top 离开视口：{ev}");

    // disconnect 之后不再回调
    e.run("__currentPage.io.disconnect(); __currentPage.data.ev = []");
    e.run("wx.pageScrollTo({ scrollTop: 0, duration: 0 })");
    let ev = e.js("__currentPage.data.ev");
    assert!(ev.as_array().unwrap().iter().all(|x| x[0] != "far"), "断开后 #far 不再回调：{ev}");
}

#[test]
fn next_tick_runs_after_current_sync_code() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let js = r#"Page({ data: { order: [] }, onLoad() { wx.nextTick(() => this.data.order.push('tick')); this.data.order.push('sync'); } });"#;
    let fx = fixture("tick", "<view>x</view>", "", js);
    let e = E::new(&fx);
    assert_eq!(e.js("__currentPage.data.order"), serde_json::json!(["sync", "tick"]));
}

#[test]
fn canvas_2d_node_draws_in_backing_pixels() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let wxml = r#"<canvas type="2d" id="c" style="width:100px;height:100px"></canvas>"#;
    let js = r#"Page({ data: { info: null },
  onReady() {
    this.createSelectorQuery().select('#c').fields({ node: true, size: true }).exec(res => {
      const canvas = res[0].node, ctx = canvas.getContext('2d');
      canvas.width = res[0].width * 2; canvas.height = res[0].height * 2; ctx.scale(2, 2);
      ctx.fillStyle = '#ff0000'; ctx.fillRect(0, 0, 50, 50);
      this.data.info = { w: canvas.width, fill: ctx.fillStyle, size: [res[0].width, res[0].height] };
    });
  } });"#;
    let fx = fixture("c2d", wxml, "page{background:#ffffff}", js);
    let mut e = E::new(&fx);
    e.frames(4);
    assert_eq!(e.js("__currentPage.data.info"), serde_json::json!({ "w": 200, "fill": "#ff0000", "size": [100, 100] }));
    let (w, _) = e.e.pixel_size();
    let px = |e: &mut E, x: usize, y: usize| {
        let p = e.e.rgba();
        let i = (y * 2 * w as usize + x * 2) * 4;
        (p[i], p[i + 1], p[i + 2])
    };
    assert_eq!(px(&mut e, 25, 25), (255, 0, 0), "左上 50×50（逻辑像素）是红的");
    assert_eq!(px(&mut e, 75, 75), (255, 255, 255), "type=2d 的画布默认透明，露出页面白底");

    // 再画一笔（另一个微任务）：前面画的还在（追加语义），不是被下一批替换
    e.run("{ const n = __c2dNodes['2d#c']; const ctx = n.getContext('2d'); ctx.fillStyle = '#0000ff'; ctx.fillRect(50, 50, 50, 50); }");
    e.frames(2);
    assert_eq!(px(&mut e, 25, 25), (255, 0, 0));
    assert_eq!(px(&mut e, 75, 75), (0, 0, 255));
}
