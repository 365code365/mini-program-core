//! `select` 的单测：§6.2 表里每条规则一条用例（Auto 与强制分别断言）。
//!
//! 单独成文件是为了让 `select.rs` 本体保持在 500 行以内、读规则时不被用例淹没。
use super::*;
use crate::compositor::BackendKind;

fn flagship() -> DeviceInfo {
    DeviceInfo { total_mem_bytes: 8_000_000_000, low_ram: false, os_api: 31 }
}

fn vulkan_igpu() -> AdapterFacts {
    AdapterFacts {
        api: GpuApi::Vulkan,
        device_type: AdapterType::IntegratedGpu,
        vendor: 0x5143,
        device: 0x0730,
        name: "Adreno 730".into(),
        driver: "Qualcomm".into(),
        driver_info: "512.615".into(),
        max_texture_dimension_2d: 8192,
        max_texture_array_layers: 256,
        downlevel_missing: Vec::new(),
    }
}

fn needs() -> GpuNeeds {
    GpuNeeds { max_texture_dimension_2d: 1500, array_layers: 3 }
}

const AUTO: BackendPref = BackendPref::Auto;
const GPU: BackendPref = BackendPref::Gpu;

fn android(pref: BackendPref, info: DeviceInfo) -> Result<(), String> {
    platform_gate(pref, Platform::Android, &info, true)
}

fn adapter(pref: BackendPref, f: &AdapterFacts) -> Result<(), String> {
    adapter_gate(pref, f, needs(), DENY, None)
}

#[test]
fn not_compiled_blocks_auto_and_forced() {
    for p in [AUTO, GPU] {
        for plat in [Platform::Desktop, Platform::Ios, Platform::Android] {
            assert_eq!(platform_gate(p, plat, &flagship(), false), Err(REASON_NOT_COMPILED.into()));
        }
    }
}

#[test]
fn android_api_28_is_cpu_under_auto_only() {
    let info = DeviceInfo { os_api: 28, ..flagship() };
    let e = android(AUTO, info).unwrap_err();
    assert!(e.contains("API 28 < 29"), "{e}");
    assert_eq!(android(GPU, info), Ok(()));
    // 未知（0）按不达标处理
    let e = android(AUTO, DeviceInfo { os_api: 0, ..flagship() }).unwrap_err();
    assert!(e.contains("API 未知"), "{e}");
}

#[test]
fn android_low_ram_is_cpu_under_auto_only() {
    let info = DeviceInfo { low_ram: true, ..flagship() };
    assert_eq!(android(AUTO, info), Err("机型分级：低内存设备".into()));
    assert_eq!(android(GPU, info), Ok(()));
}

#[test]
fn android_3_4e9_bytes_is_cpu_under_auto_only() {
    let info = DeviceInfo { total_mem_bytes: 3_400_000_000, ..flagship() };
    let e = android(AUTO, info).unwrap_err();
    assert!(e.contains("3.4GB < 3.5GB"), "{e}");
    assert_eq!(android(GPU, info), Ok(()));
    // 标称 4GB 的机器报 3.6GB：要放行
    assert_eq!(android(AUTO, DeviceInfo { total_mem_bytes: 3_600_000_000, ..flagship() }), Ok(()));
    let e = android(AUTO, DeviceInfo { total_mem_bytes: 0, ..flagship() }).unwrap_err();
    assert!(e.contains("内存未知"), "{e}");
}

#[test]
fn android_flagship_with_vulkan_igpu_passes_both_gates() {
    assert_eq!(android(AUTO, flagship()), Ok(()));
    assert_eq!(adapter(AUTO, &vulkan_igpu()), Ok(()));
}

#[test]
fn ios_and_desktop_have_no_device_rules() {
    // #18：iOS 不再按内存分级；桌面不需要 DeviceInfo
    let tiny = DeviceInfo { total_mem_bytes: 1_000_000_000, low_ram: true, os_api: 1 };
    assert_eq!(platform_gate(AUTO, Platform::Ios, &tiny, true), Ok(()));
    assert_eq!(platform_gate(AUTO, Platform::Desktop, &DeviceInfo::default(), true), Ok(()));
}

#[test]
fn other_platform_has_no_gpu_path() {
    for p in [AUTO, GPU] {
        assert!(platform_gate(p, Platform::Other, &flagship(), true).is_err());
    }
}

#[test]
fn gles_is_cpu_under_auto_but_allowed_when_forced() {
    let f = AdapterFacts { api: GpuApi::Gl, ..vulkan_igpu() };
    assert_eq!(adapter(AUTO, &f), Err("Auto 不用 GLES".into()));
    assert_eq!(adapter(GPU, &f), Ok(()));
}

#[test]
fn software_adapter_is_cpu_even_when_forced() {
    let f = AdapterFacts { device_type: AdapterType::Cpu, name: "llvmpipe".into(), ..vulkan_igpu() };
    for p in [AUTO, GPU] {
        let e = adapter(p, &f).unwrap_err();
        assert!(e.contains("软件适配器") && e.contains("llvmpipe"), "{e}");
    }
}

