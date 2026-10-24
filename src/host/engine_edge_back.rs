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
        // 还没合成过（一打开就跳页）：没有图，侧滑时露出暗底（与桌面一致）
        let shot = self.snapshot_frame().unwrap_or(PageShot::empty());
        // 只有最后一张会被用到（侧滑 / 返回转场），之前那张压起来（shot_pack.rs，一张 10MB → 几百 KB）
        if let Some(prev) = self.back_shots.last_mut() {
            prev.pack();
        }
        self.back_shots.push(shot);
        if self.back_shots.len() > MAX_SHOTS {
            self.back_shots.remove(0);
        }
    }

    /// 上一次合成好的整帧（含导航栏、tabBar、浮层）。还没合成过时为 None。
    /// 留图（侧滑）与返回转场（离开的那页）都用它
    pub(super) fn snapshot_frame(&mut self) -> Option<PageShot> {
        #[cfg(feature = "gpu")]
        self.sync_cpu_frame();
        let (w, h) = self.pixel_size();
        if self.compose.len() != (w as usize) * (h as usize) || self.compose.is_empty() {
            return None;
        }
        // 直接拿走合成缓冲（不再拷一份 10MB）：下一帧合成时重新分配、整帧写满。
        // GPU 上屏时这块缓冲只为留图才合成，拿走之后就不再常驻
        self.rgba_stale = true;
        Some(PageShot::new(w, h, std::mem::take(&mut self.compose)))
    }

    /// 退栈之后：新的最后一张要能直接显示，解开它（shot_pack.rs）
    pub(super) fn unpack_last_shot(&mut self) {
        if let Some(s) = self.back_shots.last_mut() {
            s.unpack();
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
        // 按帧时刻推进（frame_clock.rs），收尾动画每帧挪得均匀
        let now = crate::frame_clock::now();
        let dt = self.edge_back_tick.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(0.016).min(0.05);
        self.edge_back_tick = Some(now);
        match eb.tick(dt) {
            None => true,
            Some(commit) => {
                self.edge_back = None;
                self.edge_back_tick = None;
                if commit {
                    // 页面已经跟着手指滑出去了：不再转场
                    self.back_with(false);
                }
                // 收手这一帧整帧：位移归零，画布内容也可能换了页
                self.mark_full();
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
            shot_on_top: false,
        })
    }

    /// 本帧上屏时当前页整帧右移了多少（视口逻辑像素，没有侧滑为 0）。
    ///
    /// 宿主承载的原生组件（`nativeViews`）不在引擎画面里，`edge_back::compose` 搬像素时
    /// 搬不到它们，只能把这段位移报给宿主让它自己挪。取整必须与上面 `edge_back_frame`
    /// 的 `offset_px` 一致（设备像素整数，合成器再夹到帧宽），否则控件和引擎画面之间
    /// 会错开半个像素的缝。
    pub(super) fn edge_back_shift(&self) -> f32 {
        let Some(eb) = self.edge_back.as_ref() else { return 0.0 };
        let (pw, _) = self.pixel_size();
        let px = ((eb.offset() * self.dpr).round().max(0.0) as u32).min(pw);
        px as f32 / self.dpr
    }
}
