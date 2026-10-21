//! 第 10 批的 `wx.*`（G-10）：WebSocket、FileSystemManager、InnerAudioContext，以及
//! canIUse / 胶囊 / 网络类型 / getImageInfo / compressImage / setBackgroundColor / hideKeyboard。
//!
//! 依据官方 api/network/websocket、api/file/FileSystemManager、api/media/audio/InnerAudioContext、
//! api/base/wx.canIUse、api/ui/menu/wx.getMenuButtonBoundingClientRect、api/ui/background/wx.setBackgroundColor。
//! 走 `MiniEngine`（SDK 链路）；WebSocket 对端是本进程里起的 tungstenite 回声服务。
use base64::Engine as _;
use mini_render::host::MiniEngine;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static SERIAL: Mutex<()> = Mutex::new(());

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn fixture(name: &str, wxml: &str, wxss: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("io-api-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", wxss),
        ("pages/index/index.js", "Page({ data: { log: [], v: '' }, rec(x) { this.data.log.push(x); }, onIn(e) {}, onBlur(e) { this.rec('blur:' + e.detail.value); } });"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    std::fs::create_dir_all(root.join("img")).unwrap();
    image::RgbaImage::from_pixel(40, 20, image::Rgba([255, 0, 0, 255])).save(root.join("img/red.png")).unwrap();
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
    fn log(&self) -> Value {
        self.js("__currentPage.data.log")
    }
    /// 一直出帧，直到 `cond` 成立或超时（墙上时钟：线程 / 播放时钟都是真实时间）
    fn until(&mut self, secs: f64, cond: impl Fn(&Value) -> bool) -> Value {
        let t0 = Instant::now();
        loop {
            self.frames(1);
            let log = self.log();
            if cond(&log) || t0.elapsed().as_secs_f64() > secs {
                return log;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn px(&mut self, x: usize, y: usize) -> (u8, u8, u8) {
        let (w, _) = self.e.pixel_size();
        let p = self.e.rgba();
        let i = (y * 2 * w as usize + x * 2) * 4;
        (p[i], p[i + 1], p[i + 2])
    }
}

fn has(log: &Value, s: &str) -> bool {
    log.as_array().map(|a| a.iter().any(|x| x.as_str() == Some(s))).unwrap_or(false)
}

/// 本地回声服务：文本原样回、二进制原样回；收到关闭帧按协议回关闭帧
fn echo_server() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let Ok(mut ws) = tungstenite::accept(stream) else { return };
                while let Ok(msg) = ws.read() {
                    if msg.is_text() || msg.is_binary() {
                        if ws.send(msg).is_err() {
                            break;
                        }
                    }
                }
            });
        }
    });
    port
}

/// SocketTask：onOpen → 发文本与二进制 → 两条回声（二进制还原成 ArrayBuffer）→ close(4000) → onClose 带码
#[test]
fn websocket_round_trip_text_binary_and_close() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let port = echo_server();
    let fx = fixture("ws", "<view>ws</view>", "");
    let mut e = E::new(&fx);
    e.run(&format!(
        r#"var p = __currentPage;
        var t = wx.connectSocket({{ url: 'ws://127.0.0.1:{port}/echo', success: function() {{ p.rec('connect:ok'); }} }});
        t.onOpen(function(r) {{ p.rec('open'); t.send({{ data: 'hello' }}); t.send({{ data: new Uint8Array([1, 2, 250]).buffer }}); }});
        t.onMessage(function(m) {{
            if (typeof m.data === 'string') p.rec('text:' + m.data);
            else p.rec('bin:' + Array.prototype.join.call(new Uint8Array(m.data), ','));
            if (p.data.log.filter(function(x) {{ return /^(text|bin):/.test(x); }}).length === 2) t.close({{ code: 4000, reason: 'bye' }});
        }});
        t.onClose(function(r) {{ p.rec('close:' + r.code + ':' + r.reason); }});
        t.onError(function(r) {{ p.rec('error:' + r.errMsg); }});"#
    ));
    let log = e.until(5.0, |l| l.as_array().map(|a| a.iter().any(|x| x.as_str().unwrap_or("").starts_with("close:"))).unwrap_or(false));
    assert!(has(&log, "connect:ok"), "{log}");
    assert!(has(&log, "open"), "{log}");
    assert!(has(&log, "text:hello"), "{log}");
    assert!(has(&log, "bin:1,2,250"), "二进制帧还原成 ArrayBuffer：{log}");
    assert!(has(&log, "close:4000:bye"), "对端按协议回关闭帧，码与原因带回来：{log}");
    assert_eq!(e.js("typeof wx.onSocketOpen === 'function' && typeof wx.sendSocketMessage === 'function'"), true);
}

