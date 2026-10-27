//! EGL 的加载与上下文创建（design-webgl.md §1、§3）。
//!
//! 不链接任何 GL 库：libEGL 运行时加载（khronos-egl 的 `dynamic`），GLES 函数一律从 `eglGetProcAddress` 取。
//! 后端按顺序试，进程里每种后端只初始化一次（display 是进程级的，`eglTerminate` 不调）：
//!
//! | 后端 | 库 | 用在 |
//! |---|---|---|
//! | `angle` | ANGLE（macOS：Metal；Android：系统或包里带的 `libEGL_angle.so`） | 交互运行默认 |
//! | `swiftshader` | ANGLE + SwiftShader（Vulkan 软件实现） | 无头 / 固定时钟（逐字节基线）；硬件后端建不了时兜底 |
//! | `system` | 系统 libEGL（Android 的驱动） | ANGLE 加载不到时 |
//!
//! `MINI_WEBGL=auto|angle|swiftshader|system|off` 可强制（与创建选项 `webgl` 同义，环境变量优先）。

use khronos_egl as egl;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

pub type Egl = egl::DynamicInstance<egl::EGL1_4>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    Angle,
    SwiftShader,
    System,
}

impl Backend {
    pub fn name(self) -> &'static str {
        match self {
            Backend::Angle => "angle",
            Backend::SwiftShader => "swiftshader",
            Backend::System => "system",
        }
    }
}

/// 一个初始化好的 display（进程级缓存，'static）
pub struct Display {
    pub egl: Egl,
    pub dpy: egl::Display,
    pub backend: Backend,
    /// display 扩展串里有没有 WebGL 兼容上下文（有 = ANGLE 做校验）
    pub webgl_compat: bool,
    pub exts: String,
    /// 从哪个库加载的
    pub lib: String,
    /// 初始化时的说明（第一次建上下文时打进游戏日志）
    pub note: String,
}

// SAFETY: EGL display 句柄是进程级的、线程无关；Egl 只是一张函数表
unsafe impl Send for Display {}
unsafe impl Sync for Display {}

impl Display {
    pub fn has_ext(&self, name: &str) -> bool {
        self.exts.split(' ').any(|e| e == name)
    }
}

/// 解析偏好：`auto` 返回按顺序要试的后端
pub fn order(pref: &str, headless: bool) -> Vec<Backend> {
    match pref {
        "off" | "none" => vec![],
        "angle" => vec![Backend::Angle],
        "swiftshader" => vec![Backend::SwiftShader],
        "system" => vec![Backend::System],
        // 无头：SwiftShader 是确定性的（逐字节基线），硬件只在交互运行时用
        // Android 没有随包的 SwiftShader：ANGLE（包里带了才有）→ 系统驱动
        _ if cfg!(target_os = "android") => vec![Backend::Angle, Backend::System],
        _ if headless => vec![Backend::SwiftShader, Backend::Angle, Backend::System],
        _ => vec![Backend::Angle, Backend::SwiftShader, Backend::System],
    }
}

static DISPLAYS: OnceLock<Mutex<HashMap<Backend, Option<&'static Display>>>> = OnceLock::new();

/// 拿某个后端的 display（第一次初始化，失败也缓存：同一进程里不反复试）。返回 (display, 失败原因)
pub fn display(b: Backend) -> Result<&'static Display, String> {
    let map = DISPLAYS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut m = map.lock().unwrap();
    if let Some(d) = m.get(&b) {
        return d.ok_or_else(|| format!("{} 之前初始化失败", b.name()));
    }
    let r = init(b).map(|d| &*Box::leak(Box::new(d)));
    m.insert(b, r.as_ref().ok().copied());
    r
}

/// 候选库（按顺序）
fn candidates(b: Backend) -> Vec<String> {
    let mut v = Vec::new();
    let angle_dirs = || -> Vec<String> {
        let mut d = Vec::new();
        if let Ok(x) = std::env::var("MINI_ANGLE_DIR") {
            d.push(x);
        }
        if let Some(exe) = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())) {
            d.push(exe.display().to_string());
            d.push(exe.join("../Frameworks").display().to_string());
        }
        // 开发机：仓库里 tools/fetch-angle.sh 放的那份
        d.push(format!("{}/third_party/angle/{}", env!("CARGO_MANIFEST_DIR"), plat_dir()));
        d
    };
    match b {
        Backend::Angle | Backend::SwiftShader => {
            if cfg!(target_os = "android") {
                // 包里带的优先（jniLibs），再试系统的（Android 10+ 系统镜像里常有，能不能加载看链接器命名空间）
                v.push("libEGL_angle.so".into());
                v.push("/system/lib64/libEGL_angle.so".into());
            } else {
                for d in angle_dirs() {
                    v.push(format!("{d}/{}", if cfg!(target_os = "macos") { "libEGL.dylib" } else if cfg!(windows) { "libEGL.dll" } else { "libEGL.so" }));
                }
            }
        }
        Backend::System => {
            if cfg!(target_os = "android") {
                v.push("libEGL.so".into());
            } else if cfg!(target_os = "linux") {
                v.push("libEGL.so.1".into());
            }
        }
    }
    v
}

