//! 高性能模式 e2e 第二部分：覆盖层、生命周期、尺寸、hostCall、文件、网络、WebSocket、Worker、WASM、分包、音频、
//! 第三方引擎与示例游戏（web.rs 的续篇，同一个 WebHarness）。
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

fn ui(h: &WebHarness, tag: &str) -> Vec<serde_json::Value> {
    h.events("ui").into_iter().filter(|e| e["tag"] == tag).collect()
}

/// U1 / U3：Toast（不带 mask 不拦触摸）、Modal（点「好的」→ confirm:true；弹着时触摸不进游戏）、ActionSheet（点第 2 项 → tapIndex 1）
#[test]
fn web_u1_u3_覆盖层() {
    let _g = serial();
    let mut h = open!(WebHarness::open("g-overlays", WebOpts::default()));
    h.until("T:READY", 10.0);
    h.eval("go('toast')");
    h.until("T:EV ui", 5.0);
    assert_eq!(ui(&h, "toast")[0]["r"]["errMsg"], "showToast:ok");
    assert_eq!(h.eval("document.querySelectorAll('.__wg_tip').length"), 1, "Toast 画出来了");
    h.tap(30.0, 600.0);
    h.until("T:EV gameTouch", 5.0);
    h.sleep(1200);
    assert_eq!(h.eval("document.querySelectorAll('.__wg_tip').length"), 0, "1 秒后 Toast 收起");
    h.eval("go('modal')");
    h.sleep(200);
    let touches = h.events("gameTouch").len();
    h.tap(30.0, 100.0);   // 遮罩上：不进游戏
    h.sleep(200);
    assert_eq!(h.events("gameTouch").len(), touches, "Modal 弹着时触摸不进游戏");
    let r = h.eval("(function(){ var b = document.querySelectorAll('.__wg_modal .b div'); var r = b[b.length-1].getBoundingClientRect(); return [r.left + r.width/2, r.top + r.height/2, b[b.length-1].textContent, b.length]; })()");
    assert_eq!((r[2].as_str(), r[3].as_u64()), (Some("好的"), Some(2)), "取消 + 好的：{r}");
    h.tap(r[0].as_f64().unwrap() as f32, r[1].as_f64().unwrap() as f32);
    h.until("\"tag\":\"modal\"", 5.0);
    let m = &ui(&h, "modal")[0];
    assert_eq!((m["ok"].as_bool(), m["r"]["confirm"].as_bool(), m["r"]["cancel"].as_bool()), (Some(true), Some(true), Some(false)), "{m}");
    h.eval("go('sheet')");
    h.sleep(200);
    let r = h.eval("(function(){ var d = document.querySelectorAll('.__wg_sheet div')[1].getBoundingClientRect(); return [d.left + 20, d.top + d.height/2]; })()");
    h.tap(r[0].as_f64().unwrap() as f32, r[1].as_f64().unwrap() as f32);
    h.until("\"tag\":\"sheet\"", 5.0);
    assert_eq!(ui(&h, "sheet")[0]["r"]["tapIndex"], 1);
    h.eval("go('loading')");
    h.sleep(100);
    assert_eq!(h.eval("document.querySelectorAll('.__wg_tip .__wg_spin').length"), 1);
    h.eval("go('hideLoading')");
    h.sleep(100);
    assert_eq!(h.eval("document.querySelectorAll('.__wg_tip').length"), 0);
}

