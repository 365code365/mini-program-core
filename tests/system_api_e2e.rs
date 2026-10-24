//! 第 8 批的 `wx.*`：showActionSheet 面板、previewImage 浮层、原生 tabBar 运行时 API、剪贴板、振动、
//! 「要宿主才能做」的能力的默认实现、downloadFile / uploadFile（G-10 / G-11）。
//!
//! 依据官方 api/ui/interaction/wx.showActionSheet、api/media/image/wx.previewImage、api/ui/tab-bar/*、
//! api/device/clipboard、api/device/vibrate、api/network/download、api/network/upload。
//! 走 `MiniEngine`（SDK 链路），独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use serde_json::Value;
use std::io::{Read, Write};
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

const APP_JSON: &str = r##"{"pages":["pages/index/index","pages/other/other"],"window":{"navigationStyle":"custom"},
 "tabBar":{"color":"#999999","selectedColor":"#07c160","backgroundColor":"#ffffff",
   "list":[{"pagePath":"pages/index/index","text":"首页"},{"pagePath":"pages/other/other","text":"我的"}]}}"##;

fn fixture(name: &str, wxml: &str, js: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("system-api-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", APP_JSON),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", "page{background:#00ff00}"),
        ("pages/index/index.js", js),
        ("pages/other/other.json", "{}"),
        ("pages/other/other.wxml", "<view>other</view>"),
        ("pages/other/other.wxss", ""),
        ("pages/other/other.js", "Page({});"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    std::fs::create_dir_all(root.join("img")).unwrap();
    image::RgbaImage::from_pixel(10, 10, image::Rgba([255, 0, 0, 255])).save(root.join("img/red.png")).unwrap();
    image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 0, 255, 255])).save(root.join("img/blue.png")).unwrap();
    Fixture(root)
}

const PAGE_JS: &str = r#"Page({ data: { log: [], taps: 0 },
  rec(x) { this.data.log.push(x); },
  onTap() { this.data.taps++; } });"#;
const PAGE_WXML: &str = r#"<view style="height:600px" bindtap="onTap">page</view>"#;

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
    /// 浮层的入场 / 退场、预览换张的弹簧按墙上时钟走（0.2~0.35s）
    fn settle(&mut self) {
        std::thread::sleep(Duration::from_millis(450));
        self.frames(3);
    }
    fn run(&mut self, code: &str) {
        self.e.eval(code).unwrap();
        self.frames(3);
    }
    fn tap(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.frames(1);
        self.e.pointer_up(x, y);
        self.frames(2);
    }
    fn drag(&mut self, x0: f32, x1: f32, y: f32) {
        self.e.pointer_down(x0, y);
        self.frames(1);
        for i in 1..=6 {
            self.e.pointer_move(x0 + (x1 - x0) * i as f32 / 6.0, y);
            self.frames(1);
        }
        self.e.pointer_up(x1, y);
        self.frames(2);
    }
    fn js(&self, code: &str) -> Value {
        let s = self.e.eval(&format!("JSON.stringify({code})")).unwrap_or_default();
        let s: String = serde_json::from_str(&s).unwrap_or(s);
        serde_json::from_str(&s).unwrap_or(Value::Null)
    }
    fn log(&self) -> Value {
        self.js("__currentPage.data.log")
    }
    /// 逻辑坐标 (x, y) 处的像素
    fn px(&mut self, x: usize, y: usize) -> (u8, u8, u8) {
        let (w, _) = self.e.pixel_size();
        let p = self.e.rgba();
        let i = (y * 2 * w as usize + x * 2) * 4;
        (p[i], p[i + 1], p[i + 2])
    }
}

const SHOW_SHEET: &str = "wx.showActionSheet({ itemList: ['A', 'B', 'C'], \
    success: r => __currentPage.rec(['ok', r.tapIndex, typeof r.tapIndex]), \
    fail: r => __currentPage.rec(['fail', r.errMsg]) })";

