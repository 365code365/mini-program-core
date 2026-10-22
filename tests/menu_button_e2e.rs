//! 右上角胶囊按钮（`menuButton` 选项）的端到端用例：画在哪、深浅两套配色、
//! 点左右两半发什么、点它不漏到下面的页面、按住滑出去不算点、改前景色跟着换配色。
//!
//! 判据都是可判定的：位置 = `wx.getMenuButtonBoundingClientRect()` 返回的矩形
//! （默认 375 宽、状态栏 20 时是 281,24 87×32）；配色按像素算（半透明白底 / 黑底叠在页面颜色上）。
//! 点击走 `pointer_down` / `pointer_up` 真实指针链路，不调任何「直接点击」的捷径。
//!
//! 夹具写在 `CARGO_TARGET_TMPDIR` 下，串行跑（小程序根目录是进程级的，见 embed_host.rs）。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

/// 页面底色（像素判据都拿它当参照）
const GREEN: (u8, u8, u8) = (0x07, 0xc1, 0x60);
const WANTS_MORE: u32 = 16;

/// 一页 custom 导航栏的小程序：整页绿底，顶上一条 fixed 的「导航栏」带 bindtap（数点了几下）
fn fixture(name: &str, page_json: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("menu-button").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/p/p"],"window":{"navigationBarTitleText":"夹具"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", r#"<view class="bar" bindtap="onBar">导航栏</view><view class="body">{{hits}}</view>"#),
        (
            "pages/p/p.wxss",
            "page{background-color:#07c160;} .bar{position:fixed;top:0;left:0;width:750rpx;height:88px;background-color:#07c160;} .body{height:2000px;}",
        ),
        ("pages/p/p.js", "Page({data:{hits:0}, onBar(){ this.setData({hits: this.data.hits + 1}); }});"),
        ("pages/p/p.json", page_json),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, menu_button: bool) -> MiniEngine {
    let data = root.join("_data");
    let opts = EngineOptions { menu_button, ..Default::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0, HostEnv::default(), opts)
        .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e.pump(1);
    e
}

fn events(e: &mut MiniEngine) -> Vec<Value> {
    std::iter::from_fn(|| e.poll_event()).map(|s| serde_json::from_str(&s).unwrap()).collect()
}

fn menu_actions(e: &mut MiniEngine) -> Vec<String> {
    events(e)
        .into_iter()
        .filter(|v| v["type"] == "menuButton")
        .map(|v| v["action"].as_str().unwrap_or("").to_string())
        .collect()
}

/// 逻辑坐标处的像素（dpr 2）
fn px(e: &mut MiniEngine, x: f32, y: f32) -> (u8, u8, u8) {
    let (w, _) = e.pixel_size();
    let i = (((y * 2.0) as u32 * w + (x * 2.0) as u32) * 4) as usize;
    let p = e.rgba();
    (p[i], p[i + 1], p[i + 2])
}

/// 一次完整的点击（按下、出一帧、抬起、出一帧）
fn tap(e: &mut MiniEngine, x: f32, y: f32, t: &mut u64) {
    e.pointer_down(x, y);
    *t += 16;
    e.pump(*t);
    e.pointer_up(x, y);
    *t += 16;
    e.pump(*t);
}

fn hits(e: &MiniEngine) -> String {
    e.eval("String(getCurrentPages()[0].data.hits)").unwrap_or_default()
}

/// 胶囊里只有底色的一点（左半上沿附近，避开图标与描边）
const BG_AT: (f32, f32) = (300.0, 30.0);
/// 左半中间那个点的圆心
const DOT_AT: (f32, f32) = (281.0 + 87.0 / 4.0, 40.0);

#[test]
fn custom_页面画在_js_返回的位置_浅色是半透明白底黑点() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("light", r#"{"navigationStyle":"custom","navigationBarTextStyle":"black"}"#);
    let mut e = engine(&root, true);
    let r = e.menu_button_rect().expect("custom 页面要画胶囊");
    assert_eq!((r.x, r.y, r.width, r.height), (281.0, 24.0, 87.0, 32.0));
    let js: Value = serde_json::from_str(&e.eval("JSON.stringify(wx.getMenuButtonBoundingClientRect())").unwrap()).unwrap();
    assert_eq!(
        (js["left"].as_f64(), js["top"].as_f64(), js["width"].as_f64(), js["height"].as_f64()),
        (Some(r.x as f64), Some(r.y as f64), Some(r.width as f64), Some(r.height as f64)),
        "画的位置必须就是接口返回的矩形（开发者拿它对齐自定义导航栏）"
    );
    // rgba(255,255,255,.6) 叠在 #07c160 上
    let bg = px(&mut e, BG_AT.0, BG_AT.1);
    let want = |c: u8| (255.0 * 0.6 + c as f32 * 0.4) as i32;
    for (got, base) in [(bg.0, GREEN.0), (bg.1, GREEN.1), (bg.2, GREEN.2)] {
        assert!((got as i32 - want(base)).abs() <= 2, "底色 {bg:?}");
    }
    assert_eq!(px(&mut e, DOT_AT.0, DOT_AT.1), (0, 0, 0), "图标是黑的");
    assert_eq!(px(&mut e, 279.0, 40.0), GREEN, "胶囊外一个像素不碰");
    assert_eq!(px(&mut e, 300.0, 22.0), GREEN);
}

