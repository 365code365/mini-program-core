//! Android：小游戏的 JNI 入口（Kotlin `dev.minirender.MiniGame`）。只在 android 目标编译。
//!
//! 全部调用必须在同一条引擎线程上（`MiniGameView` 的 HandlerThread），与 `mr_game_*` 同一约定；
//! 每个入口都经 `ffi_game` 的 panic 防护。
//!
//! ## 上屏：主画布分辨率直写，系统缩放到全屏
//! `ANativeWindow_setBuffersGeometry(主画布宽, 主画布高)` 之后，SurfaceFlinger 把缓冲按视图尺寸拉伸显示（硬件缩放器，
//! 零 CPU 成本）。默认主画布 = 窗口逻辑尺寸（如 393×857），每帧只拷 1.3MB；如果在 CPU 上按物理分辨率（1272×2772）
//! 缩放再写，单是双线性就要 10ms 以上（game-doc/02 §5.5 的预算）。
#![cfg(all(target_os = "android", feature = "game"))]

use crate::ffi_game::*;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;
use std::ffi::{c_void, CStr, CString};

#[repr(C)]
struct ANativeWindowBuffer {
    width: i32,
    height: i32,
    stride: i32,
    format: i32,
    bits: *mut c_void,
    reserved: [u32; 6],
}
const WINDOW_FORMAT_RGBA_8888: i32 = 1;

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
    fn ANativeWindow_setBuffersGeometry(window: *mut c_void, width: i32, height: i32, format: i32) -> i32;
    fn ANativeWindow_lock(window: *mut c_void, out: *mut ANativeWindowBuffer, dirty: *mut c_void) -> i32;
    fn ANativeWindow_unlockAndPost(window: *mut c_void) -> i32;
}

/// 句柄：MRGame + 挂着的 ANativeWindow
struct Handle {
    game: *mut MRGame,
    window: usize,
    geometry: (u32, u32),
    /// 最近一次上屏：等缓冲（ANativeWindow_lock，vsync 背压）与拷贝各花了多少微秒。
    /// 等缓冲不是引擎的工作量：宿主统计「慢帧」时要扣掉（与 GPU 路径的「等 vsync」同一口径）
    last_wait_us: u32,
    last_copy_us: u32,
    last_post_us: u32,
}

fn handle<'a>(h: jlong) -> Option<&'a mut Handle> {
    // SAFETY: 非 0 时是 nativeCreate 交出去、还没 destroy 的句柄
    unsafe { (h as *mut Handle).as_mut() }
}

fn jstr(env: &mut JNIEnv, s: &JString) -> Option<CString> {
    if s.is_null() {
        return None;
    }
    let s: String = env.get_string(s).ok()?.into();
    CString::new(s).ok()
}

