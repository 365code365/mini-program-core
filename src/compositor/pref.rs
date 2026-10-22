//! 后端偏好：从哪来、谁说了算（设计 §6.1、§10；v2 修订 R14）。
//!
//! 三个输入：命令行 `--backend`、环境变量 `MINI_BACKEND`、SDK 宿主的
//! `mr_app_set_backend`。优先级因宿主而异：
//!
//! | 宿主 | 高 → 低 | 都没给时 |
//! |---|---|---|
//! | 桌面交互窗体、`--backend-probe` | `--backend` > `MINI_BACKEND` | `auto` |
//! | 桌面无头（`--snapshot` 及其上的所有动作） | `--backend` > `MINI_BACKEND` | `cpu` |
//! | SDK（C ABI / JNI） | `MINI_BACKEND` > `mr_app_set_backend` | `cpu` |
//!
//! - 无头默认 `cpu`：所有逐字节判据都在无头下跑，而判据以 CPU 为准。
//! - SDK 默认 `cpu`：老的调用方从不挂 surface，行为必须与以前完全一样。
//! - SDK 里环境变量**压过**宿主配置：`MINI_BACKEND` 是诊断开关，用途就是不改宿主代码
//!   也能强制某个后端排查问题；桌面里反过来，命令行压过环境变量，与其它参数的习惯一致。
//!
//! ## 进程环境只在宿主入口读一次（R14）
//! [`HostEnv::from_process`] 是整个后端选择里**唯一**碰 `std::env` 的地方，由宿主入口
//! （`window.rs` 的 `main`、`MiniEngine::new`）调用，之后只传解析好的值。
//! 测试一律用 [`HostEnv::from_lookup`] 显式给值，禁止 `std::env::set_var`：进程环境是
//! 全局的，并行跑的测试会互相看到对方设的值，表现是「单独跑必过、全量跑偶发失败」。
use super::fault::{self, FaultSpec};
use super::BackendPref;

/// 偏好来自哪里（写进原因串，排查「为什么不是 GPU」时一眼能看出该改哪）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrefSource {
    /// 都没给，用该宿主的默认值
    Default,
    /// 命令行 `--backend`
    Cli,
    /// 环境变量 `MINI_BACKEND`
    Env,
    /// SDK 宿主调用 `mr_app_set_backend`
    Host,
}

impl PrefSource {
    pub fn label(self) -> &'static str {
        match self {
            PrefSource::Default => "默认",
            PrefSource::Cli => "--backend",
            PrefSource::Env => "MINI_BACKEND",
            PrefSource::Host => "宿主配置",
        }
    }
}

/// 哪一种宿主在做选择：决定优先级与默认值
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKind {
    /// 桌面交互窗体
    Interactive,
    /// 桌面无头（`--snapshot` 及其上的 `--frames` / `--drag` 等动作）
    Headless,
    /// `mini-app-window --backend-probe`：按交互窗体的规则选一次就退出
    Probe,
    /// 移动端 SDK（`MiniEngine`，C ABI / JNI）
    Sdk,
}

impl HostKind {
    /// 都没给偏好时用什么
    pub fn default_pref(self) -> BackendPref {
        match self {
            HostKind::Interactive | HostKind::Probe => BackendPref::Auto,
            HostKind::Headless | HostKind::Sdk => BackendPref::Cpu,
        }
    }
}

/// 不区分大小写的 `auto` / `cpu` / `gpu`（两端空白忽略）。别的值一律 `None`。
pub fn parse_pref(s: &str) -> Option<BackendPref> {
    match s.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(BackendPref::Auto),
        "cpu" => Some(BackendPref::Cpu),
        "gpu" => Some(BackendPref::Gpu),
        _ => None,
    }
}

/// C ABI 的整数：0 = auto、1 = cpu、2 = gpu（`MR_BACKEND_*`）。越界返回 `None`。
pub fn pref_from_code(code: i32) -> Option<BackendPref> {
    match code {
        0 => Some(BackendPref::Auto),
        1 => Some(BackendPref::Cpu),
        2 => Some(BackendPref::Gpu),
        _ => None,
    }
}

/// 一个环境变量的解析结果
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvValue<T> {
    /// 没设，或设成了空串（空串视为没设，`MINI_BACKEND= cmd` 是常见的「临时关掉」写法）
    Unset,
    Valid(T),
    /// 设了但认不出来：按没设处理，并在入口 warn 一次
    Invalid { raw: String, why: String },
}

impl<T> Default for EnvValue<T> {
    fn default() -> Self {
        EnvValue::Unset
    }
}

impl<T: Copy> EnvValue<T> {
    /// 认得出来的值；没设或认不出都是 `None`
    pub fn valid(&self) -> Option<T> {
        match self {
            EnvValue::Valid(v) => Some(*v),
            _ => None,
        }
    }
}

