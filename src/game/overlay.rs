//! 系统覆盖层（game-doc/02 §7.4、04 G7.1，GT-U1）：Toast / Loading / Modal / ActionSheet 盖在游戏上。
//!
//! 状态、绘制、命中全部复用小程序那一套（`host::ui_overlay`）：JS 侧也是同一份 ui.js / system.js，
//! 请求走 `MiniApp::drain_ui_events` 与 `take_host_requests("actionSheet")`，按钮结果回 `__handleModalResult` / `__resolveActionSheet`。
//!
//! 输出：没有覆盖层时输出帧就是主画布本身（与从前逐字节相同）。有覆盖层时在引擎里合成：主画布按
//! `min(dpr, 2)` 放大到输出分辨率（文字才清楚；系统缩放器再拉到屏幕），覆盖层按同一个比例画上去。
//! 上限 2 倍：一加 PLY110（dpr 2.975）上 3 倍是 350 万像素一帧，覆盖层下面的游戏还在动，每帧都得重合成。
//!
//! 输入（02 §7.4 的优先级）：ActionSheet > 带 mask 的提示框 > Modal > 游戏。一串触摸在按下那一刻判归属，
//! 归了覆盖层的整串（移动、抬起）都不进游戏；按下时覆盖层还没弹出来的那串照常给游戏。
//!
//! 局限：ActionSheet 的状态在 `ui_overlay::action_sheet` 的线程局部里，同一线程上同时有小程序引擎与小游戏实例时两边共用一份。

use super::GameEngine;
use crate::host::ui_overlay::{self, LoadingState, ModalState, ToastState};

/// 合成分辨率相对逻辑尺寸的上限
const MAX_SCALE: f32 = 2.0;

#[derive(Default)]
pub struct Overlay {
    toast: Option<ToastState>,
    loading: Option<LoadingState>,
    modal: Option<ModalState>,
    /// 当前这串触摸归覆盖层（宿主指针 id）
    owner: Option<i32>,
    /// 上一帧画过覆盖层（收起之后要再出一帧干净的）
    drawn: bool,
    /// 覆盖层的动画时钟（秒，游戏时间轴：固定时钟下逐帧可复现）与这一帧的时刻（操作菜单的滑入按它取样）
    secs: f32,
    at: Option<std::time::Instant>,
}

impl Overlay {
    fn visible(&self) -> bool {
        self.toast.as_ref().is_some_and(|t| t.visible)
            || self.loading.as_ref().is_some_and(|l| l.visible)
            || self.modal.as_ref().is_some_and(|m| m.visible)
            || ui_overlay::action_sheet::visible()
    }
}

/// 逻辑尺寸 → 合成用的 (宽, 高, 比例)
pub fn compose_size(w: u32, h: u32, dpr: f32) -> (u32, u32, f32) {
    let s = dpr.clamp(1.0, MAX_SCALE);
    (((w as f32) * s).round().max(1.0) as u32, ((h as f32) * s).round().max(1.0) as u32, s)
}

/// 0x00RRGGBB ↔ RGBA（覆盖层的缓冲格式见 `ui_overlay::draw`）
fn rgba_to_u32(src: &[u8], out: &mut Vec<u32>) {
    out.clear();
    out.extend(src.chunks_exact(4).map(|p| ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | p[2] as u32));
}

fn u32_to_rgba(src: &[u32], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(src.len() * 4);
    for &p in src {
        out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
    }
}

impl GameEngine {
    /// pump 里：取走这一帧的提示框 / 弹窗 / 菜单请求。返回覆盖层要不要重画（变了、在动、刚收起）
    pub(super) fn overlay_tick(&mut self, t_us: u64) -> bool {
        self.overlay.secs = (t_us as f64 / 1e6) as f32;
        self.overlay.at = Some(self.clock.borrow().instant_at(t_us));
        let mut changed = false;
        for e in self.app.drain_ui_events() {
            let o = &mut self.overlay;
            let _ = ui_overlay::apply_feedback_event(e, &mut o.toast, &mut o.loading, &mut o.modal);
            changed = true;
        }
        // 操作菜单的起止时刻、命中都在 frame_clock 上取：一律按游戏这一帧的时刻（固定时钟下逐帧可复现）
        crate::frame_clock::set(self.overlay.at);
        changed |= ui_overlay::action_sheet::take_requests(&mut self.app);
        crate::frame_clock::set(None);
        let anim = ui_overlay::action_sheet::animating()
            || ui_overlay::tip_needs_recompose(self.overlay.secs, &self.overlay.toast, &self.overlay.loading);
        let visible = self.overlay.visible();
        let buttons = std::mem::take(&mut self.buttons.borrow_mut().dirty);
        let visible = visible || self.buttons_visible();
        let redraw = changed || anim || buttons || visible != self.overlay.drawn;
        self.overlay.drawn = visible;
        redraw
    }

