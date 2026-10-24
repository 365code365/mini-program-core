//! swiper 换页动画 / 自动播放：只重画动了的那几个 swiper 的盒子。
//!
//! 从前 `advance_due_swipers()` 一返回「有 swiper 动了」就 `mark_full`：swiper 页上每次自动翻页的 0.5s 里
//! 每帧整页重画（桌面基准 `sdk_scroll_bench pages/swiper/swiper` 惯性那段 94 帧 `整帧·结构`，手机上一帧 20ms 往上）。
//! swiper 的偏移只在绘制期用到，布局不变：重画它自己那块就够了，与点击态同一条路（`dmg.extra` 并进局部重画）。
//!
//! 盒子取渲染器记下的「最近一次画在哪」（`WxmlRenderer::known_swiper`，跨帧留着）。找不到（不在当前页、没画过）
//! 就不画（屏上看不见它）；在覆盖层（fixed）里的 swiper 重画覆盖层。
use super::engine::MiniEngine;
use crate::Rect as GeoRect;

/// 盒子外多画一圈（逻辑像素）：抗锯齿的边、阴影溢出一点点
const PAD_LOGICAL: f32 = 2.0;

impl MiniEngine {
    /// 推进 swiper；动了的只记它们的盒子为这一帧要重画的区域
    pub(super) fn advance_swipers(&mut self) {
        if !crate::renderer::components::advance_due_swipers() {
            return;
        }
        let ids = crate::renderer::components::take_moved_swipers();
        let Some(r) = self.renderer.as_ref() else { return self.mark_full() };
        let mut page: Option<GeoRect> = None;
        for id in &ids {
            // 当前页没画过这个 swiper：它在**别的页**上（swiper 状态是进程级的，被盖住的那一页的
            // 自动播放照样在翻），或者在视口 ± 余量之外。两种情况屏上都看不见它，不用重画。
            // 从前这里退回整页重画：首页轮播每 3.2s 翻一次，打开详情页之后详情页每 3.7s 一个 60~72ms
            // 的整帧、接着 0.5s 的换页动画里每帧 21ms 整帧（一加 PLY110 上传的诊断 20261024-150812，
            // 「详情上下滑动卡顿」）。当前页上新出现、还没画过的 swiper 会随这次结构变化整帧画出来
            let Some(region) = r.known_swiper(id) else { continue };
            if region.is_fixed {
                // 与点击态同一个标记：同一帧 setData 的失效分析会改写 fixed_dirty（engine_damage.rs）
                self.dmg.press_fixed = true;
                continue;
            }
            let b = region.bounds;
            let d = self.dpr;
            let rect = GeoRect::new((b.x - PAD_LOGICAL) * d, (b.y - PAD_LOGICAL) * d, (b.width + PAD_LOGICAL * 2.0) * d, (b.height + PAD_LOGICAL * 2.0) * d);
            page = Some(union(page, rect));
        }
        if let Some(rect) = page {
            self.dmg.extra = Some(union(self.dmg.extra, rect));
        }
    }
}

fn union(a: Option<GeoRect>, r: GeoRect) -> GeoRect {
    match a {
        None => r,
        Some(p) => {
            let (x0, y0) = (p.x.min(r.x), p.y.min(r.y));
            let (x1, y1) = (p.right().max(r.right()), p.bottom().max(r.bottom()));
            GeoRect::new(x0, y0, x1 - x0, y1 - y0)
        }
    }
}
