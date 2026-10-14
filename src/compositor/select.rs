//! 机型分级与适配器门槛（设计 §6.2；v2 修订 #18、#19）。
//!
//! 全是纯函数，只吃数据结构（[`DeviceInfo`]、[`AdapterFacts`]），**不依赖 wgpu 类型**：
//! 单测不需要 GPU，也不会因为升级 wgpu 大版本而跟着改。wgpu 的适配器信息到
//! [`AdapterFacts`] 的转换放在以后的 `gpu/device.rs` 里。
//!
//! 两道门槛，先便宜后贵：
//! 1. [`platform_gate`]：不碰 GPU（有没有编进 `gpu` feature、Android 的 API 与内存）；
//! 2. [`adapter_gate`]：拿到适配器之后（软件适配器、类型、GLES、黑名单、纹理上限、
//!    下层能力、判定缓存）。
//!
//! 「强制 gpu」放行**软规则**（机型分级、GLES、适配器类型、黑名单、判定缓存），但
//! **硬规则**照样挡：没编进来、软件适配器（llvmpipe 一类不算 GPU）、能力不够 ——
//! 这些放过去也只会在建设备或第一帧时失败。
//!
//! iOS 没有内存规则（#18）：iOS 13+ 的设备都不会触发，交给启动自检决定。
use super::pref::PrefSource;
use super::{BackendPref, BackendStatus};

/// 编进 `gpu` feature 了没有。所有「未编译」的判断都看这一个常量。
pub const GPU_COMPILED: bool = cfg!(feature = "gpu");

/// 没编进 `gpu` feature（桌面 / SDK 的日志、`MINI_FPS` 标签、FFI 原因串都是这一句）
pub const REASON_NOT_COMPILED: &str = "未编译 GPU 支持";
/// SDK：偏好不是 cpu，但宿主还没挂 surface
pub const REASON_NO_SURFACE: &str = "未挂载 surface（宿主需调用 mr_app_attach_surface）";
/// 编进了 `gpu` feature、门槛也都过了，但 GPU 合成器还没实现（`gpu = []` 目前是空壳）
pub const REASON_GPU_PENDING: &str = "GPU 合成器还在开发中（feature gpu 目前不含实现）";

/// Android 的 Auto 门槛：Android 10（API 29）起要求 64 位设备支持 Vulkan 1.1，
/// 所以有了这一条就不必再单独查 Vulkan 版本（#19）
pub const ANDROID_MIN_API: u32 = 29;
/// 3.5e9 而不是 4GB：`ActivityManager.MemoryInfo.totalMem` 报的是内核可用内存，
/// 标称 4GB 的机器一般只报 3.6~3.8GB，按 4GB 卡会把它们全部挡掉
pub const ANDROID_MIN_MEM_BYTES: u64 = 3_500_000_000;

/// 跑在哪种平台上
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// macOS / Windows / Linux 桌面
    Desktop,
    Ios,
    Android,
    /// 没有 GPU 路径的目标（鸿蒙等，继续只走 CPU）
    Other,
}

impl Platform {
    /// 编译目标对应的平台
    pub fn current() -> Self {
        if cfg!(target_os = "ios") {
            Platform::Ios
        } else if cfg!(target_os = "android") {
            Platform::Android
        } else if cfg!(target_env = "ohos") {
            Platform::Other
        } else if cfg!(any(target_os = "macos", target_os = "windows", target_os = "linux")) {
            Platform::Desktop
        } else {
            Platform::Other
        }
    }
}

/// 宿主报上来的机型信息（`mr_app_set_device_info`）。0 表示未知。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeviceInfo {
    pub total_mem_bytes: u64,
    pub low_ram: bool,
    pub os_api: u32,
}

/// 适配器类型（对应 wgpu 的 `DeviceType`）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterType {
    IntegratedGpu,
    DiscreteGpu,
    VirtualGpu,
    /// 软件光栅（llvmpipe / SwiftShader / lavapipe）
    Cpu,
    Other,
}

/// 图形 API（对应 wgpu 的 `Backend`）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuApi {
    Metal,
    Vulkan,
    Dx12,
    Gl,
    Other,
}

