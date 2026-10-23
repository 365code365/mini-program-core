//! GPU 预热：surface 还没来之前，在另一条线程上先把合成器（设备、管线、自检）开好。
//!
//! 为什么要：第一次打开小程序时，宿主建完引擎、开完首页才建出 SurfaceView，挂上去那一刻才开 GPU ——
//! 一加 PLY110 上开 Vulkan 设备 + 自检约 0.5s，这段时间 SurfaceView 是空的（宿主那边藏着，露底色）。
//! 首页启动（JS、建树、排版，约 0.3s）与开 GPU 互不依赖，并行起来首帧能早出约 0.3s。
//!
//! - 宿主在设好偏好与机型信息之后、`launch` 之前调 [`prewarm_gpu`](MiniEngine::prewarm_gpu)（Android Flutter 插件 /
//!   原生视图都这么调；不调就是从前的样子：挂 surface 时现开）；
//! - 门槛（偏好、机型分级、本实例失败过）与挂载时同一套，按「会有 surface」判；
//! - 自检照跑（离屏），不过的话挂载时直接用那个结论（CPU + 原因），不再开一遍；
//! - 挂载时取预热结果（还没开完就等它），之后与「回桌面再回来」同一条路：只换 surface（`reattach_parked`）；
//! - 预热线程也绑大核：自检要量 GPU 主线程耗时（门槛 4ms），放在小核上会被误判成「GPU 太慢」。
//!
//! 实测只在 `launch` 之前预热不够：一加上建引擎本身就要 0.3s（JS 运行时、注册模块、解析系统中文字体），
//! 首页启动只要 50ms，挂 surface 时还得等预热 0.5s。所以再往前一步：宿主**建引擎之前**调
//! [`warmup`]（`mr_warmup` / `nativeWarmup`），GPU 与字体各开一条线程跟建引擎并行；
//! 引擎的 `prewarm_gpu` 认领这份（像素尺寸要对得上），没有就自己开。
use super::engine::MiniEngine;
use crate::compositor::gpu::{self, GpuCompositor, OpenOptions};
use crate::compositor::select::{self, SelectInput, GPU_COMPILED};
use crate::compositor::{pref, BackendStatus, HostKind, Platform};

pub(crate) type Warming = std::thread::JoinHandle<Result<(GpuCompositor, BackendStatus), BackendStatus>>;

/// 建引擎之前开的那份（像素尺寸, 线程）。进程里同时只有一个小程序
static EARLY: std::sync::Mutex<Option<((u32, u32), Warming)>> = std::sync::Mutex::new(None);

/// 建引擎之前调：字体与 GPU 各开一条线程预热（见文件头）。`width` / `height` / `dpr` 与之后建引擎的一致，
/// `pref` 与之后 `set_backend_pref` 的一致。返回是否开始预热 GPU（字体总会预热）
pub fn warmup(width: u32, height: u32, dpr: f32, pref: Option<crate::compositor::BackendPref>, device: crate::compositor::DeviceInfo) -> bool {
    // 字体：`shared_fonts` 是进程级 OnceLock，建引擎时再取会等这条线程解析完，不会解析两遍
    let _ = std::thread::Builder::new().name("mini-font-warm".into()).spawn(|| {
        super::cpu_affinity::pin_current_thread();
        let t0 = std::time::Instant::now();
        let ok = crate::text::shared_fonts().is_some();
        crate::compositor::diag::emit(&format!("🔤 字体预热{}：{:.0}ms", if ok { "完成" } else { "失败" }, t0.elapsed().as_secs_f32() * 1000.0));
    });
    let env = crate::compositor::HostEnv::from_process();
    let r = pref::resolve(HostKind::Sdk, None, &env, pref);
    let dpr = if dpr > 0.1 { dpr } else { 1.0 };
    let size = ((width.max(1) as f32 * dpr) as u32, (height.max(1) as f32 * dpr) as u32);
    let Some(h) = spawn_open(r.pref, r.source, device, size, &env) else { return false };
    *EARLY.lock().unwrap_or_else(|e| e.into_inner()) = Some((size, h));
    true
}