/// 连不上：onError 后 onClose(1006)；非 ws 地址 connectSocket 本身 fail
#[test]
fn websocket_connect_failures() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // 拿一个确定没人监听的端口
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let fx = fixture("ws-fail", "<view>ws</view>", "");
    let mut e = E::new(&fx);
    e.run(&format!(
        r#"var p = __currentPage;
        var t = wx.connectSocket({{ url: 'ws://127.0.0.1:{port}/' }});
        t.onError(function() {{ p.rec('error'); }});
        t.onClose(function(r) {{ p.rec('close:' + r.code); }});
        wx.connectSocket({{ url: 'http://x', fail: function(r) {{ p.rec('bad:' + r.errMsg); }} }});"#
    ));
    let log = e.until(5.0, |l| has(l, "close:1006"));
    assert!(has(&log, "error") && has(&log, "close:1006"), "{log}");
    assert!(log.as_array().unwrap().iter().any(|x| x.as_str().unwrap_or("").starts_with("bad:connectSocket:fail")), "{log}");
}

/// FileSystemManager：USER_DATA_PATH 可读写；代码包只读；错误信息；ArrayBuffer 往返；目录操作；unzip 不存在的包报错
#[test]
fn file_system_manager_semantics() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture("fs", "<view>fs</view>", "");
    let mut e = E::new(&fx);
    let r = e.js(
        r#"(function() {
        var fs = wx.getFileSystemManager(), u = wx.env.USER_DATA_PATH, out = {};
        fs.writeFileSync(u + '/a.txt', '你好 world', 'utf8');
        out.text = fs.readFileSync(u + '/a.txt', 'utf8');
        out.b64 = fs.readFileSync(u + '/a.txt', 'base64');
        fs.appendFileSync(u + '/a.txt', '!', 'utf8');
        out.appended = fs.readFileSync(u + '/a.txt', 'utf8');
        fs.writeFileSync(u + '/b.bin', new Uint8Array([0, 255, 7]).buffer);
        var buf = fs.readFileSync(u + '/b.bin');
        out.binIsBuffer = buf instanceof ArrayBuffer;
        out.bin = Array.prototype.join.call(new Uint8Array(buf), ',');
        fs.mkdirSync(u + '/d/e', true);
        fs.writeFileSync(u + '/d/e/f.txt', 'x');
        out.readdir = fs.readdirSync(u + '/d');
        out.isDir = fs.statSync(u + '/d').isDirectory();
        out.size = fs.statSync(u + '/d/e/f.txt').size;
        out.recursive = fs.statSync(u + '/d', true).map(function(x) { return x.path; });
        try { fs.writeFileSync('/pages/x.txt', 'no'); out.pkgWrite = 'wrote'; } catch (e) { out.pkgWrite = e.message; }
        try { fs.readFileSync(u + '/nope.txt', 'utf8'); } catch (e) { out.missing = e.message; }
        out.pkgRead = fs.readFileSync('/app.json', 'utf8').indexOf('pages') >= 0;
        try { fs.rmdirSync(u + '/d'); out.rmdirNonEmpty = 'removed'; } catch (e) { out.rmdirNonEmpty = e.message; }
        fs.rmdirSync(u + '/d', true);
        try { fs.accessSync(u + '/d'); out.gone = false; } catch (e) { out.gone = true; }
        fs.renameSync(u + '/a.txt', u + '/c.txt');
        out.renamed = fs.readFileSync(u + '/c.txt', 'utf8');
        fs.unlinkSync(u + '/c.txt');
        var fd = fs.openSync({ filePath: u + '/fd.txt', flag: 'w+' });
        fs.writeSync({ fd: fd, data: 'abcdef' });
        var ab = new ArrayBuffer(3);
        out.readSync = fs.readSync({ fd: fd, arrayBuffer: ab, position: 2, length: 3 }).bytesRead;
        out.readBytes = String.fromCharCode.apply(null, new Uint8Array(ab));
        fs.closeSync({ fd: fd });
        return out;
    })()"#,
    );
    assert_eq!(r["text"], "你好 world");
    assert_eq!(r["b64"], base64::engine::general_purpose::STANDARD.encode("你好 world"));
    assert_eq!(r["appended"], "你好 world!");
    assert_eq!(r["binIsBuffer"], true);
    assert_eq!(r["bin"], "0,255,7");
    assert_eq!(r["readdir"], serde_json::json!(["e"]));
    assert_eq!(r["isDir"], true);
    assert_eq!(r["size"], 1);
    assert_eq!(r["recursive"], serde_json::json!(["/", "/e", "/e/f.txt"]));
    assert!(r["pkgWrite"].as_str().unwrap().starts_with("writeFileSync:fail permission denied"), "{}", r["pkgWrite"]);
    assert!(r["missing"].as_str().unwrap().contains("no such file or directory"), "{}", r["missing"]);
    assert_eq!(r["pkgRead"], true, "代码包文件可读");
    assert!(r["rmdirNonEmpty"].as_str().unwrap().contains("directory not empty"), "{}", r["rmdirNonEmpty"]);
    assert_eq!(r["gone"], true);
    assert_eq!(r["renamed"], "你好 world!");
    assert_eq!(r["readSync"], 3);
    assert_eq!(r["readBytes"], "cde");

    // 异步版：回调在下一轮事件循环，带 errMsg
    e.run(
        r#"var p = __currentPage, fs = wx.getFileSystemManager();
        fs.writeFile({ filePath: wx.env.USER_DATA_PATH + '/z.txt', data: 'zz', success: function(r) { p.rec(r.errMsg); } });
        fs.readFile({ filePath: wx.env.USER_DATA_PATH + '/none', fail: function(r) { p.rec('fail'); } });
        fs.getFileInfo({ filePath: wx.env.USER_DATA_PATH + '/z.txt', success: function(r) { p.rec('md5:' + r.digest); } });
        fs.unzip({ zipFilePath: wx.env.USER_DATA_PATH + '/z.txt', targetPath: wx.env.USER_DATA_PATH + '/out', fail: function(r) { p.rec('unzip:' + r.errMsg); } });"#,
    );
    let log = e.log();
    assert!(has(&log, "writeFile:ok"), "{log}");
    assert!(has(&log, "fail"), "{log}");
    // md5("zz")
    assert!(has(&log, "md5:25ed1bcb423b0b7200f485fc5ff71c8e"), "{log}");
    assert!(has(&log, "unzip:unzip:fail not a zip file"), "{log}");
}

