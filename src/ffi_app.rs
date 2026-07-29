//! App 级 C ABI：给 iOS / Android / 鸿蒙 / C++ 宿主用的**极简**接口。
//!
//! 设计目标是「宿主只要会画位图、会转发触摸」就能集成，所以整套只有十几个函数，
//! 没有回调注册、没有线程模型要求（除了下面这一条）。
//!
//! ## 线程约定
//! **所有 `mr_app_*` 必须在同一个线程调用**。逻辑层是 QuickJS，不是线程安全的；
//! 渲染也在调用线程同步完成。推荐宿主起一个专用渲染线程，把触摸事件投递过去。
//!
//! ## 典型调用序列
//! ```c
//! MRApp* app = mr_app_create("/data/.../unpacked", "/data/.../sandbox", 375, 667, 3.0f);
//! mr_app_launch(app, NULL);                    // NULL = app.json 的首页
//! // 每帧：
//! if (mr_app_pump(app, now_ms)) {              // 返回 1 说明有新画面
//!     mr_app_pixels(app, buf, len);            // RGBA8，len = w*h*4
//! }
//! // 触摸：
//! mr_app_pointer_down(app, x, y); mr_app_pointer_move(app, x, y); mr_app_pointer_up(app, x, y);
//! // 返回键：
//! if (!mr_app_back(app)) { /* 页面栈空了，关掉小程序 */ }
//! mr_app_destroy(app);
//! ```
//!
//! 坐标一律是**逻辑坐标**（iOS 的 pt / Android 的 dp，左上原点）；像素缓冲的尺寸
//! 是 `逻辑尺寸 × dpr`，用 [`mr_app_pixel_size`] 问。

use crate::host::MiniEngine;
use std::ffi::{c_char, c_int, CStr};

/// 引擎实例句柄
pub struct MRApp {
    engine: MiniEngine,
    last_error: String,
}

fn cstr(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok().map(|s| s.to_string())
}

/// 创建实例。失败返回 NULL。
///
/// - `app_dir`：解包后的小程序目录（含 `app.json`）
/// - `data_dir`：宿主沙盒里可写的目录。**不传的话** storage 与图片磁盘缓存会
///   静默失效（写不进去也不报错），"首次拉数据存起来、之后走缓存"的小程序每次
///   冷启都会走首次分支。
/// - `width` / `height`：逻辑尺寸；`dpr`：设备像素比
#[no_mangle]
pub extern "C" fn mr_app_create(
    app_dir: *const c_char,
    data_dir: *const c_char,
    width: u32,
    height: u32,
    dpr: f32,
) -> *mut MRApp {
    let Some(dir) = cstr(app_dir) else { return std::ptr::null_mut() };
    let data = cstr(data_dir);
    match MiniEngine::new(&dir, data.as_deref(), width, height, dpr) {
        Ok(engine) => Box::into_raw(Box::new(MRApp { engine, last_error: String::new() })),
        Err(e) => {
            eprintln!("❌ mr_app_create 失败: {e}");
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn mr_app_destroy(app: *mut MRApp) {
    if !app.is_null() {
        unsafe { drop(Box::from_raw(app)) };
    }
}

macro_rules! with_app {
    ($app:expr, $ret:expr, |$e:ident| $body:expr) => {{
        if $app.is_null() {
            return $ret;
        }
        let $e = unsafe { &mut *$app };
        $body
    }};
}

/// 打开页面。`route` 传 NULL 表示 `app.json` 的首页。成功返回 1。
#[no_mangle]
pub extern "C" fn mr_app_launch(app: *mut MRApp, route: *const c_char) -> c_int {
    with_app!(app, 0, |a| {
        let r = cstr(route);
        match a.engine.launch(r.as_deref()) {
            Ok(()) => 1,
            Err(e) => {
                a.last_error = e;
                0
            }
        }
    })
}

/// 主动跳页（宿主深链：扫码/推送直接落到某个页面，可带 query）。成功返回 1。
#[no_mangle]
pub extern "C" fn mr_app_navigate(app: *mut MRApp, url: *const c_char) -> c_int {
    with_app!(app, 0, |a| {
        let Some(u) = cstr(url) else { return 0 };
        match a.engine.navigate(&u) {
            Ok(()) => 1,
            Err(e) => {
                a.last_error = e;
                0
            }
        }
    })
}

/// 跑一帧。返回 1 表示有新画面（该取像素上屏），0 表示这一帧没变化。
///
/// `now_ms` 用宿主的单调时钟（iOS `CACurrentMediaTime()*1000`、
/// Android `System.nanoTime()/1_000_000`）；传 0 让引擎自己取。
#[no_mangle]
pub extern "C" fn mr_app_pump(app: *mut MRApp, now_ms: u64) -> c_int {
    with_app!(app, 0, |a| if a.engine.pump(now_ms) { 1 } else { 0 })
}

/// 当前帧的像素尺寸（= 逻辑尺寸 × dpr）
#[no_mangle]
pub extern "C" fn mr_app_pixel_size(app: *mut MRApp, out_w: *mut u32, out_h: *mut u32) {
    with_app!(app, (), |a| {
        let (w, h) = a.engine.pixel_size();
        if !out_w.is_null() {
            unsafe { *out_w = w };
        }
        if !out_h.is_null() {
            unsafe { *out_h = h };
        }
    })
}

/// 拷出 RGBA8 像素，返回实际写入的字节数（`len` 不够时返回 0 不写）。
#[no_mangle]
pub extern "C" fn mr_app_pixels(app: *mut MRApp, out: *mut u8, len: usize) -> usize {
    with_app!(app, 0, |a| {
        let src = a.engine.rgba();
        if out.is_null() || len < src.len() {
            return 0;
        }
        unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), out, src.len()) };
        src.len()
    })
}

