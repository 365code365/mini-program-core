//! 桌面窗体的后端选择：`--backend` / `MINI_BACKEND` / `--backend-probe`（设计 §6.1、§6.6）。
//!
//! 选择本身（优先级、门槛、原因串、日志格式）全在 lib 的 `compositor` 里，与 SDK 共用；
//! 这里只做命令行那一层：解析参数、决定是交互还是无头、probe 的退出码。
//!
//! 现在还没有 GPU 合成器：偏好不是 cpu 时一律落到 CPU，并打一行原因
//! （没编进 `gpu` feature 时是 `🎨 渲染后端：CPU — 未编译 GPU 支持`），照常运行、退出码 0。
//! 不因为「要 GPU 但给不了」就拒绝启动：steering 要求 GPU 不可用时一律回退 CPU。
use mini_render::compositor::select::{self, SelectInput};
use mini_render::compositor::{
    diag, pref, BackendKind, BackendPref, BackendStatus, DeviceInfo, HostEnv, HostKind, Platform,
};

/// `--backend-probe`：GPU 可用
pub const PROBE_GPU_OK: i32 = 0;
/// `--backend-probe`：没有合格的适配器，或被门槛挡掉
pub const PROBE_NO_GPU: i32 = 3;
/// `--backend-probe`：没编进 `gpu` feature
pub const PROBE_NOT_COMPILED: i32 = 4;
/// `--backend` 的值不对
pub const EXIT_BAD_ARG: i32 = 2;

/// 解析 `--backend` 的值（不区分大小写的 auto / cpu / gpu）。
/// 不合法或缺值时打印错误，并以退出码 2 结束进程 —— 拼错的后端名不能被静默当成默认值，
/// 否则「我明明指定了 gpu」的排查会从错误的前提开始。
pub fn parse_backend_arg(v: Option<String>) -> BackendPref {
    match v.as_deref().and_then(pref::parse_pref) {
        Some(p) => p,
        None => {
            eprintln!("{}", diag::invalid_backend_arg(v.as_deref()));
            std::process::exit(EXIT_BAD_ARG);
        }
    }
}

/// 不碰 GPU 的门槛都过了、还要去建 GPU 合成器（建在哪取决于宿主：窗口 surface 或离屏）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingGpu {
    pub pref: BackendPref,
    pub source: mini_render::compositor::PrefSource,
}

/// 按宿主种类选一次后端。两段式：
/// - 不碰 GPU 就能下结论（偏好 cpu、没编进 gpu feature……）：打一行选择日志，返回 `(结论, None)`；
/// - 否则返回 `(暂定 CPU, Some(待建 GPU))`，由宿主建合成器（窗口要等 `resumed` 拿到窗口），
///   建完（或回退）再打那一行选择日志。日志不受 `MINI_GPU_LOG` 控制：每次选定都要看得见。
pub fn choose(kind: HostKind, cli: Option<BackendPref>, env: &HostEnv) -> (BackendStatus, Option<PendingGpu>) {
    for w in env.warnings(kind) {
        diag::emit(&w);
    }
    let r = pref::resolve(kind, cli, env, None);
    // 桌面不需要机型信息（§6.2：桌面只看适配器）；窗口本身就是 surface
    let device = DeviceInfo::default();
    let input = SelectInput {
        pref: r.pref,
        source: r.source,
        platform: Platform::current(),
        device: &device,
        gpu_compiled: select::GPU_COMPILED,
        has_surface: true,
    };
    match select::pre_gpu(&input) {
        Err(status) => {
            diag::emit(&diag::selected(&status));
            (status, None)
        }
        Ok(()) => (
            BackendStatus::cpu(r.pref, r.source, "GPU 合成器初始化中"),
            Some(PendingGpu { pref: r.pref, source: r.source }),
        ),
    }
}

/// probe 的退出码：脚本据此区分「没编进来」「编了但没有可用的 GPU」「GPU 可用」
pub fn probe_exit_code(status: &BackendStatus, gpu_compiled: bool) -> i32 {
    if !gpu_compiled {
        PROBE_NOT_COMPILED
    } else if status.active == BackendKind::Gpu {
        PROBE_GPU_OK
    } else {
        PROBE_NO_GPU
    }
}

/// `--backend-probe`：按交互窗体的规则做一次选择（不开窗、不加载小程序），返回退出码。
/// 编了 gpu 时真的建一次离屏合成器并跑自检 —— 「门槛过了」不等于「GPU 能用」
pub fn probe(cli: Option<BackendPref>, env: &HostEnv) -> i32 {
    let (status, pending) = choose(HostKind::Probe, cli, env);
    let status = match pending {
        Some(p) => super::gpu_host::probe_offscreen(p, env).unwrap_or(status),
        None => status,
    };
    probe_exit_code(&status, select::GPU_COMPILED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mini_render::compositor::PrefSource;

    #[test]
    fn probe_codes() {
        let cpu = BackendStatus::cpu(BackendPref::Auto, PrefSource::Default, "x");
        let gpu = BackendStatus::gpu(BackendPref::Auto, PrefSource::Default, "Metal · M3");
        assert_eq!(probe_exit_code(&cpu, false), PROBE_NOT_COMPILED);
        assert_eq!(probe_exit_code(&gpu, false), PROBE_NOT_COMPILED, "没编进来时优先报 4");
        assert_eq!(probe_exit_code(&cpu, true), PROBE_NO_GPU);
        assert_eq!(probe_exit_code(&gpu, true), PROBE_GPU_OK);
    }

    #[test]
    fn headless_defaults_to_cpu_interactive_to_auto() {
        let env = HostEnv::default();
        let (s, pending) = choose(HostKind::Headless, None, &env);
        assert_eq!((s.pref, s.active), (BackendPref::Cpu, BackendKind::Cpu));
        assert_eq!(pending, None, "无头默认 cpu：不去建 GPU");
        assert_eq!(diag::backend_tag(&s), "[后端 CPU]");
        let (s, pending) = choose(HostKind::Interactive, None, &env);
        assert_eq!((s.pref, s.active), (BackendPref::Auto, BackendKind::Cpu));
        if select::GPU_COMPILED {
            assert!(pending.is_some(), "编了 gpu：交互窗体 auto 要去建 GPU 合成器");
        } else {
            assert_eq!(pending, None);
            assert_eq!(diag::selected(&s), "🎨 渲染后端：CPU — 未编译 GPU 支持");
            assert_eq!(diag::backend_tag(&s), "[后端 CPU（未编译 GPU 支持）]");
        }
    }
}
