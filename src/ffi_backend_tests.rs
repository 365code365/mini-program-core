//! `ffi_backend` 的 C ABI 用例（设计 §13.1 的 FFI 部分）。
//!
//! 照 `ffi_app.rs` 的写法用 `sample/` 里固定的 sample-app，只跑首页、不联网、不开窗口。
//! 除了「默认值」那条走真正的 `mr_app_create`，其余用 [`HostEnv`] 显式给环境建实例：
//! 开发者 shell 里设着的 `MINI_BACKEND` 不该让用例时过时不过（R14：测试不读进程环境）。
use super::*;
use crate::compositor::{BackendPref, EnvValue, HostEnv};
use crate::ffi_app::*;
use crate::host::MiniEngine;
use std::ffi::{CStr, CString};

fn sample_dir() -> Option<String> {
    let p = crate::app_dir::resolve("sample-app");
    // 示例不在时跳过（与 ffi_app 的用例一致）
    p.join("app.json").exists().then(|| p.to_string_lossy().to_string())
}

/// 每个用例一个独立 storage 目录：并行跑时共用会互相读到对方写的数据
fn data_dir(slot: &str) -> String {
    let d = crate::data_dir::subdir(&format!("ffi-backend-tests/{slot}"));
    std::fs::remove_dir_all(&d).ok();
    d.to_string_lossy().to_string()
}

fn app_with(slot: &str, env: HostEnv) -> Option<*mut MRApp> {
    let root = sample_dir()?;
    let engine = MiniEngine::new_with_env(&root, Some(&data_dir(slot)), 375, 667, 2.0, env)
        .expect("创建引擎");
    let app = Box::into_raw(Box::new(MRApp::new(engine)));
    assert_eq!(mr_app_launch(app, std::ptr::null()), 1, "首页应能打开");
    Some(app)
}

fn reason(app: *mut MRApp) -> String {
    let p = mr_app_backend_reason(app);
    assert!(!p.is_null(), "有实例时原因串不该是 NULL");
    unsafe { CStr::from_ptr(p) }.to_str().expect("原因串应是 UTF-8").to_string()
}

fn last_error(app: *mut MRApp) -> String {
    let p = mr_app_last_error(app);
    assert!(!p.is_null(), "参数错误应写 last_error");
    unsafe { CStr::from_ptr(p) }.to_string_lossy().to_string()
}

/// 当前编译目标认的那种 surface（Apple → Metal 层、Android → ANativeWindow）
fn supported_kind() -> Option<c_int> {
    if cfg!(target_vendor = "apple") {
        Some(MR_SURFACE_METAL_LAYER)
    } else if cfg!(target_os = "android") {
        Some(MR_SURFACE_ANDROID_WINDOW)
    } else {
        None
    }
}

/// 一个非空、但永远不会被解引用的「原生 surface」（现在 attach 不碰指针）
fn fake_native(slot: &mut u64) -> *mut c_void {
    (slot as *mut u64).cast()
}

#[test]
fn null_app_never_crashes() {
    let null = std::ptr::null_mut::<MRApp>();
    let mut x = 0u64;
    assert_eq!(mr_app_set_backend(null, MR_BACKEND_GPU), 0);
    mr_app_set_device_info(null, 8_000_000_000, 0, 31);
    assert_eq!(mr_app_attach_surface(null, MR_SURFACE_METAL_LAYER, fake_native(&mut x)), 0);
    mr_app_detach_surface(null);
    assert_eq!(mr_app_backend(null), 0);
    assert!(mr_app_backend_reason(null).is_null());
    assert_eq!(mr_app_pump_ex(null, 16), 0);
}

#[test]
fn constants_match_the_header() {
    assert_eq!((MR_BACKEND_AUTO, MR_BACKEND_CPU, MR_BACKEND_GPU), (0, 1, 2));
    assert_eq!((MR_SURFACE_METAL_LAYER, MR_SURFACE_ANDROID_WINDOW), (1, 2));
    assert_eq!(
        (MR_FRAME_NEW, MR_FRAME_PRESENTED, MR_FRAME_BACKEND_CHANGED, MR_FRAME_PENDING),
        (1, 2, 4, 8)
    );
}

#[test]
fn default_is_cpu_and_first_frame_is_just_new() {
    // 走真正的 mr_app_create（读进程环境）：现在没有 GPU 合成器，不管环境怎么设都是 CPU
    let Some(dir) = sample_dir() else { return };
    let dir = CString::new(dir).unwrap();
    let data = CString::new(data_dir("default")).unwrap();
    let app = mr_app_create(dir.as_ptr(), data.as_ptr(), 375, 667, 2.0);
    assert!(!app.is_null());
    assert_eq!(mr_app_launch(app, std::ptr::null()), 1);
    assert_eq!(mr_app_backend(app), MR_BACKEND_CPU, "C ABI 默认 CPU");
    assert!(!reason(app).is_empty());
    assert_eq!(mr_app_pump_ex(app, 1), MR_FRAME_NEW, "首帧只该是 NEW");
    // CPU：只会是 0 或 NEW，从不出现 PRESENTED / BACKEND_CHANGED / PENDING
    for i in 0..30u64 {
        if i % 7 == 0 {
            mr_app_on_show(app); // 逼一帧重画，确保循环里真的出过帧
        }
        let f = mr_app_pump_ex(app, 2 + i * 16);
        assert!(f == 0 || f == MR_FRAME_NEW, "第 {i} 帧标志位 {f}");
        assert_eq!(f & MR_FRAME_PRESENTED, 0);
    }
    mr_app_destroy(app);
}

