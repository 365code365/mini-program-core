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
