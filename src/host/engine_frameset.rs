//! 分栏模式（微信的「frameset」，阔折叠屏展开 / PC 窗口拉宽）：窗口分成左右两栏，各显示一页。
//!
//! 与微信 PC 分栏同一套规则（https://developers.weixin.qq.com/miniprogram/dev/framework/view/frameset.html）：
//! - 页面栈只有一页时它在**左栏**，右栏是占位（包里有 `frameset/placeholder.png` 就居中画它）；
//! - 栈上有两页以上时，栈顶在**右栏**、它下面那一页在**左栏**；从右栏再 navigateTo，右栏那页挪到左栏、新页进右栏；
//!   返回则反过来；
//! - 页面尺寸变成一栏的宽（`windowWidth`、rpx、`wx.onWindowResize` 都按一栏算）。
//!
//! ## 怎么做的（第一步：左右两栏都画对，活页面照常交互）
//! 引擎内部仍只有一个**活页面**（栈顶）。分栏时引擎的逻辑宽度就是一栏的宽 —— 渲染、命中、增量重画、两个合成器
//! 全都不用改；宿主拿到的主画面就是活页面那一栏。另一栏（[side]）是一张整栏的图：
//! - 平时就是 `navigateTo` 盖住那一页时留的那张（侧滑返回用的同一张，`back_shots.last()`），零额外开销；
//! - 尺寸变了（开 / 关分栏、拖窗口）那张图对不上了：把下面那页**临时换成当前页**（只换引擎侧的渲染状态，
//!   逻辑层不动）重画一帧再换回来（[`MiniEngine::repaint_side`]）；
//! - 左栏的滚轮：同样临时换上去、按新的滚动位置重画（第一步只支持滚轮，点击在第二步）。
//!
//! 分栏时不做页面转场与侧滑返回（新页本来就出现在另一栏）。
//!
//! 宿主怎么摆：`frameset` 事件（`on`、`livePane` = "left"/"right"、`paneWidth`），主画面放在活页面那一栏，
//! [`MiniEngine::side_pixels_into`] 的图放另一栏；`FRAME_SIDE` 位表示这张图换了。
use super::engine::MiniEngine;
use crate::host::edge_back::PageShot;
use crate::host::engine_pixels::PixelFormat;
use serde_json::json;

/// 宽到多少（逻辑像素）自动分栏。阔折叠屏展开后约 680~760，PC 小程序窗口默认 390 左右
pub const DEFAULT_MIN_WIDTH: f32 = 600.0;

/// 分栏的开关方式（创建选项 `"frameset"`）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FramesetMode {
    /// 永不分栏
    Off,
    /// 窗口够宽（≥ `min_width`）就分栏，窄了退回单栏
    Auto { min_width: f32 },
}

#[derive(Default)]
pub struct Frameset {
    pub mode: Option<FramesetMode>,
    /// 此刻是否分栏
    pub active: bool,
    /// 宿主给的整个窗口宽（逻辑像素）；分栏时引擎的 `width` 是它的一半
    pub total_width: u32,
    /// 另一栏的图（整栏，0xRRGGBB）；None = 还没画 / 不分栏
    pub side: Option<PageShot>,
    /// 另一栏那页滚了：下一次 pump 按新位置重画（一帧最多一次，滚轮再密也不会每个事件画两页）
    pub stale: bool,
    /// 上一次报给宿主的 (on, 活页面在左栏, 一栏宽)
    pub reported: Option<(bool, bool, u32)>,
}

impl Frameset {
    /// 宿主给了整个窗口宽：该不该分栏，分栏时一栏多宽（逻辑像素）
    pub fn pane_width(&self, total: u32) -> (bool, u32) {
        match self.mode {
            Some(FramesetMode::Auto { min_width }) if total as f32 >= min_width => (true, total / 2),
            _ => (false, total),
        }
    }
}

/// 创建选项 → 分栏方式。`"frameset": "auto"`（或 true）开、`"off"`（或 false）关；不给时看 app.json：
/// 与微信一样要 `"resizable": true` 加 `"frameset": true` 才开
pub fn mode_from(opts: &serde_json::Value, app_json: &serde_json::Value) -> Option<FramesetMode> {
    let min_width = opts.get("framesetMinWidth").and_then(|v| v.as_f64()).map(|v| v as f32).unwrap_or(DEFAULT_MIN_WIDTH);
    let auto = Some(FramesetMode::Auto { min_width });
    match opts.get("frameset") {
        Some(serde_json::Value::Bool(true)) => auto,
        Some(serde_json::Value::Bool(false)) => Some(FramesetMode::Off),
        Some(serde_json::Value::String(s)) if s == "auto" || s == "on" => auto,
        Some(serde_json::Value::String(s)) if s == "off" => Some(FramesetMode::Off),
        _ => {
            let on = |k: &str| app_json.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
            (on("frameset") && on("resizable")).then_some(FramesetMode::Auto { min_width }).or(Some(FramesetMode::Off))
        }
    }
}

