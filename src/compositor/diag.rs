//! 后端相关的日志行：**被脚本 grep 的每一行都由这里的函数产出**（v2 修订 R16）。
//!
//! 为什么集中到一处：`gpu-parity.sh`、基准编排端都靠 grep 日志判断「到底用了哪个后端、
//! 有没有回退、有没有空转」。这些行要是散在各个宿主里各写各的，改一个字脚本就静默失效 ——
//! 「全绿但没测到」。所以固定前缀做成常量，格式化函数配单测断言前缀。
//!
//! | 事件 | 固定前缀（脚本 grep 这个） |
//! |---|---|
//! | 选定 GPU | [`PREFIX_GPU_SELECTED`] |
//! | 选定 CPU | [`PREFIX_CPU_SELECTED`] |
//! | 启动期回退 | [`PREFIX_FALLBACK`] |
//! | 运行中回退 | [`PREFIX_RUNTIME_FALLBACK`] |
//! | 合成模式（`MINI_GPU_LOG`） | [`PREFIX_MODE_LAYERED`] / [`PREFIX_MODE_PASSTHROUGH`] |
//! | 逐帧校验汇总 | [`PREFIX_VERIFY_SUMMARY`] |
//! | 逐帧校验失败 | [`PREFIX_VERIFY_MISMATCH`] |
//! | `MINI_FPS` 行 | [`PREFIX_FPS`] |
//!
//! 出口 [`emit`]：现在一律写 stderr（桌面、iOS 进 Xcode 控制台）。Android 的 stderr
//! 没人看，要改走 `liblog`（tag `mini-render`）—— 那一步要链接 `liblog`，放在移动端
//! GPU 接线（P2.1）里一起做。
use super::{BackendKind, BackendPref, BackendStatus};

pub const PREFIX_GPU_SELECTED: &str = "🎨 渲染后端：GPU";
pub const PREFIX_CPU_SELECTED: &str = "🎨 渲染后端：CPU";
pub const PREFIX_FALLBACK: &str = "⚠️ GPU 回退 CPU：";
pub const PREFIX_RUNTIME_FALLBACK: &str = "⚠️ GPU 运行中回退 CPU：";
pub const PREFIX_MODE_LAYERED: &str = "🧩 合成模式 分层";
pub const PREFIX_MODE_PASSTHROUGH: &str = "🧩 合成模式 直通";
pub const PREFIX_VERIFY_SUMMARY: &str = "GPU 逐帧校验：";
pub const PREFIX_VERIFY_MISMATCH: &str = "❌ GPU 校验不一致";
pub const PREFIX_FPS: &str = "📊 [后端 ";

/// 回退的原因串以它开头（[`BackendStatus::fell_back`]），标签据此显示回退原因
pub const FALLBACK_REASON_HEAD: &str = "回退：";

/// 打一行后端日志
pub fn emit(line: &str) {
    eprintln!("{line}");
}

/// 选定（或回退后定下）后端的那一行：
/// `🎨 渲染后端：GPU（Metal · Apple M3 Pro）— 自动选择` / `🎨 渲染后端：CPU — 未编译 GPU 支持`
pub fn selected(s: &BackendStatus) -> String {
    match (s.active, &s.adapter) {
        (BackendKind::Gpu, Some(a)) => format!("{PREFIX_GPU_SELECTED}（{a}）— {}", s.reason),
        (BackendKind::Gpu, None) => format!("{PREFIX_GPU_SELECTED} — {}", s.reason),
        (BackendKind::Cpu, _) => format!("{PREFIX_CPU_SELECTED} — {}", s.reason),
    }
}

/// 启动期回退：`⚠️ GPU 回退 CPU：自检像素不一致（…）`
pub fn fallback(why: &str) -> String {
    format!("{PREFIX_FALLBACK}{why}")
}

/// 运行中回退：`⚠️ GPU 运行中回退 CPU：设备丢失（帧#120）`
pub fn runtime_fallback(why: &str, gpu_frame: u32) -> String {
    format!("{PREFIX_RUNTIME_FALLBACK}{why}（帧#{gpu_frame}）")
}

/// 字节数写成 `5.9MB` / `12.0KB` / `300B`（合成模式日志里的上传量）
fn bytes(n: u64) -> String {
    match n {
        n if n >= 1 << 20 => format!("{:.1}MB", n as f64 / (1u64 << 20) as f64),
        n if n >= 1 << 10 => format!("{:.1}KB", n as f64 / (1u64 << 10) as f64),
        n => format!("{n}B"),
    }
}