/// L1~L3：进后台 onHide（按着的手指 touchcancel、rAF 停）、回前台 onShow（参数是 getEnterOptionsSync）、rAF 继续
#[test]
fn web_l1_l3_前后台() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
var frames = 0; function f() { frames++; requestAnimationFrame(f); } requestAnimationFrame(f);
wx.onHide(function (r) { T.ev('hide', { frames: frames }); setTimeout(function () { T.ev('hidden', { frames: frames }); }, 300); });
wx.onShow(function (r) { T.ev('show', { scene: r.scene, query: r.query }); });
wx.onTouchCancel(function (e) { T.ev('cancel', { n: e.changedTouches.length, left: e.touches.length }); });
globalThis.frames = function () { return frames; };
T.ready('lc');
"#;
    let mut h = open!(WebHarness::open_inline("lifecycle", "{}", js, &[], WebOpts { query: "k=v", ..Default::default() }));
    h.until("T:READY", 10.0).sleep(300);
    h.chrome.touch("touchStart", &[(50.0, 50.0, 1)]).unwrap();
    h.sleep(100);
    h.with_game(|g| g.set_background(true));
    h.until("T:EV hidden", 5.0);
    let (hide, hidden) = (&h.events("hide")[0], &h.events("hidden")[0]);
    assert_eq!(hide["frames"], hidden["frames"], "后台不跑 rAF：{hide} → {hidden}");
    assert_eq!(h.events("cancel")[0], serde_json::json!({ "n": 1, "left": 0 }), "按着的手指 touchcancel");
    h.with_game(|g| g.set_background(false));
    h.until("T:EV show", 5.0).sleep(300);
    let s = &h.events("show")[0];
    assert_eq!((s["scene"].as_u64(), s["query"]["k"].as_str()), (Some(1001), Some("v")), "{s}");
    let f0 = hidden["frames"].as_u64().unwrap();
    assert!(h.eval("frames()").as_u64().unwrap() > f0 + 5, "回前台 rAF 继续");
}

/// O1：宿主改尺寸 → onWindowResize({windowWidth, windowHeight})、方向变了 onDeviceOrientationChange、系统信息跟着变
#[test]
fn web_o1_转屏() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
wx.onWindowResize(function (r) { T.ev('resize', { r: r, info: wx.getSystemInfoSync().windowWidth }); });
wx.onDeviceOrientationChange(function (r) { T.ev('orient', r); });
T.ready('o');
"#;
    let mut h = open!(WebHarness::open_inline("resize", "{}", js, &[], WebOpts::default()));
    h.until("T:READY", 10.0);
    h.with_game(|g| g.resize(667, 375, 2.0, Some("landscape")));
    h.until("T:EV orient", 5.0);
    let r = &h.events("resize")[0];
    assert_eq!((r["r"]["windowWidth"].as_f64(), r["r"]["windowHeight"].as_f64(), r["info"].as_f64()), (Some(667.0), Some(375.0), Some(667.0)), "{r}");
    assert_eq!(h.events("orient")[0]["value"], "landscape");
}

/// H1：hostApis 点名的 wx.login → hostCall 宿主事件（带 tempDir）→ 宿主回包 → success；没点名的立刻 fail
#[test]
fn web_h1_宿主实现的_api() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
wx.login({ success: function (r) { T.ev('login', r); }, fail: function (r) { T.ev('loginFail', r); } });
wx.requestMidasPayment({ fail: function (r) { T.ev('payFail', { errMsg: r.errMsg }); } });
"#;
    let mut h = open!(WebHarness::open_inline("hostcall", "{}", js, &[], WebOpts { host_apis: &["login"], ..Default::default() }));
    let ev = h.until_host("hostCall", 10.0);
    assert_eq!(ev["api"], "login");
    assert!(ev["tempDir"].is_string(), "{ev}");
    let id = ev["id"].as_u64().unwrap() as u32;
    h.with_game(|g| g.host_reply(id, true, r#"{"code":"abc"}"#));
    h.until("T:EV login", 5.0);
    assert_eq!(h.events("login")[0]["code"], "abc");
    assert!(h.events("login")[0]["errMsg"].as_str().unwrap().starts_with("login:ok"));
    h.until("T:EV payFail", 5.0);
}

/// F1：FileSystemManager（同步 / 异步、二进制、权限）走同一份 Rust 实现；canvas.toTempFilePathSync 落成 wxfile://tmp
#[test]
fn web_f1_文件系统() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
var fs = wx.getFileSystemManager(), p = wx.env.USER_DATA_PATH + '/d/a.bin';
fs.mkdirSync(wx.env.USER_DATA_PATH + '/d', true);
fs.writeFileSync(p, new Uint8Array([1, 2, 3, 255]).buffer);
var back = new Uint8Array(fs.readFileSync(p));
fs.writeFileSync(wx.env.USER_DATA_PATH + '/t.txt', '你好', 'utf8');
var denied = ''; try { fs.writeFileSync('game.js', 'x', 'utf8'); } catch (e) { denied = e.message; }
var pkg = fs.readFileSync('game.json', 'utf8');
var c = wx.createCanvas(); var g = c.getContext('2d'); g.fillStyle = '#ff0000'; g.fillRect(0, 0, 10, 10);
var tmp = c.toTempFilePathSync({ x: 0, y: 0, width: 10, height: 10, destWidth: 4, destHeight: 4 });
var st = fs.statSync(tmp);
fs.readFile({ filePath: wx.env.USER_DATA_PATH + '/t.txt', encoding: 'utf8', success: function (r) {
  T.ev('fs', { back: Array.prototype.slice.call(back), text: r.data, denied: denied, pkg: pkg.trim(), tmp: tmp.indexOf('wxfile://tmp') === 0, size: st.size > 0 });
  var img = wx.createImage(); img.onload = function () { T.ev('tmpimg', { w: img.width, h: img.height }); }; img.src = tmp;
} });
"#;
    let mut h = open!(WebHarness::open_inline("fs", "{}", js, &[], WebOpts::default()));
    h.until("T:EV tmpimg", 10.0);
    let e = &h.events("fs")[0];
    assert_eq!(e["back"], serde_json::json!([1, 2, 3, 255]));
    assert_eq!((e["text"].as_str(), e["pkg"].as_str(), e["tmp"].as_bool(), e["size"].as_bool()), (Some("你好"), Some("{}"), Some(true), Some(true)), "{e}");
    assert!(e["denied"].as_str().unwrap().contains("permission denied"), "包内文件只读：{e}");
    assert_eq!(h.events("tmpimg")[0], serde_json::json!({ "w": 4, "h": 4 }), "toTempFilePath 的 dest 尺寸");
}

