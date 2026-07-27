//! 绘制耗时归因（`MINI_DRAW_LOG=1`）。
//!
//! 自绘引擎里「一帧为什么要 8ms」这个问题，只能按**组件类型**摊开才有答案：
//! 是几张大图在重采样、是上百个文字在光栅化，还是圆角/阴影的逐像素混合。
//! 之前只有整帧总耗时，优化只能靠猜。
//!
//! 计数器是线程本地的，绘制全在主线程；关掉开关时 `record` 只做一次
//! `OnceLock<bool>` 读取，热路径上可忽略。

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;

fn switch() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("MINI_DRAW_LOG").is_ok())
}

/// 是否开启归因统计（关闭时调用方应完全跳过计时）
#[inline]
pub fn enabled() -> bool {
    switch()
}

thread_local! {
    /// tag -> (次数, 累计毫秒)
    static ACC: RefCell<HashMap<&'static str, (u64, f64)>> = RefCell::new(HashMap::new());
}

/// 记一次绘制耗时。`tag` 用 `&'static str`（组件类型是有限集合），避免热路径上分配。
pub fn record(tag: &'static str, elapsed_ms: f64) {
    if !switch() {
        return;
    }
    ACC.with(|acc| {
        let mut acc = acc.borrow_mut();
        let e = acc.entry(tag).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += elapsed_ms;
    });
}

/// 计时辅助：`let _t = Timer::start();` 落域时自动记账。
pub struct Timer {
    tag: &'static str,
    at: Instant,
}

impl Timer {
    #[inline]
    pub fn start(tag: &'static str) -> Option<Self> {
        if switch() {
            Some(Self { tag, at: Instant::now() })
        } else {
            None
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        record(self.tag, self.at.elapsed().as_secs_f64() * 1000.0);
    }
}

/// 取出并清空统计，按累计耗时降序返回 `(tag, 次数, 毫秒)`。
pub fn take_totals() -> Vec<(&'static str, u64, f64)> {
    ACC.with(|acc| {
        let mut v: Vec<(&'static str, u64, f64)> =
            acc.borrow().iter().map(|(k, (n, ms))| (*k, *n, *ms)).collect();
        acc.borrow_mut().clear();
        v.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        v
    })
}

/// 一行摘要（累计耗时前 N 项）。无数据时返回 None。
pub fn summary(top: usize) -> Option<String> {
    let totals = take_totals();
    if totals.is_empty() {
        return None;
    }
    let all: f64 = totals.iter().map(|t| t.2).sum();
    let parts: Vec<String> = totals
        .iter()
        .take(top)
        .map(|(tag, n, ms)| format!("{tag} {ms:.1}ms/{n}次"))
        .collect();
    Some(format!("绘制归因（合计 {all:.1}ms）：{}", parts.join("  ")))
}