    /// 覆盖层弹着（输出帧要在引擎里合成，宿主上屏走 RGBA 那条路）
    pub fn overlay_visible(&self) -> bool {
        self.overlay.visible() || self.buttons_visible()
    }

    /// 还要不要接着出帧（菜单在动、圈在转）
    pub(super) fn overlay_animating(&self) -> bool {
        ui_overlay::action_sheet::animating() || ui_overlay::tip_spinning(&self.overlay.toast, &self.overlay.loading)
    }

    /// 主画布 RGBA（`w × h`，逻辑尺寸）+ 覆盖层 → 合成好的 RGBA 与它的尺寸
    pub(super) fn compose_overlay(&self, out: &mut Vec<u8>) -> (u32, u32) {
        let (lw, lh) = (self.width, self.height);
        let (w, h, sf) = compose_size(lw, lh, self.dpr);
        let base = self.scaled_unrotated(w, h);
        let mut buf = Vec::new();
        if self.buttons_visible() {
            // 原生按钮在游戏之上、系统覆盖层之下（buttons.rs）
            let mut c = crate::Canvas::new(w, h);
            for (d, s) in c.pixels_mut().iter_mut().zip(base.chunks_exact(4)) {
                *d = crate::Color::new(s[0], s[1], s[2], 255);
            }
            self.draw_buttons(&mut c, sf);
            buf.extend(c.pixels().iter().map(|p| ((p.r as u32) << 16) | ((p.g as u32) << 8) | p.b as u32));
        } else {
            rgba_to_u32(&base, &mut buf);
        }
        let text = crate::text::shared_fonts();
        let tr = text.as_deref();
        let rows = (0, h);
        let o = &self.overlay;
        // 按这一帧的时刻画（操作菜单的滑入动画在 frame_clock 上取样），画完还原
        crate::frame_clock::set(o.at);
        ui_overlay::render_sheet_overlays(&mut buf, w, h, sf, tr, rows);
        ui_overlay::render_ui_overlay(&mut buf, w, h, sf, o.secs, &o.toast, &o.loading, &o.modal, tr, rows);
        crate::frame_clock::set(None);
        u32_to_rgba(&buf, out);
        (w, h)
    }

    /// 触摸先给覆盖层（游戏坐标）。返回 true = 这一下归覆盖层，不进游戏
    pub(super) fn overlay_touch(&mut self, phase: i32, id: i32, x: f32, y: f32) -> bool {
        crate::frame_clock::set(self.overlay.at);
        let r = self.overlay_touch_inner(phase, id, x, y);
        crate::frame_clock::set(None);
        r
    }

    fn overlay_touch_inner(&mut self, phase: i32, id: i32, x: f32, y: f32) -> bool {
        let vp = (self.width as f32, self.height as f32);
        let sf = compose_size(self.width, self.height, self.dpr).2;
        let text = crate::text::shared_fonts();
        let tr = text.as_deref();
        if phase == 0 {
            let o = &self.overlay;
            let tip_mask = ui_overlay::tip_blocks_input(&o.toast, &o.loading);
            let modal = o.modal.as_ref().is_some_and(|m| m.visible);
            let sheet = ui_overlay::action_sheet::visible();
            if !(sheet || tip_mask || modal) || self.overlay.owner.is_some() {
                return self.overlay.owner.is_some();
            }
            self.overlay.owner = Some(id);
            if !ui_overlay::sheet_overlay_press(vp, x, y) && !tip_mask {
                ui_overlay::modal_press(&mut self.overlay.modal, vp, sf, tr, x, y);
            }
            self.frame.stale = true;
            return true;
        }
        if self.overlay.owner != Some(id) {
            return false;
        }
        match phase {
            1 => {
                ui_overlay::sheet_overlay_move(x, y);
            }
            _ => {
                self.overlay.owner = None;
                if phase == 2 && !ui_overlay::sheet_overlay_release(&mut self.app, vp, x, y) {
                    let Self { overlay, app, .. } = self;
                    if !ui_overlay::tip_blocks_input(&overlay.toast, &overlay.loading) {
                        ui_overlay::modal_release(&mut overlay.modal, app, vp, sf, tr, x, y);
                    }
                }
                if phase != 2 {
                    if let Some(m) = self.overlay.modal.as_mut() {
                        m.pressed_button = None;
                    }
                }
                self.after_js();
            }
        }
        self.frame.stale = true;
        true
    }
}