/// N1~N4：wx.request（JSON 解析、4xx 也走 success、header、arraybuffer、GET data 拼 query）、downloadFile → 图片能加载
#[test]
fn web_n1_n4_网络() {
    let _g = serial();
    let png = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/sample-game/g-image/img/px4x2.png")).unwrap();
    let port = super::web_cases::web_serve(png);
    let js = format!(r#"
var T = require('_lib/t.js'), base = 'http://127.0.0.1:{port}';
wx.request({{ url: base + '/json', data: {{ a: 1, b: 'x y' }}, success: function (r) {{ T.ev('json', {{ s: r.statusCode, d: r.data, h: r.header['x-test'] || r.header['X-Test'] }}); }} }});
wx.request({{ url: base + '/nope', success: function (r) {{ T.ev('404', {{ s: r.statusCode }}); }} }});
wx.request({{ url: base + '/json', method: 'POST', data: {{ k: 1 }}, success: function (r) {{ T.ev('post', r.data); }} }});
wx.request({{ url: base + '/bin', responseType: 'arraybuffer', success: function (r) {{ T.ev('bin', {{ b: Array.prototype.slice.call(new Uint8Array(r.data)) }}); }} }});
wx.request({{ url: 'http://127.0.0.1:1/x', fail: function (r) {{ T.ev('refused', {{ m: r.errMsg }}); }} }});
wx.downloadFile({{ url: base + '/px.png', success: function (r) {{
  var img = wx.createImage(); img.onload = function () {{ T.ev('dl', {{ p: r.tempFilePath.indexOf('wxfile://tmp') === 0, s: r.statusCode, w: img.width }}); }}; img.src = r.tempFilePath;
}} }});
"#);
    let mut h = open!(WebHarness::open_inline("net", "{}", &js, &[], WebOpts::default()));
    h.until("T:EV dl", 15.0).until("T:EV refused", 15.0).until("T:EV bin", 10.0).until("T:EV post", 10.0).until("T:EV 404", 10.0);
    let j = &h.events("json")[0];
    assert_eq!((j["s"].as_u64(), j["d"]["ok"].as_bool(), j["h"].as_str()), (Some(200), Some(true), Some("yes")), "{j}");
    assert_eq!(j["d"]["q"], "/json?a=1&b=x%20y", "GET 的 data 拼进 query");
    assert_eq!(h.events("404")[0]["s"], 404, "4xx 也走 success");
    assert_eq!(h.events("post")[0]["m"], "POST");
    assert_eq!(h.events("bin")[0]["b"], serde_json::json!([1, 2, 3, 250]));
    assert!(h.events("refused")[0]["m"].as_str().unwrap().starts_with("request:fail"));
    assert_eq!(h.events("dl")[0], serde_json::json!({ "p": true, "s": 200, "w": 4 }));
}

/// N6：wx.connectSocket（浏览器的 WebSocket）：open → 文本与二进制回显 → close(code)
#[test]
fn web_n6_websocket() {
    let _g = serial();
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Some(Ok(s)) = l.incoming().next() {
            let mut ws = tungstenite::accept(s).unwrap();
            // 收到 Close 也接着 read：tungstenite 在下一次 read 里回 Close 帧、完成关闭握手（直接 break 对端看到的是 1006）
            while let Ok(m) = ws.read() {
                if m.is_text() || m.is_binary() { let _ = ws.send(m); }
            }
        }
    });
    let js = format!(r#"
var T = require('_lib/t.js');
var t = wx.connectSocket({{ url: 'ws://127.0.0.1:{port}/' }});
var got = [];
t.onOpen(function () {{ t.send({{ data: 'hi' }}); t.send({{ data: new Uint8Array([9, 8]).buffer }}); }});
t.onMessage(function (m) {{ got.push(typeof m.data === 'string' ? m.data : Array.prototype.slice.call(new Uint8Array(m.data))); if (got.length === 2) t.close({{ code: 1000 }}); }});
t.onClose(function (r) {{ T.ev('ws', {{ got: got, code: r.code }}); }});
"#);
    let mut h = open!(WebHarness::open_inline("ws", "{}", &js, &[], WebOpts::default()));
    h.until("T:EV ws", 10.0);
    assert_eq!(h.events("ws")[0], serde_json::json!({ "got": ["hi", [9, 8]], "code": 1000 }));
}

