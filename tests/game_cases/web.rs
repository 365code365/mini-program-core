//! 高性能模式（game/web：游戏 JS 跑在 WebView 里，iOS 的 WKWebView）的 e2e —— 页面在 headless Chrome 里跑，
//! 原生侧由 devserver 扮演 iOS 宿主。用例与 QuickJS 模式同一批夹具、同一套断言口径（同名用例对照着看）。
//! 找不到 Chrome 时整组跳过（打一行说明）。
use super::*;
use game_harness::web::{WebHarness, WebOpts};

macro_rules! open {
    ($e:expr) => {
        match $e {
            Some(h) => h,
            None => return,
        }
    };
}

/// R1：console、onError、onUnhandledRejection、GameGlobal === globalThis（模块里）、game.js 只执行一次
#[test]
fn web_r1_g_hello() {
    let _g = serial();
    let mut h = open!(WebHarness::open("g-hello", WebOpts::default()));
    h.until("T:DONE", 10.0);
    h.assert_no_fail();
    assert!(h.has("hello 1 true null undefined {\"a\":[1,2]}"));
    assert!(h.has("[WARN] warn-line") && h.has("[ERROR] Error: boom-for-test"));
    assert_eq!(h.log.lock().unwrap().iter().filter(|l| l.contains("T:READY g-hello")).count(), 1, "game.js 只执行一次");
    let un = h.events("unhandled");
    assert_eq!(un.len(), 1, "只有真正没人处理的拒绝才派发：{un:?}");
    assert_eq!(un[0]["reason"], "nope");
    assert!(h.events("onError").iter().any(|e| e["msg"] == "in-timer"));
    // 栈里的行号对得上源文件（包装头不占行）
    assert!(h.has("(/game.js:5:"), "console.error(new Error) 在第 5 行：{:?}", h.log.lock().unwrap());
}

/// R5：game.js 顶层抛错 → error 事件（行号对）、页面照常活着
#[test]
fn web_r5_顶层抛错() {
    let _g = serial();
    let mut h = open!(WebHarness::open_inline("throw", "{}", "var a = 1;\nnull.x;\n", &[], WebOpts::default()));
    h.sleep(200);
    let errs = h.errors();
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(errs[0]["kind"], "script");
    assert!(errs[0]["stack"].as_str().unwrap_or("").contains("game.js:2"), "{errs:?}");
    assert_eq!(h.eval("1 + 1"), 2);
}

/// S6：一次 1 万行日志只交出 500 行 + 一行丢弃计数（与 QuickJS 模式同一个上限）
#[test]
fn web_s6_日志限流() {
    let _g = serial();
    let mut h = open!(WebHarness::open_inline("flood", "{}", "setTimeout(function () { for (var i = 0; i < 10000; i++) console.log('L' + i); }, 0);", &[], WebOpts::default()));
    h.until("丢弃 9500 行", 10.0);
    assert_eq!(h.log.lock().unwrap().iter().filter(|l| l.starts_with('L')).count(), 500);
}

/// 适配层在浏览器里：往 window 上挂 location / document / navigator 不跳页、不抛错，读回游戏设的；
/// GameGlobal.foo = 1 之后裸写 foo 读得到；原生方法经 window. 调用 this 不丢；Float32Array 之类构造器照常
#[test]
fn web_适配层_全局代理() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
'use strict';
window.location = { href: 'game.js', reload: function () {} };
GameGlobal.document = { fake: true };
window.navigator = { userAgent: 'x' };
GameGlobal.foo = 41;
T.check('loc', window.location.href === 'game.js' && typeof location.reload === 'function');
T.check('doc', GameGlobal.document.fake === true && typeof document.createElement === 'function');
T.check('nav', window.navigator.userAgent === 'x');
T.check('bare', foo === 41 && window.foo === 41 && 'foo' in GameGlobal);
T.check('same', GameGlobal === globalThis && window === GameGlobal && self === window);
var id = window.setTimeout(function () {}, 1); window.clearTimeout(id);
var raf = window.requestAnimationFrame(function () {});
T.check('ctor', window.Float32Array.BYTES_PER_ELEMENT === 4 && new window.Image() instanceof HTMLImageElement);
T.check('wx', typeof wx.getSystemInfoSync === 'function' && typeof window.wx === 'object');
T.done({ href: String(globalThis.__realHref) });
"#;
    let mut h = open!(WebHarness::open_inline("adapter", "{}", js, &[], WebOpts::default()));
    h.until("T:DONE", 10.0);
    h.assert_no_fail();
    assert!(h.eval("location.pathname").as_str().unwrap().ends_with("/__wg.html"), "页面没跳走");
}