fn out_str(env: &JNIEnv, p: *const std::ffi::c_char) -> jstring {
    if p.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: 指针活到下一次 mr_game_* 调用，这里立刻转成 Java 字符串
    let text = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
    env.new_string(text).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeCreate(
    mut env: JNIEnv,
    _c: JClass,
    dir: JString,
    data: JString,
    w: jint,
    h: jint,
    dpr: f32,
    options: JString,
) -> jlong {
    let (Some(dir), data, opts) = (jstr(&mut env, &dir), jstr(&mut env, &data), jstr(&mut env, &options)) else { return 0 };
    // 宿主在引擎线程上建实例：这条线程只给游戏用，绑大核（与小程序同一套，host/cpu_affinity.rs）。
    // 不绑的话调度器按「每帧只醒几毫秒」把它放在小核上，同样的绘制慢 3~5 倍
    crate::compositor::diag::emit(&crate::host::cpu_affinity::pin_current_thread());
    let g = mr_game_create(
        dir.as_ptr(),
        data.as_ref().map(|d| d.as_ptr()).unwrap_or(std::ptr::null()),
        w.max(1) as u32,
        h.max(1) as u32,
        dpr,
        opts.as_ref().map(|o| o.as_ptr()).unwrap_or(std::ptr::null()),
    );
    if g.is_null() {
        return 0;
    }
    Box::into_raw(Box::new(Handle { game: g, window: 0, geometry: (0, 0), last_wait_us: 0, last_copy_us: 0, last_post_us: 0 })) as jlong
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeDestroy(_e: JNIEnv, _c: JClass, h: jlong) {
    if h == 0 {
        return;
    }
    // SAFETY: nativeCreate 交出的句柄，之后不再使用
    let hd = unsafe { Box::from_raw(h as *mut Handle) };
    if hd.window != 0 {
        unsafe { ANativeWindow_release(hd.window as *mut c_void) };
    }
    mr_game_destroy(hd.game);
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativePump(_e: JNIEnv, _c: JClass, h: jlong, now_us: jlong) -> jint {
    handle(h).map(|hd| mr_game_pump_ex(hd.game, now_us.max(0) as u64)).unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativePollEvent(env: JNIEnv, _c: JClass, h: jlong) -> jstring {
    match handle(h) {
        Some(hd) => out_str(&env, mr_game_poll_event(hd.game)),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeEval(mut env: JNIEnv, _c: JClass, h: jlong, code: JString) -> jstring {
    let (Some(hd), Some(code)) = (handle(h), jstr(&mut env, &code)) else { return std::ptr::null_mut() };
    out_str(&env, mr_game_eval(hd.game, code.as_ptr()))
}

/// 前后台：1 = 进后台（不跑 rAF），0 = 回前台
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeSetBackground(_e: JNIEnv, _c: JClass, h: jlong, bg: jboolean) {
    if let Some(hd) = handle(h) {
        mr_game_set_background(hd.game, bg as i32);
    }
}

/// 窗口变了（见 `mr_game_resize`）；orientation 可以是 null
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeResize(mut env: JNIEnv, _c: JClass, h: jlong, w: jint, hh: jint, dpr: f32, orientation: JString) {
    let Some(hd) = handle(h) else { return };
    let o: Option<String> = if orientation.is_null() { None } else { env.get_string(&orientation).ok().map(Into::into) };
    let c = o.and_then(|s| CString::new(s).ok());
    mr_game_resize(hd.game, w.max(1) as u32, hh.max(1) as u32, dpr, c.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null()));
}
/// 系统音频打断：true = 开始，false = 结束（见 `mr_game_audio_interruption`）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeAudioInterruption(_e: JNIEnv, _c: JClass, h: jlong, begin: jboolean) {
    if let Some(hd) = handle(h) {
        mr_game_audio_interruption(hd.game, begin as i32);
    }
}
/// 一批指针事件（一个 MotionEvent 拆出来的全部指针 × 全部历史采样，按时间顺序）：每个事件 6 个 float
/// `[phase, id, x, y, force, t_ms_offset]`，外加批的基准时刻 `base_us`（µs，与 nativePump 同一时钟）。
/// 一次 JNI 进来，不是每个指针一次（1000Hz 屏上一次 MOVE 可能带十几个历史采样）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeTouchBatch(
    env: JNIEnv,
    _c: JClass,
    h: jlong,
    data: jni::objects::JFloatArray,
    count: jint,
    base_us: jlong,
) {
    let Some(hd) = handle(h) else { return };
    let n = count.max(0) as usize;
    let mut buf = vec![0f32; n * 6];
    if n == 0 || env.get_float_array_region(&data, 0, &mut buf).is_err() {
        return;
    }
    for e in buf.chunks_exact(6) {
        // 偏移是相对 base 的毫秒（float 装不下 µs 级的绝对时刻）
        let t = (base_us as f64 + e[5] as f64 * 1000.0).max(0.0) as u64;
        mr_game_touch(hd.game, e[0] as i32, e[1] as i32, e[2], e[3], e[4], t);
    }
}

/// 音频输出统计（JSON：frames / latencyMs / block / callbacks / xruns / rate / device / voices），诊断用
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeAudioStats(env: JNIEnv, _c: JClass, h: jlong) -> jstring {
    let Some(hd) = handle(h) else { return std::ptr::null_mut() };
    let s = crate::ffi_game::mr_game_with_engine(hd.game, |e| e.audio_stats().to_string()).unwrap_or_default();
    env.new_string(s).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// rAF 目标帧率（宿主据此 `Surface.setFrameRate`）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativePreferredFps(_e: JNIEnv, _c: JClass, h: jlong) -> jint {
    handle(h).map(|hd| mr_game_preferred_fps(hd.game)).unwrap_or(60)
}

/// 只剩远期定时器时多久之后再来（µs，-1 = 没有定时器）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeNextWakeUs(_e: JNIEnv, _c: JClass, h: jlong) -> jlong {
    handle(h).map(|hd| mr_game_next_wake_us(hd.game)).unwrap_or(-1)
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeAttachWindow(env: JNIEnv, _c: JClass, h: jlong, surface: JObject) -> jboolean {
    let Some(hd) = handle(h) else { return 0 };
    if hd.window != 0 {
        unsafe { ANativeWindow_release(hd.window as *mut c_void) };
        hd.window = 0;
    }
    if surface.is_null() {
        return 0;
    }
    // SAFETY: surface 是 android.view.Surface，env 在本次调用内有效；fromSurface 自带 +1 引用
    let win = unsafe { ANativeWindow_fromSurface(env.get_raw(), surface.as_raw()) };
    if win.is_null() {
        return 0;
    }
    hd.window = win as usize;
    hd.geometry = (0, 0);
    1
}

/// surface 要没了：同步放手（返回之后不再碰它）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeDetachWindow(_e: JNIEnv, _c: JClass, h: jlong) {
    if let Some(hd) = handle(h) {
        if hd.window != 0 {
            unsafe { ANativeWindow_release(hd.window as *mut c_void) };
            hd.window = 0;
        }
    }
}

/// 主画布写进 surface。1 = 上屏了；0 = 没挂 surface / 实例失效；负数 = 锁不上（宿主应换一块 surface）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeBlit(_e: JNIEnv, _c: JClass, h: jlong) -> jint {
    let Some(hd) = handle(h) else { return 0 };
    if hd.window == 0 {
        return 0;
    }
    let win = hd.window as *mut c_void;
    let game = hd.game;
    if mr_game_with_engine(game, |e| e.rotated_output()).unwrap_or(false) {
        return blit_rgba(hd);
    }
    let geometry = &mut hd.geometry;
    let (wait, copy, post) = (&mut hd.last_wait_us, &mut hd.last_copy_us, &mut hd.last_post_us);
    mr_game_with_main_pixels(game, |w, hgt, px| {
        // SAFETY: win 是挂上的 ANativeWindow；buffer 由 lock 填好，unlockAndPost 之前有效
        unsafe {
            if *geometry != (w, hgt) {
                if ANativeWindow_setBuffersGeometry(win, w as i32, hgt as i32, WINDOW_FORMAT_RGBA_8888) != 0 {
                    return -1;
                }
                *geometry = (w, hgt);
            }
            let t0 = std::time::Instant::now();
            let mut buf = ANativeWindowBuffer { width: 0, height: 0, stride: 0, format: 0, bits: std::ptr::null_mut(), reserved: [0; 6] };
            let r = ANativeWindow_lock(win, &mut buf, std::ptr::null_mut());
            let t1 = std::time::Instant::now();
            *wait = (t1 - t0).as_micros() as u32;
            if r != 0 || buf.bits.is_null() {
                return if r < 0 { r } else { -2 };
            }
            if buf.width as u32 == w && buf.height as u32 == hgt && buf.format == WINDOW_FORMAT_RGBA_8888 {
                let stride = buf.stride.max(0) as usize * 4;
                let dst = std::slice::from_raw_parts_mut(buf.bits as *mut u8, stride * hgt as usize);
                crate::game::canvas::compose::write_opaque_rows(px, w, hgt, dst, stride);
            }
            let t_copied = std::time::Instant::now();
            ANativeWindow_unlockAndPost(win);
            *post = t_copied.elapsed().as_micros() as u32;
            *copy = t1.elapsed().as_micros() as u32;
            1
        }
    })
    .unwrap_or(0)
}

/// 引擎内旋转（game/rotate.rs）时的上屏：转好的 RGBA 逐行拷进 surface（多一次整帧转置，横屏嵌竖屏页面这种少见场景才走）
fn blit_rgba(hd: &mut Handle) -> jint {
    let win = hd.window as *mut c_void;
    let geometry = &mut hd.geometry;
    mr_game_with_frame(hd.game, |w, hgt, px| unsafe {
        // SAFETY: 同 nativeBlit
        if *geometry != (w, hgt) {
            if ANativeWindow_setBuffersGeometry(win, w as i32, hgt as i32, WINDOW_FORMAT_RGBA_8888) != 0 {
                return -1;
            }
            *geometry = (w, hgt);
        }
        let mut buf = ANativeWindowBuffer { width: 0, height: 0, stride: 0, format: 0, bits: std::ptr::null_mut(), reserved: [0; 6] };
        let r = ANativeWindow_lock(win, &mut buf, std::ptr::null_mut());
        if r != 0 || buf.bits.is_null() {
            return if r < 0 { r } else { -2 };
        }
        if buf.width as u32 == w && buf.height as u32 == hgt {
            let stride = buf.stride.max(0) as usize * 4;
            let row = w as usize * 4;
            let dst = std::slice::from_raw_parts_mut(buf.bits as *mut u8, stride * hgt as usize);
            for y in 0..hgt as usize {
                dst[y * stride..y * stride + row].copy_from_slice(&px[y * row..(y + 1) * row]);
            }
        }
        ANativeWindow_unlockAndPost(win);
        1
    })
    .unwrap_or(0)
}
/// 最近一次上屏的耗时：高 32 位 = 等缓冲（µs），低 32 位 = 拷贝 + 提交（µs）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeBlitStats(_e: JNIEnv, _c: JClass, h: jlong) -> jlong {
    handle(h).map(|hd| ((hd.last_wait_us as i64) << 32) | hd.last_copy_us as i64).unwrap_or(0)
}

/// 其中提交（unlockAndPost）占多少微秒
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniGame_nativeBlitPostUs(_e: JNIEnv, _c: JClass, h: jlong) -> jlong {
    handle(h).map(|hd| hd.last_post_us as i64).unwrap_or(0)
}