/// 宿主入口一次性读出的、与后端相关的环境变量（R14）
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostEnv {
    /// `MINI_BACKEND`
    pub backend: EnvValue<BackendPref>,
    /// `MINI_GPU_FAULT`：故障注入。现在没有 GPU 合成器，只解析不注入
    pub gpu_fault: EnvValue<FaultSpec>,
    /// `MINI_GPU_VERIFY`：每帧 GPU 合成后与 CPU 逐字节对拍（只看是否设置）
    pub gpu_verify: bool,
    /// `MINI_GPU_LOG`：GPU 合成器的详细日志（只看是否设置）
    pub gpu_log: bool,
    /// `MINI_FPS`：每秒一行帧统计（只看是否设置，与桌面窗体同一语义）
    pub fps: bool,
    /// `MINI_NO_DAMAGE`：SDK 关掉局部重画、一律整帧（与桌面窗体同一语义，逐像素对照用）
    pub no_damage: bool,
    /// `MINI_DAMAGE_LOG`：SDK 每一帧重画了多少（局部矩形 / 补条带 / 整帧原因）打一行
    pub damage_log: bool,
    /// `MINI_NO_RESAMPLE`：SDK 的页面拖动不按事件时间重采样，按到达顺序逐事件走（对照 / 排查用）
    pub no_resample: bool,
}

impl HostEnv {
    /// 读当前进程环境。**只在宿主入口调用**（见模块注释）。
    pub fn from_process() -> Self {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// 按给定的查表函数构造，不碰进程环境（测试用这个显式给值）
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let non_empty = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let backend = match non_empty("MINI_BACKEND") {
            None => EnvValue::Unset,
            Some(raw) => match parse_pref(&raw) {
                Some(p) => EnvValue::Valid(p),
                None => EnvValue::Invalid { raw, why: "可选 auto / cpu / gpu".into() },
            },
        };
        let gpu_fault = match non_empty("MINI_GPU_FAULT") {
            None => EnvValue::Unset,
            Some(raw) => match fault::parse(&raw) {
                Ok(f) => EnvValue::Valid(f),
                Err(why) => EnvValue::Invalid { raw, why },
            },
        };
        Self {
            backend,
            gpu_fault,
            gpu_verify: get("MINI_GPU_VERIFY").is_some(),
            gpu_log: get("MINI_GPU_LOG").is_some(),
            fps: get("MINI_FPS").is_some(),
            no_damage: get("MINI_NO_DAMAGE").is_some(),
            damage_log: get("MINI_DAMAGE_LOG").is_some(),
            no_resample: get("MINI_NO_RESAMPLE").is_some(),
        }
    }

    /// 入口要打的 warn（无法识别的值）。每个宿主入口打一次，之后不再提。
    pub fn warnings(&self, kind: HostKind) -> Vec<String> {
        let mut out = Vec::new();
        if let EnvValue::Invalid { raw, .. } = &self.backend {
            out.push(super::diag::invalid_env_backend(raw, kind.default_pref()));
        }
        if let EnvValue::Invalid { raw, why } = &self.gpu_fault {
            out.push(super::diag::invalid_fault(raw, why));
        }
        out
    }
}

/// 偏好的最终结论
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub pref: BackendPref,
    pub source: PrefSource,
    /// SDK：`MINI_BACKEND` 压过了宿主给的、且与之不同的偏好（入口据此打一行说明）
    pub overrode_host: Option<BackendPref>,
}

