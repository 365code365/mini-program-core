//! 合成层：把一帧的各个图层按固定顺序合成到输出缓冲。
//!
//! 内容（布局、命中、事件、`setData` 增量失效、动画时钟、光栅化）都在别处，
//! 而且与后端无关；这里只拿到**本帧各图层的只读引用**（[`FrameLayers`]），
//! 拿不到 `MiniApp` / `InteractionManager` / `ScrollController`。所以「合成后端
//! 不碰布局、命中、事件」是类型层面保证的，不靠约定（设计见
//! `.agents/tasks/gpu-dual-backend-2026-10-12/design.md` §2）。
//!
//! 现在只有 CPU 合成器 [`cpu::compose`]：它是参考实现，永远可用。
//! 图层画布（[`PageCanvas`] / [`LayerCanvas`]）记录「哪块被写过」，给以后按脏区上传的
//! 合成后端用；CPU 合成器不读这些记账。
//!
//! ## 为什么字段与 `present_to_buffer` 的参数一一对应
//! 合成序列原来在四个宿主调用点各抄了一份（桌面上屏、无头快照、侧滑留图、SDK）。
//! 把它们收拢成「构造 `FrameLayers` → 调 `cpu::compose`」时，字段只是把原来的实参
//! 原样装进来，宿主怎么算（截断方式、f32/f64、相位时钟）都不变，逐字节可证。
//!
//! ## 后端选择（纯函数，与 GPU 无关）
//! - [`pref`]：偏好从哪来（`--backend` / `MINI_BACKEND` / FFI 整数）、谁说了算；
//!   宿主入口一次性读进程环境（[`HostEnv`]），之后只传值；
//! - [`select`]：机型分级与适配器门槛，只吃数据结构，单测不需要 GPU；
//! - [`verdict`]：「这个驱动上次出过问题」的判定缓存；
//! - [`fault`]：`MINI_GPU_FAULT` 的语法；
//! - [`diag`]：被脚本 grep 的日志行，全部由这里的格式化函数产出。
//!
//! 现在还没有 GPU 合成器，偏好不是 `cpu` 时一律落到 CPU，并说清原因
//! （没编进 `gpu` feature 时是「未编译 GPU 支持」）。
pub mod cpu;
pub mod diag;
pub mod fault;
pub mod footprint;
pub mod layer;
pub mod pref;
pub mod select;
pub(crate) mod sheet_base;
pub mod verdict;
#[cfg(feature = "gpu")]
pub mod gpu;

pub use fault::FaultSpec;
pub use footprint::Footprint;
pub use layer::{color_bytes, DirtyOp, LayerCanvas, PageCanvas, PAGE_LOG_CAP};
pub use pref::{EnvValue, HostEnv, HostKind, PrefSource};
pub use select::{AdapterFacts, DeviceInfo, Platform, GPU_COMPILED};

/// 后端偏好：宿主 / 命令行 / 环境变量给的「想用什么」。实际用了什么见 [`BackendKind`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackendPref {
    /// 由机型分级、适配器门槛与启动自检决定
    Auto,
    Cpu,
    /// 强制 GPU：放行机型分级里的软规则，但软件适配器、能力不足、自检不一致仍会落到 CPU
    Gpu,
}

impl BackendPref {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendPref::Auto => "auto",
            BackendPref::Cpu => "cpu",
            BackendPref::Gpu => "gpu",
        }
    }

    /// C ABI 里的整数（`MR_BACKEND_AUTO/CPU/GPU`）
    pub fn code(self) -> i32 {
        match self {
            BackendPref::Auto => 0,
            BackendPref::Cpu => 1,
            BackendPref::Gpu => 2,
        }
    }
}

/// 实际在用的后端
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackendKind {
    Cpu,
    Gpu,
}

impl BackendKind {
    pub fn label(self) -> &'static str {
        match self {
            BackendKind::Cpu => "CPU",
            BackendKind::Gpu => "GPU",
        }
    }

    /// C ABI 里的整数：1 = CPU、2 = GPU（与 `MR_BACKEND_CPU/GPU` 同值）
    pub fn code(self) -> i32 {
        match self {
            BackendKind::Cpu => 1,
            BackendKind::Gpu => 2,
        }
    }
}

