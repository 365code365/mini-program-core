//! 被盖住那一页的页面画布与覆盖层留着，返回时放回去，不再整页重画。
//!
//! 为什么要它：navigateTo 之后新页用的是同一块页面画布 / 覆盖层，返回时上一页只能整页重画 ——
//! 云听音乐从播放页返回歌单页那一帧，一加 PLY110 上 46~54ms（画 27~29、覆盖层 4~9、合成 9，传 29MB），
//! 正好压在返回转场的第一步上（真机 damage_log，2026-11-06）。渲染器（布局缓存）本来就留着（SavedView），
//! 画布也留着的话，返回那一帧只剩失效分析：页面被盖着时 setData 改了什么（播放进度、迷你条），就只重画那几块。
//!
//! 只留**最近一次**盖住的那一页（返回最常回的就是它；再往下的几层照旧整页重画）。内存多一份页面画布的条带
//! （条带以外的页已经还给系统，engine_memory.rs）与一份覆盖层。
//!
//! 放回去的前提（任何一条不成立就照旧整页重画）：
//! - 回来的正是留的那一页（[`SavedView::canvas_token`] 对得上：中间 redirectTo / reLaunch / switchTab 换过的都对不上）；
//! - 画布宽度没变（转屏、分栏改了尺寸）、渲染器还在（尺寸 / 安全区变了会丢掉渲染器）；
//! - 盖着的这段时间里没有「签名看不出来」的变化：canvas 画过、字体换了（[`MiniEngine::content_epoch`]）。
//!   后台下完的图不算：记下是哪几张（[`KeptCanvas::arrived`]），放回去之后只重画用到它们的那几块。
//!   从前图也作废留着的画布：播放页上新歌的封面一到，歌单页留着的画布就白留了，返回那一帧照旧整页 57ms（一加）
//!
//! 页面数据在盖着时变了（逻辑层照样 setData）：放回去之后照常走失效分析，只重画变了的。
//! `MINI_NO_KEEP_CANVAS=1` 关掉（对照用）。
use super::engine::MiniEngine;
use super::page_keep::SavedView;
use crate::compositor::{LayerCanvas, PageCanvas};
use crate::renderer::wxml_renderer::EventBinding;
use crate::{Canvas, Rect as GeoRect};
use std::collections::HashMap;

/// 留着的那一页的画布与局部重画的账
pub(crate) struct KeptCanvas {
    token: u64,
    epoch: u64,
    canvas: PageCanvas,
    fixed_canvas: LayerCanvas,
    drawn_band: Option<(f32, f32)>,
    fixed_rows: Option<(u32, u32)>,
    last_animated_bounds: Vec<GeoRect>,
    page_animates: bool,
    fixed_layer_animates: bool,
    fixed_anim_bounds: Vec<GeoRect>,
    cached_fixed_bindings: Vec<EventBinding>,
    cached_fixed_regions: Vec<GeoRect>,
    drawn_area_positions: HashMap<String, f32>,
    layout_scroll: Option<f32>,
    /// 盖着时后台下完的图（engine_image_arrival.rs）：放回去之后按这几张只重画用到它们的那几块
    pub(super) arrived: Vec<String>,
}

fn disabled() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("MINI_NO_KEEP_CANVAS").is_some())
}

impl MiniEngine {
    /// 盖住当前页（`cover_current_page` 里、视图刚留进 `v` 之后）：把画布换下来留着，新页拿一块同尺寸的新画布
    /// （calloc 来的，不碰就不占内存）
    pub(super) fn stash_page_canvas(&mut self, v: &mut SavedView) {
        self.kept_canvas = None;
        if disabled() || self.drawn_band.is_none() {
            return;
        }
        self.keep_token += 1;
        v.canvas_token = self.keep_token;
        let (w, h) = (self.canvas.read().width(), self.canvas.read().height());
        let (fw, fh) = (self.fixed_canvas.read().width(), self.fixed_canvas.read().height());
        let canvas = std::mem::replace(&mut self.canvas, PageCanvas::new(Canvas::new(w, h)));
        let fixed_canvas = std::mem::replace(&mut self.fixed_canvas, LayerCanvas::new(Canvas::new(fw, fh)));
        let d = &mut self.dmg;
        self.kept_canvas = Some(KeptCanvas {
            token: self.keep_token,
            epoch: self.content_epoch,
            canvas,
            fixed_canvas,
            drawn_band: self.drawn_band.take(),
            fixed_rows: self.fixed_rows.take(),
            last_animated_bounds: std::mem::take(&mut d.last_animated_bounds),
            page_animates: std::mem::take(&mut d.page_animates),
            fixed_layer_animates: std::mem::take(&mut d.fixed_layer_animates),
            fixed_anim_bounds: std::mem::take(&mut d.fixed_anim_bounds),
            cached_fixed_bindings: std::mem::take(&mut d.cached_fixed_bindings),
            cached_fixed_regions: std::mem::take(&mut d.cached_fixed_regions),
            drawn_area_positions: std::mem::take(&mut d.drawn_area_positions),
            layout_scroll: d.layout_scroll.take(),
            arrived: Vec::new(),
        });
        self.mem_kept = Default::default();
    }

    /// 回到 `token` 那一页（视图已经放回去）：画布能放回去就放回去，返回 true（调用方不再整页重画）
    pub(super) fn restore_page_canvas(&mut self, token: u64) -> bool {
        let Some(k) = self.kept_canvas.take() else { return false };
        let ok = token != 0
            && k.token == token
            && k.epoch == self.content_epoch
            && self.renderer.is_some()
            && k.canvas.read().width() == self.pixel_size().0
            && k.fixed_canvas.read().width() == self.pixel_size().0;
        if !ok {
            return false;
        }
        self.canvas = k.canvas;
        self.fixed_canvas = k.fixed_canvas;
        // 换代数：GPU 镜像上一次看到的是新页那块，按代数判出来要整块重传（不重画，只传）
        self.canvas.bump_gen();
        let _ = self.fixed_canvas.write();
        self.drawn_band = k.drawn_band;
        self.fixed_rows = k.fixed_rows;
        self.mem_kept = Default::default();
        let d = &mut self.dmg;
        d.last_animated_bounds = k.last_animated_bounds;
        d.page_animates = k.page_animates;
        d.fixed_layer_animates = k.fixed_layer_animates;
        d.fixed_anim_bounds = k.fixed_anim_bounds;
        d.cached_fixed_bindings = k.cached_fixed_bindings;
        d.cached_fixed_regions = k.cached_fixed_regions;
        d.drawn_area_positions = k.drawn_area_positions;
        d.layout_scroll = k.layout_scroll;
        d.press_drawn.clear();
        d.focus_drawn = None;
        if let Some(r) = self.renderer.as_mut() {
            r.set_animations_active(d.page_animates || d.fixed_layer_animates);
        }
        // 盖着时到的图：放回去的画布上它们还是占位，只重画用到它们的那几块（页面树里没人用的就不用画）
        if !k.arrived.is_empty() {
            self.images_arrived(&k.arrived);
        }
        true
    }

    /// 签名看不出来的内容变化（图片下完、canvas 画过、字体换了）：留着的画布作废
    pub(super) fn bump_content_epoch(&mut self) {
        self.content_epoch += 1;
    }
}
