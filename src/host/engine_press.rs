//! 点击态（`hover-class` / `:active` / button 的按下样式）的重画范围。
//!
//! 从前点击态一律走 `needs_redraw`（整页条带 + 覆盖层 + tabBar 全画）：
//! - 按下那一帧整页画一次（这时点击态还没出现，`hover-start-time` 之后才亮）；
//! - 等出现的那几十毫秒里，`press_feedback_pending` 让**每一帧**都整页画；
//! - 松手后 `hover-stay-time`（70ms）里又是每一帧整页画 —— 正好压在惯性的头几帧上。
//!
//! 120Hz 的中端机上整页一趟 20~30ms，一次滑动在按下与松手各卡一下（一加 PLY110 真机：
//! 滑动时帧间隔峰值 33~44ms）。现在每帧只比对「画成点击态的元素」这一组有没有变，
//! 变了只重画这几个元素自己的盒子（覆盖层里的元素就重画覆盖层），没变就什么都不画。
//!
//! 这里也放 [`MiniEngine::mark_full`]：「这一帧整页重画」的唯一入口，记下是哪一行要的。
//! 滑动里冒出一串 `整帧·结构` 时，`MINI_DAMAGE_LOG=1` 那一行末尾直接是源码位置 ——
//! 不用再一个个猜（这次就是在真机上猜了半天才发现是点击态）。
use super::engine::MiniEngine;
use crate::Rect as GeoRect;

/// 盒子四周多画一点：边框、圆角抗锯齿与按压态的 transform 可能出盒子一两个像素
const PAD_LOGICAL: f32 = 4.0;

impl MiniEngine {
    /// 这一帧整页重画（页面条带 + 覆盖层 + tabBar）。开着 `MINI_DAMAGE_LOG` 时记下调用位置
    #[track_caller]
    pub(crate) fn mark_full(&mut self) {
        self.needs_redraw = true;
        if self.dmg.log && self.dmg.full_why.is_none() {
            self.dmg.full_why = Some(std::panic::Location::caller());
        }
    }

    /// 点击态这一帧与上次画的不一样：记下要重画的区域（页面画布 / 覆盖层）
    pub(super) fn sync_press_visual(&mut self) {
        let now = self.interaction.pressed_visual();
        if now == self.dmg.press_drawn {
            return;
        }
        let old = std::mem::replace(&mut self.dmg.press_drawn, now.clone());
        let mut page: Option<GeoRect> = None;
        for id in old.iter().chain(now.iter()) {
            let Some((b, fixed)) = self.interaction.element_bounds(id) else {
                // 元素不在了（换页 / 重建）：不知道画在哪，整页
                self.mark_full();
                return;
            };
            if fixed {
                // 单独一个标记：同一帧 setData 的失效分析会改写 fixed_dirty（engine_damage.rs）
                self.dmg.press_fixed = true;
                continue;
            }
            let pad = PAD_LOGICAL;
            let r = GeoRect::new(
                (b.x - pad) * self.dpr,
                (b.y - pad) * self.dpr,
                (b.width + pad * 2.0) * self.dpr,
                (b.height + pad * 2.0) * self.dpr,
            );
            page = Some(match page {
                None => r,
                Some(p) => {
                    let (x0, y0) = (p.x.min(r.x), p.y.min(r.y));
                    let (x1, y1) = (p.right().max(r.right()), p.bottom().max(r.bottom()));
                    GeoRect::new(x0, y0, x1 - x0, y1 - y0)
                }
            });
        }
        if let Some(r) = page {
            self.dmg.extra = Some(match self.dmg.extra {
                None => r,
                Some(p) => {
                    let (x0, y0) = (p.x.min(r.x), p.y.min(r.y));
                    let (x1, y1) = (p.right().max(r.right()), p.bottom().max(r.bottom()));
                    GeoRect::new(x0, y0, x1 - x0, y1 - y0)
                }
            });
        }
    }
}
