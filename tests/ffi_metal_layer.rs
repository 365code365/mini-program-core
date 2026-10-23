//! iOS 原生视图（`sdk/ios/Sources/MiniRender/EngineSession.swift`）那一串 **C ABI** 调用，在 macOS 上跑一遍：
//! `mr_warmup` → `mr_app_create_ex` → `set_device_info` → `set_backend(AUTO)` → `prewarm_gpu` → `launch`
//! → `attach_surface(MR_SURFACE_METAL_LAYER, CAMetalLayer*)` → `pump_ex` 到 PRESENTED → 取像素 → 改尺寸
//! → 卸层 → 销毁。
//!
//! 为什么在 macOS 上测：本机没有 Xcode / iPhone SDK，iOS 的库编不出来；而 CAMetalLayer → wgpu 的
//! `CoreAnimationLayer` 这条路在 macOS 与 iOS 上是同一段代码（`engine_gpu.rs::surface_spec`）。
//! `tests/embed_gpu.rs` 测的是 Rust 层的 `MiniEngine`；这里测的是 Swift 实际调的那几个 C 函数与它们的顺序。
//! 没有 GPU 适配器（CI 虚机）时跳过并打「GPU 未测」。
#![cfg(all(target_os = "macos", feature = "gpu"))]

use mini_render::*;
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::Mutex;

/// 两条用例共用 GPU 设备与进程级的预热槽：串行跑
static SERIAL: Mutex<()> = Mutex::new(());

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}
#[link(name = "QuartzCore", kind = "framework")]
extern "C" {}

/// `[obj sel]`。arm64 上 objc_msgSend 必须按真实签名调用
unsafe fn send(obj: *mut c_void, sel: &CStr) -> *mut c_void {
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    f(obj, sel_registerName(sel.as_ptr()))
}

fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ffi-metal-layer").join(name);
    std::fs::remove_dir_all(&root).ok();
    let blocks: String = ["#07c160", "#1677ff", "#fa5151", "#ffc300", "#6467f0"]
        .iter()
        .map(|c| format!(r#"<view style="height:200px;background:{c}">{c}</view>"#))
        .collect();
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"],"window":{"navigationBarTitleText":"Metal"}}"#.to_string()),
        ("app.js", "App({});".into()),
        ("app.wxss", "".into()),
        ("pages/p/p.wxml", blocks),
        ("pages/p/p.wxss", "".into()),
        ("pages/p/p.js", "Page({data:{}});".into()),
        ("pages/p/p.json", "{}".into()),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn reason(app: *mut MRApp) -> String {
    let p = mr_app_backend_reason(app);
    if p.is_null() { String::new() } else { unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned() }
}

/// 跑到 PRESENTED 为止（drawable 一时取不到是 PENDING，下一次补）
fn pump_presented(app: *mut MRApp, t: &mut u64) -> i32 {
    for _ in 0..30 {
        *t += 8;
        let f = mr_app_pump_ex(app, *t);
        if f & MR_FRAME_PRESENTED != 0 {
            return f;
        }
    }
    panic!("30 帧都没上屏：{}", reason(app));
}

#[test]
fn ios_视图的调用顺序_auto_走_gpu_改尺寸_卸层() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("auto");
    let dir = CString::new(root.to_string_lossy().as_ref()).unwrap();
    let data = CString::new(root.join("_data").to_string_lossy().as_ref()).unwrap();
    // 与 Swift 视图同一组：390×844 @3（iPhone 14），auto，8GB，iOS 17
    let (w, h, dpr) = (390u32, 844u32, 3.0f32);
    let mem = 8u64 << 30;
    let warming = mr_warmup(w, h, dpr, MR_BACKEND_AUTO, mem, 0, 17);
    let opts = CString::new(r#"{"systemInfo":{"platform":"ios"},"menuButton":true,"navigationBar":true,"routeAnimation":true,"hostApis":["login"]}"#).unwrap();
    let app = mr_app_create_ex(dir.as_ptr(), data.as_ptr(), w, h, dpr, opts.as_ptr());
    assert!(!app.is_null(), "建引擎失败");
    mr_app_set_device_info(app, mem, 0, 17);
    assert_eq!(mr_app_set_backend(app, MR_BACKEND_AUTO), 1);
    let prewarm = mr_app_prewarm_gpu(app);
    assert_eq!(mr_app_launch(app, std::ptr::null()), 1, "首页没打开");
    let (mut pw, mut ph) = (0u32, 0u32);
    mr_app_pixel_size(app, &mut pw, &mut ph);
    assert_eq!((pw, ph), (1170, 2532));

    // 视图那一侧：一块 CAMetalLayer（Swift 里由视图持有，活到 detach 之后）
    let layer = unsafe { send(objc_getClass(c"CAMetalLayer".as_ptr()), c"new") };
    assert!(!layer.is_null());
    // 挂层之前的帧（Swift 视图在 GPU 模式下不显示它们）：照常出帧、不带 PRESENTED
    let mut t = 1_000u64;
    let f = mr_app_pump_ex(app, t);
    assert_eq!(f & MR_FRAME_PRESENTED, 0, "还没挂层就 PRESENTED：{f:#x}");

    let r = mr_app_attach_surface(app, MR_SURFACE_METAL_LAYER, layer);
    if r != MR_BACKEND_GPU {
        eprintln!("⚠ GPU 未测：attach 返回 {r}（{}）", reason(app));
        mr_app_destroy(app);
        unsafe { send(layer, c"release") };
        return;
    }
    assert_eq!(mr_app_backend(app), MR_BACKEND_GPU);
    eprintln!("🎨 auto → GPU：{}（mr_warmup {warming}，prewarm {prewarm}）", reason(app));
    let f = pump_presented(app, &mut t);
    assert_eq!(f & MR_FRAME_BACKEND_CHANGED, 0, "宿主主动挂载不带 BACKEND_CHANGED：{f:#x}");

    // GPU 帧也能取像素（Swift 的 snapshot 走这条）：BGRA，第一块是导航栏下的 #07c160
    let stride = pw as usize * 4;
    let mut buf = vec![0u8; stride * ph as usize];
    let n = mr_app_pixels_into(app, buf.as_mut_ptr(), buf.len(), stride as u32, MR_PIXEL_BGRA8);
    assert_eq!(n, buf.len());
    let y = 2400 / 2; // 屏幕中部（第 3 块以内）不是白的：画面确实出来了
    let px = &buf[y * stride + 100 * 4..y * stride + 100 * 4 + 4];
    assert_ne!(px, &[255, 255, 255, 255], "中部是白的：没画出内容");

    // 转屏：改尺寸后同一块层照样上屏（合成器自己 reconfigure，Swift 只改层的 frame）
    assert_eq!(mr_app_resize(app, 844, 390, dpr), 1);
    mr_app_pixel_size(app, &mut pw, &mut ph);
    assert_eq!((pw, ph), (2532, 1170));
    pump_presented(app, &mut t);
    assert_eq!(mr_app_backend(app), MR_BACKEND_GPU, "{}", reason(app));

    // 进后台 / 销毁：先卸层，之后是 CPU，照常出 CPU 帧
    mr_app_detach_surface(app);
    t += 8;
    let f = mr_app_pump_ex(app, t);
    assert_eq!(f & MR_FRAME_PRESENTED, 0);
    assert_eq!(mr_app_backend(app), MR_BACKEND_CPU);
    // 重挂（回前台）：复用合成器
    assert_eq!(mr_app_attach_surface(app, MR_SURFACE_METAL_LAYER, layer), MR_BACKEND_GPU, "{}", reason(app));
    pump_presented(app, &mut t);
    mr_app_detach_surface(app);
    mr_app_destroy(app);
    unsafe { send(layer, c"release") };
}

#[test]
fn ios_视图_偏好_cpu_不碰层() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("cpu");
    let dir = CString::new(root.to_string_lossy().as_ref()).unwrap();
    let app = mr_app_create_ex(dir.as_ptr(), std::ptr::null(), 200, 300, 2.0, std::ptr::null());
    assert!(!app.is_null());
    assert_eq!(mr_app_set_backend(app, MR_BACKEND_CPU), 1);
    assert_eq!(mr_app_prewarm_gpu(app), 0, "偏好 cpu 不预热");
    assert_eq!(mr_app_launch(app, std::ptr::null()), 1);
    // 偏好 cpu 时根本不碰指针（给个假的也不会解引用）
    let fake = 0x10 as *mut c_void;
    assert_eq!(mr_app_attach_surface(app, MR_SURFACE_METAL_LAYER, fake), MR_BACKEND_CPU);
    let f = mr_app_pump_ex(app, 16);
    assert!(f & MR_FRAME_NEW != 0 && f & MR_FRAME_PRESENTED == 0, "{f:#x}");
    mr_app_detach_surface(app);
    mr_app_destroy(app);
}
