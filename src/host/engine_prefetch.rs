//! SDK 引擎的图片预取：视口上一屏、下两屏里的图，交给后台线程先解码、降采样、按框缩放好
//! （`components::image::prefetch`，与绘制同一份缓存，像素不变）。
//!
//! 页面上有哪些图、画在哪按布局树数一遍（`image_plans`），只在布局变了、而且离上一次数超过
//! [`RESCAN_MS`] 时重数 —— 手势 / worklet 每帧 setData 的页面上，每帧遍历整棵树不划算。
//! 每滑过四分之一屏按新位置排一次。
use super::engine::MiniEngine;
use crate::renderer::components::image_prefetch;

/// 布局变了以后至少隔这么久才重数一遍图
const RESCAN_MS: u64 = 250;

#[derive(Default)]
pub struct Prefetch {
    /// 每张图画的时候的几何（设备像素、页面画布坐标），按上沿排好
    images: Vec<crate::renderer::wxml_renderer::ImagePlan>,
    /// 数的时候的布局代数与时刻
    scanned: Option<(u64, u64)>,
    /// 上一次排队时的滚动位置
    last_scroll: Option<f32>,
}

impl MiniEngine {
    /// 每次 pump 在重画之前调
    pub(super) fn prefetch_images(&mut self) {
        // `MINI_NO_PREFETCH=1`：关掉预取（对照「是不是预取的功劳 / 锅」）
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *OFF.get_or_init(|| std::env::var_os("MINI_NO_PREFETCH").is_some()) {
            return;
        }
        let Some(r) = self.renderer.as_ref() else { return };
        let gen = r.layout_generation();
        let now = self.mono_ms();
        let stale = match self.prefetch.scanned {
            None => true,
            Some((g, t)) => g != gen && now.saturating_sub(t) >= RESCAN_MS,
        };
        if stale {
            let mut list = r.image_plans();
            list.sort_by(|a, b| a.y.total_cmp(&b.y));
            self.prefetch.images = list;
            self.prefetch.scanned = Some((gen, now));
            self.prefetch.last_scroll = None;
        }
        let vp = self.viewport_height();
        let scroll = self.scroll.get_position();
        if self.prefetch.last_scroll.is_some_and(|s| (s - scroll).abs() < vp / 4.0) {
            return;
        }
        self.prefetch.last_scroll = Some(scroll);
        let dpr = self.dpr;
        let (top, bottom) = ((scroll - vp) * dpr, (scroll + vp * 3.0) * dpr);
        // 离视口近的后排（预取线程后进先出）：先排远的，最后排正在看的那一屏
        let mut near: Vec<_> = self.prefetch.images.iter().filter(|i| i.y + i.h >= top && i.y <= bottom).collect();
        let mid = (scroll + vp / 2.0) * dpr;
        near.sort_by(|a, b| (b.y + b.h / 2.0 - mid).abs().total_cmp(&(a.y + a.h / 2.0 - mid).abs()));
        for p in near {
            image_prefetch::request_plan(p);
        }
    }
}