/// 按 §6.1 的优先级定偏好。`cli` 只对桌面有意义，`host` 只对 SDK 有意义，其余宿主忽略。
pub fn resolve(
    kind: HostKind,
    cli: Option<BackendPref>,
    env: &HostEnv,
    host: Option<BackendPref>,
) -> Resolved {
    let env_pref = env.backend.valid();
    let plain = |pref, source| Resolved { pref, source, overrode_host: None };
    match kind {
        HostKind::Interactive | HostKind::Headless | HostKind::Probe => match (cli, env_pref) {
            (Some(p), _) => plain(p, PrefSource::Cli),
            (None, Some(p)) => plain(p, PrefSource::Env),
            (None, None) => plain(kind.default_pref(), PrefSource::Default),
        },
        HostKind::Sdk => match (env_pref, host) {
            (Some(p), h) => Resolved {
                pref: p,
                source: PrefSource::Env,
                overrode_host: h.filter(|h| *h != p),
            },
            (None, Some(h)) => plain(h, PrefSource::Host),
            (None, None) => plain(kind.default_pref(), PrefSource::Default),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> HostEnv {
        let m: HashMap<String, String> =
            pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        HostEnv::from_lookup(|k| m.get(k).cloned())
    }

    #[test]
    fn parse_is_case_insensitive_and_strict() {
        assert_eq!(parse_pref("auto"), Some(BackendPref::Auto));
        assert_eq!(parse_pref("CPU"), Some(BackendPref::Cpu));
        assert_eq!(parse_pref(" Gpu "), Some(BackendPref::Gpu));
        for bad in ["", "foo", "gpu1", "metal", "c p u"] {
            assert_eq!(parse_pref(bad), None, "{bad:?} 不该被认出来");
        }
    }

    #[test]
    fn ffi_codes_round_trip_and_reject_out_of_range() {
        for p in [BackendPref::Auto, BackendPref::Cpu, BackendPref::Gpu] {
            assert_eq!(pref_from_code(p.code()), Some(p));
        }
        for bad in [-1, 3, 7, i32::MAX] {
            assert_eq!(pref_from_code(bad), None);
        }
    }

    #[test]
    fn host_env_parses_values_and_treats_empty_as_unset() {
        let e = env(&[("MINI_BACKEND", "GPU"), ("MINI_GPU_FAULT", "lost@3"), ("MINI_FPS", "1")]);
        assert_eq!(e.backend, EnvValue::Valid(BackendPref::Gpu));
        assert_eq!(e.gpu_fault, EnvValue::Valid(FaultSpec::Lost(3)));
        assert!(e.fps && !e.gpu_verify && !e.gpu_log);

        let e = env(&[("MINI_BACKEND", "  "), ("MINI_GPU_FAULT", "")]);
        assert_eq!(e.backend, EnvValue::Unset);
        assert_eq!(e.gpu_fault, EnvValue::Unset);

        // 只看是否设置：MINI_GPU_VERIFY=0 也算打开（与 MINI_FPS 等现有开关一致）
        let e = env(&[("MINI_GPU_VERIFY", "0"), ("MINI_GPU_LOG", "")]);
        assert!(e.gpu_verify && e.gpu_log);
        assert_eq!(HostEnv::from_lookup(|_| None), HostEnv::default());
    }

    #[test]
    fn invalid_env_values_fall_back_and_warn_once() {
        let e = env(&[("MINI_BACKEND", "vulkan"), ("MINI_GPU_FAULT", "lost@x")]);
        assert!(matches!(e.backend, EnvValue::Invalid { .. }));
        assert!(matches!(e.gpu_fault, EnvValue::Invalid { .. }));
        let w = e.warnings(HostKind::Headless);
        assert_eq!(w.len(), 2);
        assert!(w[0].contains("vulkan") && w[0].contains("cpu"), "{}", w[0]);
        assert!(w[1].contains("lost@x"), "{}", w[1]);
        // 认不出就按默认处理
        let r = resolve(HostKind::Headless, None, &e, None);
        assert_eq!((r.pref, r.source), (BackendPref::Cpu, PrefSource::Default));
        assert!(env(&[]).warnings(HostKind::Sdk).is_empty());
    }

    #[test]
    fn defaults_per_host() {
        let none = HostEnv::default();
        let d = |k| {
            let r = resolve(k, None, &none, None);
            assert_eq!(r.source, PrefSource::Default);
            r.pref
        };
        assert_eq!(d(HostKind::Interactive), BackendPref::Auto);
        assert_eq!(d(HostKind::Probe), BackendPref::Auto);
        assert_eq!(d(HostKind::Headless), BackendPref::Cpu);
        assert_eq!(d(HostKind::Sdk), BackendPref::Cpu);
    }

    #[test]
    fn desktop_cli_beats_env() {
        let e = env(&[("MINI_BACKEND", "gpu")]);
        for k in [HostKind::Interactive, HostKind::Headless, HostKind::Probe] {
            let r = resolve(k, Some(BackendPref::Cpu), &e, None);
            assert_eq!((r.pref, r.source), (BackendPref::Cpu, PrefSource::Cli));
            let r = resolve(k, None, &e, None);
            assert_eq!((r.pref, r.source), (BackendPref::Gpu, PrefSource::Env));
            // 桌面没有「宿主配置」这一层：给了也不看
            let r = resolve(k, None, &HostEnv::default(), Some(BackendPref::Gpu));
            assert_eq!(r.source, PrefSource::Default);
        }
    }

    #[test]
    fn sdk_env_beats_host_and_says_so() {
        let e = env(&[("MINI_BACKEND", "cpu")]);
        let r = resolve(HostKind::Sdk, None, &e, Some(BackendPref::Gpu));
        assert_eq!((r.pref, r.source), (BackendPref::Cpu, PrefSource::Env));
        assert_eq!(r.overrode_host, Some(BackendPref::Gpu));
        // 一样的值不算「覆盖」，不用打那行说明
        let r = resolve(HostKind::Sdk, None, &e, Some(BackendPref::Cpu));
        assert_eq!(r.overrode_host, None);
        // 没有环境变量时听宿主的
        let r = resolve(HostKind::Sdk, None, &HostEnv::default(), Some(BackendPref::Auto));
        assert_eq!((r.pref, r.source), (BackendPref::Auto, PrefSource::Host));
        // SDK 没有命令行：给了也不看
        let r = resolve(HostKind::Sdk, Some(BackendPref::Gpu), &HostEnv::default(), None);
        assert_eq!((r.pref, r.source), (BackendPref::Cpu, PrefSource::Default));
    }
}