#[test]
fn action_sheet_taps_items_cancel_and_mask() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("sheet", PAGE_WXML, PAGE_JS);
    let mut e = E::new(&fx);
    // 视口 667（tabBar 页：引擎视口不含 tabBar，但浮层盖整个窗口）；面板高 3×56 + 8 + 56 = 232
    let top = 667.0 - 232.0;
    e.run(SHOW_SHEET);
    e.settle();
    assert!(e.px(187, 100).1 < 200, "遮罩压暗了页面：{:?}", e.px(187, 100));
    e.tap(187.0, top + 56.0 + 28.0);
    assert_eq!(e.log(), serde_json::json!([["ok", 1, "number"]]), "点第二项 → tapIndex 1（number）");
    e.settle();

    e.run(SHOW_SHEET);
    e.settle();
    e.tap(187.0, top + 3.0 * 56.0 + 8.0 + 28.0);
    e.settle();
    e.run(SHOW_SHEET);
    e.settle();
    e.tap(187.0, 100.0);
    e.settle();
    let log = e.log();
    assert_eq!(log[1], serde_json::json!(["fail", "showActionSheet:fail cancel"]), "取消：{log}");
    assert_eq!(log[2], serde_json::json!(["fail", "showActionSheet:fail cancel"]), "点遮罩：{log}");
    assert_eq!(e.js("__currentPage.data.taps"), 0, "面板弹着时点击不落到页面上");
    assert_eq!(e.px(187, 100), (0, 255, 0), "收起后页面恢复");

    // 超过 6 项：直接 fail
    e.run("wx.showActionSheet({ itemList: ['1','2','3','4','5','6','7'], fail: r => __currentPage.rec(r.errMsg) })");
    e.frames(2);
    assert!(e.log()[3].as_str().unwrap_or("").contains("large than 6"), "{}", e.log());
}

#[test]
fn preview_image_switches_by_swipe_and_closes_on_tap() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("preview", PAGE_WXML, PAGE_JS);
    let mut e = E::new(&fx);
    e.run("wx.previewImage({ urls: ['/img/red.png', '/img/blue.png'], current: '/img/blue.png', success: () => __currentPage.rec('ok') })");
    e.settle();
    assert_eq!(e.log(), serde_json::json!(["ok"]));
    assert_eq!(e.px(187, 333), (0, 0, 255), "current 指定第二张");
    e.drag(100.0, 260.0, 333.0);
    e.settle();
    assert_eq!(e.px(187, 333), (255, 0, 0), "右划回到第一张");
    e.drag(100.0, 300.0, 333.0);
    e.settle();
    assert_eq!(e.px(187, 333), (255, 0, 0), "到头不再切");
    e.tap(187.0, 333.0);
    e.settle();
    assert_eq!(e.px(187, 333), (0, 255, 0), "轻点关闭，页面回来");
    assert_eq!(e.js("__currentPage.data.taps"), 0, "预览里的点击不落到页面上");
}

/// 换张是滑过去的（松手后中间那几帧两张图各占一部分），不是一下跳过去
#[test]
fn preview_image_slides_between_images() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("preview-slide", PAGE_WXML, PAGE_JS);
    let mut e = E::new(&fx);
    e.run("wx.previewImage({ urls: ['/img/red.png', '/img/blue.png'] })");
    e.settle();
    // 往左拖 120（不到半屏），松手立刻看：左边还露着红、右边已经是蓝 —— 正在滑
    e.e.pointer_down(300.0, 333.0);
    e.frames(1);
    for i in 1..=6 {
        e.e.pointer_move(300.0 - 20.0 * i as f32, 333.0);
        e.frames(1);
    }
    e.e.pointer_up(180.0, 333.0);
    e.frames(1);
    assert_eq!(e.px(20, 333), (255, 0, 0), "松手那一刻左边还是红");
    assert_eq!(e.px(355, 333), (0, 0, 255), "右边已经露出蓝");
    e.settle();
    assert_eq!(e.px(20, 333), (0, 0, 255), "停稳后整屏是第二张");
    assert_eq!(e.px(355, 333), (0, 0, 255));
}