impl MiniEngine {
    /// 活页面在左栏（栈上只有一页）
    pub fn frameset_live_left(&self) -> bool {
        self.page_stack.len() <= 1
    }

    pub fn frameset_active(&self) -> bool {
        self.frameset.active
    }

    /// 要不要做页面转场：宿主开了 `routeAnimation` 且没分栏（分栏时新页本来就出现在另一栏）
    pub(super) fn route_motion(&self) -> bool {
        self.opts.route_animation && !self.frameset.active
    }

    /// 每次 pump（出帧之后）：另一栏该显示什么、要不要告诉宿主换摆法。
    /// 返回 (另一栏的图换了, 临时换过页 —— 活页面要整帧重画)
    pub(super) fn sync_frameset(&mut self) -> (bool, bool) {
        let fs = &self.frameset;
        let state = (fs.active, self.frameset_live_left(), self.width);
        if fs.reported != Some(state) {
            // 一直没分栏：不发（老宿主、窄窗口、拉宽了但没开分栏，一个事件都看不到）
            let silent = !state.0 && fs.reported.map_or(true, |r| !r.0);
            self.frameset.reported = Some(state);
            if !silent {
                self.events.push(json!({
                    "type": "frameset",
                    "on": state.0,
                    "livePane": if state.1 { "left" } else { "right" },
                    "paneWidth": state.2,
                    "totalWidth": self.frameset.total_width,
                }));
            }
        }
        if !self.frameset.active {
            let had = self.frameset.side.take().is_some();
            return (had, false);
        }
        let (w, h) = self.pixel_size();
        let fits = |s: &PageShot| s.width == w && s.height == h && !s.pixels.is_empty();
        let stale = std::mem::take(&mut self.frameset.stale);
        if !stale && self.frameset.side.as_ref().is_some_and(|s| fits(s) && s.id == self.side_source_id()) {
            return (false, false);
        }
        let mut repainted = false;
        let side = if self.frameset_live_left() {
            self.placeholder_shot(w, h)
        } else {
            if let Some(s) = self.back_shots.last_mut() {
                s.unpack();
            }
            match self.back_shots.last() {
                Some(s) if !stale && fits(s) => PageShot { id: s.id, ..PageShot::new(w, h, s.pixels.clone()) },
                // 留的图对不上（开分栏 / 拖窗口改了宽）或那页滚了：把下面那页临时换上来重画一张
                _ => {
                    repainted = true;
                    match self.repaint_side() {
                        Some(s) => s,
                        None => self.placeholder_shot(w, h),
                    }
                }
            }
        };
        self.frameset.side = Some(side);
        (true, repainted)
    }

    /// 另一栏图的来源编号：栈顶下面那页的留图（单页时是 0 = 占位）
    fn side_source_id(&self) -> u64 {
        if self.frameset_live_left() {
            0
        } else {
            self.back_shots.last().map(|s| s.id).unwrap_or(0)
        }
    }

    /// 占位栏：页面底色上居中画 `frameset/placeholder.png`（没有就一片底色）
    fn placeholder_shot(&self, w: u32, h: u32) -> PageShot {
        let bg = 0x00F7_F7F7u32;
        let mut px = vec![bg; (w as usize) * (h as usize)];
        if let Some((rgba, iw, ih)) = crate::renderer::components::image_rgba("/frameset/placeholder.png")
            .filter(|_| crate::assets::resolve("/frameset/placeholder.png").is_some())
        {
            // 按 1 个图片像素 = 1 个逻辑像素摆（与 `<image>` 默认一样），超出一栏就等比缩小
            let scale = (self.dpr).min(w as f32 / iw as f32).min(h as f32 / ih as f32);
            let (dw, dh) = ((iw as f32 * scale) as u32, (ih as f32 * scale) as u32);
            let (x0, y0) = ((w - dw) / 2, (h - dh) / 2);
            for y in 0..dh {
                for x in 0..dw {
                    let (sx, sy) = ((x as f32 / scale) as u32, (y as f32 / scale) as u32);
                    let i = ((sy * iw + sx) * 4) as usize;
                    let a = rgba[i + 3] as u32;
                    let blend = |c: u32, b: u32| (c * a + b * (255 - a)) / 255;
                    let p = blend(rgba[i] as u32, 0xF7) << 16 | blend(rgba[i + 1] as u32, 0xF7) << 8 | blend(rgba[i + 2] as u32, 0xF7);
                    px[((y0 + y) * w + x0 + x) as usize] = p;
                }
            }
        }
        PageShot { id: 0, ..PageShot::new(w, h, px) }
    }