/// R9：系统信息与 QuickJS 模式同一套口径（platform / SDKVersion 不许宿主改、窗口尺寸、胶囊）
#[test]
fn web_r9_系统信息() {
    let _g = serial();
    let js = "var i = wx.getSystemInfoSync(), m = wx.getMenuButtonBoundingClientRect(); console.log('T:EV info ' + JSON.stringify({ w: i.windowWidth, h: i.windowHeight, pr: i.pixelRatio, sdk: i.SDKVersion, platform: i.platform, mw: m.width, mh: m.height, launch: wx.getLaunchOptionsSync().query }));";
    let mut h = open!(WebHarness::open_inline("info", "{}", js, &[], WebOpts { query: "a=1", ..Default::default() }));
    h.until("T:EV info", 10.0);
    let i = &h.events("info")[0];
    assert_eq!((i["w"].as_f64(), i["h"].as_f64(), i["pr"].as_f64()), (Some(375.0), Some(667.0), Some(2.0)), "{i}");
    assert_eq!(i["sdk"], mini_render::game::GAME_SDK_VERSION);
    assert_eq!(i["platform"], mini_render::game::target_platform());
    assert!(i["mw"].as_f64().unwrap() > 50.0 && i["mh"].as_f64().unwrap() > 20.0, "胶囊：{i}");
    assert_eq!(i["launch"]["a"], "1");
}

/// 存档：类型不变、跨重启（与 QuickJS 模式同一个文件）、getStorageInfoSync
#[test]
fn web_storage_跨重启() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
var n = wx.getStorageSync('n') || 0;
wx.setStorageSync('n', n + 1);
wx.setStorageSync('obj', { a: [1, 'x'], t: true });
wx.setStorageSync('s', '42');
var i = wx.getStorageInfoSync();
T.ev('st', { n: n, obj: wx.getStorageSync('obj'), s: typeof wx.getStorageSync('s'), keys: i.keys.sort(), missing: wx.getStorageSync('nope') });
wx.removeStorage({ key: 's', success: function () { T.done({}); } });
"#;
    let mut h = open!(WebHarness::open_inline("storage", "{}", js, &[], WebOpts::default()));
    h.until("T:DONE", 10.0);
    let e = &h.events("st")[0];
    assert_eq!(e["n"], 0);
    assert_eq!(e["obj"], serde_json::json!({ "a": [1, "x"], "t": true }));
    assert_eq!((e["s"].as_str(), e["missing"].as_str()), (Some("string"), Some("")), "存 '42' 读回字符串、没有的键读回 ''");
    assert_eq!(e["keys"], serde_json::json!(["n", "obj", "s"]));
    // 原生落盘之后再开一次（同一个数据目录）：n 变成 1，s 已删
    h.with_game(|g| { let _ = g.take_js(); });
    let mut h = open!(h.reopen(WebOpts::default()));
    h.until("T:DONE", 10.0);
    let e = &h.events("st")[0];
    assert_eq!(e["n"], 1, "存档跨重启：{e}");
    assert_eq!(e["keys"], serde_json::json!(["n", "obj", "s"]), "removeStorage 也落盘了，这次又写了 s");
}

/// K5 / K7：rAF 同一批同一个时间戳、回调里取消同批的下一个就不执行；setPreferredFramesPerSecond(30) 跳帧
#[test]
fn web_k5_k7_raf() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
var ts = [], order = [];
var b = 0;
requestAnimationFrame(function (t) { ts.push(t); order.push('a'); cancelAnimationFrame(b); requestAnimationFrame(function () { order.push('next'); next(); }); });
b = requestAnimationFrame(function () { order.push('b'); });
requestAnimationFrame(function (t) { ts.push(t); order.push('c'); });
function next() {
  T.ev('batch', { same: ts[0] === ts[1], order: order, perf: Math.abs(performance.now() - ts[0]) < 1000 });
  wx.setPreferredFramesPerSecond(30);
  var last = 0, gaps = [];
  function f(t) { if (last) gaps.push(t - last); last = t; if (gaps.length < 20) requestAnimationFrame(f); else T.ev('fps30', { min: Math.min.apply(null, gaps) }); }
  requestAnimationFrame(f);
}
"#;
    let mut h = open!(WebHarness::open_inline("raf", "{}", js, &[], WebOpts::default()));
    h.until("T:EV fps30", 15.0);
    let b = &h.events("batch")[0];
    assert_eq!(b["same"], true, "同一批同一个时间戳");
    assert_eq!(b["order"], serde_json::json!(["a", "c", "next"]), "取消同批的 b、回调里新注册的进下一批");
    assert_eq!(b["perf"], true, "rAF 时间戳与 performance.now 同一时间轴");
    assert!(h.events("fps30")[0]["min"].as_f64().unwrap() > 25.0, "30fps 时最短间隔 ≈33ms：{:?}", h.events("fps30"));
}