/// 右上角关闭键；往下拉没拉够回弹、拉够了关
#[test]
fn preview_image_close_button_and_pull_down() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("preview-close", PAGE_WXML, PAGE_JS);
    let mut e = E::new(&fx);
    e.run("wx.previewImage({ urls: ['/img/red.png'] })");
    e.settle();
    // 关闭键在右上角（状态栏 20 下面，圆心 (343, 44)）：圈是深色的，叉是白的
    assert_eq!(e.px(343, 44), (255, 255, 255), "关闭键的叉");
    e.tap(343.0, 44.0);
    e.settle();
    assert_eq!(e.px(187, 333), (0, 255, 0), "点关闭键：页面回来");

    let pull = |e: &mut E, dy: f32| {
        e.e.pointer_down(187.0, 300.0);
        e.frames(1);
        for i in 1..=6 {
            e.e.pointer_move(187.0, 300.0 + dy * i as f32 / 6.0);
            e.frames(1);
        }
    };
    e.run("wx.previewImage({ urls: ['/img/red.png'] })");
    e.settle();
    pull(&mut e, 200.0);
    let mid = e.px(187, 100);
    assert!(mid.1 > 0, "下拉时露出页面：{mid:?}");
    e.e.pointer_move(187.0, 340.0);
    e.frames(1);
    std::thread::sleep(Duration::from_millis(120));
    e.e.pointer_up(187.0, 340.0);
    e.settle();
    assert_eq!(e.px(187, 333), (255, 0, 0), "只拉了 40：回弹，预览还在");
    pull(&mut e, 200.0);
    e.e.pointer_up(187.0, 500.0);
    e.settle();
    assert_eq!(e.px(187, 333), (0, 255, 0), "拉了 200：关掉");
    assert_eq!(e.js("__currentPage.data.taps"), 0);
}

/// previewMedia 里的视频：翻到它就作为一路同层视频报给宿主（键 `__preview_video`），关掉就撤
#[test]
fn preview_media_video_is_reported_to_host() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("preview-media", PAGE_WXML, PAGE_JS);
    let data = fx.0.join("_data");
    let opts = mini_render::host::EngineOptions::from_json(r#"{"sameLayer":["video"]}"#).unwrap();
    let env = mini_render::compositor::HostEnv::default();
    let mut eng = MiniEngine::new_with_options(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0, env, opts).unwrap();
    eng.launch(Some("pages/index/index")).unwrap();
    let mut e = E { e: eng, t: 1000 };
    e.frames(3);
    let videos = |e: &mut E| -> Vec<Value> {
        let mut out = Vec::new();
        while let Some(ev) = e.e.poll_event() {
            let v: Value = serde_json::from_str(&ev).unwrap_or(Value::Null);
            if v["type"] == "videos" {
                out.push(v["views"].clone());
            }
        }
        out
    };
    videos(&mut e);
    e.run("wx.previewMedia({ sources: [{ url: '/img/red.png' }, { url: '/v/a.mp4', type: 'video', poster: '/img/blue.png' }], current: 1, success: () => __currentPage.rec('ok') })");
    e.settle();
    assert_eq!(e.log(), serde_json::json!(["ok"]));
    let got = videos(&mut e);
    let last = got.last().cloned().unwrap_or(Value::Null);
    assert_eq!(last[0]["key"], "__preview_video", "翻到视频那一项：报给宿主 {got:?}");
    assert!(last[0]["url"].as_str().unwrap_or("").ends_with("a.mp4"));
    assert_eq!(e.px(187, 200), (0, 0, 255), "首帧没到：先画封面");
    // 视频暂停（还没起播）时中间有播放键：圈里是白色三角
    assert_eq!(e.px(190, 333), (255, 255, 255), "播放键");
    // 翻回图片：撤掉这一路
    e.drag(60.0, 260.0, 500.0);
    e.settle();
    let got = videos(&mut e);
    assert_eq!(got.last().cloned(), Some(serde_json::json!([])), "翻走后撤掉：{got:?}");
    assert_eq!(e.px(187, 333), (255, 0, 0));
}

