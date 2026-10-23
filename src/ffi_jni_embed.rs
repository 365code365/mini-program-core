//! Android 的 JNI 入口：嵌入宿主那组新 C ABI（`ffi_embed.rs`）的 Kotlin 绑定，
//! 外加两条 Android 专有的像素出口。只在 android 目标编译。
//!
//! ## 两条像素出口（Flutter 插件按顺序试）
//! 1. `nativeBlitToWindow`：引擎直接 `ANativeWindow_lock` 宿主给的 Surface，把帧写进
//!    图形缓冲 —— 零中间拷贝。Flutter 的 `SurfaceProducer` 在 API 29+ 底层是
//!    `ImageReader(PRIVATE, GPU_SAMPLED)`，这种缓冲**不保证**能被 CPU 锁上（取决于
//!    gralloc），所以锁失败要如实返回错误码，由宿主换第二条路。
//! 2. `nativePixelsToBitmap`：写进一张 `Bitmap` 的像素内存（`AndroidBitmap_lockPixels`），
//!    宿主再 `lockHardwareCanvas` + `drawBitmap` 画到 Surface 上 —— Flutter 自己的
//!    平台视图（`PlatformViewWrapper`）就是这样往这种 Surface 上画的。
//!    比旧的 `nativePixels`（拷进 ByteArray 再 `copyPixelsFromBuffer`）少一次整帧拷贝。
//!
//! 注意：一块 Surface 一旦被 CPU（`ANativeWindow_lock`）连上，销毁前不能再换 GPU 生产者
//! （AOSP 文档的明文限制），所以第 1 条失败后宿主要换一块新的 Surface 再走第 2 条。
#![cfg(target_os = "android")]

use crate::ffi_app::MRApp;
use crate::ffi_embed::*;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jboolean, jfloat, jint, jlong, jstring};
use jni::JNIEnv;
use std::ffi::{c_void, CStr, CString};

#[repr(C)]
struct AndroidBitmapInfo {
    width: u32,
    height: u32,
    stride: u32,
    format: i32,
    flags: u32,
}

#[repr(C)]
struct ANativeWindowBuffer {
    width: i32,
    height: i32,
    stride: i32,
    format: i32,
    bits: *mut c_void,
    reserved: [u32; 6],
}

const ANDROID_BITMAP_FORMAT_RGBA_8888: i32 = 1;
const WINDOW_FORMAT_RGBA_8888: i32 = 1;

#[link(name = "jnigraphics")]
extern "C" {
    fn AndroidBitmap_getInfo(env: *mut jni::sys::JNIEnv, bitmap: jni::sys::jobject, info: *mut AndroidBitmapInfo) -> i32;
    fn AndroidBitmap_lockPixels(env: *mut jni::sys::JNIEnv, bitmap: jni::sys::jobject, addr: *mut *mut c_void) -> i32;
    fn AndroidBitmap_unlockPixels(env: *mut jni::sys::JNIEnv, bitmap: jni::sys::jobject) -> i32;
}

#[link(name = "android")]
extern "C" {
    pub(crate) fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jni::sys::jobject) -> *mut c_void;
    pub(crate) fn ANativeWindow_acquire(window: *mut c_void);
    pub(crate) fn ANativeWindow_release(window: *mut c_void);
    fn ANativeWindow_setBuffersGeometry(window: *mut c_void, width: i32, height: i32, format: i32) -> i32;
    fn ANativeWindow_lock(window: *mut c_void, out: *mut ANativeWindowBuffer, dirty: *mut c_void) -> i32;
    fn ANativeWindow_unlockAndPost(window: *mut c_void) -> i32;
}

fn cstring(env: &mut JNIEnv, s: &JString) -> Option<CString> {
    if s.is_null() {
        return None;
    }
    let text: String = env.get_string(s).ok()?.into();
    CString::new(text).ok()
}

