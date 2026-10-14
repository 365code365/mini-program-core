//! 渲染后端的 C ABI（设计 §8.1、§8.2；v2 修订 R4、R5）：只增不改。
//!
//! - 老的 `mr_app_*` 签名与语义一个都没动。尤其 `mr_app_pump` 继续只返回 0 / 1 ——
//!   头文件对它的承诺是「1 表示有新画面」，写 `== 1` 的调用方遇到别的值会漏帧。
//!   带标志位的版本是新函数 [`mr_app_pump_ex`]。
//! - 这里的符号**有没有 `gpu` feature 都导出**：C ABI 的符号集合不随构建配置变化，
//!   宿主不用为两种构建写两套链接。没编 gpu 时它们如实报「CPU」与原因。
//! - 不调这些新函数的宿主永远是 CPU，与以前逐字节一致（`tools/sdk-parity.sh` 守着）。
//!
//! 现在还没有 GPU 合成器：`mr_app_attach_surface` 校验参数后只记下 surface 的种类，
//! 指针既不解引用也不保留，返回 1（CPU）；`mr_app_pump_ex` 只会返回 0 或
//! `MR_FRAME_NEW`。线程约定与其它 `mr_app_*` 相同：同一个线程调用。
use crate::compositor::{pref, DeviceInfo, SurfaceKind};
use crate::ffi_app::MRApp;
use crate::host::engine_backend;
use std::ffi::{c_char, c_int, c_void, CString};

/// 后端偏好 / 实际后端
pub const MR_BACKEND_AUTO: c_int = 0;
pub const MR_BACKEND_CPU: c_int = 1;
pub const MR_BACKEND_GPU: c_int = 2;
/// surface 种类：`CAMetalLayer*`（iOS / macOS）
pub const MR_SURFACE_METAL_LAYER: c_int = 1;
/// surface 种类：`ANativeWindow*`（Android）
pub const MR_SURFACE_ANDROID_WINDOW: c_int = 2;
/// `mr_app_pump_ex` 的返回位：有新画面
pub const MR_FRAME_NEW: c_int = engine_backend::FRAME_NEW as c_int;
/// 已由引擎画到挂载的 surface 上，宿主不要再取像素显示
pub const MR_FRAME_PRESENTED: c_int = engine_backend::FRAME_PRESENTED as c_int;
/// 实际后端被引擎改了（回退 / `set_backend` 之后重新选择），宿主要立刻切换显示路径
pub const MR_FRAME_BACKEND_CHANGED: c_int = engine_backend::FRAME_BACKEND_CHANGED as c_int;
/// 本次没上屏，下一次 `pump_ex` 会强制重新合成（即使没有新内容）
pub const MR_FRAME_PENDING: c_int = engine_backend::FRAME_PENDING as c_int;

fn app_mut<'a>(app: *mut MRApp) -> Option<&'a mut MRApp> {
    // SAFETY: 非空时是 mr_app_create 交出去、还没 destroy 的句柄（调用方的约定）
    unsafe { app.as_mut() }
}

/// 设后端偏好（`MR_BACKEND_*`）。成功 1；`pref` 不在 0..=2 返回 0 并写 `last_error`。
///
/// 环境变量 `MINI_BACKEND` 设了的话以它为准（诊断开关压过宿主配置）。
/// 已挂 surface 时会重新选择，实际后端变了的话下一次 `pump_ex` 带 `MR_FRAME_BACKEND_CHANGED`。
#[no_mangle]
pub extern "C" fn mr_app_set_backend(app: *mut MRApp, pref: c_int) -> c_int {
    let Some(a) = app_mut(app) else { return 0 };
    match pref::pref_from_code(pref) {
        Some(p) => {
            a.engine.set_backend_pref(p);
            1
        }
        None => {
            a.last_error = format!("mr_app_set_backend: pref={pref} 不在 0..=2（MR_BACKEND_AUTO/CPU/GPU）");
            0
        }
    }
}

