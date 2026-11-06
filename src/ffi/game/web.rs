//! 小游戏高性能模式的 C ABI（`mr_wg_*`，game/web）：宿主（iOS 的 MiniWebGameView.swift）只做传话，
//! 页面跑在它的 WKWebView 里。
//!
//! 线程：协议处理器（WKURLSchemeHandler）在后台线程取资源，其余在主线程 —— 实例包在 Mutex 里，任意线程可调。
//! 返回的字符串 / 字节由调用方用 [`mr_wg_free_string`] / [`mr_wg_free_bytes`] 释放（多线程下不能复用实例里的缓冲）。
//! panic 不穿过 FFI：每个入口 catch_unwind，捕获后返回失败值（实例的锁可能中毒，之后按中毒的锁照常取用）。

use crate::game::web::WebGame;
use crate::game::GameOptions;
use std::ffi::{c_char, c_int, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

pub struct MRWebGame {
    game: Mutex<WebGame>,
}

fn cstr(p: *const c_char) -> Option<String> {
    crate::ffi::app::cstr(p)
}

fn with<R>(h: *mut MRWebGame, fail: R, f: impl FnOnce(&mut WebGame) -> R) -> R {
    let Some(h) = (unsafe { h.as_ref() }) else { return fail };
    catch_unwind(AssertUnwindSafe(|| {
        let mut g = h.game.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut g)
    }))
    .unwrap_or(fail)
}

fn out(s: Option<String>) -> *mut c_char {
    match s {
        Some(s) => CString::new(s.replace('\0', "")).map(CString::into_raw).unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
    }
}