fn app<'a>(h: jlong) -> Option<&'a mut MRApp> {
    // SAFETY: Kotlin 侧只会传 nativeCreate(Ex) 交出去、还没 destroy 的句柄
    unsafe { (h as *mut MRApp).as_mut() }
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeCreateEx(
    mut env: JNIEnv,
    _class: JClass,
    app_dir: JString,
    data_dir: JString,
    width: jint,
    height: jint,
    dpr: jfloat,
    options: JString,
) -> jlong {
    let Some(dir) = cstring(&mut env, &app_dir) else { return 0 };
    let data = cstring(&mut env, &data_dir);
    let opts = cstring(&mut env, &options);
    // 宿主在引擎线程上建引擎：此后这条线程只给引擎用，就在这里把它绑到大核
    crate::compositor::diag::emit(&crate::host::cpu_affinity::pin_current_thread());
    mr_app_create_ex(
        dir.as_ptr(),
        data.as_ref().map(|d| d.as_ptr()).unwrap_or(std::ptr::null()),
        width.max(1) as u32,
        height.max(1) as u32,
        dpr,
        opts.as_ref().map(|o| o.as_ptr()).unwrap_or(std::ptr::null()),
    ) as jlong
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeResize(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    width: jint,
    height: jint,
    dpr: jfloat,
) -> jboolean {
    (mr_app_resize(h as *mut MRApp, width.max(1) as u32, height.max(1) as u32, dpr) != 0) as jboolean
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeSetSafeArea(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    top: jfloat,
    right: jfloat,
    bottom: jfloat,
    left: jfloat,
) -> jboolean {
    (mr_app_set_safe_area(h as *mut MRApp, top, right, bottom, left) != 0) as jboolean
}

/// 滚轮 / 触控板（平板接鼠标、Chromebook）。`precise` 为真表示像素级精密滚动
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeWheel(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    dy: jfloat,
    precise: jboolean,
) {
    crate::ffi_app::mr_app_wheel(h as *mut MRApp, dy, (precise != 0) as i32);
}

/// 页面栈深度（宿主的返回键 / Flutter `PopScope.canPop` 要提前知道）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativePageDepth(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
) -> jint {
    crate::ffi_app::mr_app_page_depth(h as *mut MRApp) as jint
}

/// 把当前帧写进一张 ARGB_8888 的 Bitmap（内存序 RGBA）。返回写入字节数；
/// Bitmap 尺寸与 `nativePixelSize` 不一致、格式不对、锁不上时返回 ≤ 0（错误码取负）。
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativePixelsToBitmap(
    env: JNIEnv,
    _class: JClass,
    h: jlong,
    bitmap: JObject,
) -> jint {
    if h == 0 || bitmap.is_null() {
        return 0;
    }
    let raw_env = env.get_raw();
    let obj = bitmap.as_raw();
    let mut info = AndroidBitmapInfo { width: 0, height: 0, stride: 0, format: 0, flags: 0 };
    // SAFETY: env / bitmap 来自 JNI 调用本身，在本次调用内有效
    unsafe {
        if AndroidBitmap_getInfo(raw_env, obj, &mut info) != 0 {
            return -1;
        }
        if info.format != ANDROID_BITMAP_FORMAT_RGBA_8888 {
            return -2;
        }
        let mut addr: *mut c_void = std::ptr::null_mut();
        if AndroidBitmap_lockPixels(raw_env, obj, &mut addr) != 0 || addr.is_null() {
            return -3;
        }
        let len = info.stride as usize * info.height as usize;
        let n = mr_app_pixels_into(h as *mut MRApp, addr.cast(), len, info.stride, MR_PIXEL_RGBA8);
        AndroidBitmap_unlockPixels(raw_env, obj);
        n as jint
    }
}

/// 挂一块给 CPU 直写用的 Surface（Flutter 的 `SurfaceProducer.getSurface()`）。
/// 之前挂的先放掉。返回 1 成功。
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeAttachCpuWindow(
    env: JNIEnv,
    _class: JClass,
    h: jlong,
    surface: JObject,
) -> jboolean {
    let Some(a) = app(h) else { return 0 };
    release_cpu_window(a);
    if surface.is_null() {
        return 0;
    }
    // SAFETY: surface 是 android.view.Surface，env 在本次调用内有效；fromSurface 自带 +1 引用
    let win = unsafe { ANativeWindow_fromSurface(env.get_raw(), surface.as_raw()) };
    if win.is_null() {
        return 0;
    }
    a.cpu_window = win as usize;
    a.cpu_window_size = (0, 0);
    1
}

/// 放掉 CPU 直写的 Surface（`onSurfaceCleanup` 里**同步**调：返回后引擎不再碰它）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeDetachCpuWindow(_env: JNIEnv, _class: JClass, h: jlong) {
    if let Some(a) = app(h) {
        release_cpu_window(a);
    }
}

/// 还 `mr_app_attach_surface` acquire 的那份引用。调用前引擎必须已经放掉合成器
/// （它的 swapchain 还连着这块 window）
pub(crate) fn release_gpu_window(a: &mut MRApp) {
    if a.gpu_window != 0 {
        // SAFETY: 是 mr_app_attach_surface acquire 的、还没放过的引用；合成器已先放掉
        unsafe { ANativeWindow_release(a.gpu_window as *mut c_void) };
        a.gpu_window = 0;
    }
}

pub(crate) fn release_cpu_window(a: &mut MRApp) {
    if a.cpu_window != 0 {
        // SAFETY: 是 nativeAttachCpuWindow 拿到的、还没放过的引用
        unsafe { ANativeWindow_release(a.cpu_window as *mut c_void) };
        a.cpu_window = 0;
        a.cpu_window_size = (0, 0);
    }
}

/// 把当前帧直接写进挂上的 Surface 并提交。返回 1 成功；≤ 0 是失败（锁不上：
/// 这块 Surface 不支持 CPU 写，宿主应换一块新 Surface 改走 Bitmap 那条路）。
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeBlitToWindow(_env: JNIEnv, _class: JClass, h: jlong) -> jint {
    let Some(a) = app(h) else { return 0 };
    if a.cpu_window == 0 {
        return 0;
    }
    let win = a.cpu_window as *mut c_void;
    let (pw, ph) = a.engine.pixel_size();
    // SAFETY: win 是挂上的 ANativeWindow；buffer 由 lock 填好，unlockAndPost 之前有效
    unsafe {
        if a.cpu_window_size != (pw, ph) {
            if ANativeWindow_setBuffersGeometry(win, pw as i32, ph as i32, WINDOW_FORMAT_RGBA_8888) != 0 {
                return -1;
            }
            a.cpu_window_size = (pw, ph);
        }
        let t0 = std::time::Instant::now();
        let mut buf = ANativeWindowBuffer { width: 0, height: 0, stride: 0, format: 0, bits: std::ptr::null_mut(), reserved: [0; 6] };
        let r = ANativeWindow_lock(win, &mut buf, std::ptr::null_mut());
        if r != 0 || buf.bits.is_null() {
            return if r < 0 { r } else { -2 };
        }
        let t1 = std::time::Instant::now();
        let stride = (buf.stride.max(0) as u32) * 4;
        let len = stride as usize * buf.height.max(0) as usize;
        let n = if buf.width as u32 == pw && buf.height as u32 == ph && buf.format == WINDOW_FORMAT_RGBA_8888 {
            a.engine.write_pixels(std::slice::from_raw_parts_mut(buf.bits.cast(), len), stride as usize, crate::host::PixelFormat::Rgba8)
        } else {
            0
        };
        let t2 = std::time::Instant::now();
        ANativeWindow_unlockAndPost(win);
        blit_stats::record(t0, t1, t2);
        if n == 0 {
            -3
        } else {
            1
        }
    }
}

/// 在逻辑层执行 JS；抛异常返回 null
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeEval(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    code: JString,
) -> jstring {
    let Some(c) = cstring(&mut env, &code) else { return std::ptr::null_mut() };
    let p = mr_app_eval(h as *mut MRApp, c.as_ptr());
    if p.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: 指针活到下一次 mr_app_* 调用，这里立刻转成 Java 字符串
    let text = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
    env.new_string(text).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// 取一条事件（JSON）；没有了返回 null
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativePollEvent(env: JNIEnv, _class: JClass, h: jlong) -> jstring {
    let p = mr_app_poll_event(h as *mut MRApp);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: 指针活到下一次 mr_app_* 调用，这里立刻转成 Java 字符串
    let text = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
    env.new_string(text).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeHostReply(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    id: jint,
    ok: jboolean,
    result: JString,
) -> jboolean {
    let res = cstring(&mut env, &result);
    (mr_app_host_reply(
        h as *mut MRApp,
        id.max(0) as u32,
        (ok != 0) as i32,
        res.as_ref().map(|r| r.as_ptr()).unwrap_or(std::ptr::null()),
    ) != 0) as jboolean
}

/// 宿主推事件给 `wx.onXxx`（见 `mr_app_dispatch_event`）；返回收到它的监听者个数
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeDispatchEvent(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    name: JString,
    payload: JString,
) -> jint {
    let Some(n) = cstring(&mut env, &name) else { return 0 };
    let p = cstring(&mut env, &payload);
    crate::ffi_embed::mr_app_dispatch_event(h as *mut MRApp, n.as_ptr(), p.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null()))
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeNativeViewEvent(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    key: JString,
    name: JString,
    detail: JString,
) -> jboolean {
    let (Some(k), Some(n)) = (cstring(&mut env, &key), cstring(&mut env, &name)) else { return 0 };
    let d = cstring(&mut env, &detail);
    (mr_app_native_view_event(
        h as *mut MRApp,
        k.as_ptr(),
        n.as_ptr(),
        d.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null()),
    ) != 0) as jboolean
}

/// 同层渲染的视频帧（`ImageReader` 的 `YUV_420_888`，三个平面是 direct ByteBuffer）。
/// 不要句柄：帧存在进程级的表里，宿主在自己的解码线程上调（见 `mr_video_frame_yuv`）
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeVideoFrameYuv(
    mut env: JNIEnv,
    _class: JClass,
    key: JString,
    width: jint,
    height: jint,
    y: JObject,
    y_stride: jint,
    u: JObject,
    v: JObject,
    uv_stride: jint,
    uv_pixel_stride: jint,
) -> jboolean {
    let Some(k) = cstring(&mut env, &key) else { return 0 };
    let addr = |env: &mut JNIEnv, o: JObject| -> *const u8 {
        let buf = jni::objects::JByteBuffer::from(o);
        env.get_direct_buffer_address(&buf).map(|p| p as *const u8).unwrap_or(std::ptr::null())
    };
    let (yp, up, vp) = (addr(&mut env, y), addr(&mut env, u), addr(&mut env, v));
    (mr_video_frame_yuv(
        k.as_ptr(),
        width.max(0) as u32,
        height.max(0) as u32,
        yp,
        y_stride.max(0) as u32,
        up,
        vp,
        uv_stride.max(0) as u32,
        uv_pixel_stride.max(1) as u32,
    ) != 0) as jboolean
}

/// 同层渲染的视频帧（RGBA，direct ByteBuffer，`stride` 字节一行；插件用 GLES 从 SurfaceTexture 读回来的）。
/// 不要句柄，任何线程都可以调（见 `mr_video_frame_rgba`）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeVideoFrameRgba(
    mut env: JNIEnv,
    _class: JClass,
    key: JString,
    width: jint,
    height: jint,
    stride: jint,
    data: JObject,
) -> jboolean {
    let Some(k) = cstring(&mut env, &key) else { return 0 };
    let buf = jni::objects::JByteBuffer::from(data);
    let Ok(p) = env.get_direct_buffer_address(&buf) else { return 0 };
    (mr_video_frame_rgba(k.as_ptr(), width.max(0) as u32, height.max(0) as u32, stride.max(0) as u32, p) != 0) as jboolean
}

/// 同层渲染的通用组件（`<map>` 这类）宿主画好的一帧（RGBA，direct ByteBuffer）。不要句柄，任何线程都可以调
/// （见 `mr_native_view_frame_rgba`）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeViewFrameRgba(
    mut env: JNIEnv,
    _class: JClass,
    key: JString,
    width: jint,
    height: jint,
    stride: jint,
    data: JObject,
) -> jboolean {
    let Some(k) = cstring(&mut env, &key) else { return 0 };
    let buf = jni::objects::JByteBuffer::from(data);
    let Ok(p) = env.get_direct_buffer_address(&buf) else { return 0 };
    (mr_native_view_frame_rgba(k.as_ptr(), width.max(0) as u32, height.max(0) as u32, stride.max(0) as u32, p) != 0) as jboolean
}