/// InnerAudioContext：自动播放一段 0.4s 的 WAV → canplay / play / timeUpdate / ended，状态读得到
#[test]
fn inner_audio_context_plays_to_end() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture("audio", "<view>audio</view>", "");
    let mut e = E::new(&fx);
    let wav = base64::engine::general_purpose::STANDARD.encode(mini_render::audio_ctx::decode::make_wav(8000, 1, 0.4));
    e.run(&format!(
        r#"var p = __currentPage, fs = wx.getFileSystemManager();
        fs.writeFileSync(wx.env.USER_DATA_PATH + '/beep.wav', '{wav}', 'base64');
        var a = wx.createInnerAudioContext();
        ['Canplay', 'Play', 'Pause', 'TimeUpdate', 'Ended', 'Error'].forEach(function(n) {{ a['on' + n](function(r) {{ p.rec(n); }}); }});
        a.autoplay = true;
        a.src = wx.env.USER_DATA_PATH + '/beep.wav';
        globalThis.__a = a;"#
    ));
    let log = e.until(5.0, |l| has(l, "Ended") || has(l, "Error"));
    assert!(has(&log, "Canplay") && has(&log, "Play") && has(&log, "TimeUpdate") && has(&log, "Ended"), "{log}");
    assert!(!has(&log, "Error"), "{log}");
    let d = e.js("__a.duration").as_f64().unwrap();
    assert!((d - 0.4).abs() < 0.01, "duration {d}");
    assert_eq!(e.js("__a.paused"), true);
    // 不存在的文件：onError，码 10003
    e.run(r#"var b = wx.createInnerAudioContext(); b.onError(function(r) { __currentPage.rec('err:' + r.errCode); }); b.src = '/nope.mp3';"#);
    let log = e.until(3.0, |l| has(l, "err:10003"));
    assert!(has(&log, "err:10003"), "{log}");
}