fn plat_dir() -> &'static str {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) { "macos-arm64" }
    else if cfg!(target_os = "macos") { "macos-x64" }
    else if cfg!(all(target_os = "linux", target_arch = "aarch64")) { "linux-arm64" }
    else { "linux-x64" }
}

// ANGLE 的平台常量（include/EGL/eglext_angle.h）
const PLATFORM_ANGLE: u32 = 0x3202;
const PLATFORM_ANGLE_TYPE: i32 = 0x3203;
const PLATFORM_ANGLE_DEVICE_TYPE: i32 = 0x3209;
const TYPE_METAL: i32 = 0x3489;
const TYPE_VULKAN: i32 = 0x3450;
const DEVICE_SWIFTSHADER: i32 = 0x3487;

type GetPlatformDisplayExt = unsafe extern "C" fn(u32, *mut c_void, *const i32) -> *mut c_void;

fn init(b: Backend) -> Result<Display, String> {
    let mut errs = Vec::new();
    for lib in candidates(b) {
        if lib.starts_with('/') && !std::path::Path::new(&lib).exists() {
            continue;
        }
        let egl = match unsafe { Egl::load_required_from_filename(&lib) } {
            Ok(e) => e,
            Err(e) => {
                errs.push(format!("{lib}: {e}"));
                continue;
            }
        };
        let dpy = match b {
            Backend::System => unsafe { egl.get_display(egl::DEFAULT_DISPLAY) },
            _ => platform_display(&egl, b),
        };
        let Some(dpy) = dpy else {
            errs.push(format!("{lib}: 拿不到 display"));
            continue;
        };
        if let Err(e) = egl.initialize(dpy) {
            errs.push(format!("{lib}: eglInitialize {e}"));
            continue;
        }
        let exts = egl.query_string(Some(dpy), egl::EXTENSIONS).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let vendor = egl.query_string(Some(dpy), egl::VENDOR).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        // 系统 EGL 本身也可能就是 ANGLE（Android 开发者选项把 ANGLE 设成驱动时）：看扩展，不看库名
        let webgl_compat = exts.split(' ').any(|e| e == "EGL_ANGLE_create_context_webgl_compatibility");
        let note = format!("🧊 WebGL 后端 {}：{lib}（{vendor}，WebGL 兼容上下文{}）", b.name(), if webgl_compat { "有" } else { "没有，引擎自己校验" });
        return Ok(Display { egl, dpy, backend: b, webgl_compat, exts, lib, note });
    }
    Err(if errs.is_empty() { format!("{}：没有候选库", b.name()) } else { errs.join("；") })
}

fn platform_display(egl: &Egl, b: Backend) -> Option<egl::Display> {
    let f = egl.get_proc_address("eglGetPlatformDisplayEXT")?;
    // SAFETY: 签名照 EGL_EXT_platform_base
    let f: GetPlatformDisplayExt = unsafe { std::mem::transmute(f) };
    let mut attrs = vec![PLATFORM_ANGLE_TYPE];
    match b {
        Backend::SwiftShader => attrs.extend([TYPE_VULKAN, PLATFORM_ANGLE_DEVICE_TYPE, DEVICE_SWIFTSHADER]),
        _ if cfg!(target_os = "macos") || cfg!(target_os = "ios") => attrs.push(TYPE_METAL),
        // Android 等：ANGLE 自己挑（Vulkan 或系统 GLES）
        _ => attrs.push(0x3206),
    }
    attrs.push(egl::NONE);
    let p = unsafe { f(PLATFORM_ANGLE, std::ptr::null_mut(), attrs.as_ptr()) };
    if p.is_null() {
        return None;
    }
    Some(unsafe { egl::Display::from_ptr(p) })
}

/// 要的上下文属性（WebGL `getContext` 的第二个参数）
#[derive(Clone, Copy, Debug)]
pub struct Want {
    pub alpha: bool,
    pub depth: bool,
    pub stencil: bool,
    pub antialias: bool,
    pub es3: bool,
}

/// 实际拿到的
#[derive(Clone, Copy, Debug, Default)]
pub struct Got {
    pub alpha: bool,
    pub depth: bool,
    pub stencil: bool,
    pub samples: i32,
}

