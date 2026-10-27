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

#[cfg(feature = "game")]
use crate::game::{GameEngine, GameOptions};
use std::ffi::{c_char, c_int, CString};
#[cfg(feature = "game")]
use std::panic::{catch_unwind, AssertUnwindSafe};

#[cfg(feature = "game")]
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

#[cfg(feature = "game")]
const SDK_VERSION: &str = crate::game::GAME_SDK_VERSION;
#[cfg(not(feature = "game"))]
const SDK_VERSION: &str = "";

/// 本库编进了哪些能力（JSON，静态字符串，不用释放）：宿主据此决定是否展示小游戏入口
#[no_mangle]
pub extern "C" fn mr_capabilities() -> *const c_char {
    static CAPS: std::sync::OnceLock<CString> = std::sync::OnceLock::new();
    CAPS.get_or_init(|| {
        let v = serde_json::json!({
            "game": cfg!(feature = "game"),
            // 下面三项随实现进度改（G8 WebGL、G5 移动端音频、G10 wasm）
            "webgl": false,
            "audio": cfg!(feature = "audio"),
            "wasm": false,
            "gpu": cfg!(feature = "gpu"),
            "version": env!("CARGO_PKG_VERSION"),
            "sdkVersion": SDK_VERSION,
        });
        CString::new(v.to_string()).unwrap_or_default()
    })
    .as_ptr()
}

/// 建小游戏实例并执行 `game.js`。失败（不是小游戏包、game.json 坏了）返回 NULL，原因打到 stderr。
/// `options_json` 见 `GameOptions::from_json`（`appId`、`systemInfo`、`launchOptions`、`watchdogMs`、`jsMemoryLimitMB`）。
/// game.js 顶层抛错**不算**失败：实例照常返回，错误在事件队列里
#[cfg(feature = "game")]
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

#[cfg(feature = "game")]
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
#[cfg(feature = "game")]
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

#[cfg(feature = "game")]
fn panic_text(p: &Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_else(|| "未知 panic".into())
}

/// 跑一帧，返回 `MR_FRAME_*` 位（与 `mr_app_pump_ex` 同一套，另加 `FRAME_WAKE_AT = 128`）。
/// `now_us` 是宿主 vsync 时刻，**微秒**（Android `frameTimeNanos / 1000`、iOS `targetTimestamp * 1e6`），0 = 引擎自己取。
/// 与 `mr_app_pump_ex` 的毫秒不同：rAF 的时间戳要微秒精度（144Hz 一帧 6.944ms）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_pump_ex(game: *mut MRGame, now_us: u64) -> c_int {
    let flags = guarded(game, 0, |e| e.pump_ex(now_us) as c_int);
    // 中毒那一刻还有一条 panic 事件没交出去：带上 EVENTS 位让宿主来取
    match unsafe { game.as_ref() } {
        Some(g) if g.panic_event.is_some() => flags | crate::host::engine_backend::FRAME_EVENTS as c_int,
        _ => flags,
    }
}

/// 取一条事件（JSON），没有了返回 NULL。返回的指针活到下一次对同一实例的调用
#[cfg(feature = "game")]
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
#[cfg(feature = "game")]
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
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_last_error(game: *mut MRGame) -> *const c_char {
    let Some(g) = (unsafe { game.as_mut() }) else { return std::ptr::null() };
    g.last_error_buf = CString::new(g.last_error.replace('\0', "")).unwrap_or_default();
    g.last_error_buf.as_ptr()
}

/// 实例是否已因 panic 失效（1 = 失效）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_is_poisoned(game: *mut MRGame) -> c_int {
    unsafe { game.as_ref() }.map(|g| g.poisoned as c_int).unwrap_or(1)
}

/// 前后台：非 0 = 进后台（不跑 rAF、不要帧），0 = 回前台（从下一个 vsync 重新起算，不补帧）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_set_background(game: *mut MRGame, background: c_int) {
    guarded(game, (), |e| e.set_background(background != 0));
}

/// 系统音频打断：非 0 = 开始（来电、闹钟、别的 App 拿走独占焦点），0 = 结束。引擎暂停全部音频后派发
/// `wx.onAudioInterruptionBegin` / `End`，结束时不自动续播（game/audio/session.rs）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_audio_interruption(game: *mut MRGame, begin: c_int) {
    guarded(game, (), |e| e.audio_interruption(begin != 0));
}
/// 窗口变了（转屏、分屏、折叠屏）：逻辑尺寸、像素比、方向（"portrait" / "landscape" / "landscapeReverse"，NULL = 按宽高推）。
/// 引擎更新系统信息、派发 wx.onWindowResize / onDeviceOrientationChange；主画布尺寸不变（游戏自己改，见 game/window.rs）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_resize(game: *mut MRGame, width: u32, height: u32, dpr: f32, orientation: *const c_char) {
    let o = cstr(orientation);
    guarded(game, (), |e| e.resize(width, height, dpr, o.as_deref()));
}
/// 系统内存告警：`level` 5 / 10 / 15 派发 wx.onMemoryWarning（Android 前台三档），其它值只回收（图片缓存、JS GC）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_memory_warning(game: *mut MRGame, level: c_int) {
    guarded(game, (), |e| e.memory_warning(level));
}
/// rAF 目标帧率（1~60）。宿主据此请求显示刷新率（60 的整数倍，game-doc/02 §4.2）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_preferred_fps(game: *mut MRGame) -> c_int {
    guarded(game, 60, |e| e.preferred_fps() as c_int)
}

