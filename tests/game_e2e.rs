#![cfg(feature = "game")]
//! 小游戏 e2e：驱动 GameEngine 跑 sample-game/ 下的夹具，断言事件序列与日志（game-doc/05、08）。
//!
//! 为什么只有一个文件：每个 tests/*.rs 是独立二进制（约 75MB）外加一份增量缓存，还要单独链接一次
//! （steering 30）。为什么串行：包根目录（assets::APP_ROOT）与数据目录是进程级的，两个用例并行会互相读到
//! 对方的 wxfile / storage。

mod game_harness;
use game_harness::*;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// GT-R1 / R5 / S4：game.js 执行一次，console、onError、onUnhandledRejection、GameGlobal
#[test]
fn r1_g_hello_运行模式骨架() {
    let _g = serial();
    let mut h = GameHarness::open("g-hello", Opts::default());
    h.until("T:DONE", 200);
    h.assert_no_fail();
    assert!(h.has("hello 1 true null undefined"), "console.log 没交给宿主：{:#?}", h.log);
    assert!(h.has("warn-line"));
    assert!(h.has("boom-for-test"));
    assert_eq!(h.log.iter().filter(|l| l.contains("T:READY g-hello")).count(), 1, "game.js 只执行一次");
    let un = h.events("unhandled");
    assert_eq!(un.len(), 1, "只有真正没人处理的拒绝才派发（handled-late 不算）：{un:?}");
    assert_eq!(un[0]["reason"], "nope");
    let err = h.events("onError");
    assert!(err.iter().any(|e| e["msg"] == "in-timer"), "定时器里抛的错要交给 wx.onError：{err:?}");
}

/// GT-R5：game.js 顶层抛错不算创建失败；实例照常可以 pump，宿主收到 error 事件，行号指向真实行
#[test]
fn r5_顶层抛错_实例仍可用且行号不偏() {
    let _g = serial();
    let mut h = GameHarness::open_inline("throw", "var a = 1;\nnull.x;\n", Opts::default());
    assert_eq!(h.errors.len(), 1, "{:?}", h.errors);
    assert_eq!(h.errors[0]["kind"], "script");
    let stack = h.errors[0]["stack"].as_str().unwrap_or("");
    assert!(stack.contains(":2"), "栈里的行号应当是源文件的第 2 行（包装头不占行）：{stack}");
    h.vsyncs(3);
    assert_eq!(h.engine.eval("1 + 1").unwrap(), "2");
}

/// GT-S1：rAF / 定时器回调里的死循环被看门狗中断，下一帧照常
#[test]
fn s1_看门狗中断死循环() {
    let _g = serial();
    let mut h = GameHarness::open_inline(
        "spin",
        "var n = 0; setInterval(function () { n++; if (n === 2) { while (true) {} } console.log('T:FRAME ' + n + ' {}'); }, 0);",
        Opts { watchdog_ms: 200, ..Default::default() },
    );
    let t0 = std::time::Instant::now();
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.vsyncs(1);
    }
    assert!(t0.elapsed().as_millis() < 3000, "看门狗没生效：{:?}", t0.elapsed());
    assert!(h.errors.iter().any(|e| e["kind"] == "watchdog"), "{:?}", h.errors);
    assert!(h.frames().iter().any(|(n, _)| *n >= 3), "中断之后定时器没有继续：{:?}", h.log);
}

/// GT-S6：一帧 1 万行日志只交出 500 行 + 一行丢弃计数
#[test]
fn s6_日志限流() {
    let _g = serial();
    let mut h = GameHarness::open_inline("flood", "setTimeout(function () { for (var i = 0; i < 10000; i++) console.log('L' + i); }, 0);", Opts::default());
    for _ in 0..5 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        h.vsyncs(1);
    }
    let lines = h.log.iter().filter(|l| l.starts_with('L')).count();
    assert_eq!(lines, 500, "每帧最多 500 行");
    assert!(h.has("丢弃 9500 行"), "{:?}", h.log.iter().rev().take(3).collect::<Vec<_>>());
}