/// W1 / W5：Worker 真线程（死算期间主线程 rAF 照常）、环境（没有 wx、有 worker / 定时器 / require）、
/// 结构化克隆、console 进宿主日志、未接住的异常 → onError、路径必须在 workers 目录
#[test]
fn web_w1_w5_worker() {
    let _g = serial();
    let mut h = open!(WebHarness::open("g-worker", WebOpts::default()));
    h.until("T:READY", 10.0);
    h.eval("go('badPath')");
    h.until("\"cmd\":\"badPath\"", 5.0);
    assert!(h.events("w")[0]["msg"].as_str().unwrap().contains("workers"));
    h.eval("go('create')");
    h.until("\"cmd\":\"ready\"", 10.0);
    h.eval("go('env')");
    h.until("\"cmd\":\"env\"", 5.0);
    let env = h.events("w").into_iter().find(|e| e["cmd"] == "env").unwrap();
    assert_eq!((env["wx"].as_str(), env["worker"].as_str(), env["setTimeout"].as_str(), env["util"].as_str()),
        (Some("undefined"), Some("object"), Some("function"), Some("util-in-worker")), "{env}");
    h.eval("go('log')");
    h.until("T:EV workerLog", 5.0);
    let f0 = h.eval("frames()").as_f64().unwrap();
    h.eval("go('busy', 1500)");
    h.until("\"cmd\":\"busyDone\"", 10.0);
    let f1 = h.eval("frames()").as_f64().unwrap();
    assert!(f1 - f0 > 60.0, "Worker 死算 1.5s 期间主线程 rAF 照常（{} 帧）", f1 - f0);
    h.eval("go('echo', { mb: 1, transfer: true, back: true })");
    h.until("\"cmd\":\"echoed\"", 10.0);
    let e = h.events("w").into_iter().find(|e| e["cmd"] == "echoed").unwrap();
    let sent = h.events("w").into_iter().find(|e| e["cmd"] == "sent").unwrap();
    assert_eq!(e["back"]["sum"], sent["sum"], "1MB 往返内容不变：{e}");
    assert_eq!(sent["lenAfter"], 0, "transfer 之后发送方的 ArrayBuffer 被转移");
    for k in ["nan", "negZero", "undef", "cyc"] {
        assert_eq!(e["back"][k], true, "结构化克隆 {k}：{e}");
    }
    h.eval("go('throw')");
    h.until("\"cmd\":\"onError\"", 5.0);
}

/// WA1：WXWebAssembly.instantiate(包内路径) 跑得起来（WebKit 的 WASM 带 JIT）
#[test]
fn web_wa1_wasm() {
    let _g = serial();
    let mut h = open!(WebHarness::open("g-wasm", WebOpts::default()));
    h.until("T:EV done", 20.0);
    h.assert_no_fail();
    assert!(!h.events("bench").is_empty());
}