#[test]
fn virtual_or_other_adapter_type_is_cpu_under_auto_only() {
    for t in [AdapterType::VirtualGpu, AdapterType::Other] {
        let f = AdapterFacts { device_type: t, ..vulkan_igpu() };
        assert!(adapter(AUTO, &f).unwrap_err().starts_with("适配器类型"));
        assert_eq!(adapter(GPU, &f), Ok(()));
    }
    // 独显照常
    let f = AdapterFacts { device_type: AdapterType::DiscreteGpu, ..vulkan_igpu() };
    assert_eq!(adapter(AUTO, &f), Ok(()));
}

#[test]
fn texture_limits_too_small_is_cpu_even_when_forced() {
    let f = AdapterFacts { max_texture_dimension_2d: 1024, ..vulkan_igpu() };
    for p in [AUTO, GPU] {
        assert!(adapter(p, &f).unwrap_err().contains("纹理上限 1024 不足"));
    }
    let f = AdapterFacts { max_texture_array_layers: 2, ..vulkan_igpu() };
    assert!(adapter(GPU, &f).unwrap_err().contains("层数上限 2 不足"));
}

#[test]
fn missing_downlevel_caps_is_cpu_even_when_forced() {
    let f = AdapterFacts { downlevel_missing: vec!["VIEW_FORMATS".into()], ..vulkan_igpu() };
    for p in [AUTO, GPU] {
        assert!(adapter(p, &f).unwrap_err().contains("VIEW_FORMATS"));
    }
}

#[test]
fn deny_list_hit_is_cpu_under_auto_only() {
    const RULES: &[DenyRule] = &[DenyRule {
        vendor: Some(0x5143),
        device: None,
        driver_info_contains: Some("512."),
        note: "某驱动合成错位",
    }];
    let f = vulkan_igpu();
    assert_eq!(adapter_gate(AUTO, &f, needs(), RULES, None), Err("黑名单：某驱动合成错位".into()));
    assert_eq!(adapter_gate(GPU, &f, needs(), RULES, None), Ok(()));
    // 字段要全部对上才算命中
    let other = AdapterFacts { driver_info: "615.0".into(), ..vulkan_igpu() };
    assert_eq!(adapter_gate(AUTO, &other, needs(), RULES, None), Ok(()));
    // 编译期黑名单初始为空
    assert!(DENY.is_empty());
}

#[test]
fn cached_verdict_is_cpu_under_auto_only() {
    let f = vulkan_igpu();
    let e = adapter_gate(AUTO, &f, needs(), DENY, Some("自检像素不一致")).unwrap_err();
    assert_eq!(e, "缓存判定：该驱动上次自检像素不一致");
    assert_eq!(adapter_gate(GPU, &f, needs(), DENY, Some("自检像素不一致")), Ok(()));
}

#[test]
fn needs_cover_output_canvas_and_ring() {
    // 2× 屏：750×1334 输出、视口 1334 行、余量 600 → 1334+1200+2 = 2536 行 → 3 层
    let n = GpuNeeds::for_output(750, 1334, 750, 1334, 600);
    assert_eq!(n, GpuNeeds { max_texture_dimension_2d: 1334, array_layers: 3 });
    // 小输出也至少要 1024（环形条带每层 1024 行）
    assert_eq!(GpuNeeds::for_output(100, 100, 100, 0, 0).max_texture_dimension_2d, 1024);
    assert_eq!(GpuNeeds::for_output(100, 100, 100, 0, 0).array_layers, 1);
}

fn input(pref: BackendPref, compiled: bool, surface: bool) -> BackendStatus {
    let info = flagship();
    decide(&SelectInput {
        pref,
        source: PrefSource::Cli,
        platform: Platform::Desktop,
        device: &info,
        gpu_compiled: compiled,
        has_surface: surface,
    })
}

#[test]
fn decide_reports_the_first_blocking_rule() {
    // 偏好 cpu：原因写来源，不谈「未编译」
    let s = input(BackendPref::Cpu, false, true);
    assert_eq!((s.active, s.reason.as_str()), (BackendKind::Cpu, "偏好 cpu（--backend）"));
    // 没编进来：Auto 与强制都是这一句
    for p in [AUTO, GPU] {
        let s = input(p, false, true);
        assert_eq!((s.active, s.pref, s.reason.as_str()), (BackendKind::Cpu, p, REASON_NOT_COMPILED));
    }
    // 编进来了但 SDK 没挂 surface
    assert_eq!(input(AUTO, true, false).reason, REASON_NO_SURFACE);
    // 门槛全过：现在也只能是 CPU（还没有 GPU 合成器）
    let s = input(GPU, true, true);
    assert_eq!((s.active, s.reason.as_str()), (BackendKind::Cpu, REASON_GPU_PENDING));
    assert_eq!(s.adapter, None);
}

#[test]
fn compiled_flag_tracks_the_cargo_feature() {
    assert_eq!(GPU_COMPILED, cfg!(feature = "gpu"));
}