impl GpuApi {
    pub fn label(self) -> &'static str {
        match self {
            GpuApi::Metal => "Metal",
            GpuApi::Vulkan => "Vulkan",
            GpuApi::Dx12 => "DX12",
            GpuApi::Gl => "GLES",
            GpuApi::Other => "其它",
        }
    }
}

/// 适配器的事实（纯数据，由 GPU 模块从 wgpu 的适配器信息转换而来）
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterFacts {
    pub api: GpuApi,
    pub device_type: AdapterType,
    pub vendor: u32,
    pub device: u32,
    pub name: String,
    pub driver: String,
    pub driver_info: String,
    pub max_texture_dimension_2d: u32,
    pub max_texture_array_layers: u32,
    /// 我们要用、而这块适配器缺的下层能力（wgpu `DownlevelFlags` 的名字）。
    /// 由转换方对照「要用的那几项」算好，这里只看空不空（#19）
    pub downlevel_missing: Vec<String>,
}

impl AdapterFacts {
    /// 日志与 `MINI_FPS` 标签里的适配器描述：`Metal · Apple M3 Pro`
    pub fn describe(&self) -> String {
        format!("{} · {}", self.api.label(), self.name)
    }
}

/// 本次合成对纹理的需求（页面环形条带 + 输出尺寸，设计 §4.3、§5.7）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuNeeds {
    pub max_texture_dimension_2d: u32,
    pub array_layers: u32,
}

/// 环形条带纹理每层的行数
pub const RING_SLICE_ROWS: u32 = 1024;

impl GpuNeeds {
    /// `out_w × out_h` 的输出、宽 `canvas_w` 的页面画布、视口高 `viewport_px` 行、
    /// 裁剪余量 `margin_px` 行（渲染器的 `VIEWPORT_CULL_MARGIN_PX`）
    pub fn for_output(out_w: u32, out_h: u32, canvas_w: u32, viewport_px: u32, margin_px: u32) -> Self {
        let rows = viewport_px + 2 * margin_px + 2;
        Self {
            max_texture_dimension_2d: out_w.max(out_h).max(canvas_w).max(RING_SLICE_ROWS),
            array_layers: rows.div_ceil(RING_SLICE_ROWS),
        }
    }
}

/// 编译期黑名单的一条：字段都给了才算命中（`None` 表示不看这一项）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenyRule {
    pub vendor: Option<u32>,
    pub device: Option<u32>,
    pub driver_info_contains: Option<&'static str>,
    /// 为什么拉黑（进原因串）
    pub note: &'static str,
}

impl DenyRule {
    fn matches(&self, f: &AdapterFacts) -> bool {
        self.vendor.is_none_or(|v| v == f.vendor)
            && self.device.is_none_or(|d| d == f.device)
            && self.driver_info_contains.is_none_or(|s| f.driver_info.contains(s))
    }
}

/// 黑名单初始为空：条目只在真机矩阵测出问题后添加，不预先编型号
pub const DENY: &[DenyRule] = &[];

/// 不碰 GPU 的门槛。`Err` 是挡住 GPU 的那条规则（原因串）。
pub fn platform_gate(
    pref: BackendPref,
    platform: Platform,
    info: &DeviceInfo,
    gpu_compiled: bool,
) -> Result<(), String> {
    if !gpu_compiled {
        return Err(REASON_NOT_COMPILED.into());
    }
    if platform == Platform::Other {
        return Err("此平台没有 GPU 路径".into());
    }
    // 机型分级只是「Auto 不冒险」；强制 gpu 时放行
    if pref != BackendPref::Auto || platform != Platform::Android {
        return Ok(());
    }
    if info.os_api == 0 {
        return Err("机型分级：Android API 未知".into());
    }
    if info.os_api < ANDROID_MIN_API {
        return Err(format!("机型分级：Android API {} < {ANDROID_MIN_API}", info.os_api));
    }
    if info.low_ram {
        return Err("机型分级：低内存设备".into());
    }
    if info.total_mem_bytes == 0 {
        return Err("机型分级：内存未知".into());
    }
    if info.total_mem_bytes < ANDROID_MIN_MEM_BYTES {
        let gb = info.total_mem_bytes as f64 / 1e9;
        return Err(format!("机型分级：内存 {gb:.1}GB < 3.5GB"));
    }
    Ok(())
}