#[test]
fn 深色导航栏是半透明黑底白点() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("dark", r#"{"navigationStyle":"custom","navigationBarTextStyle":"white"}"#);
    let mut e = engine(&root, true);
    let bg = px(&mut e, BG_AT.0, BG_AT.1);
    // rgba(0,0,0,.15)：页面颜色 × 0.85
    assert!((bg.1 as i32 - (GREEN.1 as f32 * 0.85) as i32).abs() <= 2, "底色 {bg:?}");
    assert_eq!(px(&mut e, DOT_AT.0, DOT_AT.1), (255, 255, 255));
}

#[test]
fn 选项没开_或不是_custom_页面_一个像素都不画() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("off", r#"{"navigationStyle":"custom"}"#);
    let mut e = engine(&root, false);
    assert!(e.menu_button_rect().is_none());
    assert_eq!(px(&mut e, BG_AT.0, BG_AT.1), GREEN);
    assert_eq!(px(&mut e, DOT_AT.0, DOT_AT.1), GREEN);

    // 普通页面的导航栏是宿主画的，胶囊在宿主的导航栏里，引擎不画
    let root = fixture("default-nav", "{}");
    let mut e = engine(&root, true);
    assert!(e.menu_button_rect().is_none());
    assert_eq!(px(&mut e, DOT_AT.0, DOT_AT.1), GREEN);
}

#[test]
fn 点左半发_menu_右半发_close_都不漏到下面的页面() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("tap", r#"{"navigationStyle":"custom"}"#);
    let mut e = engine(&root, true);
    let mut t = 100;
    events(&mut e);
    tap(&mut e, 300.0, 40.0, &mut t);
    assert_eq!(menu_actions(&mut e), vec!["menu"]);
    tap(&mut e, 350.0, 40.0, &mut t);
    assert_eq!(menu_actions(&mut e), vec!["close"]);
    assert_eq!(hits(&e), "0", "胶囊下面那条导航栏的 bindtap 不能被触发");
    // 胶囊外面照常是页面的
    tap(&mut e, 100.0, 40.0, &mut t);
    assert_eq!(hits(&e), "1");
    assert!(menu_actions(&mut e).is_empty());
}

#[test]
fn 按住有按压态_滑出去松手不算点() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("press", r#"{"navigationStyle":"custom"}"#);
    let mut e = engine(&root, true);
    let idle = px(&mut e, BG_AT.0, BG_AT.1);
    e.pointer_down(300.0, 40.0);
    e.pump(200);
    let pressed = px(&mut e, BG_AT.0, BG_AT.1);
    assert_ne!(pressed, idle, "按着的那一半要压暗");
    assert_eq!(px(&mut e, 350.0, 30.0), {
        let mut e2 = engine(&root, true);
        px(&mut e2, 350.0, 30.0)
    }, "另一半不动");
    e.pointer_move(200.0, 40.0);
    e.pump(216);
    assert_eq!(px(&mut e, BG_AT.0, BG_AT.1), idle, "滑出去按压态褪掉");
    e.pointer_up(200.0, 40.0);
    e.pump(232);
    assert!(menu_actions(&mut e).is_empty(), "滑出去松手不发事件");
    assert_eq!(hits(&e), "0", "这串触摸归胶囊，松手在导航栏上也不算点了导航栏");
    // 被系统打断：按压态褪掉、什么也不发
    e.pointer_down(350.0, 40.0);
    e.pump(248);
    e.pointer_cancel();
    e.pump(264);
    assert_eq!(px(&mut e, 350.0, 30.0), {
        let mut e2 = engine(&root, true);
        px(&mut e2, 350.0, 30.0)
    });
    assert!(menu_actions(&mut e).is_empty());
}

#[test]
fn 启动后发一次_app_事件带关于信息() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("about", r#"{"navigationStyle":"custom"}"#);
    std::fs::write(root.join("project.config.json"), r#"{"projectname":"夹具小程序","appid":"wxtest","description":"测胶囊用"}"#).unwrap();
    // 不开胶囊也发：「•••」也可能是宿主自己导航栏里的那个
    let mut e = engine(&root, false);
    let evs = events(&mut e);
    let apps: Vec<&Value> = evs.iter().filter(|v| v["type"] == "app").collect();
    assert_eq!(apps.len(), 1, "{evs:?}");
    let a = &apps[0]["about"];
    assert_eq!((a["name"].as_str(), a["appId"].as_str(), a["description"].as_str()), (Some("夹具小程序"), Some("wxtest"), Some("测胶囊用")));
    assert_eq!((a["entryPage"].as_str(), a["pageCount"].as_u64()), (Some("pages/p/p"), Some(1)));
    assert!(a["engine"].as_str().unwrap_or("").starts_with("mini-render "));
    e.pump(100);
    assert!(events(&mut e).iter().all(|v| v["type"] != "app"), "只发一次");
}

#[test]
fn 改前景色之后换配色() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("restyle", r#"{"navigationStyle":"custom","navigationBarTextStyle":"black"}"#);
    let mut e = engine(&root, true);
    assert_eq!(px(&mut e, DOT_AT.0, DOT_AT.1), (0, 0, 0));
    e.eval("wx.setNavigationBarColor({frontColor:'#ffffff', backgroundColor:'#07c160'})").unwrap();
    // 这条事件在帧末尾才收到：这一帧要报「还要一帧」，下一帧换成白点
    let f = e.pump_ex(300);
    assert!(f & WANTS_MORE != 0, "改了前景色要续一帧（flags {f}）");
    e.pump_ex(316);
    assert_eq!(px(&mut e, DOT_AT.0, DOT_AT.1), (255, 255, 255));
    assert!(e.pump_ex(332) & WANTS_MORE == 0, "换完了就别再要帧");
}
