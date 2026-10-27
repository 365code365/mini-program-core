//! 小游戏的 C ABI（`mr_game_*`）与能力查询 `mr_capabilities`（game-doc/02 §13、09 §2.1）。
//!
//! ## 为什么是单独的句柄 `MRGame`，不是在 `MRApp` 里按种类分派
//! 计划第一版写的是 `MRApp { kind: Mini | Game }`。落地时改成独立句柄：
//! - 小程序的 46 处 `a.engine.*` 一处都不用动（零回归风险，小程序 FFI 逐字节不变）；
//! - 只对小程序有意义的函数（navigate / back / 分栏 / 同层视频帧）在类型上就拿不到游戏句柄，
//!   不用一个个约定「游戏实例上返回 0」；
//! - 宿主本来就要按包种类建不同的视图（`MiniGameView`），多一组函数名不增加负担。
//!   `mr_app_kind` 因此不需要了：宿主先用 `mr_package_kind(dir)` 判包种类，再决定调哪个 create。
//!
//! ## panic 不能穿过 FFI
//! Rust 1.81 起，panic 穿过 `extern "C"` 会直接 abort 宿主进程。小游戏跑第三方代码，畸形参数很常见，
//! 所以每个入口都包 `catch_unwind`。捕获后实例进入 **poisoned**：引擎状态可能半途而废（Mutex 也可能中毒），
//! 之后的调用一律返回失败值，`mr_game_last_error` 给出 panic 位置，事件队列里留一条 `error{kind:"panic"}`，
//! 由宿主决定销毁重建。不在同一实例上继续跑。

use crate::game::{GameEngine, GameOptions};
use std::ffi::{c_char, c_int, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};

pub struct MRGame {
    engine: Option<GameEngine>,
    poisoned: bool,
    last_error: String,
    last_error_buf: CString,
    event_buf: CString,
    eval_buf: CString,
    /// poisoned 之后还要交给宿主的那条 panic 事件
    panic_event: Option<String>,
}

fn cstr(p: *const c_char) -> Option<String> {
    crate::ffi_app::cstr(p)
}

/// 包种类：0 = 认不出（没有 / 两个都有），1 = 小程序（app.json），2 = 小游戏（game.json）
#[no_mangle]
pub extern "C" fn mr_package_kind(dir: *const c_char) -> c_int {
    let Some(d) = cstr(dir) else { return 0 };
    match crate::app_profile::detect(std::path::Path::new(&d)) {
        Ok(crate::app_profile::AppProfile::MiniProgram) => 1,
        Ok(crate::app_profile::AppProfile::MiniGame) => 2,
        Err(_) => 0,
    }
}

/// 本库编进了哪些能力（JSON，静态字符串，不用释放）：宿主据此决定是否展示小游戏入口
#[no_mangle]
pub extern "C" fn mr_capabilities() -> *const c_char {
    static CAPS: std::sync::OnceLock<CString> = std::sync::OnceLock::new();
    CAPS.get_or_init(|| {
        let v = serde_json::json!({
            "game": true,
            // 下面三项随实现进度改（G8 WebGL、G5 移动端音频、G10 wasm）
            "webgl": false,
            "audio": cfg!(feature = "audio"),
            "wasm": false,
            "gpu": cfg!(feature = "gpu"),
            "version": env!("CARGO_PKG_VERSION"),
            "sdkVersion": crate::game::GAME_SDK_VERSION,
        });
        CString::new(v.to_string()).unwrap_or_default()
    })
    .as_ptr()
}

