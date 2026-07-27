//! 无头滑动性能测量（`--drag <每帧像素>x<帧数>`）。
//!
//! 为什么需要它：Skyline 里滑动是纯合成，手感由**最慢一帧**和**帧间隔峰值**决定
//! （见 `.kiro/steering/10-skyline-target.md`）。而我们唯一能自动化的入口是快照，
//! 快照永远只出一帧、且不走拖动路径 —— 于是「滑动发抖」这类问题在 CI 里完全测不到，
//! 只能靠人肉拖窗口。这里用与交互窗体同一套鼠标事件模拟一次手指拖动，
//! 逐帧计时后给出分位数，改动前后可直接比对。

use std::time::Duration;

/// 一次拖动的参数：每帧位移 + 帧数
#[derive(Clone, Copy, Debug)]
pub struct DragSpec {
    /// 每帧手指上移的逻辑像素（正数=内容往上走，即向下浏览）
    pub dy: f32,
    /// 拖动持续的帧数
    pub frames: u32,
}

/// 解析 `--drag` 的取值：`8x120`、`8,120`、`8`（默认 120 帧）。
pub fn parse(arg: &str) -> Option<DragSpec> {
    let arg = arg.trim();
    let (dy_str, frames_str) = match arg.split_once(['x', 'X', ',']) {
        Some((a, b)) => (a, Some(b)),
        None => (arg, None),
    };
    let dy: f32 = dy_str.trim().parse().ok()?;
    let frames: u32 = match frames_str {
        Some(s) => s.trim().parse().ok()?,
        None => 120,
    };
    if frames == 0 {
        return None;
    }
    Some(DragSpec { dy, frames })
}

/// 打印分位数报告。`budget` 是一帧的预算（刷新率倒数）。
pub fn report(label: &str, frame_ms: &[f32], budget: Duration) {
    if frame_ms.is_empty() {
        println!("🖐  {label}：没有采到帧");
        return;
    }
    let budget_ms = budget.as_secs_f32() * 1000.0;
    let mut sorted = frame_ms.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pick = |q: f32| sorted[((sorted.len() - 1) as f32 * q).round() as usize];
    let avg: f32 = frame_ms.iter().sum::<f32>() / frame_ms.len() as f32;
    let over = frame_ms.iter().filter(|ms| **ms > budget_ms).count();
    println!(
        "🖐  {label}：{} 帧，单帧耗时 平均 {:.2}ms  p50 {:.2}ms  p95 {:.2}ms  最慢 {:.2}ms；\
         超预算({:.2}ms) {} 帧 = {:.0}%",
        frame_ms.len(),
        avg,
        pick(0.50),
        pick(0.95),
        sorted[sorted.len() - 1],
        budget_ms,
        over,
        over as f32 * 100.0 / frame_ms.len() as f32,
    );
}

/// 最慢那一帧的渲染归因（页面画布 / fixed 覆盖层 / tabBar）。
/// 只报最慢一帧：滑动手感由尖峰决定，平均值会把尖峰藏起来。
pub fn report_worst_parts(total_ms: f32, parts: (f32, f32, f32)) {
    let (page, fixed, tabbar) = parts;
    println!(
        "   ↳ 最慢一帧 {total_ms:.2}ms 的渲染构成：页面 {page:.2}ms  覆盖层 {fixed:.2}ms  tabBar {tabbar:.2}ms"
    );
}