/// 建实例（不执行任何 JS：页面加载 `wxgame://app/__wg.html` 之后才开始跑）。`options_json` 与 `mr_game_create` 同一份。
/// 失败（不是小游戏包、game.json 坏了）返回 NULL
#[no_mangle]
pub extern "C" fn mr_wg_create(app_dir: *const c_char, data_dir: *const c_char, width: u32, height: u32, dpr: f32, options_json: *const c_char) -> *mut MRWebGame {
    let r = catch_unwind(|| {
        let dir = cstr(app_dir).ok_or("app_dir 为空")?;
        let opts = GameOptions::from_json(&cstr(options_json).unwrap_or_default())?;
        WebGame::new(&dir, cstr(data_dir).as_deref(), width, height, dpr, &opts)
    });
    match r {
        Ok(Ok(g)) => Box::into_raw(Box::new(MRWebGame { game: Mutex::new(g) })),
        Ok(Err(e)) => {
            crate::compositor::diag::emit(&format!("❌ mr_wg_create 失败：{e}"));
            std::ptr::null_mut()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn mr_wg_destroy(h: *mut MRWebGame) {
    if h.is_null() {
        return;
    }
    let b = unsafe { Box::from_raw(h) };
    let _ = catch_unwind(AssertUnwindSafe(move || drop(b)));
}

/// 页面里一个 URL 的内容（`path` 含查询串，如 `/__wg/m/js/a.js`、`/img/a.png?v=1`）。
/// 找不到返回 NULL；找到时 `*len` 是字节数、`*mime` 是静态的 MIME 串（不用释放）
#[no_mangle]
pub extern "C" fn mr_wg_resource(h: *mut MRWebGame, path: *const c_char, len: *mut usize, mime: *mut *const c_char) -> *mut u8 {
    let Some(p) = cstr(path) else { return std::ptr::null_mut() };
    // 远程代理不加实例的锁（会阻塞在网络上，主线程的同步调用跟着卡住）
    let r = match crate::game::web::unlocked_resource(&p) {
        Some(r) => r,
        None => with(h, None, |g| g.resource(&p)),
    };
    let Some(res) = r else { return std::ptr::null_mut() };
    static MIMES: Mutex<Vec<(String, &'static std::ffi::CStr)>> = Mutex::new(Vec::new());
    let m = {
        let mut list = MIMES.lock().unwrap_or_else(|e| e.into_inner());
        match list.iter().find(|(k, _)| k == res.mime) {
            Some((_, c)) => *c,
            None => {
                let c: &'static std::ffi::CStr = Box::leak(CString::new(res.mime).unwrap_or_default().into_boxed_c_str());
                list.push((res.mime.to_string(), c));
                c
            }
        }
    };
    let mut bytes = res.bytes.into_boxed_slice();
    unsafe {
        if !len.is_null() {
            *len = bytes.len();
        }
        if !mime.is_null() {
            *mime = m.as_ptr();
        }
    }
    let ptr = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    ptr
}

#[no_mangle]
pub extern "C" fn mr_wg_free_bytes(p: *mut u8, len: usize) {
    if !p.is_null() {
        unsafe { drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, len))) };
    }
}

#[no_mangle]
pub extern "C" fn mr_wg_free_string(p: *mut c_char) {
    if !p.is_null() {
        unsafe { drop(CString::from_raw(p)) };
    }
}

/// 页面的同步调用（消息体 `{"n","a"}`），返回结果字符串（可能是空串，不是 NULL）
#[no_mangle]
pub extern "C" fn mr_wg_call(h: *mut MRWebGame, msg: *const c_char) -> *mut c_char {
    let m = cstr(msg).unwrap_or_default();
    out(Some(with(h, String::new(), |g| g.call_message(&m))))
}

/// 页面的异步消息（消息体 `{"n","a"}`）
#[no_mangle]
pub extern "C" fn mr_wg_post(h: *mut MRWebGame, msg: *const c_char) {
    let m = cstr(msg).unwrap_or_default();
    with(h, (), |g| g.post_message(&m));
}

/// 要在页面里执行的一段 JS（没有返回 NULL）：宿主 `evaluateJavaScript`
#[no_mangle]
pub extern "C" fn mr_wg_take_js(h: *mut MRWebGame) -> *mut c_char {
    out(with(h, None, |g| g.take_js()))
}

/// 一条宿主事件（JSON，type 与 `mr_game_poll_event` 相同；没有返回 NULL）
#[no_mangle]
pub extern "C" fn mr_wg_take_event(h: *mut MRWebGame) -> *mut c_char {
    out(with(h, None, |g| g.take_event()))
}

/// 还有在等的异步活（网络请求在飞、有待执行的 JS）：1 = 宿主要继续轮询 [`mr_wg_take_js`]
#[no_mangle]
pub extern "C" fn mr_wg_pending(h: *mut MRWebGame) -> c_int {
    with(h, 0, |g| g.pending() as c_int)
}

#[no_mangle]
pub extern "C" fn mr_wg_resize(h: *mut MRWebGame, width: u32, height: u32, dpr: f32, orientation: *const c_char) {
    let o = cstr(orientation);
    with(h, (), |g| g.resize(width, height, dpr, o.as_deref()));
}

#[no_mangle]
pub extern "C" fn mr_wg_set_safe_area(h: *mut MRWebGame, top: f32, right: f32, bottom: f32, left: f32) {
    with(h, (), |g| g.set_safe_area(crate::host::SafeAreaInsets { top, right, bottom, left }));
}

#[no_mangle]
pub extern "C" fn mr_wg_set_background(h: *mut MRWebGame, bg: c_int) {
    with(h, (), |g| g.set_background(bg != 0));
}

#[no_mangle]
pub extern "C" fn mr_wg_dispatch_event(h: *mut MRWebGame, name: *const c_char, json: *const c_char) {
    let (n, j) = (cstr(name).unwrap_or_default(), cstr(json).unwrap_or_else(|| "{}".into()));
    with(h, (), |g| g.dispatch_event(&n, &j));
}

#[no_mangle]
pub extern "C" fn mr_wg_host_reply(h: *mut MRWebGame, id: u32, ok: c_int, result_json: *const c_char) {
    let r = cstr(result_json).unwrap_or_else(|| "{}".into());
    with(h, (), |g| g.host_reply(id, ok != 0, &r));
}

/// 胶囊位置（逻辑像素）
#[no_mangle]
pub extern "C" fn mr_wg_menu_button(h: *mut MRWebGame, x: *mut f32, y: *mut f32, w: *mut f32, hh: *mut f32) {
    let (a, b, c, d) = with(h, (0.0, 0.0, 0.0, 0.0), |g| g.menu_button());
    unsafe {
        for (p, v) in [(x, a), (y, b), (w, c), (hh, d)] {
            if !p.is_null() {
                *p = v;
            }
        }
    }
}

/// 高性能模式支不支持这个包（宿主据此决定走哪种模式）：0 = 支持；1 = 用了开放数据域；2 = 用了插件；-1 = 不是小游戏包
#[no_mangle]
pub extern "C" fn mr_wg_support(app_dir: *const c_char) -> c_int {
    let Some(d) = cstr(app_dir) else { return -1 };
    let Ok(text) = std::fs::read_to_string(std::path::Path::new(&d).join("game.json")) else { return -1 };
    let Ok(c) = crate::game::config::GameConfig::parse(&text) else { return -1 };
    if c.open_data_context.is_some() {
        1
    } else if !c.plugins.is_empty() {
        2
    } else {
        0
    }
}

/// game.json 要哪种模式：2 = 高性能+（`iOSHighPerformance+`）、1 = 高性能（`iOSHighPerformance`）、0 = 普通、-1 读不出
#[no_mangle]
pub extern "C" fn mr_wg_wanted(app_dir: *const c_char) -> c_int {
    let Some(d) = cstr(app_dir) else { return -1 };
    let Ok(text) = std::fs::read_to_string(std::path::Path::new(&d).join("game.json")) else { return -1 };
    let Ok(c) = crate::game::config::GameConfig::parse(&text) else { return -1 };
    let on = |k: &str| c.raw.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    if on("iOSHighPerformance+") {
        2
    } else {
        on("iOSHighPerformance") as c_int
    }
}

/// 高性能+：回放页面发来的一包命令（game/remote）。`out_flags`：1 = 主画布这一帧画过、要上屏，2 = 这包以帧尾结束（回页面一个 ack）。
/// 返回同步调用的结果（JSON，用 mr_wg_free_string 释放）；没有返回 NULL。在引擎线程上调（与 mr_game_pump_ex 同一条）
#[no_mangle]
pub extern "C" fn mr_game_replay(game: *mut super::MRGame, data: *const u8, len: usize, out_flags: *mut c_int) -> *mut c_char {
    if data.is_null() && len > 0 {
        return std::ptr::null_mut();
    }
    let bytes = if len == 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(data, len) } };
    let r = super::guarded(game, None, |e| Some(e.replay(bytes)));
    let Some(r) = r else { return std::ptr::null_mut() };
    if !out_flags.is_null() {
        unsafe { *out_flags = (r.present as c_int) | ((r.frame_end as c_int) << 1) };
    }
    out(r.result)
}