#[test]
fn tab_bar_badge_dot_style_and_hide() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("tabbar", PAGE_WXML, PAGE_JS);
    let mut e = E::new(&fx);
    // tabBar 在 617~667；第 0 项图标中心 x≈94，角标胶囊左半边 ≈(103, 630)（避开中间的白字）
    let before = e.px(103, 630);
    e.run("wx.setTabBarBadge({ index: 0, text: '5', success: r => __currentPage.rec(r.errMsg) })");
    e.frames(2);
    assert_eq!(e.log(), serde_json::json!(["setTabBarBadge:ok"]));
    let red = e.px(103, 630);
    assert!(red.0 > 200 && red.1 < 120 && before != red, "角标是红的：{red:?}（之前 {before:?}）");

    e.run("wx.setTabBarBadge({ index: 5, text: '1', fail: r => __currentPage.rec(r.errMsg) })");
    assert!(e.log()[1].as_str().unwrap().starts_with("setTabBarBadge:fail"), "{}", e.log());
    e.run("wx.removeTabBarBadge({ index: 0 }); wx.showTabBarRedDot({ index: 1, success: r => __currentPage.rec(r.errMsg) })");
    assert_eq!(e.log()[2], "showTabBarRedDot:ok");
    assert_ne!(e.px(103, 630), red, "角标去掉了");

    e.run("wx.setTabBarStyle({ backgroundColor: '#ff00ff' })");
    assert_eq!(e.px(10, 660), (255, 0, 255), "改背景色");
    e.run("wx.hideTabBar({ success: r => __currentPage.rec(r.errMsg) })");
    e.frames(2);
    assert_eq!(e.px(10, 660), (0, 255, 0), "隐藏后底部露出页面");
    e.run("wx.showTabBar()");
    e.frames(2);
    assert_eq!(e.px(10, 660), (255, 0, 255));
    e.run("wx.hideTabBarRedDot({ index: 1 }); wx.setTabBarStyle({ backgroundColor: '#ffffff' })");
}

#[test]
fn clipboard_vibrate_and_host_capability_defaults() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("clip", PAGE_WXML, PAGE_JS);
    let mut e = E::new(&fx);
    e.run("wx.setClipboardData({ data: 'hello 剪贴板', success: r => __currentPage.rec(r.errMsg) })");
    e.run("wx.getClipboardData({ success: r => __currentPage.rec(r.data) })");
    e.run("wx.vibrateShort({ type: 'heavy', success: r => __currentPage.rec(r.errMsg) })");
    e.run("wx.login({ fail: r => __currentPage.rec(r.errMsg) })");
    e.run("__currentPage.rec([typeof wx.chooseImage, typeof wx.requestPayment, typeof wx.getLocation])");
    // 宿主给的设置覆盖默认值（真实的定位开关只有宿主知道）
    let default_loc = e.js("wx.getSystemSetting().locationEnabled");
    e.run("globalThis.__hostSystemInfo = { systemSetting: { locationEnabled: true }, appAuthorizeSetting: { cameraAuthorized: 'denied' } }");
    assert_eq!(default_loc, false);
    assert_eq!(e.js("wx.getSystemSetting().locationEnabled"), true);
    assert_eq!(e.js("[wx.getAppAuthorizeSetting().cameraAuthorized]"), serde_json::json!(["denied"]));
    assert_eq!(e.js("[wx.getAppAuthorizeSetting().albumAuthorized]"), serde_json::json!(["authorized"]));
    let log = e.log();
    assert_eq!(log[0], "setClipboardData:ok");
    assert_eq!(log[1], "hello 剪贴板");
    assert_eq!(log[2], "vibrateShort:ok");
    assert!(log[3].as_str().unwrap().starts_with("login:fail"), "{log}");
    assert_eq!(log[4], serde_json::json!(["function", "function", "function"]));
    let mut kinds = Vec::new();
    while let Some(ev) = e.e.poll_event() {
        let v: Value = serde_json::from_str(&ev).unwrap_or(Value::Null);
        kinds.push(v["type"].as_str().unwrap_or("").to_string());
        if v["type"] == "vibrate" {
            assert_eq!(v["style"], "heavy");
        }
        if v["type"] == "clipboard" {
            assert_eq!(v["data"], "hello 剪贴板");
        }
    }
    assert!(kinds.iter().any(|k| k == "clipboard") && kinds.iter().any(|k| k == "vibrate"), "宿主事件：{kinds:?}");
}

