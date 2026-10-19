//! 实例 / 适配器 / 设备的创建，适配器信息到 [`AdapterFacts`] 的转换，丢失与未捕获错误钩子
//! （设计 §5.7、§6.2；v2 修订 R6、#19）。
//!
//! 每个合成器实例各建一套 Instance / Device，不做进程级共享：设备丢失时影响面只限一个实例，
//! 代码也简单（多开小程序的场景少，多花的初始化时间记进 `MINI_GPU_LOG`）。
use crate::compositor::select::{AdapterFacts, AdapterType, GpuApi, GpuNeeds};
use crate::compositor::BackendPref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 设备丢失 / 未捕获错误的旗标（回调线程写，合成线程每帧读）
#[derive(Default)]
pub struct Health {
    lost: AtomicBool,
    lost_msg: Mutex<Option<String>>,
    error: Mutex<Option<String>>,
}

impl Health {
    /// 本帧之前发生的致命问题。优先级：设备丢失 > 校验错误（v2 修订 R6）
    pub fn fatal(&self) -> Option<super::GpuError> {
        if self.lost.load(Ordering::Acquire) {
            let msg = self.lost_msg.lock().ok().and_then(|m| m.clone()).unwrap_or_default();
            return Some(super::GpuError::DeviceLost(msg));
        }
        let err = self.error.lock().ok().and_then(|e| e.clone());
        err.map(super::GpuError::Validation)
    }
}

impl Health {
    /// 我们自己调了 `device.destroy()`：按定义就是设备丢失（wgpu 的 `DeviceLostReason::Destroyed`）。
    /// 不等回调 —— 实测上屏路径里回调要过好几帧才来，下一次 acquire 先报 Validation，
    /// 回退原因就被误报成「校验错误」
    pub fn mark_lost(&self, msg: &str) {
        if let Ok(mut m) = self.lost_msg.lock() {
            m.get_or_insert_with(|| msg.to_string());
        }
        self.lost.store(true, Ordering::Release);
    }
}

pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub facts: AdapterFacts,
    pub health: Arc<Health>,
}

/// 本平台允许的图形 API。Auto 不用 GLES（驱动参差、性能不占优）；强制 gpu 时再放开
fn backends(pref: BackendPref) -> wgpu::Backends {
    if cfg!(target_vendor = "apple") {
        wgpu::Backends::METAL
    } else if cfg!(target_os = "windows") {
        let b = wgpu::Backends::DX12 | wgpu::Backends::VULKAN;
        if pref == BackendPref::Gpu { b | wgpu::Backends::GL } else { b }
    } else if pref == BackendPref::Gpu {
        wgpu::Backends::VULKAN | wgpu::Backends::GL
    } else {
        wgpu::Backends::VULKAN
    }
}

pub fn new_instance(pref: BackendPref) -> Result<wgpu::Instance, String> {
    // Instance::new 在「本目标一个后端都没编进来」时会 panic：先查，不让它崩
    if wgpu::Instance::enabled_backend_features().is_empty() {
        return Err("此平台没有可用的 GPU 后端".into());
    }
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends(pref);
    Ok(wgpu::Instance::new(desc))
}

