//! SDK 的 GPU 上屏（`host/engine_gpu.rs`）端到端：macOS 上现建一个 `CAMetalLayer` 当宿主
//! surface（与 Android 的 `ANativeWindow` 走同一段引擎代码，只差 surface 种类），跑
//! 挂载 → 出帧（PRESENTED）→ 取像素 → 宿主改偏好 → 故障注入回退 → 重挂 的全过程。
//!
//! 没有可用适配器（CI 虚拟机）时打印「GPU 未测」并跳过。GPU 输出与 CPU 逐字节一致的
//! 决定性判据仍是启动自检与 `tools/gpu-parity.sh`；这里守的是 SDK 侧的**状态与标志位**：
//! 什么时候带 PRESENTED / BACKEND_CHANGED、回退是不是粘性的、GPU 帧之后取像素是不是新的。
//!
//! 独立进程 + 一把锁串行：引擎的小程序根目录是进程级的（见 tests/embed_host.rs 的说明）。
#![cfg(all(target_os = "macos", feature = "gpu"))]

use mini_render::compositor::{BackendKind, BackendPref, EnvValue, FaultSpec, HostEnv, SurfaceKind};
use mini_render::host::engine_backend::{FRAME_BACKEND_CHANGED, FRAME_NEW, FRAME_PENDING, FRAME_PRESENTED};
use mini_render::host::{EngineOptions, MiniEngine};
use std::ffi::{c_char, c_void};
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

// ── 不引第三方 crate：直接用 objc 运行时建 / 放 CAMetalLayer ──
#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}
#[link(name = "QuartzCore", kind = "framework")]
extern "C" {}

/// `[obj sel]`，返回 id。arm64 上 objc_msgSend 必须按真实签名调用，不能当可变参数函数用
unsafe fn send(obj: *mut c_void, sel: &[u8]) -> *mut c_void {
    let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    f(obj, sel_registerName(sel.as_ptr().cast()))
}

/// 宿主那一侧的 surface：一个不进窗口的 CAMetalLayer（`+new`，+1 引用）
struct Layer(*mut c_void);

impl Layer {
    fn new() -> Self {
        // SAFETY: 类名与选择子都是以 NUL 结尾的常量；+new 返回已持有的对象
        let p = unsafe { send(objc_getClass(c"CAMetalLayer".as_ptr()), b"new\0") };
        assert!(!p.is_null(), "建不出 CAMetalLayer");
        Layer(p)
    }
}

impl Drop for Layer {
    fn drop(&mut self) {
        // SAFETY: 引擎已先 detach（合成器自己的那份引用已还），这里还 +new 的那一份
        unsafe { send(self.0, b"release\0") };
    }
}

