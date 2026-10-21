//! swiper 的缓动曲线与时钟。
//!
//! ## 时钟
//! 换页动画、自动播放的间隔都按毫秒时钟走。SDK 宿主每帧把自己的时间喂进来（[`set_clock_ms`]，
//! `MiniEngine::pump(now_ms)` 的 now），于是同一串 pump 时间得到同一串画面 —— 测试与回放可复现。
//! 没人喂（桌面窗体、画廊、单测里只建渲染器）时用进程启动以来的墙上时钟，与从前一样。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static CLOCK_MS: AtomicU64 = AtomicU64::new(0);

/// 宿主喂时钟（毫秒，单调即可；0 表示「不喂了，回到墙上时钟」）
pub fn set_clock_ms(ms: u64) {
    CLOCK_MS.store(ms, Ordering::Relaxed);
}

/// 当前时刻（毫秒）
pub fn now_ms() -> u64 {
    let fed = CLOCK_MS.load(Ordering::Relaxed);
    if fed > 0 {
        return fed;
    }
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}

/// `easing-function`（官方合法值：default / linear / easeInCubic / easeOutCubic / easeInOutCubic）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// 文档没给 default 的曲线；取 ease-out cubic —— 起步快、收尾稳，是从前固定使用的曲线，
    /// 也和开发者工具里的观感一致
    #[default]
    Default,
    Linear,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
}

impl Easing {
    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "linear" => Self::Linear,
            "easeInCubic" => Self::EaseInCubic,
            "easeOutCubic" => Self::EaseOutCubic,
            "easeInOutCubic" => Self::EaseInOutCubic,
            _ => Self::Default,
        }
    }

    /// `t ∈ [0, 1]` → 进度 `[0, 1]`
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseInCubic => t * t * t,
            Self::Default | Self::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
        }
    }
}
