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
    /// 栈顶两页对调着：左栏那页是活页面、右栏那页留成图（engine_frameset_focus.rs）
    pub swapped: bool,
    /// 正在临时换上来画另一栏（[`MiniEngine::repaint_side`]）：Some(那一栏在左)。胶囊只画在右栏
    pub painting_left: Option<bool>,
    /// 另一栏那页滚了：下一次 pump 按新位置重画（一帧最多一次，滚轮再密也不会每个事件画两页）
    pub stale: bool,
    /// 上一次报给宿主的 (on, 活页面在左栏, 一栏宽, 另一栏有页面)
    pub reported: Option<(bool, bool, u32, bool)>,
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
    /// 活页面在左栏：栈上只有一页，或者点了左栏把栈顶两页对调了（engine_frameset_focus.rs）
    pub fn frameset_live_left(&self) -> bool {
        self.page_stack.len() <= 1 || self.frameset.swapped
    }

    /// 另一栏显示的是一页（栈上两页以上），不是占位
    fn side_has_page(&self) -> bool {
        self.page_stack.len() >= 2
    }

    pub fn frameset_active(&self) -> bool {
        self.frameset.active
    }

    /// 要不要做页面转场（宿主开了 `routeAnimation`）。分栏时只在**右栏里**做：进页前 / 返回后右栏都有页
    /// （栈上 ≥ 3 页 —— 进页时在入栈之后判，返回时在出栈之前判），新页从右栏右边滑进来、盖住的是右栏原来那页，
    /// 与单栏同一套（下面那张就是 `back_shots.last()`）；左栏那张直接换。栈上两页时活页面要在两栏之间挪
    /// （单页 → 右栏打开第一页、返回到只剩一页），没有「下面那一页」可露，一换就到
    pub(super) fn route_motion(&self) -> bool {
        self.opts.route_animation && (!self.frameset.active || (self.page_stack.len() >= 3 && !self.frameset.swapped))
    }

    /// 活页面在页面栈里是第几层（返回箭头按它）。分栏对调着时活页面是栈顶往下那一页（engine_frameset_focus.rs）：
    /// 左栏的首页不该有返回箭头
    pub(super) fn page_depth_of_live(&self) -> usize {
        self.page_stack.len() - usize::from(self.frameset.swapped && self.page_stack.len() >= 2)
    }

    /// 给另一栏留图的这一帧不画浮层（Toast / loading / Modal / picker）：浮层属于整个小程序、跟着活页面走，
    /// 冻进留图的话左栏会一直顶着一个 Toast
    pub(super) fn overlays_hidden(&self) -> bool {
        self.frameset.painting_left.is_some()
    }

    /// 要挪到另一栏（`left` = 左栏）的这页留图前要不要按「另一栏」重画一帧：带着该去掉的胶囊或浮层时要
    pub(super) fn side_shot_needs_repaint(&self, left: bool) -> bool {
        self.frameset.active
            && ((left && self.capsule_spec_raw().is_some() && self.capsule_in_this_pane())
                || !self.interaction.pressed_visual().is_empty()
                || self.toast.is_some()
                || self.loading.is_some()
                || self.modal.is_some()
                || self.picker_sheet.is_some())
    }

    /// 胶囊只在窗口右上角（微信 PC 分栏：左栏的导航栏上没有胶囊，右栏是占位时胶囊也在右栏那里）
    pub(super) fn capsule_in_this_pane(&self) -> bool {
        !self.frameset.active || !self.frameset.painting_left.unwrap_or_else(|| self.frameset_live_left())
    }

    /// 这一帧不上 GPU、走 CPU 合成交给宿主：分栏期间一直这样，外加开 / 关分栏的那一帧。
    ///
    /// 为什么分栏时不用 GPU：分栏里一次点击要换两次摆法（按下时左栏换成活页面、松手打开新页又回到右栏），
    /// 活页面所在的图层要跟着挪。GPU 是引擎线程直接画进 Metal 层的，宿主挪层在主线程、晚一拍；
    /// 换回 GPU 时把 Metal 层重新显示出来，屏上先露出的是它**上一张** drawable（另一页的画面）——
    /// 实测（2026-10-26 灵犀 for Mac，聊天列表点会话）每点一下闪两次。走 CPU 时新画面、另一栏的图、
    /// 挪层在宿主的同一次提交里，一下到位。一栏只有半个窗口宽，CPU 合成一帧的代价也小一半
    pub(super) fn frameset_holds_gpu(&self) -> bool {
        self.frameset.active || self.frameset_layout_pending()
    }

    /// 这一帧画完之后摆法要变（活页面换栏、开 / 关分栏）：宿主还没挪图层，GPU 直接画上屏的话新画面会在旧位置上
    /// 闪一帧（两栏一抖）。这一帧改走 CPU 合成，跟 `frameset` 事件一起交给宿主（宿主在同一次提交里挪层、换图）
    pub(super) fn frameset_layout_pending(&self) -> bool {
        let fs = &self.frameset;
        let state = (fs.active, self.frameset_live_left(), self.width, self.side_has_page());
        fs.reported != Some(state) && !(!state.0 && fs.reported.map_or(true, |r| !r.0))
    }

    /// 每次 pump（出帧之后）：另一栏该显示什么、要不要告诉宿主换摆法。
    /// 返回 (另一栏的图换了, 临时换过页 —— 活页面要整帧重画)
    pub(super) fn sync_frameset(&mut self) -> (bool, bool) {
        if self.page_stack.len() < 2 {
            self.frameset.swapped = false; // 对调的那两页里有一页没了（switchTab / reLaunch 清了栈）
        }
        let fs = &self.frameset;
        let state = (fs.active, self.frameset_live_left(), self.width, self.side_has_page());
        if fs.reported != Some(state) {
            // 一直没分栏：不发（老宿主、窄窗口、拉宽了但没开分栏，一个事件都看不到）
            let silent = !state.0 && fs.reported.map_or(true, |r| !r.0);
            self.frameset.reported = Some(state);
            if !silent {
                // 这一帧是 CPU 合成的（frameset_layout_pending）：下一次 pump 补一次 GPU 上屏
                #[cfg(feature = "gpu")]
                if self.gpu.is_some() {
                    self.present_pending = true;
                }
                self.events.push(json!({
                    "type": "frameset",
                    "on": state.0,
                    "livePane": if state.1 { "left" } else { "right" },
                    "paneWidth": state.2,
                    "totalWidth": self.frameset.total_width,
                    // 另一栏是一页（点它要 `frameset_focus`）还是占位
                    "sidePage": state.3,
                    // 另一栏的图上画着窗口的胶囊（右栏是占位）：宿主在这里接点击（左半 menu、右半 close）
                    "sideCapsule": self.side_capsule_rect().map(|r| json!({"x": r.x, "y": r.y, "width": r.width, "height": r.height})),
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
        let side = if !self.side_has_page() {
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
        if !self.side_has_page() {
            0
        } else {
            self.back_shots.last().map(|s| s.id).unwrap_or(0)
        }
    }

    /// 占位栏：页面底色上居中画 `frameset/placeholder.png`（没有就一片底色）
    fn placeholder_shot(&self, w: u32, h: u32) -> PageShot {
        let bg = 0x00F7_F7F7u32;
        let mut px = vec![bg; (w as usize) * (h as usize)];
        // 活页面在左栏时窗口的胶囊落在右栏（占位）上
        if let Some(r) = self.side_capsule_rect() {
            let mut c = crate::Canvas::new(w, h);
            c.clear(crate::Color::new(0xF7, 0xF7, 0xF7, 255));
            crate::host::capsule::draw(&mut c, r, self.dpr, false, None);
            for (d, s) in px.iter_mut().zip(c.pixels()) {
                *d = (s.r as u32) << 16 | (s.g as u32) << 8 | s.b as u32;
            }
        }
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
        // 另一栏在左还是右（胶囊只画在右栏，capsule_in_this_pane）
        self.frameset.painting_left = Some(!self.frameset_live_left());
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
        self.frameset.painting_left = None;
        self.pump_notes.add("分栏：重画另一栏", Some(t0));
        let shot = shot?;
        // 留进侧滑那一叠（返回时这页从留图上出来，宽度也对了）
        if let Some(last) = self.back_shots.last_mut() {
            *last = PageShot::new(shot.width, shot.height, shot.pixels.clone());
            return Some(PageShot { id: last.id, ..shot });
        }
        Some(shot)
    }

    /// 当前页按「在另一栏」整帧画进合成缓冲，不上屏（留图用，宿主看不到这一帧）：不画浮层，
    /// `left` 时也不画胶囊
    pub(super) fn frameset_render_for_side(&mut self, left: bool) {
        let t0 = std::time::Instant::now();
        #[cfg(feature = "gpu")]
        let gpu = self.gpu.take();
        self.frameset.painting_left = Some(left);
        // 点击态（hover-class，松手后还留 hover-stay-time）不进留图：点会话打开聊天时，那一行的按压底色
        // 会被冻在左栏的图里一直亮着，下次点别的会话换栏时又一下灭掉 —— 每点一下整行闪一次
        self.interaction.cancel_press();
        self.mark_full();
        self.drawn_band = None;
        self.dmg.fixed_dirty = true;
        self.render();
        self.frameset.painting_left = None;
        #[cfg(feature = "gpu")]
        {
            self.gpu = gpu;
        }
        self.mark_full();
        self.dmg.fixed_dirty = true;
        self.pump_notes.add("分栏：按另一栏重画留图", Some(t0));
    }

    /// 右栏是占位时画在它上面的胶囊（一栏内的逻辑坐标）。没开胶囊 / 不是这种摆法时 None
    fn side_capsule_rect(&self) -> Option<crate::Rect> {
        (self.opts.menu_button && self.frameset.active && !self.side_has_page() && self.capsule_spec_raw().is_some())
            .then(|| crate::host::capsule::rect(&self.page_system_info()))
    }

    /// 另一栏上的滚轮 / 触控板（逻辑像素，与 `wheel` 同号）：滚它、重画那一栏
    pub fn side_wheel(&mut self, delta_y: f32, precise: bool) {
        if !self.frameset.active || !self.side_has_page() {
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
