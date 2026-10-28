//! 小游戏的一帧（game-doc/02 §4.1）与输出像素。
//!
//! ```text
//!  0. 帧时刻：宿主 vsync（µs）→ 游戏时刻（clock.rs）；定时器时钟跟它走
//!  1. 输入：排队的触摸 / 键鼠按到达顺序派发（input.rs）
//!  1b. 图片：后台解码完的交回 onload / onerror（image_events.rs）；音频：离线渲染、回报、事件（audio_events.rs）
//!  2~4. 逻辑层：定时器、回包、微任务（MiniApp::update）
//!  5. rAF：节拍器（raf_pacer.rs）说这是 rAF 帧，就以同一个时间戳跑完当前这一批
//!  6. 主画布这一帧画过 → FRAME_NEW（像素在后备缓冲里，宿主要的时候再转格式）
//!  7. 标志：WANTS_MORE（有挂着的 rAF、1 秒内要到点的定时器、在飞的请求）/ WAKE_AT（只有远期定时器）
//! ```
//! 输入放在 rAF 之前：手指这一帧的位置在这一帧的 rAF 里就能用上。

use super::clock::GameClock;
use super::raf_pacer::RafPacer;
use super::GameEngine;
use crate::host::engine_backend::{FRAME_EVENTS, FRAME_NEW, FRAME_WANTS_MORE};
use crate::js::JsRuntime;
use rquickjs::IntoJs;
use std::cell::RefCell;
use std::rc::Rc;

/// 只剩远期定时器（> 1 秒）：宿主不必每个 vsync 都来，按 [`GameEngine::next_wake_us`] 定时唤醒即可
/// （为一个 30 秒后的定时器每帧空转是纯耗电）。小程序那边不用这一位
pub const FRAME_WAKE_AT: u32 = 128;

/// 1 秒内要到点的定时器才值得保持出帧
const NEAR_TIMER_MS: u64 = 1000;

#[derive(Default)]
pub struct FrameOut {
    /// 主画布转成的不透明 RGBA（主画布分辨率）
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
    /// 主画布画过、rgba 还没跟上
    pub stale: bool,
    /// 已经出过的帧数（rAF 帧里主画布变了才算一帧）
    pub count: u64,
    /// 引擎内旋转时的中间缓冲（转之前的那一份）
    pub unrotated: Vec<u8>,
}

pub(super) fn register(rt: &JsRuntime, pacer: Rc<RefCell<RafPacer>>, clock: Rc<RefCell<GameClock>>) -> Result<(), String> {
    rt.register_raw("__game_set_fps", move |ctx, args| {
        let v = args.0.first().and_then(|v| v.as_number()).unwrap_or(f64::NAN);
        // 官方有效范围 1~60：越界按边界夹（真机的越界行为以取证为准，game-doc/06 §7），非数字忽略
        if v.is_finite() {
            pacer.borrow_mut().set_fps(v.round().clamp(1.0, 60.0) as u32);
        }
        Ok(rquickjs::Value::new_undefined(ctx))
    })?;
    rt.register_raw("__game_now", move |ctx, _| clock.borrow().now_ms().into_js(&ctx))?;
    Ok(())
}