/// `🧩 合成模式 分层，上传 12.0KB`
pub fn mode_layered(upload_bytes: u64) -> String {
    format!("{PREFIX_MODE_LAYERED}，上传 {}", bytes(upload_bytes))
}

/// `🧩 合成模式 直通：Modal，上传 5.9MB`（`why` 是触发直通的覆盖层）
pub fn mode_passthrough(why: &str, upload_bytes: u64) -> String {
    format!("{PREFIX_MODE_PASSTHROUGH}：{why}，上传 {}", bytes(upload_bytes))
}

/// `GPU 逐帧校验：300 帧全部一致`
pub fn verify_summary(frames: u64) -> String {
    format!("{PREFIX_VERIFY_SUMMARY}{frames} 帧全部一致")
}

/// `❌ GPU 校验不一致 帧#12 @3,4 CPU #0a0b0c / GPU #0a0b0d`（颜色是 0xRRGGBB）
pub fn verify_mismatch(frame: u64, x: u32, y: u32, cpu: u32, gpu: u32) -> String {
    format!(
        "{PREFIX_VERIFY_MISMATCH} 帧#{frame} @{x},{y} CPU #{:06x} / GPU #{:06x}",
        cpu & 0xFF_FFFF,
        gpu & 0xFF_FFFF
    )
}

/// `MINI_FPS` 行与 `--drag` 报告里的后端标签：
/// - `[后端 GPU · Metal · Apple M3 Pro]`
/// - `[后端 CPU]`（偏好就是 cpu）
/// - `[后端 CPU（未编译 GPU 支持）]` / `[后端 CPU（回退：<原因>）]` / `[后端 CPU（机型分级：…）]`
pub fn backend_tag(s: &BackendStatus) -> String {
    match (s.active, &s.adapter) {
        (BackendKind::Gpu, Some(a)) => format!("[后端 GPU · {a}]"),
        (BackendKind::Gpu, None) => "[后端 GPU]".into(),
        // 用户明确要 CPU：「为什么不是 GPU」不是个问题，标签不带原因
        (BackendKind::Cpu, _) if s.pref == BackendPref::Cpu && !s.reason.starts_with(FALLBACK_REASON_HEAD) => {
            "[后端 CPU]".into()
        }
        (BackendKind::Cpu, _) => format!("[后端 CPU（{}）]", s.reason),
    }
}

/// `📊 [后端 CPU] <rest>`：`MINI_FPS` 每秒那一行
pub fn fps_line(s: &BackendStatus, rest: &str) -> String {
    format!("📊 {} {rest}", backend_tag(s))
}

/// SDK：`MINI_BACKEND=cpu 覆盖宿主配置 gpu`
pub fn env_overrides_host(env: BackendPref, host: BackendPref) -> String {
    format!("MINI_BACKEND={} 覆盖宿主配置 {}", env.as_str(), host.as_str())
}

/// `MINI_BACKEND` 认不出来：按该宿主的默认值处理
pub fn invalid_env_backend(raw: &str, default: BackendPref) -> String {
    format!(
        "⚠️ MINI_BACKEND={raw:?} 无法识别（可选 auto / cpu / gpu），按默认 {} 处理",
        default.as_str()
    )
}

/// `MINI_GPU_FAULT` 认不出来：忽略注入
pub fn invalid_fault(raw: &str, why: &str) -> String {
    format!("⚠️ MINI_GPU_FAULT={raw:?} 无法解析（{why}），已忽略")
}