    /// 把栈顶下面那一页临时当成当前页画一帧（整帧：导航栏、页面、tabBar），画完原样换回。
    ///
    /// 只换引擎侧的东西（页面栈的最后一项、渲染器 / 滚动 / 交互状态、页面数据）；逻辑层的页面栈不动，
    /// 所以不会有 onShow / onHide。画的时候不上 GPU（`gpu` 先拿走）：这一帧只进合成缓冲，不能出现在屏上
    pub(super) fn repaint_side(&mut self) -> Option<PageShot> {
        let n = self.page_stack.len();
        if n < 2 || self.keep.saved.len() < n - 1 {
            return None;
        }
        let t0 = std::time::Instant::now();
        // ── 换上去 ──
        let top = self.page_stack.pop()?;
        let mut saved = self.keep.saved.pop()?;
        let vp = self.viewport_height();
        let live = super::page_keep::SavedView::take(&mut self.scroll, &mut self.renderer, &mut self.interaction, vp);
        let live_data = std::mem::replace(&mut self.page_data, saved.data.take().unwrap_or_default());
        let live_dirty = std::mem::replace(&mut self.page_data_dirty, false);
        #[cfg(feature = "gpu")]
        let gpu = self.gpu.take();
        let side_data = self.page_data.clone();
        saved.restore(&mut self.scroll, &mut self.renderer, &mut self.interaction);
        if self.renderer.is_none() {
            self.update_renderers();
        }
        // 留着的视图按旧宽度：滚动位置夹回新内容高里，画布按内容高重分
        self.mark_full();
        self.drawn_band = None;
        self.dmg.fixed_dirty = true;
        self.render();
        let shot = self.snapshot_frame();
        // ── 换回来 ──
        let vp_side = self.viewport_height();
        let mut back = super::page_keep::SavedView::take(&mut self.scroll, &mut self.renderer, &mut self.interaction, vp_side);
        back.data = Some(side_data);
        self.keep.saved.push(back);
        self.page_stack.push(top);
        live.restore(&mut self.scroll, &mut self.renderer, &mut self.interaction);
        if self.renderer.is_none() {
            self.update_renderers();
        }
        self.page_data = live_data;
        self.page_data_dirty = live_dirty;
        #[cfg(feature = "gpu")]
        {
            self.gpu = gpu;
        }
        self.mark_full();
        self.drawn_band = None;
        self.dmg.fixed_dirty = true;
        self.needs_redraw = true;
        self.pump_notes.add("分栏：重画另一栏", Some(t0));
        let shot = shot?;
        // 留进侧滑那一叠（返回时这页从留图上出来，宽度也对了）
        if let Some(last) = self.back_shots.last_mut() {
            *last = PageShot::new(shot.width, shot.height, shot.pixels.clone());
            return Some(PageShot { id: last.id, ..shot });
        }
        Some(shot)
    }

    /// 另一栏上的滚轮 / 触控板（逻辑像素，与 `wheel` 同号）：滚它、重画那一栏
    pub fn side_wheel(&mut self, delta_y: f32, precise: bool) {
        if !self.frameset.active || self.frameset_live_left() {
            return;
        }
        let Some(saved) = self.keep.saved.last_mut() else { return };
        let before = saved.scroll.get_position();
        saved.scroll.handle_scroll(delta_y, precise);
        // 滚轮的越界橡皮筋在另一栏不做（没有逐帧回弹）：夹在内容里
        let max = saved.scroll.get_max_scroll();
        saved.scroll.set_position(saved.scroll.get_position().clamp(0.0, max.max(0.0)));
        saved.scroll.stop();
        if (saved.scroll.get_position() - before).abs() < 0.01 {
            return;
        }
        // 下一次 pump 重画（sync_frameset）
        self.frameset.stale = true;
        self.needs_redraw = true;
    }

    /// 另一栏的图，写进宿主缓冲（格式与 `write_pixels` 相同）。没有返回 0
    pub fn side_pixels_into(&mut self, dst: &mut [u8], stride: usize, format: PixelFormat) -> usize {
        let Some(s) = self.frameset.side.as_mut() else { return 0 };
        s.unpack();
        let (w, h) = (s.width as usize, s.height as usize);
        let row = w * 4;
        if w == 0 || h == 0 || s.pixels.len() != w * h || stride < row || dst.len() < stride * (h - 1) + row {
            return 0;
        }
        for (y, src) in s.pixels.chunks_exact(w).enumerate() {
            super::engine_pixels::write_row(&mut dst[y * stride..y * stride + row], src, format);
        }
        stride * (h - 1) + row
    }

    /// 另一栏的像素尺寸（不分栏为 (0, 0)）
    pub fn side_pixel_size(&self) -> (u32, u32) {
        self.frameset.side.as_ref().map(|s| (s.width, s.height)).unwrap_or((0, 0))
    }
}