/// 一页静态内容：上下几块不同颜色，滚动后画面一定变
fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("embed-gpu").join(name);
    std::fs::remove_dir_all(&root).ok();
    let blocks: String = ["#07c160", "#1677ff", "#fa5151", "#ffc300", "#6467f0", "#10aeff"]
        .iter()
        .map(|c| format!(r#"<view style="height:160px;background:{c}">{c}</view>"#))
        .collect();
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"],"window":{"navigationBarTitleText":"GPU"}}"#.to_string()),
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

fn engine(name: &str, env: HostEnv) -> MiniEngine {
    let root = fixture(name);
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(
        &root.to_string_lossy(),
        Some(&data.to_string_lossy()),
        200,
        320,
        2.0,
        env,
        EngineOptions::default(),
    )
    .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e
}

/// 挂上并要求是 GPU；适配器不可用时返回 false（调用方跳过）
fn attach_gpu(e: &mut MiniEngine, layer: &Layer) -> bool {
    e.set_backend_pref(BackendPref::Gpu);
    if e.attach_native_surface(SurfaceKind::MetalLayer, layer.0) != BackendKind::Gpu {
        eprintln!("⚠ GPU 未测：{}", e.backend().reason);
        return false;
    }
    true
}

/// 跑到出现 PRESENTED 为止（drawable 一时取不到会是 PENDING，下一次补），返回那一帧的标志位
fn pump_presented(e: &mut MiniEngine, t: &mut u64) -> u32 {
    for _ in 0..20 {
        *t += 16;
        let f = e.pump_ex(*t);
        if f & FRAME_PRESENTED != 0 {
            return f;
        }
        assert!(f & FRAME_PENDING != 0, "GPU 在用时一帧要么 PRESENTED 要么 PENDING，实际 {f:#x}");
    }
    panic!("20 帧都没上屏");
}

#[test]
fn 上屏_取像素_宿主改偏好_重挂() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut e = engine("present", HostEnv::default());
    let layer = Layer::new();
    if !attach_gpu(&mut e, &layer) {
        return;
    }
    let mut t = 0u64;
    // ① 首帧画到 surface 上；宿主主动挂载不带 BACKEND_CHANGED（R5）
    let f = pump_presented(&mut e, &mut t);
    assert!(f & FRAME_NEW != 0 && f & FRAME_BACKEND_CHANGED == 0, "首帧 {f:#x}");
    let top = e.rgba().to_vec();

    // ② GPU 帧不做 CPU 合成：滚动之后取像素必须是新的那一帧，不是旧缓冲
    e.scroll_to(200.0);
    pump_presented(&mut e, &mut t);
    let scrolled_gpu = e.rgba().to_vec();
    assert_ne!(top, scrolled_gpu, "滚动后像素没变：取到的是 GPU 帧之前的旧合成缓冲");

    // ③ 卸掉 surface，同一位置走 CPU：与刚才 GPU 帧补出来的像素逐字节相同
    e.detach_surface();
    t += 16;
    let f = e.pump_ex(t);
    assert!(f & FRAME_NEW != 0 && f & (FRAME_PRESENTED | FRAME_BACKEND_CHANGED) == 0, "卸载后 {f:#x}");
    assert_eq!(e.backend().active, BackendKind::Cpu);
    assert!(e.rgba() == scrolled_gpu.as_slice(), "同一位置 CPU 帧与 GPU 帧补出的像素不同");

    // ④ 重挂照常再试（没失败过）
    assert!(attach_gpu(&mut e, &layer), "重挂应仍是 GPU：{}", e.backend().reason);
    let f = pump_presented(&mut e, &mut t);
    assert_eq!(f & FRAME_BACKEND_CHANGED, 0, "重挂是宿主主动的，不带 BACKEND_CHANGED");

    // ⑤ 挂着 surface 时宿主改偏好：cpu → 放掉合成器、带 BACKEND_CHANGED；再改回 gpu → 重建
    e.set_backend_pref(BackendPref::Cpu);
    assert_eq!(e.backend().active, BackendKind::Cpu);
    t += 16;
    let f = e.pump_ex(t);
    assert!(f & FRAME_BACKEND_CHANGED != 0 && f & FRAME_NEW != 0 && f & FRAME_PRESENTED == 0, "改 cpu 后 {f:#x}");
    e.set_backend_pref(BackendPref::Gpu);
    assert_eq!(e.backend().active, BackendKind::Gpu, "{}", e.backend().reason);
    let f = pump_presented(&mut e, &mut t);
    assert!(f & FRAME_BACKEND_CHANGED != 0, "改回 gpu 是引擎重新选出来的，要带 BACKEND_CHANGED：{f:#x}");

    // ⑥ 宿主换机型信息不该把正在用的 GPU 冲掉（从前 reselect 会写成「GPU 还没接入」的 CPU）
    e.set_device_info(mini_render::compositor::DeviceInfo { total_mem_bytes: 8 << 30, low_ram: false, os_api: 0 });
    assert_eq!(e.backend().active, BackendKind::Gpu, "{}", e.backend().reason);
    e.detach_surface();
}

#[test]
fn 运行中设备丢失_当帧回退且粘性() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // 第 2 个真实 GPU 帧注入设备丢失（自检帧不计数）
    let env = HostEnv { gpu_fault: EnvValue::Valid(FaultSpec::Lost(2)), ..HostEnv::default() };
    let mut e = engine("lost", env);
    let layer = Layer::new();
    if !attach_gpu(&mut e, &layer) {
        return;
    }
    let mut t = 0u64;
    pump_presented(&mut e, &mut t);
    e.scroll_to(120.0);
    t += 16;
    let f = e.pump_ex(t);
    // 当帧改走 CPU：这一帧照样出来，宿主据 BACKEND_CHANGED 切回像素出口
    assert!(f & FRAME_BACKEND_CHANGED != 0 && f & FRAME_NEW != 0 && f & FRAME_PRESENTED == 0, "丢失那帧 {f:#x}");
    assert_eq!(e.backend().active, BackendKind::Cpu);
    assert!(e.backend().reason.starts_with("回退："), "{}", e.backend().reason);
    let (w, h) = e.pixel_size();
    assert_eq!(e.rgba().len(), (w * h * 4) as usize);
    // 粘性：宿主卸了再挂也不再试 GPU
    e.detach_surface();
    assert_eq!(e.attach_native_surface(SurfaceKind::MetalLayer, layer.0), BackendKind::Cpu);
    assert!(e.backend().reason.contains("已失败过"), "{}", e.backend().reason);
    e.detach_surface();
}