/// wgpu 的适配器信息 → 纯数据（`select` 模块不依赖 wgpu 类型）
pub fn facts_of(adapter: &wgpu::Adapter) -> AdapterFacts {
    let info = adapter.get_info();
    let limits = adapter.limits();
    AdapterFacts {
        api: match info.backend {
            wgpu::Backend::Metal => GpuApi::Metal,
            wgpu::Backend::Vulkan => GpuApi::Vulkan,
            wgpu::Backend::Dx12 => GpuApi::Dx12,
            wgpu::Backend::Gl => GpuApi::Gl,
            _ => GpuApi::Other,
        },
        device_type: match info.device_type {
            wgpu::DeviceType::IntegratedGpu => AdapterType::IntegratedGpu,
            wgpu::DeviceType::DiscreteGpu => AdapterType::DiscreteGpu,
            wgpu::DeviceType::VirtualGpu => AdapterType::VirtualGpu,
            wgpu::DeviceType::Cpu => AdapterType::Cpu,
            wgpu::DeviceType::Other => AdapterType::Other,
        },
        vendor: info.vendor,
        device: info.device,
        name: info.name.clone(),
        driver: info.driver.clone(),
        driver_info: info.driver_info.clone(),
        max_texture_dimension_2d: limits.max_texture_dimension_2d,
        max_texture_array_layers: limits.max_texture_array_layers,
        // 着色器只用到「整数纹理 textureLoad + uniform 缓冲 + 无混合渲染」，都在 WebGL2 级的
        // 下层能力之内，没有要额外检查的 DownlevelFlags。将来用到计算着色器 / 存储纹理时在这里加
        downlevel_missing: Vec::new(),
    }
}

/// 判定缓存的键：驱动或引擎（含着色器、自检逻辑）一变就是新键（v2 修订 R15）
pub fn verdict_key(f: &AdapterFacts) -> String {
    format!(
        "{:?}|{:#x}|{:#x}|{}|{}|{}|{}",
        f.api, f.vendor, f.device, f.name, f.driver, f.driver_info, super::engine_key()
    )
}

/// 适配器 → 设备。`needs` 决定纹理上限；装上丢失与未捕获错误钩子
pub fn open(instance: wgpu::Instance, adapter: wgpu::Adapter, needs: GpuNeeds) -> Result<Gpu, String> {
    let facts = facts_of(&adapter);
    let mut limits = if facts.api == GpuApi::Gl {
        wgpu::Limits::downlevel_webgl2_defaults()
    } else {
        wgpu::Limits::downlevel_defaults()
    };
    limits.max_texture_dimension_2d = limits.max_texture_dimension_2d.max(needs.max_texture_dimension_2d);
    limits.max_texture_array_layers = limits.max_texture_array_layers.max(needs.array_layers);
    let desc = wgpu::DeviceDescriptor {
        label: Some("mini-render 合成器"),
        required_features: wgpu::Features::empty(),
        required_limits: limits,
        memory_hints: wgpu::MemoryHints::Performance,
        ..Default::default()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&desc))
        .map_err(|e| format!("GPU 初始化失败：{e}"))?;
    let health = Arc::new(Health::default());
    let h = Arc::clone(&health);
    device.set_device_lost_callback(move |reason, msg| {
        if let Ok(mut m) = h.lost_msg.lock() {
            *m = Some(format!("{reason:?} {msg}"));
        }
        h.lost.store(true, Ordering::Release);
    });
    let h = Arc::clone(&health);
    device.on_uncaptured_error(Arc::new(move |e: wgpu::Error| {
        if let Ok(mut slot) = h.error.lock() {
            slot.get_or_insert_with(|| e.to_string());
        }
    }));
    Ok(Gpu { instance, adapter, device, queue, facts, health })
}

/// 选适配器：`compatible` 是要上屏的 surface（离屏时为 None）
pub fn request_adapter(instance: &wgpu::Instance, compatible: Option<&wgpu::Surface<'_>>) -> Result<wgpu::Adapter, String> {
    let opts = wgpu::RequestAdapterOptions {
        power_preference: if cfg!(any(target_os = "ios", target_os = "android")) {
            wgpu::PowerPreference::None
        } else {
            wgpu::PowerPreference::HighPerformance
        },
        force_fallback_adapter: false,
        compatible_surface: compatible,
        ..Default::default()
    };
    pollster::block_on(instance.request_adapter(&opts)).map_err(|e| format!("没有可用的 GPU 适配器：{e}"))
}

impl Gpu {
    /// 让丢失回调及时触发：wgpu-core 要在 poll 里才处理（v2 修订 R6）
    pub fn poll_now(&self) {
        let _ = self.device.poll(wgpu::PollType::Poll);
    }

    /// 等 GPU 做完已提交的工作
    pub fn wait_idle(&self) {
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
    }
}
