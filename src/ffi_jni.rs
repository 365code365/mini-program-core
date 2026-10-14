//! Android 的 JNI 入口。
//!
//! 这一层刻意做得极薄：Kotlin 侧的 `MiniEngine` 只有下面这十几个 `external fun`，
//! 每个都只是把参数转一下再喂给 [`crate::ffi_app`] 的 C ABI。真正的宿主逻辑在
//! `crate::host::MiniEngine`，与 iOS/桌面共用同一份（`tools/sdk-parity.sh` 守着
//! 「SDK 那条路和桌面窗体逐像素一致」）。
//!
//! ## 为什么不用 wry
//! wry 给的是一个 WebView 子视图，本引擎是自绘的，用不上；而且 wry 的 android 后端
//! 要求宿主 Activity 继承它的基类（集成进既有 App 基本谈不下来），鸿蒙也没有后端。
//! 小程序里的原生层组件（`web-view` / `video` / `map`）应当由宿主用系统控件承载 ——
//! 这也正是微信的做法（原生组件层级最高，只有 `cover-view` 能盖住）。
//!
//! ## 线程
//! 所有方法必须在**同一个线程**调用（逻辑层是 QuickJS，不是线程安全的）。
//! Kotlin 侧用一条专用渲染线程，见 `sdk/android/MiniProgramView.kt`。

#![cfg(target_os = "android")]

use crate::ffi_app::*;
use crate::ffi_backend::*;
use jni::objects::{JByteArray, JClass, JObject, JString};
use jni::sys::{jboolean, jfloat, jint, jlong, jstring};
use jni::JNIEnv;
use std::ffi::{CStr, CString};