/// canvas 2d（G-9）：渐变填充、合成模式、getImageData 当场读到刚画的、measureText 真实量宽、
/// toDataURL、canvasToTempFilePath 落 wxfile
#[test]
fn canvas_2d_gradient_composite_and_pixel_readback() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture("canvas2d", "<view>c</view>", "");
    let mut e = E::new(&fx);
    let r = e.js(
        r#"(function() {
        var n = __canvas2dNode({ id: 'cv', width: 100, height: 50 }), ctx = n.getContext('2d'), out = {};
        var g = ctx.createLinearGradient(0, 0, 100, 0);
        g.addColorStop(0, '#ff0000'); g.addColorStop(1, '#0000ff');
        ctx.fillStyle = g;
        ctx.fillRect(0, 0, 100, 50);
        var d = ctx.getImageData(0, 0, 100, 1).data;
        out.left = [d[0], d[2]]; out.right = [d[99 * 4], d[99 * 4 + 2]]; out.mid = [d[50 * 4], d[50 * 4 + 2]];
        ctx.globalCompositeOperation = 'destination-out';
        ctx.fillStyle = '#000000';
        ctx.fillRect(0, 0, 50, 50);
        out.erased = ctx.getImageData(10, 10, 1, 1).data[3];
        out.kept = ctx.getImageData(80, 10, 1, 1).data[3];
        out.wide = ctx.measureText('你好你好').width > ctx.measureText('你好').width * 1.8;
        out.url = n.toDataURL().slice(0, 22);
        wx.canvasToTempFilePath({ canvas: n, success: function(r) { __currentPage.rec('tmp:' + /^wxfile:\/\//.test(r.tempFilePath)); } });
        return out;
    })()"#,
    );
    let l = r["left"].as_array().unwrap();
    assert!(l[0].as_u64().unwrap() > 240 && l[1].as_u64().unwrap() < 15, "左端红：{r}");
    let rr = r["right"].as_array().unwrap();
    assert!(rr[1].as_u64().unwrap() > 240 && rr[0].as_u64().unwrap() < 15, "右端蓝：{r}");
    let m = r["mid"].as_array().unwrap();
    assert!((m[0].as_i64().unwrap() - 127).abs() < 8, "中间一半一半：{r}");
    assert_eq!(r["erased"], 0, "destination-out 挖掉左半");
    assert_eq!(r["kept"], 255);
    assert_eq!(r["wide"], true, "measureText 按真实字体量");
    assert_eq!(r["url"], "data:image/png;base64,");
    e.frames(3);
    assert!(has(&e.log(), "tmp:true"), "{}", e.log());
}

/// 杂项：canIUse、胶囊、网络类型、getImageInfo、compressImage、setBackgroundColor、hideKeyboard
#[test]
fn misc_info_apis() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let fx = fixture(
        "misc",
        r#"<view style="height:100px"></view><input id="in" style="height:40px" value="abc" bindinput="onIn" bindblur="onBlur"/>"#,
        "",
    );
    let mut e = E::new(&fx);
    assert_eq!(e.js("wx.canIUse('connectSocket')"), true);
    assert_eq!(e.js("wx.canIUse('getSystemInfoSync.return.screenWidth')"), true);
    assert_eq!(e.js("wx.canIUse('swiper.indicator-dots')"), true);
    assert_eq!(e.js("wx.canIUse('login')"), false, "立刻 fail 的默认实现不算支持");
    assert_eq!(e.js("wx.canIUse('startBluetoothDevicesDiscovery')"), false);
    let mb = e.js("wx.getMenuButtonBoundingClientRect()");
    assert_eq!((mb["width"].as_f64(), mb["height"].as_f64()), (Some(87.0), Some(32.0)));
    assert_eq!(mb["right"].as_f64(), Some(375.0 - 7.0));
    assert_eq!(mb["top"].as_f64().unwrap(), e.js("wx.getSystemInfoSync().statusBarHeight").as_f64().unwrap() + 4.0);

    e.run(
        r#"var p = __currentPage;
        wx.getNetworkType({ success: function(r) { p.rec('net:' + r.networkType); } });
        wx.onNetworkStatusChange(function(r) { p.rec('change:' + r.networkType + ':' + r.isConnected); });
        __dispatchNetworkStatus('none');
        wx.getImageInfo({ src: '/img/red.png', success: function(r) { p.rec('img:' + r.width + 'x' + r.height + ':' + r.type); } });
        wx.compressImage({ src: '/img/red.png', quality: 50, compressedWidth: 20, success: function(r) {
            wx.getImageInfo({ src: r.tempFilePath, success: function(i) { p.rec('cmp:' + i.width + 'x' + i.height + ':' + i.type); } });
        } });
        wx.getUpdateManager().onCheckForUpdate(function(r) { p.rec('update:' + r.hasUpdate); });"#,
    );
    let log = e.until(3.0, |l| has(l, "cmp:20x10:jpeg"));
    for want in ["net:wifi", "change:none:false", "img:40x20:png", "cmp:20x10:jpeg", "update:false"] {
        assert!(has(&log, want), "缺 {want}：{log}");
    }

    // setBackgroundColor：页面没写 page 背景时，帧底色换成它（页面内容下方的空白处）
    let before = e.px(200, 600);
    assert_eq!(before, (0xF5, 0xF5, 0xF5));
    e.run("wx.setBackgroundColor({ backgroundColor: '#123456' })");
    assert_eq!(e.px(200, 600), (0x12, 0x34, 0x56));

    // hideKeyboard：聚焦的输入框失焦，bindblur 带当前值
    e.e.pointer_down(100.0, 120.0);
    e.frames(1);
    e.e.pointer_up(100.0, 120.0);
    e.frames(3);
    e.run("wx.hideKeyboard()");
    let log = e.log();
    assert!(has(&log, "blur:abc"), "{log}");
}