/// 一次后端选择的结论：偏好（及其来源）、实际后端、为什么、适配器描述。
///
/// `reason` 是给人看的一句话，也是 SDK `mr_app_backend_reason` 的返回值。日志行与
/// `MINI_FPS` 标签都由 [`diag`] 从这几个字段拼出来，所以这里的写法约定是：
/// - GPU：`reason` 为「自动选择」/「强制指定」，`adapter` 为 `Metal · Apple M3 Pro` 这种；
/// - CPU 且偏好就是 cpu：`reason` 为「偏好 cpu（来源）」，标签只写 `[后端 CPU]`；
/// - CPU 但偏好不是 cpu：`reason` 就是挡住 GPU 的那条规则，标签带上它；
///   回退时以「回退：」开头（见 [`BackendStatus::fell_back`]）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendStatus {
    pub pref: BackendPref,
    pub source: PrefSource,
    pub active: BackendKind,
    pub reason: String,
    pub adapter: Option<String>,
}

impl BackendStatus {
    /// 结论是 CPU
    pub fn cpu(pref: BackendPref, source: PrefSource, reason: impl Into<String>) -> Self {
        Self { pref, source, active: BackendKind::Cpu, reason: reason.into(), adapter: None }
    }

    /// 偏好就是 cpu：原因写明偏好来自哪里，排查「为什么不是 GPU」时一眼能看出该改哪
    pub fn preferred_cpu(source: PrefSource) -> Self {
        let reason = format!("偏好 cpu（{}）", source.label());
        Self::cpu(BackendPref::Cpu, source, reason)
    }

    /// 结论是 GPU。`adapter` 形如 `Metal · Apple M3 Pro`
    pub fn gpu(pref: BackendPref, source: PrefSource, adapter: impl Into<String>) -> Self {
        let reason = if pref == BackendPref::Gpu { "强制指定" } else { "自动选择" };
        Self {
            pref,
            source,
            active: BackendKind::Gpu,
            reason: reason.to_string(),
            adapter: Some(adapter.into()),
        }
    }

    /// GPU 选上之后又失败了：回到 CPU，原因以「回退：」开头（标签据此显示回退原因）
    pub fn fell_back(&self, why: &str) -> Self {
        Self::cpu(self.pref, self.source, format!("{}{why}", diag::FALLBACK_REASON_HEAD))
    }
}

/// 宿主挂上来的原生 surface 是哪一种（C ABI 的 `MR_SURFACE_*`）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceKind {
    /// `CAMetalLayer*`（iOS / macOS）
    MetalLayer,
    /// `ANativeWindow*`（Android）
    AndroidWindow,
}

impl SurfaceKind {
    /// `MR_SURFACE_METAL_LAYER = 1`、`MR_SURFACE_ANDROID_WINDOW = 2`；其它值 `None`
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            1 => Some(SurfaceKind::MetalLayer),
            2 => Some(SurfaceKind::AndroidWindow),
            _ => None,
        }
    }

    pub fn code(self) -> i32 {
        match self {
            SurfaceKind::MetalLayer => 1,
            SurfaceKind::AndroidWindow => 2,
        }
    }

    /// 当前编译目标认不认这种 surface：Apple 目标只认 Metal 层，Android 只认
    /// `ANativeWindow`。其它目标（含鸿蒙）一种都不认 —— 宿主传上来就当参数错误，
    /// 而不是悄悄接下一个永远用不上的指针。
    pub fn supported_on_target(self) -> bool {
        match self {
            SurfaceKind::MetalLayer => cfg!(target_vendor = "apple"),
            SurfaceKind::AndroidWindow => cfg!(target_os = "android"),
        }
    }
}

use crate::host::edge_back::PageShot;
use crate::host::picker_sheet::PickerSheetState;
use crate::host::ui_overlay::{LoadingState, ModalState, ToastState};
use crate::text::TextRenderer;