/// P1~P4：分包（没加载前 require 不到、按名字 / root 加载、进度 100、只执行一次、入口抛错 → fail + onError）
#[test]
fn web_p1_p4_分包() {
    let _g = serial();
    let mut h = open!(WebHarness::open("g-subpkg", WebOpts::default()));
    h.until("T:READY", 10.0);
    h.eval("go('pre')");
    h.until("T:EV pre", 5.0);
    assert!(!h.events("pre")[0]["err"].as_str().unwrap().is_empty(), "没加载的分包里的模块 require 不到");
    for (step, tag) in [("a", "byName"), ("b", "byRoot"), ("c", "again"), ("d", "missing"), ("e", "broken")] {
        h.eval(&format!("go('{step}')"));
        h.until(&format!("\"tag\":\"{tag}\""), 5.0);
    }
    let l = |tag: &str| h.events("load").into_iter().find(|e| e["tag"] == tag).unwrap();
    assert_eq!((l("byName")["ok"].as_bool(), l("byName")["prog"].clone()), (Some(true), serde_json::json!([100])));
    assert_eq!(l("byRoot")["ok"], true);
    assert_eq!(l("again")["ok"], true);
    assert_eq!(l("missing")["ok"], false);
    assert_eq!(l("broken")["ok"], false);
    let ran = l("again")["ran"].clone();
    assert_eq!(ran.as_array().unwrap().iter().filter(|x| *x == "stage1:42").count(), 1, "分包入口只执行一次：{ran}");
}

/// A1：InnerAudioContext（解码、canplay → play → ended、currentTime 走、onError 的 errCode）与 WebAudio 解码
#[test]
fn web_a1_音频() {
    let _g = serial();
    // 0.3 秒 8kHz 单声道 16 位 PCM 的 wav
    let n = 2400u32;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + n * 2).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&[16, 0, 0, 0, 1, 0, 1, 0]);
    wav.extend_from_slice(&8000u32.to_le_bytes());
    wav.extend_from_slice(&16000u32.to_le_bytes());
    wav.extend_from_slice(&[2, 0, 16, 0]);
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(n * 2).to_le_bytes());
    for i in 0..n { wav.extend_from_slice(&(((i as f32 * 0.2).sin() * 8000.0) as i16).to_le_bytes()); }
    let js = r#"
var T = require('_lib/t.js');
var a = wx.createInnerAudioContext(), seen = [];
['Canplay', 'Play', 'Ended', 'TimeUpdate'].forEach(function (k) { a['on' + k](function () { if (seen.indexOf(k) < 0) seen.push(k); if (k === 'Ended') T.ev('ended', { seen: seen, dur: Math.round(a.duration * 10) / 10, paused: a.paused }); }); });
a.src = 'beep.wav';
a.play();
var bad = wx.createInnerAudioContext(); bad.onError(function (e) { T.ev('bad', e); }); bad.src = 'nope.mp3'; bad.play();
var ctx = wx.createWebAudioContext();
var fs = wx.getFileSystemManager();
ctx.decodeAudioData(fs.readFileSync('beep.wav'), function (buf) { T.ev('decoded', { ch: buf.numberOfChannels, len: Math.round(buf.duration * 10) / 10 }); }, function () { T.ev('decoded', { err: true }); });
"#;
    let mut h = open!(WebHarness::open_inline("audio", "{}", js, &[("beep.wav", &wav)], WebOpts::default()));
    h.until("T:EV ended", 10.0).until("T:EV bad", 5.0).until("T:EV decoded", 5.0);
    let e = &h.events("ended")[0];
    let seen: Vec<&str> = e["seen"].as_array().unwrap().iter().filter_map(|x| x.as_str()).collect();
    assert!(seen.starts_with(&["Canplay", "Play"]) && seen.contains(&"Ended"), "事件顺序：{seen:?}");
    assert_eq!((e["dur"].as_f64(), e["paused"].as_bool()), (Some(0.3), Some(true)), "{e}");
    assert_eq!(h.events("bad")[0]["errCode"], 10003, "{:?}", h.events("bad"));
    assert_eq!(h.events("decoded")[0], serde_json::json!({ "ch": 1, "len": 0.3 }));
}

fn need_lib(fixture: &str, lib: &str) -> bool {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample-game").join(fixture).join("libs").join(lib);
    if !p.exists() { eprintln!("⚠ 跳过 {fixture}：{} 不存在（bash tools/fetch-engines.sh）", p.display()); }
    p.exists()
}

