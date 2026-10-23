//! 渲染后端的 C ABI（设计 §8.1、§8.2；v2 修订 R4、R5）：只增不改。
//!
//! - 老的 `mr_app_*` 签名与语义一个都没动。尤其 `mr_app_pump` 继续只返回 0 / 1 ——
//!   头文件对它的承诺是「1 表示有新画面」，写 `== 1` 的调用方遇到别的值会漏帧。
//!   带标志位的版本是新函数 [`mr_app_pump_ex`]。
//! - 这里的符号**有没有 `gpu` feature 都导出**：C ABI 的符号集合不随构建配置变化，
//!   宿主不用为两种构建写两套链接。没编 gpu 时它们如实报「CPU」与原因。
//! - 不调这些新函数的宿主永远是 CPU，与以前逐字节一致（`tools/sdk-parity.sh` 守着）。
//!
//! 编了 `gpu` 时 `mr_app_attach_surface` 把原生指针交给引擎（engine_gpu.rs）：门槛与自检
//! 都过了就由 GPU 直接上屏，`mr_app_pump_ex` 带 `MR_FRAME_PRESENTED`。所有权照头文件：
//! `ANativeWindow*` 挂载时 acquire、卸载 / destroy 时（放掉合成器之后）release；
//! `CAMetalLayer*` 由调用方持有到 detach / destroy 返回。没编 `gpu` 时只记种类、不碰指针，
//! 返回 1（CPU）。另有 `ffi_embed` 的两位提示位 WANTS_MORE / EVENTS，与后端无关。
//! 线程约定与其它 `mr_app_*` 相同：同一个线程调用。
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

/// surface 来之前先在后台线程上开 GPU 合成器（设备 + 管线 + 自检），挂 surface 时直接用。
/// 设好偏好与机型信息之后、`mr_app_launch` 之前调：开 GPU 与首页启动并行（一加 PLY110 首帧早出约 0.3s）。
/// 返回 1 表示开始预热；偏好是 cpu / 门槛不过 / 没编 gpu 时返回 0（不影响之后挂 surface）
#[no_mangle]
pub extern "C" fn mr_app_prewarm_gpu(app: *mut MRApp) -> c_int {
    let Some(_a) = app_mut(app) else { return 0 };
    #[cfg(feature = "gpu")]
    return _a.engine.prewarm_gpu() as c_int;
    #[cfg(not(feature = "gpu"))]
    0
}

/// **建引擎之前**调：字体与 GPU 合成器各开一条线程预热，与建引擎、开首页并行（一加 PLY110 首帧早出约 0.4s）。
/// 参数与之后的 `mr_app_create_ex`（逻辑宽高、dpr）、`mr_app_set_backend`、`mr_app_set_device_info` 一致；
/// `pref` 传 -1 表示不指定。之后照常 `mr_app_prewarm_gpu`：它会认领这份。返回是否开始预热 GPU
#[no_mangle]
pub extern "C" fn mr_warmup(width: u32, height: u32, dpr: f32, pref: c_int, total_mem_bytes: u64, low_ram: c_int, os_api: u32) -> c_int {
    let _device = DeviceInfo { total_mem_bytes, low_ram: low_ram != 0, os_api };
    let _pref = pref::pref_from_code(pref);
    #[cfg(feature = "gpu")]
    return crate::host::warmup(width, height, dpr, _pref, _device) as c_int;
    #[cfg(not(feature = "gpu"))]
    {
        let _ = (width, height, dpr);
        std::thread::spawn(|| crate::text::shared_fonts());
        0
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
    #[cfg(feature = "gpu")]
    {
        // 重复 attach 等于先 detach：先放掉旧合成器（它还连着旧 window），再还旧引用
        a.engine.detach_native_surface();
        #[cfg(target_os = "android")]
        {
            crate::ffi_jni_embed::release_gpu_window(a);
            if k == SurfaceKind::AndroidWindow {
                // SAFETY: 调用方给的是有效的 ANativeWindow*；这份引用到 detach / destroy 才还
                unsafe { crate::ffi_jni_embed::ANativeWindow_acquire(native) };
                a.gpu_window = native as usize;
            }
        }
        a.engine.attach_native_surface(k, native).code()
    }
    // 没编 GPU：指针不解引用、不保留，只记种类
    #[cfg(not(feature = "gpu"))]
    a.engine.attach_surface(k).code()
}

/// 卸 surface（surface 将被销毁 / 进后台前调用）。之后是 CPU。
#[no_mangle]
pub extern "C" fn mr_app_detach_surface(app: *mut MRApp) {
    if let Some(a) = app_mut(app) {
        a.engine.detach_surface();
        // 合成器已放掉，这时才还挂载时 acquire 的 window
        #[cfg(target_os = "android")]
        crate::ffi_jni_embed::release_gpu_window(a);
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

#[cfg(test)]
#[path = "ffi_backend_tests.rs"]
mod tests;
