//! SDK 引擎的左边缘侧滑返回：留图、推进收尾动画、合成参数。
//!
//! 判定与合成与桌面窗体是同一份（`host::edge_back` + `host::input` 的手势仲裁 +
//! `compositor::cpu` 的 `edge_back` 层），这里只做离不开引擎状态的三件事（对应桌面的 `edge_back_host.rs`）：
//! - **留图**：`navigateTo` 盖住当前页的那一刻，把上一次合成好的整帧（`compose` 缓冲）留一份，
//!   侧滑时下面那一页就显示它（那时它已经不是当前页，没法重画）；
//! - **收尾**：松手后逐帧推进动画，判定为「返回」时真的退栈（与 `wx.navigateBack()` 同一条路径）；
//! - **合成**：把位移 / 进度 / 上一页的图交给合成器。
//!
//! 从前 SDK 没有这个手势（`begin_gesture` 恒传「侧滑忙」把它关掉）：手机上二级页只能找页面上的返回键。
use super::engine::MiniEngine;
use crate::compositor::EdgeBackFrame;
use crate::host::edge_back::PageShot;

/// 留图的上限：微信页面栈最多 10 层
const MAX_SHOTS: usize = 10;

impl MiniEngine {
    /// 当前页将被新页盖住：留一张视口图（`navigateTo` 之前调）
    pub(super) fn capture_back_shot(&mut self) {
        #[cfg(feature = "gpu")]
        self.sync_cpu_frame();
        let (w, h) = self.pixel_size();
        if self.compose.len() != (w as usize) * (h as usize) || self.compose.is_empty() {
            // 还没合成过（一打开就跳页）：没有图，侧滑时露出暗底（与桌面一致）
            self.back_shots.push(PageShot { width: 0, height: 0, pixels: Vec::new() });
        } else {
            self.back_shots.push(PageShot { width: w, height: h, pixels: self.compose.clone() });
        }
        if self.back_shots.len() > MAX_SHOTS {
            self.back_shots.remove(0);
        }
    }

    /// 推进松手后的收尾动画；判定为「返回」且动画走完时退栈。返回是否还在动（要续帧 / 重新合成）
    pub(super) fn tick_edge_back(&mut self) -> bool {
        let Some(eb) = self.edge_back.as_mut() else {
            self.edge_back_tick = None;
            return false;
        };
        if !eb.is_settling() {
            // 还在跟手：帧由输入驱动
            self.edge_back_tick = None;
            return true;
        }
        let now = std::time::Instant::now();
        let dt = self.edge_back_tick.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.016).min(0.05);
        self.edge_back_tick = Some(now);
        match eb.tick(dt) {
            None => true,
            Some(commit) => {
                self.edge_back = None;
                self.edge_back_tick = None;
                if commit {
                    self.back();
                }
                // 收手这一帧整帧：位移归零，画布内容也可能换了页
                self.needs_redraw = true;
                true
            }
        }
    }

    /// 本帧合成要的侧滑参数（没有侧滑给 None）。只借用这两个字段（合成时 `gpu` / `compose` 还要可变借用）
    pub(super) fn edge_back_frame<'a>(
        edge_back: &Option<crate::host::edge_back::EdgeBack>,
        back_shots: &'a [PageShot],
        dpr: f32,
    ) -> Option<EdgeBackFrame<'a>> {
        let eb = edge_back.as_ref()?;
        Some(EdgeBackFrame {
            offset_px: (eb.offset() * dpr).round().max(0.0) as u32,
            shot: back_shots.last().filter(|s| !s.pixels.is_empty()),
            progress: eb.progress(),
        })
    }
}
