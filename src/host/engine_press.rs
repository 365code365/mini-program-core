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
                self.note_fixed_rect(b);
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

    /// 聚焦的输入框换了（点了另一个、失焦、`focus` 属性 / `wx.hideKeyboard` 改的）：旧框、新框各重画一次。
    ///
    /// 光标与「聚焦态」画在输入框自己的盒子里，局部重画只重画变了的那块 —— 从前只有点击那条路
    /// 在点之后读「旧框」，而那时焦点已经换到新框，旧框没人重画：两个输入框各留一个光标
    /// （兴趣圈发帖页，标题框失焦后光标还在）。每帧对一遍上次画的是哪个框，哪条路改的焦点都接得住
    pub(super) fn sync_focus_visual(&mut self) {
        let now = self.interaction.focused_input.as_ref().map(|f| (f.id.clone(), f.bounds, f.is_fixed));
        let same = match (&now, &self.dmg.focus_drawn) {
            (None, None) => true,
            (Some(a), Some(b)) => a.0 == b.0 && a.1 == b.1 && a.2 == b.2,
            _ => false,
        };
        if same {
            return;
        }
        let old = std::mem::replace(&mut self.dmg.focus_drawn, now.clone());
        for (_, b, fixed) in old.into_iter().chain(now) {
            self.damage_input_box(b, fixed);
        }
    }

    /// 聚焦输入框的光标翻了一次（亮 ↔ 灭）：只重画这个输入框的盒子（覆盖层里的就重画覆盖层）。
    ///
    /// 从前这里 `mark_full`：键盘弹着的整段时间里每 500ms 整条带 + 覆盖层 + tabBar 重画一次，
    /// 正好落在边打字边滑的那几帧上。光标画在输入框自己的盒子里（`FocusedInput::bounds`，
    /// 页面内容坐标），重画这一块与整帧逐字节相同。
    ///
    /// scroll-view 里的输入框：重画这一块时 scroll-view 照旧从离屏缓存贴回 —— 与从前整帧时一样
    /// （`mark_full` 也不作废离屏缓存），行为不变
    pub(super) fn damage_caret(&mut self) {
        let Some((b, fixed)) = self.interaction.focused_input.as_ref().map(|f| (f.bounds, f.is_fixed)) else { return };
        self.damage_input_box(b, fixed);
    }

    /// 只重画一个输入框的盒子（页面内容坐标的逻辑像素；覆盖层里的就重画覆盖层）。光标翻转、聚焦 / 失焦换框都走这里
    pub(super) fn damage_input_box(&mut self, b: GeoRect, fixed: bool) {
        if b.width <= 0.0 || b.height <= 0.0 {
            // 还没排出位置（刚聚焦、首帧）：不知道画在哪，整页
            return self.mark_full();
        }
        if fixed {
            self.dmg.press_fixed = true;
            self.note_fixed_rect(b);
            return;
        }
        let pad = PAD_LOGICAL;
        let d = self.dpr;
        let r = GeoRect::new((b.x - pad) * d, (b.y - pad) * d, (b.width + pad * 2.0) * d, (b.height + pad * 2.0) * d);
        self.dmg.extra = Some(match self.dmg.extra {
            None => r,
            Some(p) => {
                let (x0, y0) = (p.x.min(r.x), p.y.min(r.y));
                let (x1, y1) = (p.right().max(r.right()), p.bottom().max(r.bottom()));
                GeoRect::new(x0, y0, x1 - x0, y1 - y0)
            }
        });
    }

    /// 覆盖层里要重画的一块（覆盖层画布是视口坐标，逻辑像素 → 设备像素，外扩 PAD_LOGICAL 盖住阴影 / 抗锯齿）
    fn note_fixed_rect(&mut self, b: GeoRect) {
        let (pad, d) = (PAD_LOGICAL, self.dpr);
        let r = GeoRect::new((b.x - pad) * d, (b.y - pad) * d, (b.width + pad * 2.0) * d, (b.height + pad * 2.0) * d);
        self.dmg.press_fixed_rect = Some(match self.dmg.press_fixed_rect {
            None => r,
            Some(p) => {
                let (x0, y0) = (p.x.min(r.x), p.y.min(r.y));
                let (x1, y1) = (p.right().max(r.right()), p.bottom().max(r.bottom()));
                GeoRect::new(x0, y0, x1 - x0, y1 - y0)
            }
        });
    }
}