/// `--backend` 的值不对（进程随后以退出码 2 结束）
pub fn invalid_backend_arg(raw: Option<&str>) -> String {
    match raw {
        Some(v) => format!("❌ --backend 只接受 auto / cpu / gpu（不区分大小写），收到 {v:?}"),
        None => "❌ --backend 后面要跟 auto / cpu / gpu".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::select::REASON_NOT_COMPILED;
    use crate::compositor::PrefSource;

    fn gpu() -> BackendStatus {
        BackendStatus::gpu(BackendPref::Auto, PrefSource::Default, "Metal · Apple M3 Pro")
    }

    #[test]
    fn selected_lines_start_with_their_prefix() {
        assert_eq!(selected(&gpu()), "🎨 渲染后端：GPU（Metal · Apple M3 Pro）— 自动选择");
        assert!(selected(&gpu()).starts_with(PREFIX_GPU_SELECTED));
        let s = BackendStatus::cpu(BackendPref::Gpu, PrefSource::Cli, REASON_NOT_COMPILED);
        assert_eq!(selected(&s), "🎨 渲染后端：CPU — 未编译 GPU 支持");
        let s = BackendStatus::preferred_cpu(PrefSource::Default);
        assert_eq!(selected(&s), "🎨 渲染后端：CPU — 偏好 cpu（默认）");
        assert!(selected(&s).starts_with(PREFIX_CPU_SELECTED));
    }

    #[test]
    fn fallback_and_mode_lines_start_with_their_prefix() {
        assert_eq!(fallback("自检像素不一致"), "⚠️ GPU 回退 CPU：自检像素不一致");
        assert_eq!(runtime_fallback("设备丢失", 120), "⚠️ GPU 运行中回退 CPU：设备丢失（帧#120）");
        assert!(runtime_fallback("x", 1).starts_with(PREFIX_RUNTIME_FALLBACK));
        // 两个回退前缀不能互为前缀，否则 grep 启动期回退会把运行中回退也算进去
        assert!(!PREFIX_RUNTIME_FALLBACK.starts_with(PREFIX_FALLBACK));
        assert_eq!(mode_layered(12 * 1024), "🧩 合成模式 分层，上传 12.0KB");
        assert_eq!(mode_passthrough("Modal", 6_187_000), "🧩 合成模式 直通：Modal，上传 5.9MB");
        assert_eq!(mode_layered(300), "🧩 合成模式 分层，上传 300B");
        assert!(mode_passthrough("Toast", 0).starts_with(PREFIX_MODE_PASSTHROUGH));
    }

    #[test]
    fn verify_lines_start_with_their_prefix() {
        assert_eq!(verify_summary(300), "GPU 逐帧校验：300 帧全部一致");
        assert_eq!(
            verify_mismatch(12, 3, 4, 0x0a0b0c, 0xff0a0b0d),
            "❌ GPU 校验不一致 帧#12 @3,4 CPU #0a0b0c / GPU #0a0b0d"
        );
    }

    #[test]
    fn backend_tags() {
        assert_eq!(backend_tag(&gpu()), "[后端 GPU · Metal · Apple M3 Pro]");
        assert_eq!(backend_tag(&BackendStatus::preferred_cpu(PrefSource::Cli)), "[后端 CPU]");
        let nc = BackendStatus::cpu(BackendPref::Auto, PrefSource::Default, REASON_NOT_COMPILED);
        assert_eq!(backend_tag(&nc), "[后端 CPU（未编译 GPU 支持）]");
        assert_eq!(backend_tag(&gpu().fell_back("设备丢失")), "[后端 CPU（回退：设备丢失）]");
        // 偏好 cpu 也照样显示回退（不会发生，但标签不能把回退藏起来）
        let s = BackendStatus::preferred_cpu(PrefSource::Cli).fell_back("x");
        assert_eq!(backend_tag(&s), "[后端 CPU（回退：x）]");
    }

    #[test]
    fn fps_lines_start_with_the_fps_prefix() {
        let s = BackendStatus::preferred_cpu(PrefSource::Default);
        let line = fps_line(&s, "144 帧/秒");
        assert_eq!(line, "📊 [后端 CPU] 144 帧/秒");
        assert!(line.starts_with(PREFIX_FPS));
        let nc = BackendStatus::cpu(BackendPref::Auto, PrefSource::Default, REASON_NOT_COMPILED);
        assert!(fps_line(&nc, "1 帧/秒").starts_with("📊 [后端 CPU（未编译 GPU 支持）] "));
        assert!(fps_line(&gpu(), "").starts_with(PREFIX_FPS));
    }

    #[test]
    fn input_problem_lines() {
        assert_eq!(env_overrides_host(BackendPref::Cpu, BackendPref::Gpu), "MINI_BACKEND=cpu 覆盖宿主配置 gpu");
        assert!(invalid_env_backend("foo", BackendPref::Auto).contains("按默认 auto"));
        assert!(invalid_fault("lost", "要带帧号").contains("已忽略"));
        assert!(invalid_backend_arg(Some("foo")).contains("\"foo\""));
        assert!(invalid_backend_arg(None).contains("auto / cpu / gpu"));
    }
}