/// 选 config：枚举全部，按「alpha 一致 > depth/stencil 至少满足且不多给 > 抗锯齿」打分（eglChooseConfig 的排序
/// 把颜色位多的排前面，alpha:false 时会先给带 alpha 的，所以自己挑）
pub fn choose_config(d: &Display, w: &Want) -> Option<(egl::Config, Got)> {
    let n = d.egl.get_config_count(d.dpy).ok()?;
    let mut all = Vec::with_capacity(n);
    d.egl.get_configs(d.dpy, &mut all).ok()?;
    let renderable = if w.es3 { egl::OPENGL_ES3_BIT } else { egl::OPENGL_ES2_BIT };
    let q = |c: egl::Config, a: i32| d.egl.get_config_attrib(d.dpy, c, a).unwrap_or(0);
    let mut best: Option<(i64, egl::Config, Got)> = None;
    for c in all {
        if q(c, egl::RENDERABLE_TYPE) & renderable == 0 || q(c, egl::SURFACE_TYPE) & egl::PBUFFER_BIT == 0 {
            continue;
        }
        if (q(c, egl::RED_SIZE), q(c, egl::GREEN_SIZE), q(c, egl::BLUE_SIZE)) != (8, 8, 8) {
            continue;
        }
        let (a, dp, st, s) = (q(c, egl::ALPHA_SIZE), q(c, egl::DEPTH_SIZE), q(c, egl::STENCIL_SIZE), q(c, egl::SAMPLES));
        if a != 0 && a != 8 {
            continue;
        }
        let mut score = 0i64;
        if (a == 8) == w.alpha { score += 1000; }
        if w.depth { if dp >= 16 { score += 400 + if dp >= 24 { 50 } else { 0 }; } } else if dp == 0 { score += 100; }
        if w.stencil { if st >= 8 { score += 400; } } else if st == 0 { score += 100; }
        if w.antialias { if s >= 4 { score += 200 - (s - 4) as i64; } } else if s == 0 { score += 200; }
        // 也能建窗口 surface 的优先（Android 直接上屏要用同一个 config，window.rs）
        if q(c, egl::SURFACE_TYPE) & egl::WINDOW_BIT != 0 { score += 20; }
        if best.as_ref().map(|b| score > b.0).unwrap_or(true) {
            best = Some((score, c, Got { alpha: a == 8, depth: dp > 0, stencil: st > 0, samples: s }));
        }
    }
    best.map(|(_, c, g)| (c, g))
}

/// 建上下文。WebGL 兼容 / 关扩展 / 资源零初始化 / 不自动生成名字 / 不许客户端数组 / robust access 能开就开，
/// 驱动不认就逐个去掉再试
pub fn create_context(d: &Display, cfg: egl::Config, es3: bool, webgl: bool) -> Result<egl::Context, String> {
    d.egl.bind_api(egl::OPENGL_ES_API).map_err(|e| e.to_string())?;
    let mut opt: Vec<(i32, i32)> = Vec::new();
    if d.webgl_compat && webgl {
        opt.push((0x33AC, 1));
        if d.has_ext("EGL_ANGLE_create_context_extensions_enabled") { opt.push((0x345F, 0)); }
        if d.has_ext("EGL_ANGLE_robust_resource_initialization") { opt.push((0x3453, 1)); }
        if d.has_ext("EGL_CHROMIUM_create_context_bind_generates_resource") { opt.push((0x33AD, 0)); }
        if d.has_ext("EGL_ANGLE_create_context_client_arrays") { opt.push((0x3452, 0)); }
    }
    // robust access 只给系统驱动：ANGLE 开了它就跳过 WebGL 的缓冲越界检查（实测越界 drawArrays 不再报 INVALID_OPERATION），
    // 而 WebGL 兼容上下文本来就保证不会越界读
    if !d.webgl_compat && d.has_ext("EGL_EXT_create_context_robustness") {
        opt.push((0x30BF, 1)); // ROBUST_ACCESS
        opt.push((0x3138, 0x31BF)); // RESET_NOTIFICATION_STRATEGY = LOSE_CONTEXT_ON_RESET
    }
    loop {
        let mut attrs = vec![0x3098, if es3 { 3 } else { 2 }];
        for (k, v) in &opt {
            attrs.extend([*k, *v]);
        }
        attrs.push(egl::NONE);
        match d.egl.create_context(d.dpy, cfg, None, &attrs) {
            Ok(c) => return Ok(c),
            Err(e) => {
                // 先去掉 robustness（最常见的不认），再去掉其余可选项
                if opt.pop().is_none() {
                    return Err(format!("eglCreateContext: {e}"));
                }
            }
        }
    }
}

pub fn create_pbuffer(d: &Display, cfg: egl::Config, w: u32, h: u32) -> Result<egl::Surface, String> {
    d.egl.create_pbuffer_surface(d.dpy, cfg, &[egl::WIDTH, w.max(1) as i32, egl::HEIGHT, h.max(1) as i32, egl::NONE]).map_err(|e| format!("eglCreatePbufferSurface: {e}"))
}

pub fn proc_address(d: &Display, name: &str) -> *const c_void {
    d.egl.get_proc_address(name).map_or(std::ptr::null(), |p| p as *const c_void)
}