/// 建小游戏实例并执行 `game.js`。失败（不是小游戏包、game.json 坏了）返回 NULL，原因打到 stderr。
/// `options_json` 见 `GameOptions::from_json`（`appId`、`systemInfo`、`launchOptions`、`watchdogMs`、`jsMemoryLimitMB`）。
/// game.js 顶层抛错**不算**失败：实例照常返回，错误在事件队列里
#[no_mangle]
pub extern "C" fn mr_game_create(
    app_dir: *const c_char,
    data_dir: *const c_char,
    width: u32,
    height: u32,
    dpr: f32,
    options_json: *const c_char,
) -> *mut MRGame {
    let r = catch_unwind(|| {
        let dir = cstr(app_dir).ok_or("app_dir 为空")?;
        let opts = GameOptions::from_json(&cstr(options_json).unwrap_or_default())?;
        GameEngine::new(&dir, cstr(data_dir).as_deref(), width, height, dpr, opts)
    });
    match r {
        Ok(Ok(engine)) => Box::into_raw(Box::new(MRGame {
            engine: Some(engine),
            poisoned: false,
            last_error: String::new(),
            last_error_buf: CString::default(),
            event_buf: CString::default(),
            eval_buf: CString::default(),
            panic_event: None,
        })),
        Ok(Err(e)) => {
            eprintln!("❌ mr_game_create 失败: {e}");
            std::ptr::null_mut()
        }
        Err(p) => {
            eprintln!("❌ mr_game_create panic: {}", panic_text(&p));
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn mr_game_destroy(game: *mut MRGame) {
    if game.is_null() {
        return;
    }
    let g = unsafe { Box::from_raw(game) };
    // 析构里也可能 panic（中毒的锁）：吞掉，不能 abort 宿主
    let _ = catch_unwind(AssertUnwindSafe(move || drop(g)));
}

/// 在没中毒的实例上跑 `f`；panic 时把实例标成 poisoned。实例为空 / 已中毒返回 `fail`
fn guarded<R>(game: *mut MRGame, fail: R, f: impl FnOnce(&mut GameEngine) -> R) -> R {
    let Some(g) = (unsafe { game.as_mut() }) else { return fail };
    if g.poisoned {
        return fail;
    }
    let Some(engine) = g.engine.as_mut() else { return fail };
    match catch_unwind(AssertUnwindSafe(|| f(engine))) {
        Ok(r) => r,
        Err(p) => {
            let msg = panic_text(&p);
            g.poisoned = true;
            g.last_error = format!("引擎内部错误（panic），实例已失效，请销毁重建：{msg}");
            g.panic_event = Some(serde_json::json!({ "type": "error", "kind": "panic", "message": msg }).to_string());
            // 状态可能不一致：不再碰它（连析构都可能再 panic，留给 destroy 的 catch_unwind）
            std::mem::forget(g.engine.take());
            fail
        }
    }
}

fn panic_text(p: &Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "未知 panic".into())
}

/// 跑一帧，返回 `MR_FRAME_*` 位（与 `mr_app_pump_ex` 同一套）。`now_ms` 用宿主 vsync 时刻，0 = 引擎自己取
#[no_mangle]
pub extern "C" fn mr_game_pump_ex(game: *mut MRGame, now_ms: u64) -> c_int {
    let flags = guarded(game, 0, |e| e.pump_ex(now_ms) as c_int);
    // 中毒那一刻还有一条 panic 事件没交出去：带上 EVENTS 位让宿主来取
    match unsafe { game.as_ref() } {
        Some(g) if g.panic_event.is_some() => flags | crate::host::engine_backend::FRAME_EVENTS as c_int,
        _ => flags,
    }
}

/// 取一条事件（JSON），没有了返回 NULL。返回的指针活到下一次对同一实例的调用
#[no_mangle]
pub extern "C" fn mr_game_poll_event(game: *mut MRGame) -> *const c_char {
    let Some(g) = (unsafe { game.as_mut() }) else { return std::ptr::null() };
    if let Some(ev) = g.panic_event.take() {
        g.event_buf = CString::new(ev).unwrap_or_default();
        return g.event_buf.as_ptr();
    }
    match guarded(game, None, |e| e.poll_event()) {
        Some(ev) => {
            let g = unsafe { &mut *game };
            g.event_buf = CString::new(ev.replace('\0', "")).unwrap_or_default();
            g.event_buf.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// 执行一段 JS（诊断用；release 宿主可以不暴露）。出错返回 NULL，原因见 `mr_game_last_error`
#[no_mangle]
pub extern "C" fn mr_game_eval(game: *mut MRGame, code: *const c_char) -> *const c_char {
    let Some(code) = cstr(code) else { return std::ptr::null() };
    match guarded(game, Err("实例为空或已失效".to_string()), |e| e.eval(&code)) {
        Ok(v) => {
            let g = unsafe { &mut *game };
            g.eval_buf = CString::new(v.replace('\0', "")).unwrap_or_default();
            g.eval_buf.as_ptr()
        }
        Err(e) => {
            if let Some(g) = unsafe { game.as_mut() } {
                if !g.poisoned {
                    g.last_error = e;
                }
            }
            std::ptr::null()
        }
    }
}

/// 最近一次失败的原因（没有返回空串）。指针活到下一次对同一实例的调用
#[no_mangle]
pub extern "C" fn mr_game_last_error(game: *mut MRGame) -> *const c_char {
    let Some(g) = (unsafe { game.as_mut() }) else { return std::ptr::null() };
    g.last_error_buf = CString::new(g.last_error.replace('\0', "")).unwrap_or_default();
    g.last_error_buf.as_ptr()
}

/// 实例是否已因 panic 失效（1 = 失效）
#[no_mangle]
pub extern "C" fn mr_game_is_poisoned(game: *mut MRGame) -> c_int {
    unsafe { game.as_ref() }.map(|g| g.poisoned as c_int).unwrap_or(1)
}

#[cfg(test)]
mod tests;