/// 机型信息（机型分级用），挂 surface 之前调用。0 表示未知；`low_ram` 非 0 即真。
#[no_mangle]
pub extern "C" fn mr_app_set_device_info(app: *mut MRApp, total_mem_bytes: u64, low_ram: c_int, os_api: u32) {
    let Some(a) = app_mut(app) else { return };
    a.engine.set_device_info(DeviceInfo { total_mem_bytes, low_ram: low_ram != 0, os_api });
}

/// 挂 surface。返回挂载后的实际后端：2 = GPU 生效；1 = 仍是 CPU（原因见
/// `mr_app_backend_reason`）；0 = 参数错误（`app` / `native` 为空、`kind` 不认识或
/// 当前编译目标不支持这种 surface），写 `last_error`。重复调用等于先卸再挂。
#[no_mangle]
pub extern "C" fn mr_app_attach_surface(app: *mut MRApp, kind: c_int, native: *mut c_void) -> c_int {
    let Some(a) = app_mut(app) else { return 0 };
    let Some(k) = SurfaceKind::from_code(kind) else {
        a.last_error = format!("mr_app_attach_surface: kind={kind} 不认识（MR_SURFACE_METAL_LAYER/ANDROID_WINDOW）");
        return 0;
    };
    if !k.supported_on_target() {
        a.last_error = format!("mr_app_attach_surface: 当前编译目标不支持 kind={kind}");
        return 0;
    }
    if native.is_null() {
        a.last_error = "mr_app_attach_surface: native 为空".into();
        return 0;
    }
    // 没有 GPU 合成器：指针不解引用、不保留（Android 也不 acquire），只记种类
    a.engine.attach_surface(k).code()
}

/// 卸 surface（surface 将被销毁 / 进后台前调用）。之后是 CPU。
#[no_mangle]
pub extern "C" fn mr_app_detach_surface(app: *mut MRApp) {
    if let Some(a) = app_mut(app) {
        a.engine.detach_surface();
    }
}

/// 实际后端：1 = CPU、2 = GPU；`app` 为空返回 0
#[no_mangle]
pub extern "C" fn mr_app_backend(app: *mut MRApp) -> c_int {
    app_mut(app).map(|a| a.engine.backend().active.code()).unwrap_or(0)
}

/// 实际后端的原因（UTF-8）。到下一次对同一 app 调用任何 `mr_app_*` 之前有效；
/// `app` 为空返回 NULL。
#[no_mangle]
pub extern "C" fn mr_app_backend_reason(app: *mut MRApp) -> *const c_char {
    let Some(a) = app_mut(app) else { return std::ptr::null() };
    // 原因串是我们自己拼的，不含 NUL；万一有，去掉而不是返回空串
    let text = a.engine.backend().reason.replace('\0', "");
    a.backend_reason = CString::new(text).unwrap_or_default();
    a.backend_reason.as_ptr()
}

/// 跑一帧，返回 `MR_FRAME_*` 的按位或。
/// `mr_app_pump(app, t) == ((mr_app_pump_ex(app, t) & MR_FRAME_NEW) ? 1 : 0)`。
#[no_mangle]
pub extern "C" fn mr_app_pump_ex(app: *mut MRApp, now_ms: u64) -> c_int {
    app_mut(app).map(|a| a.engine.pump_ex(now_ms) as c_int).unwrap_or(0)
}

/// JNI 那层（编了 gpu 时）用：只记「挂了一块这种 surface」，不经过原生指针。
///
/// 现阶段 JNI 不调 `ANativeWindow_fromSurface`（不链接 libandroid），手里没有
/// `ANativeWindow*` 可传；移动端 GPU 接线时这里换成 fromSurface → attach → release。
#[cfg(all(feature = "gpu", target_os = "android"))]
pub(crate) fn attach_kind_only(app: *mut MRApp, kind: SurfaceKind) -> c_int {
    app_mut(app).map(|a| a.engine.attach_surface(kind).code()).unwrap_or(0)
}

#[cfg(test)]
#[path = "ffi_backend_tests.rs"]
mod tests;