/// `MINI_FPS`：上屏三段各自的耗时，每秒一行（等空闲缓冲 / 写像素 / 提交）。
/// 宿主那边只量得到总数；分不开的话不知道是 CPU 写得慢，还是 Flutter 拿着缓冲不放、在等
mod blit_stats {
    use std::cell::RefCell;
    use std::time::Instant;

    #[derive(Default)]
    struct S {
        start: Option<Instant>,
        n: u32,
        lock: (f32, f32),
        write: (f32, f32),
        post: (f32, f32),
    }

    thread_local! {
        static ON: bool = std::env::var_os("MINI_FPS").is_some();
        static STATS: RefCell<S> = RefCell::new(S::default());
    }

    fn add(acc: &mut (f32, f32), ms: f32) {
        acc.0 += ms;
        acc.1 = acc.1.max(ms);
    }

    pub(super) fn record(t0: Instant, t1: Instant, t2: Instant) {
        if !ON.with(|o| *o) {
            return;
        }
        let t3 = Instant::now();
        let ms = |a: Instant, b: Instant| (b - a).as_secs_f32() * 1000.0;
        STATS.with(|s| {
            let mut s = s.borrow_mut();
            let start = *s.start.get_or_insert(t0);
            s.n += 1;
            add(&mut s.lock, ms(t0, t1));
            add(&mut s.write, ms(t1, t2));
            add(&mut s.post, ms(t2, t3));
            if (t3 - start).as_secs_f32() >= 1.0 {
                let n = s.n as f32;
                crate::compositor::diag::emit(&format!(
                    "📤 [直写] {} 次  等缓冲 平均 {:.1} 最慢 {:.1}ms  写像素 平均 {:.1} 最慢 {:.1}ms  提交 平均 {:.1} 最慢 {:.1}ms",
                    s.n, s.lock.0 / n, s.lock.1, s.write.0 / n, s.write.1, s.post.0 / n, s.post.1
                ));
                *s = S { start: Some(t3), ..S::default() };
            }
        });
    }
}