#[test]
fn 默认偏好_cpu_不碰_surface() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut e = engine("default-cpu", HostEnv::default());
    // 一个永远不能被解引用的地址：偏好是 cpu 时引擎连看都不该看它
    let mut slot = 0u64;
    let fake = (&mut slot as *mut u64).cast::<c_void>();
    assert_eq!(e.attach_native_surface(SurfaceKind::MetalLayer, fake), BackendKind::Cpu);
    assert!(e.backend().reason.contains("偏好 cpu"), "{}", e.backend().reason);
    let f = e.pump_ex(16);
    assert!(f & FRAME_NEW != 0 && f & (FRAME_PRESENTED | FRAME_PENDING) == 0, "{f:#x}");
    e.detach_surface();
}

/// 回桌面再回来（Android 上 Surface 被系统收走、回来给一块新的）：
/// 合成器只换 surface、不重开设备不跑自检，页面也不整页重画 —— 一加 PLY110 上从前这一段是 520ms 空白
#[test]
fn 卸了换新_surface_复用合成器且不整页重画() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut e = engine("reattach", HostEnv::default());
    let first = Layer::new();
    if !attach_gpu(&mut e, &first) {
        return;
    }
    let mut t = 0u64;
    pump_presented(&mut e, &mut t);
    e.scroll_to(150.0);
    pump_presented(&mut e, &mut t);
    let before = e.rgba().to_vec();

    // 卸下：CPU 出口只重新合成，不整页重画；像素与 GPU 那一帧相同
    e.detach_surface();
    drop(first);
    t += 16;
    let f = e.pump_ex(t);
    assert!(f & FRAME_NEW != 0, "卸载后要出一帧 CPU 画面：{f:#x}");
    assert_eq!(e.last_frame_kind().label(), "只合成", "卸 surface 不该整页重画");
    assert!(e.rgba() == before.as_slice(), "卸载后 CPU 合成的像素与 GPU 那一帧不同");

    // 换一块新的 surface 挂回来：仍是 GPU，第一帧只合成（镜像整张重传，页面不重画）
    let second = Layer::new();
    assert!(attach_gpu(&mut e, &second), "换新 surface 应仍是 GPU：{}", e.backend().reason);
    let f = pump_presented(&mut e, &mut t);
    assert_eq!(f & FRAME_BACKEND_CHANGED, 0, "{f:#x}");
    assert_eq!(e.last_frame_kind().label(), "只合成", "复用合成器后第一帧不该整页重画");
    assert!(e.rgba() == before.as_slice(), "复用合成器后画面变了");
    e.detach_surface();
}