/// 拿到适配器之后的门槛。`cached` 是判定缓存对这块适配器的结论（见 `verdict`），
/// `deny` 平时传 [`DENY`]（参数化是为了单测能塞一条进去）。
pub fn adapter_gate(
    pref: BackendPref,
    f: &AdapterFacts,
    needs: GpuNeeds,
    deny: &[DenyRule],
    cached: Option<&str>,
) -> Result<(), String> {
    let auto = pref == BackendPref::Auto;
    // 硬规则：强制也挡
    if f.device_type == AdapterType::Cpu {
        return Err(format!("软件适配器（{}）", f.name));
    }
    if f.max_texture_dimension_2d < needs.max_texture_dimension_2d {
        return Err(format!(
            "纹理上限 {} 不足（需要 {}）",
            f.max_texture_dimension_2d, needs.max_texture_dimension_2d
        ));
    }
    if f.max_texture_array_layers < needs.array_layers {
        return Err(format!(
            "纹理数组层数上限 {} 不足（需要 {}）",
            f.max_texture_array_layers, needs.array_layers
        ));
    }
    if !f.downlevel_missing.is_empty() {
        return Err(format!("下层能力不足：缺 {}", f.downlevel_missing.join("、")));
    }
    if !auto {
        return Ok(());
    }
    // 软规则：只挡 Auto
    match f.device_type {
        AdapterType::IntegratedGpu | AdapterType::DiscreteGpu => {}
        t => return Err(format!("适配器类型 {t:?}")),
    }
    match f.api {
        GpuApi::Metal | GpuApi::Vulkan | GpuApi::Dx12 => {}
        GpuApi::Gl => return Err("Auto 不用 GLES".into()),
        GpuApi::Other => return Err(format!("Auto 不用 {}", f.api.label())),
    }
    if let Some(rule) = deny.iter().find(|r| r.matches(f)) {
        return Err(format!("黑名单：{}", rule.note));
    }
    if let Some(why) = cached {
        return Err(format!("缓存判定：该驱动上次{why}"));
    }
    Ok(())
}

/// 一次选择里不碰 GPU 的输入
#[derive(Clone, Copy, Debug)]
pub struct SelectInput<'a> {
    pub pref: BackendPref,
    pub source: PrefSource,
    pub platform: Platform,
    pub device: &'a DeviceInfo,
    pub gpu_compiled: bool,
    /// 有没有可以上屏的 surface。桌面恒为 true（窗口本身就是）；SDK 看宿主有没有 attach
    pub has_surface: bool,
}

/// 不碰 GPU 就能下的结论：`Err(status)` 表示到此为止、用 CPU；`Ok(())` 表示该去试 GPU 了。
///
/// 偏好是 cpu 时最先返回（原因写偏好来源），而不是报「未编译」：用户明确要 CPU 时，
/// 「为什么不是 GPU」不是个问题，标签只写 `[后端 CPU]`。
pub fn pre_gpu(input: &SelectInput<'_>) -> Result<(), BackendStatus> {
    let SelectInput { pref, source, .. } = *input;
    if pref == BackendPref::Cpu {
        return Err(BackendStatus::preferred_cpu(source));
    }
    platform_gate(pref, input.platform, input.device, input.gpu_compiled)
        .map_err(|why| BackendStatus::cpu(pref, source, why))?;
    if !input.has_surface {
        return Err(BackendStatus::cpu(pref, source, REASON_NO_SURFACE));
    }
    Ok(())
}

/// 现阶段的整段选择：门槛都过了也只能是 CPU —— 还没有 GPU 合成器可试。
/// 以后接上 GPU 时，宿主在 [`pre_gpu`] 返回 `Ok` 之后去建设备、过 [`adapter_gate`] 与自检。
pub fn decide(input: &SelectInput<'_>) -> BackendStatus {
    match pre_gpu(input) {
        Err(status) => status,
        Ok(()) => BackendStatus::cpu(input.pref, input.source, REASON_GPU_PENDING),
    }
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