/// 只剩远期定时器时多久之后再 pump（微秒，相对现在）；没有定时器 -1
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_next_wake_us(game: *mut MRGame) -> i64 {
    guarded(game, -1, |e| e.next_wake_us().map(|v| v as i64).unwrap_or(-1))
}

/// 主画布像素尺寸（宿主按它设 surface 缓冲尺寸，由系统缩放到全屏）。还没建主画布时写 0
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_frame_size(game: *mut MRGame, w: *mut u32, h: *mut u32) {
    let (fw, fh) = guarded(game, (0, 0), |e| {
        let (w, h, _) = e.frame_rgba();
        (w, h)
    });
    // SAFETY: 调用方给的输出指针，可为 NULL
    unsafe {
        if let Some(w) = w.as_mut() {
            *w = fw;
        }
        if let Some(h) = h.as_mut() {
            *h = fh;
        }
    }
}

/// 把当前帧（主画布分辨率，不透明 RGBA8）写进宿主缓冲：`stride` 字节一行，`format` 0 = RGBA8、1 = BGRA8。
/// 缓冲至少 `stride × 高` 字节。返回写入的行数，0 = 没写（尺寸为 0 / 实例失效 / 缓冲太小）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_pixels_into(game: *mut MRGame, buf: *mut u8, len: usize, stride: usize, format: c_int) -> c_int {
    if buf.is_null() {
        return 0;
    }
    mr_game_with_frame(game, |w, h, px| {
        let row = w as usize * 4;
        if stride < row || len < stride * h as usize {
            return 0;
        }
        // SAFETY: 调用方保证 buf 至少 len 字节且本次调用内有效
        let dst = unsafe { std::slice::from_raw_parts_mut(buf, len) };
        for y in 0..h as usize {
            let s = &px[y * row..(y + 1) * row];
            let d = &mut dst[y * stride..y * stride + row];
            if format == 1 {
                for (o, i) in d.chunks_exact_mut(4).zip(s.chunks_exact(4)) {
                    o.copy_from_slice(&[i[2], i[1], i[0], i[3]]);
                }
            } else {
                d.copy_from_slice(s);
            }
        }
        h as c_int
    })
    .unwrap_or(0)
}

/// 一个指针事件（game-doc/02 §7.1）：宿主**每个指针**调一次。`phase` 0 下 / 1 移 / 2 抬 / 3 取消；
/// `id` 宿主的指针 id（Android pointerId、iOS 映射后的小整数）；`x, y` 视图内逻辑像素；`force` 0~1（没有给 0）；
/// `t_us` 事件时刻，**微秒**，与 `mr_game_pump_ex` 同一时钟（0 = 此刻）。只排队，下一次 pump 在 rAF 之前派发
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_touch(game: *mut MRGame, phase: c_int, id: c_int, x: f32, y: f32, force: f32, t_us: u64) {
    guarded(game, (), |e| e.touch(phase, id, x, y, force, t_us));
}

/// 鼠标（PC）：`phase` 0 下 / 1 移 / 2 抬，`button` 0 左 1 中 2 右，坐标视图内逻辑像素
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_mouse(game: *mut MRGame, phase: c_int, button: c_int, x: f32, y: f32, t_us: u64) {
    guarded(game, (), |e| e.mouse(phase, button, x, y, t_us));
}

/// 滚轮（PC）：`wx.onWheel` 的 deltaX / deltaY / deltaZ（像素，与 DOM WheelEvent 的 deltaMode 0 相同）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_wheel(game: *mut MRGame, dx: f32, dy: f32, dz: f32, x: f32, y: f32, t_us: u64) {
    guarded(game, (), |e| e.wheel(dx, dy, dz, x, y, t_us));
}

/// 键盘：`down` 非 0 = 按下；`key` 如 `"a"` / `"Enter"`，`code` 如 `"KeyA"`（UTF-8，可为 NULL 当空串）
#[cfg(feature = "game")]
#[no_mangle]
pub extern "C" fn mr_game_key(game: *mut MRGame, down: c_int, key: *const c_char, code: *const c_char, t_us: u64) {
    let (k, c) = (cstr(key).unwrap_or_default(), cstr(code).unwrap_or_default());
    guarded(game, (), |e| e.key(down != 0, &k, &c, t_us));
}

/// 在实例上跑一段宿主侧代码（JNI 的诊断入口用）。实例为空 / 失效返回 None
#[cfg(feature = "game")]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn mr_game_with_engine<R>(game: *mut MRGame, f: impl FnOnce(&mut GameEngine) -> R) -> Option<R> {
    guarded(game, None, |e| Some(f(e)))
}

/// 拿主画布原始像素跑一段宿主侧上屏代码（JNI 直写 ANativeWindow 用，不经中间缓冲）
#[cfg(feature = "game")]
#[cfg_attr(not(target_os = "android"), allow(dead_code))] // 只有 Android 的 JNI 直写用
pub(crate) fn mr_game_with_main_pixels<R>(game: *mut MRGame, f: impl FnOnce(u32, u32, &[crate::Color]) -> R) -> Option<R> {
    guarded(game, None, |e| e.with_main_pixels(f))
}

/// 拿当前帧（宽、高、RGBA）跑一段宿主侧的上屏代码（JNI 直写 ANativeWindow 用）。实例为空 / 失效返回 None
#[cfg(feature = "game")]
pub(crate) fn mr_game_with_frame<R>(game: *mut MRGame, f: impl FnOnce(u32, u32, &[u8]) -> R) -> Option<R> {
    guarded(game, None, |e| {
        let (w, h, px) = e.frame_rgba();
        if w == 0 || h == 0 {
            return None;
        }
        Some(f(w, h, px))
    })
}

#[cfg(all(test, feature = "game"))]
mod tests;