/// 本帧要合成的全部图层（只读引用 + 覆盖层状态）
pub struct FrameLayers<'a> {
    /// 输出缓冲的像素尺寸（窗口 / 引擎像素尺寸）
    pub width: u32,
    pub height: u32,
    /// 设备像素比（覆盖层、下拉指示器按它缩放）
    pub scale: f32,
    /// 页面画布（内容坐标，整页高）
    pub page: &'a PageCanvas,
    /// 页面滚动偏移（设备像素）。由宿主按 `(scroll * scale) as i32` 截断，这里不再取整
    pub scroll_px: i32,
    /// 画布之外区域的填充色（0xRRGGBB，取 `page { background-color }`）
    pub page_bg: u32,
    /// 当前页是否是 tabBar 页。与 `tabbar` 分开：`present_to_buffer` 在 tabBar 页上
    /// 即使画布缺失也会扣掉 `tabbar_px` 的高度
    pub has_tabbar: bool,
    pub tabbar: Option<&'a LayerCanvas>,
    /// tabBar 占用的设备像素高度（非 tabBar 页为 0）
    pub tabbar_px: u32,
    /// fixed 覆盖层画布（与视口同大）
    pub fixed: Option<&'a LayerCanvas>,
    /// fixed 覆盖层真正画过的行区间（含端点）；`None` 表示整层透明、跳过
    pub fixed_rows: Option<(u32, u32)>,
    /// 下拉刷新指示器。宿主只在「顶部露出空白 > 0.5 或正在刷新」时给 `Some`
    pub pull: Option<PullState>,
    /// Toast / Loading / Modal 与 picker 面板
    pub overlays: OverlayRefs<'a>,
    /// 左边缘侧滑返回 / 页面转场进行中（整帧平移、露出另一页）
    pub edge_back: Option<EdgeBackFrame<'a>>,
    /// 顶部宿主外壳（引擎画的导航栏，`navigationBar` 选项）。`Some` 时页面与 fixed 层整体
    /// 下移 `rows` 行，这几行由导航栏画布盖住；`None`（桌面窗体、custom 导航栏的页面）
    /// 与从前逐字节相同
    pub top: Option<TopBand<'a>>,
    /// 键盘顶起画面（`adjust-position`）：导航栏以下的页面、tabBar、fixed 层整体上移这么多行（设备像素），
    /// 导航栏不动 —— 与微信一致（页面被顶上去，原生导航栏留在原处）。底下露出来的那几行在键盘后面，填页面底色。
    /// 0 时与从前逐字节相同（host/engine_keyboard.rs）
    pub content_shift: u32,
}

/// 顶部宿主外壳那一条（导航栏 + 状态栏底色）
pub struct TopBand<'a> {
    /// 占用的设备像素行数（`导航栏逻辑高 × scale` 取整）
    pub rows: u32,
    pub canvas: &'a LayerCanvas,
}

/// 下拉刷新指示器的参数（与 `render_pull_indicator` 的参数一一对应）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PullState {
    /// 顶部露出的空白高度（逻辑像素）
    pub gap_logical: f32,
    pub refreshing: bool,
    /// 下拉进度 0~1
    pub progress: f32,
    /// 刷新期间三点呼吸的相位 0~1（快照固定成 0.25，交互窗体按进程时钟走）
    pub phase: f32,
}

/// 直接画进输出缓冲的覆盖层（Toast / Loading / Modal、picker 面板）
pub struct OverlayRefs<'a> {
    /// 提示框里转圈的时钟（秒，与 `render_ui_overlay` 的同名参数一致）：交互时是进程动画时钟，
    /// 快照 / 测试是定住的值。从前这里传的是「这一帧开始的时刻」，转角按它的 elapsed 算、恒为 0，
    /// 圈根本不转
    pub anim_secs: f32,
    pub toast: &'a Option<ToastState>,
    pub loading: &'a Option<LoadingState>,
    pub modal: &'a Option<ModalState>,
    pub picker: Option<&'a PickerSheetState>,
    pub text: Option<&'a TextRenderer>,
}

impl OverlayRefs<'static> {
    /// 没有任何覆盖层。侧滑留图要的是「页面 + tabBar + fixed」那一张，不带弹窗。
    pub fn none() -> Self {
        Self { anim_secs: 0.0, toast: &None, loading: &None, modal: &None, picker: None, text: None }
    }
}

/// 侧滑返回这一帧的参数（与 `edge_back::compose` 的参数一一对应）
pub struct EdgeBackFrame<'a> {
    /// 当前页右移的距离（设备像素）
    pub offset_px: u32,
    /// 被覆盖的上一页留下的视口图
    pub shot: Option<&'a PageShot>,
    /// 0~1 的推出进度
    pub progress: f32,
    /// 图在上面：缓冲里的是**下面**那一页（返回转场：上一页已经恢复、正在被盖着的那张是
    /// 刚离开的页面留下的图）。false 是侧滑与进入转场：缓冲里的是上面那一页
    pub shot_on_top: bool,
}
