//! 后台下完 / 解完的图（远程图、本地大图）：只重画用到它的那几块。
//!
//! 从前一律整帧（页面条带 + 覆盖层 + tabBar）。播放页进去时封面、背景、缩略图一张张到，一张一帧整帧，
//! 一加 PLY110 上一帧 45~52ms，叠在转场上。画在哪由渲染器按布局树找（`wxml_renderer/image_arrival.rs`），
//! 与点击态、swiper 换页同一条路并进局部重画（`dmg.extra`；覆盖层里的用 `press_fixed` 整层重画 ——
//! 同一帧 setData 的失效分析会改写 `fixed_dirty`，见 engine_swiper.rs）。
//!
//! 照旧要做的：离屏缓存作废（scroll-view 缓存、变换缓存、背板：数据没变它们就一直「干净」，贴回去的还是占位图，G-1）。
//! 被盖住那一页留着的画布记下这几张，返回时再按它们重画（engine_keep_canvas.rs）。`MINI_NO_DAMAGE=1` 走参照路径时照旧整帧。
use super::engine::MiniEngine;
use crate::renderer::wxml_renderer::ImageArrival;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 图到了只重画几块（没整帧）的次数（测试用）
static PARTIAL: AtomicUsize = AtomicUsize::new(0);

/// 图到了只重画几块的次数（测试用：证明走的是局部那条路，不是整帧碰巧也对）
pub fn image_arrival_partial_count() -> usize {
    PARTIAL.load(Ordering::Relaxed)
}

/// 等预取线程缩好图最多等这么久（毫秒）：到点没缩完就照旧在引擎线程上现缩
const PREFETCH_WAIT_MS: u64 = 120;

/// 等预取缩好再重画的那几张
#[derive(Default)]
pub(crate) struct ArrivalWait {
    keys: Vec<String>,
    srcs: Vec<String>,
    deadline_ms: u64,
}

impl MiniEngine {
    pub(super) fn images_arrived(&mut self, keys: &[String]) {
        if keys.is_empty() {
            // 这几张上一次已经一起取走、画过了
            return;
        }
        // 被盖住那一页留着的画布不作废：记下这几张，放回去时再按它们重画（engine_keep_canvas.rs）
        if let Some(k) = self.kept_canvas.as_mut() {
            if k.arrived.len() + keys.len() > 256 {
                self.bump_content_epoch();
            } else {
                k.arrived.extend(keys.iter().cloned());
            }
        }
        // 先让预取线程按框缩好（components/image/prefetch.rs，与绘制同一份缓存），缩好了再重画用到它们的那几块。
        // 从前图一到就在引擎线程上现缩：歌单页头图 1610×1610 一次 22ms、播放页封面 560×560 4.4ms（一加），
        // 叠在返回 / 进入的转场上。图晚一两帧出现，引擎线程不卡
        if !self.dmg.no_damage {
            if let Some(r) = self.renderer.as_ref() {
                let srcs: std::collections::HashSet<&str> = keys.iter().map(|k| k.strip_prefix("local:").unwrap_or(k)).collect();
                let plans: Vec<_> = r.image_plans().into_iter().filter(|p| srcs.contains(p.src.as_str())).collect();
                if !plans.is_empty() {
                    let now = self.mono_ms();
                    let w = self.arrival_wait.get_or_insert_with(|| ArrivalWait { deadline_ms: now + PREFETCH_WAIT_MS, ..Default::default() });
                    for p in &plans {
                        crate::renderer::components::image_prefetch::request_plan(p);
                        w.srcs.push(p.src.clone());
                    }
                    w.keys.extend(keys.iter().cloned());
                    return;
                }
            }
        }
        self.apply_image_arrivals(keys);
    }

    /// 每帧调：等着预取的那几张缩好了（或等到点了）就重画用到它们的那几块
    pub(super) fn flush_image_arrivals(&mut self) {
        let Some(w) = self.arrival_wait.as_ref() else { return };
        let done = self.mono_ms() >= w.deadline_ms
            || !w.srcs.iter().any(|s| crate::renderer::components::image_prefetch::busy(s));
        if done {
            let w = self.arrival_wait.take().unwrap_or_default();
            self.apply_image_arrivals(&w.keys);
        }
    }

    /// 还有图在等预取缩好（宿主要续帧）
    pub(crate) fn arrivals_waiting(&self) -> bool {
        self.arrival_wait.is_some()
    }

    fn apply_image_arrivals(&mut self, keys: &[String]) {
        for r in [self.renderer.as_mut(), self.tabbar_renderer.as_mut()].into_iter().flatten() {
            r.invalidate_offscreen_caches();
        }
        let vp = self.viewport_height();
        let scroll = self.dmg.layout_scroll.unwrap_or_else(|| self.scroll.get_position());
        let plan = match self.renderer.as_mut() {
            Some(_) if self.dmg.no_damage => ImageArrival::Full("MINI_NO_DAMAGE".into()),
            Some(r) => r.image_arrival_damage(keys, Some((scroll, vp))),
            None => ImageArrival::Full("没有渲染器".into()),
        };
        if self.dmg.log {
            crate::compositor::diag::emit(&format!("🖼 图到了 {} 张 → {plan:?}", keys.len()));
        }
        // 页面树里没人用的那几张：被盖住那一页发起的下载（歌单封面在跳到播放页之后才到）屏上看不见，
        // 那一页留着的画布已经随 bump_content_epoch 作废，返回时重画。tabBar 与宿主浮层（toast 的图标、
        // 动作面板、预览）用到的说不准画在哪：照旧整帧
        let plan = match plan {
            ImageArrival::Rects { missing, .. }
                if missing > 0
                    && (keys.iter().any(|k| k.starts_with('\u{0}'))
                        || self.tabbar_renderer.as_ref().is_some_and(|t| t.uses_any_image(keys))
                        || self.toast.as_ref().is_some_and(|t| t.visible)
                        || self.loading.as_ref().is_some_and(|l| l.visible)
                        || super::ui_overlay::sheet_overlay_visible()
                        || self.picker_sheet.is_some()) =>
            {
                ImageArrival::Full(format!("{missing} 张不在页面树里（tabBar / 宿主浮层 / 表满过）"))
            }
            p => p,
        };
        match plan {
            ImageArrival::Rects { page, fixed, .. } => {
                PARTIAL.fetch_add(1, Ordering::Relaxed);
                if let Some(rect) = page {
                    self.dmg.extra = Some(match self.dmg.extra {
                        None => rect,
                        Some(p) => {
                            let (x0, y0) = (p.x.min(rect.x), p.y.min(rect.y));
                            let (x1, y1) = ((p.x + p.width).max(rect.x + rect.width), (p.y + p.height).max(rect.y + rect.height));
                            crate::Rect::new(x0, y0, x1 - x0, y1 - y0)
                        }
                    });
                }
                if fixed {
                    self.dmg.press_fixed = true;
                }
            }
            ImageArrival::Relayout => {
                // widthFix / heightFix：图的比例定框的大小，布局缓存（数据没变）不会自己重排
                for r in [self.renderer.as_mut(), self.tabbar_renderer.as_mut()].into_iter().flatten() {
                    r.invalidate_layout();
                }
                self.page_data_dirty = true;
                self.mark_full();
            }
            ImageArrival::Full(_) => {
                self.page_data_dirty = true;
                self.mark_full();
            }
        }
    }
}