/// E1：three.js r160 经 weapp-adapter（往 window 上挂 document / navigator / location 的那种）跑在页面里
#[test]
fn web_e1_three() {
    let _g = serial();
    if !need_lib("e-three", "three.js") { return; }
    let mut h = open!(WebHarness::open("e-three", WebOpts { query: "frames=30", ..Default::default() }));
    h.until("T:EV done", 30.0);
    assert!(h.errors().is_empty(), "{:?}", h.errors());
    let t = &h.events("three")[0];
    assert_eq!((t["rev"].as_str(), t["w"].as_u64(), t["h"].as_u64()), (Some("160"), Some(750), Some(1334)), "{t}");
    let png = h.chrome.screenshot().unwrap();
    let colors: std::collections::HashSet<_> = image::load_from_memory(&png).unwrap().to_rgb8().pixels().map(|p| p.0).collect();
    assert!(colors.len() > 200, "光照渐变，颜色数 {}", colors.len());
}

/// E2：PixiJS 7（WebGL 渲染器）：精灵、点击命中（点了变红）、Graphics 圆角矩形 —— 与 QuickJS 模式 e2_pixi 同样的像素
#[test]
fn web_e2_pixi() {
    let _g = serial();
    if !need_lib("e-pixi", "pixi.js") { return; }
    let mut h = open!(WebHarness::open("e-pixi", WebOpts { query: "dpr=1", ..Default::default() }));
    h.until("T:EV frames", 30.0);
    assert!(h.errors().is_empty(), "{:?}", h.errors());
    let p = &h.events("pixi")[0];
    assert_eq!((p["ver"].as_str(), p["type"].as_u64()), (Some("7.4.2"), Some(1)), "WebGL 渲染器：{p}");
    assert_eq!(h.pixel(100, 520), [0, 255, 0]);
    h.tap(100.0, 520.0);
    h.until("T:EV tap", 5.0).sleep(200);
    assert_eq!(h.pixel(100, 520), [255, 0, 0], "点了之后变红");
    assert_eq!(h.pixel(140, 180), [240, 160, 32], "Graphics 圆角矩形");
}

/// E3 / E4：Phaser 3.80（WebGL 与 Canvas 两种渲染器）：物理下落、像素、点击
#[test]
fn web_e3_e4_phaser() {
    let _g = serial();
    if !need_lib("e-phaser", "phaser.js") { return; }
    for ty in ["webgl", "canvas"] {
        let q: &'static str = if ty == "canvas" { "type=canvas" } else { "type=webgl" };
        let mut h = open!(WebHarness::open("e-phaser", WebOpts { query: q, ..Default::default() }));
        h.until("T:EV frames", 30.0);
        assert!(h.errors().is_empty(), "{ty}：{:?}", h.errors());
        let p = &h.events("phaser")[0];
        assert_eq!((p["ver"].as_str(), p["renderer"].as_str()), (Some("3.80.1"), Some(ty)), "{p}");
        assert!(h.events("frames")[0]["y"].as_i64().unwrap() > 100, "物理下落：{:?}", h.events("frames"));
        assert_eq!(h.pixel(140, 180), [240, 160, 32], "{ty}");
        h.tap(200.0, 300.0);
        h.until("T:EV tap", 5.0);
    }
}

/// 示例游戏（含萌宠小岛、3D 跑酷、保龄球、2048……）在页面里起得来：5 秒内没有报错、画面不是一片黑
#[test]
fn web_示例游戏都起得来() {
    let _g = serial();
    for g in ["play-2048", "play-flappy", "play-snake", "play-breakout", "play-jump", "play-fighter", "play-3d-stack", "play-3d-runner", "play-3d-bowling", "play-3d-island"] {
        if !std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample-game").join(g).is_dir() { continue; }
        let mut h = open!(WebHarness::open(g, WebOpts { query: "auto=1", ..Default::default() }));
        h.sleep(5000);
        let errs = h.errors();
        assert!(errs.is_empty(), "{g} 报错：{errs:?}\n{:?}", h.log.lock().unwrap().iter().rev().take(20).collect::<Vec<_>>());
        let png = h.chrome.screenshot().unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgb8();
        let colors: std::collections::HashSet<_> = img.pixels().map(|p| p.0).collect();
        assert!(colors.len() > 20, "{g} 画面颜色数 {}（黑屏？）", colors.len());
        eprintln!("✓ {g}：{} 种颜色", colors.len());
    }
}