impl GameEngine {
    /// 跑一帧。`now_us` 是宿主 vsync 时刻（**微秒**，任意原点的单调钟；0 = 引擎自己取）。返回 `FRAME_*` 位
    pub fn pump_ex(&mut self, now_us: u64) -> u32 {
        self.inject("panic@pump");
        let t = self.clock.borrow_mut().begin_frame(now_us);
        self.app.set_timer_clock(Some(self.clock.borrow().instant_at(t)));
        let mut flags = 0;

        // 1 输入；图片的 onload / onerror
        self.dispatch_input();
        self.dispatch_images();
        self.dispatch_audio(t);
        self.dispatch_workers();

        // 2~4 逻辑层
        self.guards.watchdog.arm();
        let r = self.app.update();
        self.guards.watchdog.disarm();
        if let Err(e) = r {
            self.report_error(super::error_kind(&e, "update"), &e);
        }
        self.after_js();
        // 2b 系统覆盖层（Toast / Modal / ActionSheet 的请求在逻辑层里登记，overlay.rs）
        if self.overlay_tick(t) {
            self.frame.stale = true;
            flags |= FRAME_NEW;
        }

        // 5 rAF（后台不跑：微信切后台后 rAF 不回调）
        let raf_frame = !self.backgrounded && self.pacer.borrow_mut().on_vsync(t);
        // 4.5 开放数据域（opendata.rs）：在主域的 rAF 之前画 sharedCanvas，主域这一帧 drawImage 拿到的是新的
        if !self.backgrounded {
            self.opendata_tick(t as f64 / 1000.0, raf_frame);
        }
        if raf_frame && self.raf_pending() > 0 {
            let ts = t as f64 / 1000.0;
            self.guards.watchdog.arm();
            let r = self.app.call_num("__gameRunRaf", &[ts]);
            self.guards.watchdog.disarm();
            if let Err(e) = r {
                self.report_error(super::error_kind(&e, "raf"), &e);
            }
            self.after_js();
            // 进程级开关只读一次：env::var 每次都拿全局锁并分配，这里每个 rAF 帧都走
            static RAF_LOG: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            if *RAF_LOG.get_or_init(|| std::env::var("MINI_RAF_LOG").is_ok()) {
                let line = {
                    let p = self.pacer.borrow();
                    format!("🎞 rAF t={ts:.3}ms 目标 {}fps 显示周期 {:.2}ms{}", p.fps(), p.display_period_ms(), if p.uneven() { " ⚠ 节拍不均匀" } else { "" })
                };
                self.host_log(&line);
            }
        }

        // 6 主画布（WebGL 的先读回后备缓冲）
        #[cfg(feature = "webgl")]
        self.webgl_frame_end();
        let t_ms = t / 1000;
        if std::mem::take(&mut self.canvases.borrow_mut().main_dirty) {
            self.frame.stale = true;
            self.frame.count += 1;
            flags |= FRAME_NEW;
            crate::host::cpu_affinity::note_produced(t_ms);
        }
        let od_busy = { let o = self.od.borrow(); o.busy || (o.wanted && self.od_rt.is_none()) || !o.inbox.is_empty() };
        let raf_waiting = !self.backgrounded && (self.raf_pending() > 0 || self.overlay_animating() || od_busy);
        // Android 大核：有挂着的 rAF 就算「在连续出帧」（游戏几乎一直是），停了 800ms 放开（桌面 / iOS 什么都不做）
        crate::host::cpu_affinity::note_frame(t_ms, raf_waiting);

        // 7 要不要下一帧
        let near_timer = self.app.next_timer_due_ms().map(|ms| ms <= NEAR_TIMER_MS).unwrap_or(false);
        if raf_waiting || (!self.backgrounded && (near_timer || self.app.has_pending_host_work() || self.images.borrow().loading() || self.audio.borrow().busy() || self.worker_alive())) {
            flags |= FRAME_WANTS_MORE;
        } else if self.app.next_timer_due_ms().is_some() {
            flags |= FRAME_WAKE_AT;
        }
        if !self.events.is_empty() {
            flags |= FRAME_EVENTS;
        }
        flags
    }

    fn raf_pending(&self) -> usize {
        self.app.call_num("__gameRafPending", &[]).map(|n| n as usize).unwrap_or(0)
    }

    /// 只有远期定时器时，宿主多久之后再来（µs，相对现在）；没有定时器 None
    pub fn next_wake_us(&self) -> Option<u64> {
        self.app.next_timer_due_ms().map(|ms| ms * 1000)
    }

    /// 当前 rAF 目标帧率（宿主据此请求显示刷新率：60 的整数倍，见 game-doc/02 §4.2）
    pub fn preferred_fps(&self) -> u32 {
        self.pacer.borrow().fps()
    }

    /// 前后台：后台不跑 rAF、不要帧；回前台从下一个 vsync 重新起算（不补帧）
    pub fn set_background(&mut self, bg: bool) {
        if self.backgrounded != bg {
            if bg {
                // 微信切后台：按着的手指收到 touchcancel（GT-T4）
                self.cancel_touches();
            }
            self.audio.borrow_mut().set_background(bg);
            self.flush_audio_events();
            self.backgrounded = bg;
            if !bg {
                self.pacer.borrow_mut().reset();
                // 回前台第一帧要重新上屏（surface 可能换过）
                self.frame.stale = true;
            }
            // 音频已经停 / 恢复之后才派发：onHide 里读到的 InnerAudio 已经是 paused（lifecycle.rs）
            self.dispatch_lifecycle(if bg { "hide" } else { "show" });
        }
    }