#[test]
fn bad_params_return_zero_and_write_last_error() {
    let Some(app) = app_with("bad-params", HostEnv::default()) else { return };
    let mut x = 0u64;
    assert_eq!(mr_app_set_backend(app, 7), 0);
    assert!(last_error(app).contains("pref=7"));
    assert_eq!(mr_app_set_backend(app, -1), 0);
    for p in [MR_BACKEND_AUTO, MR_BACKEND_CPU, MR_BACKEND_GPU] {
        assert_eq!(mr_app_set_backend(app, p), 1);
        assert_eq!(mr_app_backend(app), MR_BACKEND_CPU);
    }
    assert_eq!(mr_app_attach_surface(app, 9, fake_native(&mut x)), 0);
    assert!(last_error(app).contains("kind=9"));
    for kind in [MR_SURFACE_METAL_LAYER, MR_SURFACE_ANDROID_WINDOW] {
        assert_eq!(mr_app_attach_surface(app, kind, std::ptr::null_mut()), 0, "native 为空");
    }
    // 编译目标不认的种类也是参数错误（macOS 上传 ANativeWindow、Android 上传 Metal 层）
    let foreign = if cfg!(target_os = "android") { MR_SURFACE_METAL_LAYER } else { MR_SURFACE_ANDROID_WINDOW };
    assert_eq!(mr_app_attach_surface(app, foreign, fake_native(&mut x)), 0);
    mr_app_destroy(app);
}

#[cfg(not(feature = "gpu"))]
#[test]
fn attach_without_gpu_feature_stays_cpu_and_says_not_compiled() {
    let Some(app) = app_with("not-compiled", HostEnv::default()) else { return };
    let Some(kind) = supported_kind() else { return mr_app_destroy(app) };
    let mut x = 0u64;
    // 偏好还是默认 cpu：原因写偏好，不谈「未编译」
    assert_eq!(mr_app_attach_surface(app, kind, fake_native(&mut x)), MR_BACKEND_CPU);
    assert!(reason(app).contains("偏好 cpu"), "{}", reason(app));
    for p in [MR_BACKEND_GPU, MR_BACKEND_AUTO] {
        assert_eq!(mr_app_set_backend(app, p), 1);
        assert_eq!(mr_app_attach_surface(app, kind, fake_native(&mut x)), MR_BACKEND_CPU);
        assert!(reason(app).contains("未编译"), "{}", reason(app));
        assert_eq!(mr_app_backend(app), MR_BACKEND_CPU);
    }
    // attach → detach → attach：宿主主动的挂卸不带 BACKEND_CHANGED（R5），CPU 下也从不 PRESENTED
    assert_eq!(mr_app_pump_ex(app, 1), MR_FRAME_NEW);
    mr_app_detach_surface(app);
    assert_eq!(mr_app_pump_ex(app, 17) & !MR_FRAME_NEW, 0);
    assert_eq!(mr_app_attach_surface(app, kind, fake_native(&mut x)), MR_BACKEND_CPU);
    assert_eq!(mr_app_pump_ex(app, 33) & !MR_FRAME_NEW, 0);
    mr_app_destroy(app);
}

#[cfg(feature = "gpu")]
#[test]
fn attach_with_the_empty_gpu_feature_is_still_cpu() {
    // `gpu = []` 目前不含实现：门槛都过了也只能是 CPU，原因要说清楚不是「未编译」
    let Some(app) = app_with("gpu-pending", HostEnv::default()) else { return };
    let Some(kind) = supported_kind() else { return mr_app_destroy(app) };
    let mut x = 0u64;
    assert_eq!(mr_app_set_backend(app, MR_BACKEND_GPU), 1);
    assert!(reason(app).contains("未挂载 surface"), "{}", reason(app));
    assert_eq!(mr_app_attach_surface(app, kind, fake_native(&mut x)), MR_BACKEND_CPU);
    let r = reason(app);
    assert!(r.contains("开发中") && !r.contains("未编译"), "{r}");
    mr_app_destroy(app);
}

#[test]
fn mini_backend_env_beats_set_backend() {
    let env = HostEnv { backend: EnvValue::Valid(BackendPref::Cpu), ..HostEnv::default() };
    let Some(app) = app_with("env-wins", env) else { return };
    assert_eq!(mr_app_set_backend(app, MR_BACKEND_GPU), 1, "调用本身是成功的");
    assert_eq!(mr_app_backend(app), MR_BACKEND_CPU);
    assert_eq!(reason(app), "偏好 cpu（MINI_BACKEND）");
    mr_app_destroy(app);
}

#[test]
fn pump_agrees_with_pump_ex_new_bit() {
    // 同一帧没法调两次，所以分两部分：
    // ① 逼出帧的那几帧（on_show 置重绘）两个入口都必须报「有新画面」；
    // ② 其余帧各自只会在 {0,1} / {0,NEW} 里取值。mr_app_pump 本身就是 `pump_ex & NEW`。
    let Some(app) = app_with("pump-vs-ex", HostEnv::default()) else { return };
    assert_eq!(mr_app_pump(app, 1), 1, "首帧两个入口都是 1");
    let mut t = 1u64;
    for i in 0..20 {
        t += 16;
        mr_app_on_show(app);
        if i % 2 == 0 {
            assert_eq!(mr_app_pump(app, t), 1);
        } else {
            assert_eq!(mr_app_pump_ex(app, t) & MR_FRAME_NEW, MR_FRAME_NEW);
        }
        t += 16;
        let (a, b) = (mr_app_pump(app, t), mr_app_pump_ex(app, t + 8));
        assert!(a == 0 || a == 1, "mr_app_pump 只返回 0/1，实际 {a}");
        assert!(b == 0 || b == MR_FRAME_NEW, "CPU 下 pump_ex 只返回 0/NEW，实际 {b}");
    }
    mr_app_destroy(app);
}