/// GT-S8（第一部分）：console 不再往永远没人读的 __console_buffer 里堆
#[test]
fn s8_console_缓冲不再泄漏() {
    let _g = serial();
    let mut h = GameHarness::open_inline("leak", "for (var i = 0; i < 20000; i++) console.log('x');", Opts::default());
    h.vsyncs(1);
    let n = h.engine.eval("typeof __console_buffer.length === 'number' ? String(__console_buffer.length) : 'none'").unwrap();
    assert_eq!(n, "none", "小游戏的 __console_buffer 应当是丢弃式容器");
}

/// R7（部分）：小程序目录不能按小游戏打开，小游戏目录不能按小程序打开
#[test]
fn r7_包种类互斥() {
    let _g = serial();
    let mini = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/sample-app");
    if mini.join("app.json").is_file() {
        let e = mini_render::game::GameEngine::new(mini.to_str().unwrap(), None, 375, 667, 2.0, Default::default()).err().unwrap();
        assert!(e.contains("是小程序"), "{e}");
    }
    let game = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample-game/g-hello");
    let e = mini_render::host::MiniEngine::new(game.to_str().unwrap(), None, 375, 667, 2.0).err().unwrap();
    assert!(e.contains("是小游戏"), "{e}");
}

/// GT-B1 / B2：二进制往返（各种大小、byteOffset ≠ 0 的子视图、DataView），与老的纯 JS base64 输出逐字节相同
#[test]
fn b1_二进制往返与子视图() {
    let _g = serial();
    let mut h = GameHarness::open_inline("bin", r#"
        var T = require('_lib/t.js');
        [0, 1, 2, 3, 4095, 65536, 16 << 20].forEach(function (n) {
          var a = new Uint8Array(n + 8); for (var i = 0; i < a.length; i++) a[i] = (i * 31 + 7) & 255;
          var sub = new Uint8Array(a.buffer, 3, n);                 // byteOffset ≠ 0
          var b64 = wx.arrayBufferToBase64(sub);
          var back = new Uint8Array(wx.base64ToArrayBuffer(b64));
          var same = back.length === n; for (var j = 0; same && j < n; j++) same = back[j] === sub[j];
          T.check('roundtrip' + n, same, 'len ' + back.length);
          if (n <= 4095) T.check('same-as-codec' + n, b64 === __b64encode(sub), b64.slice(0, 20));
        });
        var dv = new DataView(new Uint8Array([1, 2, 3, 4, 5]).buffer, 1, 3);
        T.check('dataview', wx.arrayBufferToBase64(dv) === 'AgME');
        T.check('b64-loose', new Uint8Array(wx.base64ToArrayBuffer('Ag\nME')).join() === '2,3,4');
        var threw = ''; try { wx.arrayBufferToBase64('nope'); } catch (e) { threw = e.name; }
        T.check('type-error', threw === 'TypeError', threw);
        T.done({});
    "#, Opts::default());
    h.vsyncs(1);
    assert!(h.has("T:DONE"), "{:?}", h.log.iter().rev().take(5).collect::<Vec<_>>());
    h.assert_no_fail();
    assert_eq!(h.log.iter().filter(|l| l.starts_with("T:OK roundtrip")).count(), 7);
}

/// GT-B5 / 10 矩阵「本轮补入」：wx.encode / wx.decode 的官方 format 全集
#[test]
fn b5_wx_encode_decode() {
    let _g = serial();
    let mut h = GameHarness::open_inline("codec", r#"
        var T = require('_lib/t.js');
        var s = '我爱 Beijing 天安门';
        ['utf8', 'utf-8', 'ucs2', 'ucs-2', 'utf16le', 'utf-16le', 'gbk'].forEach(function (f) {
          T.check('rt-' + f, wx.decode({ data: wx.encode({ data: s, format: f }), format: f }) === s);
        });
        T.check('default-utf8', wx.encode({ data: 'é' }).byteLength === 2);
        T.check('gbk-bytes', new Uint8Array(wx.encode({ data: '我', format: 'gbk' })).join() === '206,210');
        T.check('latin1', wx.decode({ data: new Uint8Array([0xe9]).buffer, format: 'latin1' }) === 'é');
        var e1 = ''; try { wx.encode({ data: 'x', format: 'ascii' }); } catch (e) { e1 = e.name; }
        T.check('bad-format', e1 === 'TypeError', e1);
        // TextEncoder 不由我们加（取证前不多给全局）；atob / btoa 是 QuickJS 自带的（game-doc/01 §1.2 更正）
        T.check('no-textencoder-global', typeof TextEncoder === 'undefined' && typeof TextDecoder === 'undefined');
        T.done({});
    "#, Opts::default());
    h.vsyncs(1);
    assert!(h.has("T:DONE"));
    h.assert_no_fail();
}

/// GT-B3：超过单次过桥上限抛 RangeError，进程不崩
#[test]
fn b3_过桥上限() {
    let _g = serial();
    let mut h = GameHarness::open_inline("big", r#"
        var T = require('_lib/t.js');
        var name = ''; try { wx.arrayBufferToBase64(new ArrayBuffer(256 * 1024 * 1024 + 1)); } catch (e) { name = e.name; }
        T.check('range', name === 'RangeError', name);
        T.done({});
    "#, Opts::default());
    h.vsyncs(1);
    assert!(h.has("T:DONE"));
    h.assert_no_fail();
}

/// 1MB 往返的耗时（GT-B2 / 04 A1.3 的数字；断言放宽到 50ms 防机器忙时误报，实测值打印出来）
#[test]
fn b2_1mb_往返耗时() {
    let _g = serial();
    let mut h = GameHarness::open_inline("speed", r#"
        var a = new Uint8Array(1 << 20); for (var i = 0; i < a.length; i++) a[i] = i & 255;
        var t0 = Date.now(); for (var k = 0; k < 10; k++) wx.base64ToArrayBuffer(wx.arrayBufferToBase64(a));
        console.log('T:EV speed ' + JSON.stringify({ ms: (Date.now() - t0) / 10 }));
    "#, Opts::default());
    h.vsyncs(1);
    let ms = h.events("speed")[0]["ms"].as_f64().unwrap();
    println!("1MB base64 往返平均 {ms}ms");
    assert!(ms < 50.0, "1MB 往返 {ms}ms");
}

/// GT-FS6（09 §2.2）：两个 appId 不同、解包目录同名的游戏数据互不可见；同一 appId 换目录名后数据仍在
#[test]
fn fs6_数据按_appid_隔离() {
    let _g = serial();
    use mini_render::game::{GameEngine, GameOptions};
    let base = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("fs6-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let data = base.join("data");
    let mk = |dir: &str| {
        let d = base.join(dir).join("game"); // 两个包解包目录都叫 game
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("game.json"), "{}").unwrap();
        std::fs::write(d.join("game.js"), "").unwrap();
        d
    };
    let (a, b, a2) = (mk("a"), mk("b"), mk("a-renamed"));
    let open = |dir: &std::path::Path, id: &str| {
        let o = GameOptions { app_id: Some(id.into()), ..Default::default() };
        GameEngine::new(dir.to_str().unwrap(), Some(data.to_str().unwrap()), 375, 667, 2.0, o).unwrap()
    };
    let fs_write = "wx.getFileSystemManager().writeFileSync(wx.env.USER_DATA_PATH + '/save.txt', 'A', 'utf8')";
    let fs_read = "(function(){try{return wx.getFileSystemManager().readFileSync(wx.env.USER_DATA_PATH + '/save.txt', 'utf8')}catch(e){return 'none'}})()";
    {
        let mut g = open(&a, "wxAAA");
        g.eval("wx.setStorageSync('k', 'from-A')").unwrap();
        g.eval(fs_write).unwrap();
    }
    {
        let mut g = open(&b, "wxBBB");
        assert_eq!(g.eval("String(wx.getStorageSync('k'))").unwrap(), "", "B 不该看到 A 的 storage");
        assert_eq!(g.eval(fs_read).unwrap(), "none", "B 不该看到 A 的文件");
    }
    {
        let mut g = open(&a2, "wxAAA");
        assert_eq!(g.eval("String(wx.getStorageSync('k'))").unwrap(), "from-A", "同一 appId 换目录名后 storage 应当还在");
        assert_eq!(g.eval(fs_read).unwrap(), "A");
    }
    assert!(data.join("game/wxAAA/storage.json").is_file());
    // appId 里的路径字符被换掉，写不出数据目录
    {
        let _g2 = open(&b, "../../evil");
    }
    assert!(data.join("game/______evil").is_dir(), "../../evil 应当变成 ______evil");
    assert!(!base.join("evil").exists());
    let _ = std::fs::remove_dir_all(&base);
}

/// GT-R9（09 §2.3）：platform 按编译目标、不能被宿主改成 devtools；SDKVersion 是常量
#[test]
fn r9_系统信息_platform_与_sdkversion() {
    let _g = serial();
    let mut h = GameHarness::open_inline("sys", "", Opts::default());
    let p = h.engine.eval("wx.getSystemInfoSync().platform").unwrap();
    assert_eq!(p, mini_render::game::target_platform());
    if cfg!(target_os = "macos") {
        assert_eq!(p, "mac");
    }
    assert_eq!(h.engine.eval("wx.getSystemInfoSync().SDKVersion").unwrap(), mini_render::game::GAME_SDK_VERSION);
    assert_eq!(h.engine.eval("wx.getSystemInfoSync().deviceOrientation").unwrap(), "portrait");
}

/// GT-API1（11 §4）：官方小游戏 API 总览里的 382 个 `wx.*` —— 游戏实例里缺的必须正好是延期清单里的那些
#[test]
fn api1_官方_api_表面门禁() {
    let _g = serial();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("game-doc/取证");
    let idx: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("official-api-index-2026-10-26.json")).unwrap()).unwrap();
    let names: Vec<String> = idx["api"].as_object().unwrap().keys().map(|k| k.trim_start_matches("wx.").to_string()).collect();
    assert_eq!(names.len(), 382);
    let def: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("api-surface-deferred.json")).unwrap()).unwrap();
    let deferred: std::collections::BTreeSet<String> =
        def["deferred"].as_object().unwrap().values().flat_map(|v| v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string())).collect();
    let mut h = GameHarness::open_inline("api", "", Opts::default());
    let js = format!("JSON.stringify({}.filter(function (k) {{ return typeof wx[k] === 'undefined'; }}))", serde_json::to_string(&names).unwrap());
    let missing: std::collections::BTreeSet<String> = serde_json::from_str(&h.engine.eval(&js).unwrap()).unwrap();
    let not_listed: Vec<_> = missing.difference(&deferred).collect();
    let stale: Vec<_> = deferred.difference(&missing).collect();
    assert!(not_listed.is_empty(), "这些官方 API 在游戏实例里不存在，也不在延期清单里：{not_listed:?}");
    assert!(stale.is_empty(), "这些已经实现了，从 api-surface-deferred.json 里删掉：{stale:?}");
    println!("官方 API {}：已有 {}，延期 {}", names.len(), names.len() - missing.len(), missing.len());
    // 宿主没接的代理 API：存在、canIUse 为 false、调用后 fail（GT-H1 的抽样）
    assert_eq!(h.engine.eval("String(wx.canIUse('getFriendCloudStorage'))").unwrap(), "false");
    assert_eq!(h.engine.eval("String(wx.canIUse('view'))").unwrap(), "false", "小游戏没有 WXML 组件");
    assert_eq!(h.engine.eval("String(wx.canIUse('getSystemInfoSync'))").unwrap(), "true");
    h.engine.eval("wx.getFriendCloudStorage({ keyList: ['s'], fail: function (e) { console.log('T:EV hfail ' + JSON.stringify(e)); } })").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    h.vsyncs(2);
    let ev = h.events("hfail");
    assert_eq!(ev.len(), 1);
    assert!(ev[0]["errMsg"].as_str().unwrap().starts_with("getFriendCloudStorage:fail"), "{ev:?}");
}