fn cstring(env: &mut JNIEnv, s: &JString) -> Option<CString> {
    if s.is_null() {
        return None;
    }
    let text: String = env.get_string(s).ok()?.into();
    CString::new(text).ok()
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeCreate(
    mut env: JNIEnv,
    _class: JClass,
    app_dir: JString,
    data_dir: JString,
    width: jint,
    height: jint,
    dpr: jfloat,
) -> jlong {
    let Some(app) = cstring(&mut env, &app_dir) else { return 0 };
    let data = cstring(&mut env, &data_dir);
    let p = mr_app_create(
        app.as_ptr(),
        data.as_ref().map(|d| d.as_ptr()).unwrap_or(std::ptr::null()),
        width.max(1) as u32,
        height.max(1) as u32,
        dpr,
    );
    p as jlong
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
) {
    mr_app_destroy(h as *mut MRApp);
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeLaunch(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    route: JString,
) -> jboolean {
    let r = cstring(&mut env, &route);
    let ok = mr_app_launch(
        h as *mut MRApp,
        r.as_ref().map(|v| v.as_ptr()).unwrap_or(std::ptr::null()),
    );
    (ok != 0) as jboolean
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeNavigate(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    url: JString,
) -> jboolean {
    let Some(u) = cstring(&mut env, &url) else { return 0 };
    (mr_app_navigate(h as *mut MRApp, u.as_ptr()) != 0) as jboolean
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativePump(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    now_ms: jlong,
) -> jboolean {
    (mr_app_pump(h as *mut MRApp, now_ms.max(0) as u64) != 0) as jboolean
}

/// 像素尺寸打包成一个 long：高 32 位是宽、低 32 位是高（省一次 JNI 数组往返）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativePixelSize(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
) -> jlong {
    let (mut w, mut hh) = (0u32, 0u32);
    mr_app_pixel_size(h as *mut MRApp, &mut w, &mut hh);
    ((w as jlong) << 32) | hh as jlong
}

/// 把当前帧拷进 Kotlin 的 ByteArray（RGBA8）。返回写入字节数。
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativePixels(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    out: JByteArray,
) -> jint {
    let len = env.get_array_length(&out).unwrap_or(0);
    if len <= 0 {
        return 0;
    }
    // 直接锁住数组内存写进去，避免多一次中间拷贝
    let elems = match unsafe { env.get_array_elements(&out, jni::objects::ReleaseMode::CopyBack) }
    {
        Ok(e) => e,
        Err(_) => return 0,
    };
    let ptr = elems.as_ptr() as *mut u8;
    mr_app_pixels(h as *mut MRApp, ptr, len as usize) as jint
}

/// `phase`: 0 按下 / 1 移动 / 2 抬起 / 其它 取消
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativePointer(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    phase: jint,
    x: jfloat,
    y: jfloat,
) {
    let app = h as *mut MRApp;
    match phase {
        0 => mr_app_pointer_down(app, x, y),
        1 => mr_app_pointer_move(app, x, y),
        2 => mr_app_pointer_up(app, x, y),
        _ => mr_app_pointer_cancel(app),
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeTextInput(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    text: JString,
) {
    if let Some(t) = cstring(&mut env, &text) {
        mr_app_text_input(h as *mut MRApp, t.as_ptr());
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeKey(
    mut env: JNIEnv,
    _class: JClass,
    h: jlong,
    name: JString,
) {
    if let Some(n) = cstring(&mut env, &name) {
        mr_app_key(h as *mut MRApp, n.as_ptr());
    }
}

/// 返回 false 表示页面栈只剩一页，宿主该关掉小程序（`finish()`）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeBack(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
) -> jboolean {
    (mr_app_back(h as *mut MRApp) != 0) as jboolean
}

/// `what`: 0 进前台 / 1 进后台 / 其它 内存告警
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeLifecycle(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    what: jint,
) {
    let app = h as *mut MRApp;
    match what {
        0 => mr_app_on_show(app),
        1 => mr_app_on_hide(app),
        _ => mr_app_trim_memory(app),
    }
}

// ───────────────────────────── 渲染后端（设计 §8.4）─────────────────────────────
//
// 老的 JNI 函数一个没动；下面这些只是把 `ffi_backend` 的 C ABI 转一下。
// 新的 Kotlin 配老的 .so 会在调用新方法时抛 UnsatisfiedLinkError —— 两者在 AAR 里
// 一起发布，不单独升级。

/// `pref`: 0 auto / 1 cpu / 2 gpu。越界返回 false
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeSetBackend(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    pref: jint,
) -> jboolean {
    (mr_app_set_backend(h as *mut MRApp, pref) != 0) as jboolean
}

/// 机型分级用：`ActivityManager.MemoryInfo.totalMem`、`isLowRamDevice`、`Build.VERSION.SDK_INT`
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeSetDeviceInfo(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    total_mem: jlong,
    low_ram: jboolean,
    api: jint,
) {
    mr_app_set_device_info(h as *mut MRApp, total_mem.max(0) as u64, (low_ram != 0) as i32, api.max(0) as u32);
}

/// 挂 `SurfaceView` 的 surface。返回实际后端：2 GPU / 1 CPU / 0 参数错误。
///
/// 现在**不调** `ANativeWindow_fromSurface`、也不链接 libandroid：没编 gpu 时实际后端
/// 只能是 CPU，直接返回 1；编了 gpu（目前是空壳）时只把种类记给引擎，原因串照实说
/// 「GPU 合成器还在开发中」。移动端 GPU 接线时这里换成
/// `fromSurface`（+1）→ `mr_app_attach_surface`（引擎内部再 acquire）→ `release`（−1）。
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeAttachSurface(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    surface: JObject,
) -> jint {
    let app = h as *mut MRApp;
    if app.is_null() || surface.is_null() {
        return 0;
    }
    #[cfg(not(feature = "gpu"))]
    let code = MR_BACKEND_CPU;
    #[cfg(feature = "gpu")]
    let code = attach_kind_only(app, crate::compositor::SurfaceKind::AndroidWindow);
    code
}

/// `surfaceDestroyed` 里**同步**调用：返回之后引擎不再碰这块 surface
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeDetachSurface(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
) {
    mr_app_detach_surface(h as *mut MRApp);
}

/// 实际后端：1 CPU / 2 GPU；句柄为空返回 0
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeBackend(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
) -> jint {
    mr_app_backend(h as *mut MRApp)
}

/// 实际后端的原因；句柄为空返回 null
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativeBackendReason(
    env: JNIEnv,
    _class: JClass,
    h: jlong,
) -> jstring {
    let p = mr_app_backend_reason(h as *mut MRApp);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    let text = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
    env.new_string(text).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// 跑一帧，返回 `MR_FRAME_*` 的按位或（`nativePump` 照旧只返回 Boolean）
#[no_mangle]
pub extern "system" fn Java_dev_minirender_MiniEngine_nativePumpEx(
    _env: JNIEnv,
    _class: JClass,
    h: jlong,
    now_ms: jlong,
) -> jint {
    mr_app_pump_ex(h as *mut MRApp, now_ms.max(0) as u64)
}