/// T1~T3：触摸形状（identifier 从 0 起、三组坐标相同、touches / changedTouches）、两指 identifier 0 / 1
#[test]
fn web_t1_t3_触摸() {
    let _g = serial();
    let mut h = open!(WebHarness::open("g-touch-draw", WebOpts::default()));
    h.until("T:READY", 10.0);
    h.chrome.touch("touchStart", &[(100.0, 200.0, 7)]).unwrap();
    h.chrome.touch("touchMove", &[(110.0, 210.0, 7)]).unwrap();
    h.chrome.touch("touchStart", &[(110.0, 210.0, 7), (300.0, 400.0, 9)]).unwrap();
    h.chrome.touch("touchEnd", &[(300.0, 400.0, 9)]).unwrap();
    h.chrome.touch("touchEnd", &[]).unwrap();
    h.until("T:EV touchend", 5.0).sleep(200);
    let st = h.events("touchstart");
    let pt = |v: &serde_json::Value| -> Vec<f64> { v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect() };
    assert_eq!(pt(&st[0]["ch"][0]), vec![0.0, 100.0, 200.0], "{st:?}");
    assert_eq!(st[0]["eq"], true, "screen / client / page 相同");
    assert_eq!(st[1]["ids"], serde_json::json!([0, 1]), "第二根手指 identifier 1：{st:?}");
    let mv = h.events("touchmove");
    assert_eq!(pt(&mv[0]["ch"][0]), vec![0.0, 110.0, 210.0]);
    let end = h.events("touchend");
    assert_eq!(end.last().unwrap()["ids"], serde_json::json!([]), "全抬起后 touches 为空：{end:?}");
    // 点下去的位置画上了第一根手指的颜色（红）
    assert_eq!(h.pixel(100, 200), [255, 0, 0]);
}

/// I1~I3：图片来源（包内、绝对路径、data、wxfile、远程）、onerror 带 errMsg
#[test]
fn web_i1_i3_图片() {
    let _g = serial();
    let png = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/sample-game/g-image/img/px4x2.png")).unwrap();
    let port = web_serve(png);
    let q: &'static str = Box::leak(format!("http={port}").into_boxed_str());
    let mut h = open!(WebHarness::open("g-image", WebOpts { query: q, ..Default::default() }));
    h.until("T:DONE", 15.0);
    h.assert_no_fail();
    let loads = h.events("load");
    let size = |k: &str| loads.iter().find(|e| e["k"] == k).map(|e| (e["after"][0].as_u64().unwrap(), e["after"][1].as_u64().unwrap()));
    for (k, wh) in [("png", (4, 2)), ("png-abs", (4, 2)), ("jpg", (16, 8)), ("webp", (5, 5)), ("gif", (6, 6)), ("data", (4, 2)), ("wxfile", (4, 2)), ("http", (4, 2))] {
        assert_eq!(size(k), Some(wh), "{k}：{loads:#?}");
    }
    let errs = h.events("error");
    for k in ["missing", "broken", "escape", "http404"] {
        let e = errs.iter().find(|e| e["k"] == k).unwrap_or_else(|| panic!("{k} 应当 onerror：{errs:#?}"));
        assert!(e["arg"]["errMsg"].as_str().is_some_and(|m| !m.is_empty()), "onerror 参数带 errMsg：{e}");
    }
    assert!(h.has("T:EV same5"));
}

/// 只认 GET /px.png 的本地 HTTP（远程图片、wx.request 用）；/json 回 JSON，/bin 回 4 个字节，其余 404
pub fn web_serve(png: Vec<u8>) -> u16 {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap_or(0);
            let head = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (status, body): (&str, Vec<u8>) = if path.starts_with("/px.png") { ("200 OK", png.clone()) }
                else if path.starts_with("/json") { ("200 OK", format!(r#"{{"ok":true,"m":"{}","q":"{}"}}"#, head.split_whitespace().next().unwrap_or(""), path).into_bytes()) }
                else if path.starts_with("/bin") { ("200 OK", vec![1, 2, 3, 250]) }
                else { ("404 Not Found", b"not found".to_vec()) };
            let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nX-Test: yes\r\nConnection: close\r\n\r\n", body.len());
            let _ = s.write_all(&body);
        }
    });
    port
}