/// 门槛过了就开一条线程去开合成器
fn spawn_open(
    pref: crate::compositor::BackendPref,
    source: crate::compositor::PrefSource,
    device: crate::compositor::DeviceInfo,
    (pw, ph): (u32, u32),
    env: &crate::compositor::HostEnv,
) -> Option<Warming> {
    let input = SelectInput { pref, source, platform: Platform::current(), device: &device, gpu_compiled: GPU_COMPILED, has_surface: true };
    if let Err(st) = select::pre_gpu(&input) {
        crate::compositor::diag::emit(&format!("🎨 GPU 不预热：{}", st.reason));
        return None;
    }
    let (fault, log) = (env.gpu_fault.valid(), env.gpu_log);
    std::thread::Builder::new()
        .name("mini-gpu-warm".into())
        .spawn(move || {
            let pin = super::cpu_affinity::pin_current_thread();
            if log {
                crate::compositor::diag::emit(&pin);
            }
            let t0 = std::time::Instant::now();
            let select = SelectInput { pref, source, platform: Platform::current(), device: &device, gpu_compiled: GPU_COMPILED, has_surface: true };
            let opts = OpenOptions {
                select,
                fault,
                log,
                width: pw,
                height: ph,
                canvas_w: pw,
                margin_px: crate::renderer::VIEWPORT_CULL_MARGIN_PX as u32,
                use_verdict: true,
            };
            let out = gpu::open(opts, None);
            crate::compositor::diag::emit(&format!(
                "🎨 GPU 预热{}：{:.0}ms（与建引擎 / 首页启动并行）",
                if out.is_ok() { "完成" } else { "没过门槛" },
                t0.elapsed().as_secs_f32() * 1000.0
            ));
            out
        })
        .ok()
}

impl MiniEngine {
    /// 在后台线程上先开 GPU 合成器（见文件头）。已有合成器、门槛不过、本实例失败过时什么都不做。
    /// 返回是否开始预热
    pub fn prewarm_gpu(&mut self) -> bool {
        if self.gpu.is_some() || self.gpu_parked.is_some() || self.gpu_warming.is_some() || self.gpu_failed.is_some() {
            crate::compositor::diag::emit("🎨 GPU 不预热：已有合成器 / 正在预热 / 本实例失败过");
            return false;
        }
        // 建引擎之前开好的那份（warmup）：尺寸对得上就认领
        let size = self.pixel_size();
        if let Some((s, h)) = EARLY.lock().unwrap_or_else(|e| e.into_inner()).take() {
            if s == size {
                self.gpu_warming = Some(h);
                return true;
            }
            crate::compositor::diag::emit(&format!("🎨 GPU 预热的尺寸 {s:?} 与引擎 {size:?} 不一致，重开"));
        }
        let r = pref::resolve(HostKind::Sdk, None, &self.env, self.host_pref);
        match spawn_open(r.pref, r.source, self.device_info, size, &self.env) {
            Some(h) => {
                self.gpu_warming = Some(h);
                true
            }
            None => false,
        }
    }

    /// 挂载时取预热结果：成功的合成器放到 `gpu_parked`（接着走「只换 surface」）返回 None；
    /// 预热没过门槛 / 自检返回那个结论；没有预热返回 None
    pub(super) fn take_warmed_gpu(&mut self) -> Option<BackendStatus> {
        let h = self.gpu_warming.take()?;
        let t0 = std::time::Instant::now();
        let out = h.join().ok()?;
        let waited = t0.elapsed().as_secs_f32() * 1000.0;
        if waited > 1.0 {
            crate::compositor::diag::emit(&format!("🎨 挂 surface 时 GPU 预热还没完，等了 {waited:.0}ms"));
        }
        match out {
            Ok((comp, _)) => {
                self.gpu_parked = Some(Box::new(comp));
                None
            }
            Err(status) => Some(status),
        }
    }
}
