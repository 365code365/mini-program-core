//! 图片预览（`wx.previewImage` / `wx.previewMedia`）盖满整屏时的一帧。
//!
//! 与全屏视频（`engine_video_feed.rs::fullscreen_frame`）同一个道理：黑底不透明的时候页面一个像素也看不见，
//! 该重画的（动图换帧、CSS 动画、setData）标记都留着，预览开始变透明（下拉 / 关闭）那一帧照常画。
//! 从前预览底下「媒体」页的动图每 120ms 换一帧，每次都局部重画页面 + 把页面镜像传给 GPU，
//! 一加 PLY110 上那一帧 14~16ms（只合成预览的帧 7ms），左右滑的时候就是一下一下地顿。
use super::engine::MiniEngine;
use crate::host::ui_overlay::{self, image_preview};

impl MiniEngine {
    /// 预览盖满整屏：只在预览自己变了时重新合成，返回是否出了新画面；没盖满给 None（照常走局部重画）
    pub(super) fn preview_frame(&mut self) -> Option<bool> {
        if !image_preview::opaque() {
            return None;
        }
        let compose = std::mem::take(&mut self.dmg.compose_dirty) || self.present_pending || ui_overlay::sheet_overlay_animating();
        if !compose {
            self.dmg.last_kind = super::engine_damage::FrameKind::Idle;
            return Some(false);
        }
        let t0 = self.dmg.log.then(std::time::Instant::now);
        self.dmg.last_kind = super::engine_damage::FrameKind::Compose;
        self.compose_current();
        if let Some(t0) = t0 {
            let gpu = self.dmg.gpu_note.take().unwrap_or_default();
            crate::compositor::diag::emit(&format!("🩹 [SDK] 合成·图片预览 {gpu}  {:.1}ms", t0.elapsed().as_secs_f32() * 1000.0));
        }
        Some(true)
    }
}