/// 一个只答一次的本地 HTTP 服务：把收到的请求原样交回给测试，回一段固定正文
fn one_shot_server(body: &'static [u8]) -> (String, std::thread::JoinHandle<Vec<u8>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let h = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let mut req = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = s.read(&mut buf).unwrap_or(0);
            if n == 0 {
                break;
            }
            req.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&req).to_string();
            if let Some(end) = text.find("\r\n\r\n") {
                let len = text[..end]
                    .lines()
                    .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0)))
                    .unwrap_or(0);
                if req.len() >= end + 4 + len {
                    break;
                }
            }
        }
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
        s.write_all(head.as_bytes()).ok();
        s.write_all(body).ok();
        req
    });
    (url, h)
}

fn wait_for(e: &mut E, code: &str) -> Value {
    for _ in 0..200 {
        e.frames(1);
        std::thread::sleep(Duration::from_millis(10));
        let v = e.js(code);
        if !v.is_null() {
            return v;
        }
    }
    Value::Null
}

#[test]
fn download_then_upload_round_trip() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("files", PAGE_WXML, "Page({ data: { dl: null, up: null } });");
    let mut e = E::new(&fx);
    let (url, server) = one_shot_server(b"FILE-BYTES-123");
    e.run(&format!("wx.downloadFile({{ url: '{url}/a.txt', success: r => __currentPage.data.dl = r, fail: r => __currentPage.data.dl = r }})"));
    let dl = wait_for(&mut e, "__currentPage.data.dl");
    server.join().unwrap();
    assert_eq!(dl["errMsg"], "downloadFile:ok", "{dl}");
    assert_eq!(dl["statusCode"], 200);
    let tmp = dl["tempFilePath"].as_str().unwrap_or("").to_string();
    assert!(tmp.starts_with("wxfile://tmp/") && tmp.ends_with(".txt"), "{tmp}");

    let (url, server) = one_shot_server(b"{\"ok\":1}");
    e.run(&format!(
        "wx.uploadFile({{ url: '{url}/up', filePath: '{tmp}', name: 'pic', formData: {{ k: 'v' }}, \
         success: r => __currentPage.data.up = r, fail: r => __currentPage.data.up = r }})"
    ));
    let up = wait_for(&mut e, "__currentPage.data.up");
    let req = String::from_utf8_lossy(&server.join().unwrap()).to_string();
    assert_eq!(up["errMsg"], "uploadFile:ok", "{up}");
    assert_eq!(up["data"], "{\"ok\":1}", "uploadFile 的 data 是字符串");
    assert!(req.starts_with("POST /up"), "{req}");
    assert!(req.to_ascii_lowercase().contains("content-type: multipart/form-data; boundary="), "{req}");
    assert!(req.contains("name=\"k\"\r\n\r\nv\r\n"), "formData 字段：{req}");
    assert!(req.contains("name=\"pic\"; filename=") && req.contains("FILE-BYTES-123"), "下载回来的文件原样上传：{req}");
}