// ───────────────────────────── 输入 ─────────────────────────────
//
// 必须送**完整三段**。不要在宿主侧自己合成"点击"再调过来：那样会绕过触摸状态机，
// `touchstart/touchmove/touchend/longpress` 全都不会发，而且
// 「按下时提前 return 导致抬手没有 tap」这类 bug 在测试里全绿、真机上点不动。

#[no_mangle]
pub extern "C" fn mr_app_pointer_down(app: *mut MRApp, x: f32, y: f32) {
    with_app!(app, (), |a| a.engine.pointer_down(x, y))
}

#[no_mangle]
pub extern "C" fn mr_app_pointer_move(app: *mut MRApp, x: f32, y: f32) {
    with_app!(app, (), |a| a.engine.pointer_move(x, y))
}

#[no_mangle]
pub extern "C" fn mr_app_pointer_up(app: *mut MRApp, x: f32, y: f32) {
    with_app!(app, (), |a| a.engine.pointer_up(x, y))
}

/// 手势被系统打断（来电、系统返回手势接管）
#[no_mangle]
pub extern "C" fn mr_app_pointer_cancel(app: *mut MRApp) {
    with_app!(app, (), |a| a.engine.pointer_cancel())
}

/// 滚轮/触控板（`precise` 非 0 表示像素级精密滚动）
#[no_mangle]
pub extern "C" fn mr_app_wheel(app: *mut MRApp, delta_y: f32, precise: c_int) {
    with_app!(app, (), |a| a.engine.wheel(delta_y, precise != 0))
}

/// 输入法/键盘提交的文字（UTF-8，可以是一整串）
#[no_mangle]
pub extern "C" fn mr_app_text_input(app: *mut MRApp, text: *const c_char) {
    with_app!(app, (), |a| {
        if let Some(t) = cstr(text) {
            a.engine.text_input(&t);
        }
    })
}

/// 功能键：`enter` / `backspace` / `delete` / `left` / `right` / `home` / `end` /
/// `selectall` / `escape`
#[no_mangle]
pub extern "C" fn mr_app_key(app: *mut MRApp, name: *const c_char) {
    with_app!(app, (), |a| {
        if let Some(n) = cstr(name) {
            a.engine.key(&n);
        }
    })
}

// ───────────────────────── 路由与生命周期 ─────────────────────────

/// 系统返回键 / 侧滑返回。返回 0 表示**页面栈只剩一页**，宿主该关掉小程序
/// （Android 的 `onBackPressed` 要在这时才 `finish()`）。
#[no_mangle]
pub extern "C" fn mr_app_back(app: *mut MRApp) -> c_int {
    with_app!(app, 0, |a| if a.engine.back() { 1 } else { 0 })
}

/// 当前页面栈深度
#[no_mangle]
pub extern "C" fn mr_app_page_depth(app: *mut MRApp) -> u32 {
    with_app!(app, 0, |a| a.engine.page_depth() as u32)
}

/// 进前台：派发 `App.onShow`，并恢复出帧
#[no_mangle]
pub extern "C" fn mr_app_on_show(app: *mut MRApp) {
    with_app!(app, (), |a| a.engine.on_show())
}

/// 进后台：派发 `App.onHide`。宿主这时应**停止调用 `mr_app_pump`**，否则白耗电。
#[no_mangle]
pub extern "C" fn mr_app_on_hide(app: *mut MRApp) {
    with_app!(app, (), |a| a.engine.on_hide())
}