    /// 主画布像素尺寸（宿主按它设 surface 的缓冲尺寸，由系统缩放到全屏）；还没建主画布时 (0, 0)
    pub fn main_canvas_size(&self) -> (u32, u32) {
        self.canvases.borrow().main_slot().map(|s| (s.w, s.h)).unwrap_or((0, 0))
    }

    /// 主画布的不透明 RGBA（主画布分辨率，压黑底）。只有画过之后第一次取才转换
    pub fn frame_rgba(&mut self) -> (u32, u32, &[u8]) {
        if self.frame.stale || self.frame.rgba.is_empty() {
            let cs = self.canvases.borrow();
            match cs.main_slot() {
                // 覆盖层弹着：在引擎里合成（overlay.rs），输出按 min(dpr, 2) 放大
                Some(_) if self.overlay_visible() => {
                    drop(cs);
                    let mut out = std::mem::take(&mut self.frame.rgba);
                    let (w, h) = self.compose_overlay(&mut out);
                    (self.frame.rgba, self.frame.w, self.frame.h) = (out, w, h);
                }
                Some(slot) => {
                    let canvas = slot.ctx.get_canvas();
                    let c = canvas.lock().unwrap();
                    super::canvas::compose::to_rgba_opaque(c.pixels(), &mut self.frame.rgba);
                    self.frame.w = c.width();
                    self.frame.h = c.height();
                }
                None => {
                    // 还没有主画布：一块黑（窗口逻辑尺寸）
                    let (w, h) = cs.window;
                    self.frame.rgba = [0u8, 0, 0, 255].repeat((w * h) as usize);
                    self.frame.w = w;
                    self.frame.h = h;
                }
            }
            self.frame.stale = false;
            if let Some(r) = self.rotation {
                // 引擎内旋转（rotate.rs）：整帧转 90°，宽高对调
                std::mem::swap(&mut self.frame.rgba, &mut self.frame.unrotated);
                r.rotate_rgba(&self.frame.unrotated, self.frame.w, self.frame.h, &mut self.frame.rgba);
                (self.frame.w, self.frame.h) = (self.frame.h, self.frame.w);
            }
        }
        (self.frame.w, self.frame.h, &self.frame.rgba)
    }

    /// 输出帧是不是引擎合成过的（转过方向、或盖着覆盖层）：宿主上屏要走 RGBA，不能直取主画布像素
    pub fn rgba_output(&self) -> bool {
        self.rotation.is_some() || self.overlay_visible()
    }
    /// 拿主画布的原始像素（宽、高、`Color` 行优先）跑一段宿主侧上屏代码：热路径直写宿主缓冲，
    /// 不经 [`frame_rgba`](Self::frame_rgba) 的中间缓冲（`compose::write_opaque_rows`）。还没有主画布返回 None
    pub fn with_main_pixels<R>(&self, f: impl FnOnce(u32, u32, &[crate::Color]) -> R) -> Option<R> {
        let cs = self.canvases.borrow();
        let canvas = cs.main_slot()?.ctx.get_canvas();
        let c = canvas.lock().ok()?;
        Some(f(c.width(), c.height(), c.pixels()))
    }

    /// 缩放到 `(w, h)` 的 RGBA（无头快照 / 桌面窗体；移动端交给系统缩放器，不走这里）
    pub fn frame_rgba_scaled(&mut self, w: u32, h: u32) -> Vec<u8> {
        if let Some(r) = self.rotation {
            // 转之前按对调的尺寸缩，再转
            let raw = self.scaled_unrotated(h, w);
            let mut out = Vec::new();
            r.rotate_rgba(&raw, h, w, &mut out);
            return out;
        }
        self.scaled_unrotated(w, h)
    }

    pub(super) fn scaled_unrotated(&self, w: u32, h: u32) -> Vec<u8> {
        let cs = self.canvases.borrow();
        let mut out = Vec::new();
        match cs.main_slot() {
            Some(slot) => {
                let canvas = slot.ctx.get_canvas();
                let c = canvas.lock().unwrap();
                super::canvas::compose::scale_rgba_opaque(c.pixels(), c.width(), c.height(), w, h, &mut out);
            }
            None => out = [0u8, 0, 0, 255].repeat((w * h) as usize),
        }
        out
    }

    /// 出过的帧数（诊断）
    pub fn frames_presented(&self) -> u64 {
        self.frame.count
    }

    /// 还活着的画布个数（泄漏检查）
    pub fn live_canvases(&self) -> usize {
        self.canvases.borrow().live()
    }
}
