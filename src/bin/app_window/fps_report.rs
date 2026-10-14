//! 交互窗体 `MINI_FPS=1` 的每秒那一行。
//!
//! 从 `winit_app.rs` 的 `RedrawRequested` 里抽出来，只为了**能单测**：交互窗体要开窗口，
//! 无头跑不到这一行，而脚本 grep 的是它的固定前缀 `📊 [后端 `（v2 修订 R16）。
//! 字段与原来那句 `println!` 的实参一一对应，数字的格式一个没改，只在最前面加了后端标签。
use mini_render::compositor::{diag, BackendStatus};

/// 一个统计窗口（1 秒）的数据，单位 ms
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FpsWindow {
    pub frames: u32,
    pub gap_min_ms: f32,
    pub gap_max_ms: f32,
    pub worst_ms: f32,
    /// 最慢那一帧的（逻辑, 渲染, 上屏）
    pub worst_parts: (f32, f32, f32),
    /// 最慢那一帧的渲染内部（页面, 覆盖层, tabBar）
    pub worst_render_parts: (f32, f32, f32),
    pub animating: bool,
}

/// `📊 [后端 CPU] 144 帧/秒，帧间隔 6.8~7.1ms，最慢一帧 4.2ms（逻辑 … 上屏 …）`
pub fn fps_line(backend: &BackendStatus, w: &FpsWindow) -> String {
    let (l, r, p) = w.worst_parts;
    let (rp, rf, rt) = w.worst_render_parts;
    let rest = format!(
        "{} 帧/秒，帧间隔 {:.1}~{:.1}ms，最慢一帧 {:.1}ms（逻辑 {:.1} 渲染 {:.1}[页面 {:.1} 覆盖层 {:.1} tabBar {:.1}] 上屏 {:.1}）{}",
        w.frames,
        // 窗口里只出了一帧时没有间隔可比：最小值还是初始的 MAX，按 0 显示（与原来一致）
        if w.gap_min_ms == f32::MAX { 0.0 } else { w.gap_min_ms },
        w.gap_max_ms,
        w.worst_ms,
        l, r, rp, rf, rt, p,
        if w.animating { "（动画中）" } else { "" }
    );
    diag::fps_line(backend, &rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mini_render::compositor::select::REASON_NOT_COMPILED;
    use mini_render::compositor::{BackendPref, PrefSource};

    fn window() -> FpsWindow {
        FpsWindow {
            frames: 144,
            gap_min_ms: 6.84,
            gap_max_ms: 7.12,
            worst_ms: 4.21,
            worst_parts: (0.3, 3.1, 0.8),
            worst_render_parts: (2.5, 0.4, 0.2),
            animating: true,
        }
    }

    #[test]
    fn line_starts_with_the_backend_tag() {
        let cpu = BackendStatus::preferred_cpu(PrefSource::Cli);
        assert_eq!(
            fps_line(&cpu, &window()),
            "📊 [后端 CPU] 144 帧/秒，帧间隔 6.8~7.1ms，最慢一帧 4.2ms（逻辑 0.3 渲染 3.1[页面 2.5 覆盖层 0.4 tabBar 0.2] 上屏 0.8）（动画中）"
        );
        // 交互窗体默认 auto：没编 gpu 时标签带上原因，但仍以 `📊 [后端 CPU` 开头
        let nc = BackendStatus::cpu(BackendPref::Auto, PrefSource::Default, REASON_NOT_COMPILED);
        let line = fps_line(&nc, &FpsWindow { animating: false, ..window() });
        assert!(line.starts_with("📊 [后端 CPU（未编译 GPU 支持）] 144 帧/秒"), "{line}");
        assert!(line.starts_with(diag::PREFIX_FPS));
        assert!(!line.ends_with("（动画中）"));
    }

    #[test]
    fn single_frame_window_shows_zero_min_gap() {
        let cpu = BackendStatus::preferred_cpu(PrefSource::Default);
        let w = FpsWindow { frames: 1, gap_min_ms: f32::MAX, ..FpsWindow::default() };
        assert!(fps_line(&cpu, &w).contains("帧间隔 0.0~0.0ms"));
    }
}