/// 系统内存告警时调用（iOS `didReceiveMemoryWarning` / Android `onTrimMemory` /
/// 鸿蒙 `onMemoryLevel`）。放掉解码后的图片与自定义字体，**不动**系统主字体。
#[no_mangle]
pub extern "C" fn mr_app_trim_memory(app: *mut MRApp) {
    with_app!(app, (), |a| a.engine.trim_memory())
}

/// 最近一次失败的原因（UTF-8，进程内静态生命周期到下次调用为止）。
/// 没有错误时返回 NULL。
#[no_mangle]
pub extern "C" fn mr_app_last_error(app: *mut MRApp) -> *const c_char {
    if app.is_null() {
        return std::ptr::null();
    }
    let a = unsafe { &mut *app };
    if a.last_error.is_empty() {
        return std::ptr::null();
    }
    if !a.last_error.ends_with('\0') {
        a.last_error.push('\0');
    }
    a.last_error.as_ptr() as *const c_char
}

/// 引擎版本（编译期常量）
#[no_mangle]
pub extern "C" fn mr_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn sample() -> Option<CString> {
        let p = crate::app_dir::resolve("sample-app");
        if !p.join("app.json").exists() {
            return None; // 示例不在时跳过
        }
        CString::new(p.to_string_lossy().to_string()).ok()
    }

    #[test]
    fn null_handles_never_crash() {
        // 宿主传空指针是常态（创建失败后还继续调），不能崩
        assert_eq!(mr_app_launch(std::ptr::null_mut(), std::ptr::null()), 0);
        assert_eq!(mr_app_pump(std::ptr::null_mut(), 0), 0);
        assert_eq!(mr_app_pixels(std::ptr::null_mut(), std::ptr::null_mut(), 0), 0);
        assert_eq!(mr_app_back(std::ptr::null_mut()), 0);
        mr_app_pointer_down(std::ptr::null_mut(), 1.0, 1.0);
        mr_app_pointer_cancel(std::ptr::null_mut());
        mr_app_trim_memory(std::ptr::null_mut());
        mr_app_destroy(std::ptr::null_mut());
        assert!(mr_app_last_error(std::ptr::null_mut()).is_null());
    }

    #[test]
    fn create_launch_pump_and_read_pixels() {
        let Some(dir) = sample() else { return };
        let data = CString::new(
            crate::data_dir::subdir("ffi-test").to_string_lossy().to_string(),
        )
        .unwrap();
        let app = mr_app_create(dir.as_ptr(), data.as_ptr(), 375, 667, 2.0);
        assert!(!app.is_null(), "创建实例失败");
        assert_eq!(mr_app_launch(app, std::ptr::null()), 1, "首页应能打开");
        assert!(mr_app_page_depth(app) >= 1);

        let (mut w, mut h) = (0u32, 0u32);
        mr_app_pixel_size(app, &mut w, &mut h);
        assert_eq!((w, h), (750, 1334), "像素尺寸应为逻辑尺寸 × dpr");

        assert_eq!(mr_app_pump(app, 1), 1, "第一帧必须产出画面");
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let n = mr_app_pixels(app, buf.as_mut_ptr(), buf.len());
        assert_eq!(n, buf.len(), "应写满整块缓冲");
        // 不能是全黑/全透明：那说明没真的画
        assert!(buf.iter().step_by(4).any(|v| *v != 0), "画面不该是全黑");
        assert!(buf.iter().skip(3).step_by(4).all(|a| *a == 255), "alpha 应为不透明");

        // 缓冲不够时不写、返回 0（宿主算错尺寸时不能越界）
        let mut small = vec![0u8; 16];
        assert_eq!(mr_app_pixels(app, small.as_mut_ptr(), small.len()), 0);

        // 触摸三段 + 返回键都不该崩
        mr_app_pointer_down(app, 100.0, 200.0);
        mr_app_pointer_move(app, 100.0, 210.0);
        mr_app_pointer_up(app, 100.0, 210.0);
        mr_app_pump(app, 20);
        let _ = mr_app_back(app);
        mr_app_on_hide(app);
        mr_app_on_show(app);
        mr_app_trim_memory(app);
        mr_app_destroy(app);
    }

    #[test]
    fn version_is_a_valid_cstring() {
        let p = mr_version();
        assert!(!p.is_null());
        let s = unsafe { CStr::from_ptr(p) }.to_str().unwrap();
        assert!(!s.is_empty() && s.contains('.'), "版本号: {s}");
    }
}
