//! 小游戏的 WebGL（game-doc/04 G8、design-webgl.md）。
//!
//! 每块拿了 webgl / webgl2 上下文的画布一个 [`GlCtx`]（EGL 上下文 + pbuffer）。所有 GL 调用在引擎线程上，
//! 换画布时才 `eglMakeCurrent`。上屏：帧末把主画布的绘图缓冲读回它的后备缓冲（pixels.rs），之后与 2D 画布同一条路。

mod args;
pub mod ctx;
pub mod egl;
mod extensions;
mod natives;
mod ops_buffer;
mod ops_fbo;
mod ops_gl2;
mod ops_shader;
mod ops_state;
mod ops_texture;
mod params;
pub mod pixels;
mod validate;
mod window;

use crate::game::canvas::GameCanvases;
use ctx::{Attrs, GlCtx};
use std::cell::RefCell;
use std::collections::HashMap;

pub use natives::register;

pub struct WebGl {
    ctxs: HashMap<u32, GlCtx>,
    /// 哪块画布的上下文当前是 current（EGL 是按线程的）
    current: Option<u32>,
    /// 后端偏好：auto / angle / swiftshader / system / off
    pref: String,
    headless: bool,
    /// 要进游戏日志的说明（后端、回退原因）
    pub notes: Vec<String>,
    /// 要派发给 canvas 的事件：(画布 id, "webglcontextlost" | "webglcontextrestored")
    pub events: Vec<(u32, &'static str)>,
    /// `MINI_GPU_FAULT=lost@N`：第 N 次上屏时让主画布的上下文丢失（A8.4）
    fault_lost_at: Option<u64>,
    presents: u64,
    announced: Vec<&'static str>,
    /// 直接上屏的窗口 surface（window.rs）
    window: Option<window::Window>,
    /// 建不了窗口 surface 的那块窗口（CPU 锁过 / 驱动拒绝）：一直走读回，直到换一块
    window_blocked: usize,
    /// 上一次 present 读回了（覆盖层 / 旋转 / 窗口刚接上）：上屏走 CPU 合成好的整帧
    pub rgba_pending: bool,
    /// 直接上屏时：这一帧游戏往窗口 surface 上画过（还没 swap）
    game_frame_new: bool,
}

impl WebGl {
    pub fn new(pref: Option<&str>, headless: bool, fault: Option<&str>) -> Self {
        let pref = std::env::var("MINI_WEBGL").ok().or(pref.map(str::to_string)).unwrap_or_else(|| "auto".into());
        let fault = std::env::var("MINI_GPU_FAULT").ok().or(fault.map(str::to_string));
        let fault_lost_at = fault.as_deref().and_then(|f| f.strip_prefix("lost@")).and_then(|n| n.parse().ok());
        Self { ctxs: HashMap::new(), current: None, pref, headless, notes: Vec::new(), events: Vec::new(), fault_lost_at, presents: 0, announced: Vec::new(), window: None, window_blocked: 0, rgba_pending: false, game_frame_new: false }
    }

    /// 建上下文：按偏好顺序试后端，一个建不了换下一个（日志写明原因）。返回实际属性 JSON
    pub fn create(&mut self, canvases: &RefCell<GameCanvases>, id: u32, version: u8, a: &serde_json::Value) -> Option<String> {
        let kind = if version == 2 { 3 } else { 2 };
        let (w, h) = {
            let mut cs = canvases.borrow_mut();
            let s = cs.get(id)?;
            if s.kind != 0 && s.kind != kind {
                return None;
            }
            (s.w, s.h)
        };
        if let Some(c) = self.ctxs.get(&id) {
            return Some(attrs_json(c));
        }
        let b = |k: &str, d: bool| a.get(k).map(|v| v.as_bool().unwrap_or(d)).unwrap_or(d);
        let attrs = Attrs { alpha: b("alpha", true), depth: b("depth", true), stencil: b("stencil", false), antialias: b("antialias", true),
            premultiplied_alpha: b("premultipliedAlpha", true), preserve_drawing_buffer: b("preserveDrawingBuffer", false) };
        let mut why = Vec::new();
        for backend in egl::order(&self.pref, self.headless) {
            let d = match egl::display(backend) {
                Ok(d) => d,
                Err(e) => {
                    why.push(e);
                    continue;
                }
            };
            self.current = None;
            match GlCtx::new(d, version, attrs, w, h) {
                Ok(c) => {
                    if !self.announced.contains(&backend.name()) {
                        self.announced.push(backend.name());
                        self.notes.push(d.note.clone());
                    }
                    if !why.is_empty() && !self.announced.iter().any(|a| a.starts_with("fallback")) {
                        self.announced.push("fallback");
                        // 只记第一条原因（同一个库在几个候选路径上的报错大同小异）
                        self.notes.push(format!("ℹ️ WebGL 用 {}（前面的后端不可用：{}）", backend.name(), why[0]));
                    }
                    let json = attrs_json(&c);
                    self.ctxs.insert(id, c);
                    self.current = Some(id);
                    canvases.borrow_mut().get(id)?.kind = kind;
                    return Some(json);
                }
                Err(e) => why.push(format!("{}: {e}", backend.name())),
            }
        }
        self.notes.push(format!("⚠️ getContext('webgl{}') 返回 null：{}", if version == 2 { "2" } else { "" }, if why.is_empty() { format!("MINI_WEBGL={}", self.pref) } else { why.join("；") }));
        None
    }

    /// 拿某块画布的上下文并设成 current
    pub fn current(&mut self, id: u32) -> Option<&mut GlCtx> {
        let switch = self.current != Some(id);
        let c = self.ctxs.get_mut(&id)?;
        if switch {
            if !c.make_current() && c.lost {
                self.events.push((id, "webglcontextlost"));
            }
            self.current = Some(id);
        }
        Some(c)
    }

    pub fn resize(&mut self, id: u32, w: u32, h: u32) {
        let mut note = None;
        if let Some(c) = self.current(id) {
            if let Err(e) = c.resize(w, h) {
                note = Some(format!("⚠️ WebGL 画布改尺寸 {w}×{h} 失败：{e}"));
            }
        }
        self.notes.extend(note);
    }

    /// 把绘图缓冲读回画布的后备缓冲（当作图片源用之前）
    pub fn sync(&mut self, canvases: &RefCell<GameCanvases>, id: u32) {
        let Some(c) = self.current(id) else { return };
        if c.lost {
            return;
        }
        let mut cs = canvases.borrow_mut();
        if let Some(slot) = cs.get(id) {
            pixels::read_back(c, slot);
        }
    }

    /// 帧末：主画布的默认帧缓冲这一帧画过 → 读回（或直接上屏时只做标记）、标「待清」。返回主画布像素有没有变。
    /// `rotated` = 引擎内旋转（输出帧要在 CPU 上转）；覆盖层直接上屏时由 GPU 叠（window.rs），不用读回
    pub fn present(&mut self, canvases: &RefCell<GameCanvases>, main: Option<u32>, rotated: bool) -> bool {
        let Some(id) = main else { return false };
        if !self.ctxs.get(&id).map(|c| c.drew && !c.lost).unwrap_or(false) {
            return false;
        }
        self.presents += 1;
        if self.fault_lost_at == Some(self.presents) {
            self.lose(id);
            return false;
        }
        let direct = self.window_mode() && !rotated;
        self.rgba_pending = !direct;
        self.game_frame_new = direct;
        let Some(c) = self.current(id) else { return false };
        if !direct {
            let mut cs = canvases.borrow_mut();
            let Some(slot) = cs.get(id) else { return false };
            pixels::read_back(c, slot);
        }
        c.drew = false;
        c.needs_clear = !c.attrs.preserve_drawing_buffer;
        true
    }

    /// 让一块画布的上下文丢失（WEBGL_lose_context / 故障注入 / 驱动报丢失）
    pub fn lose(&mut self, id: u32) {
        if let Some(c) = self.ctxs.get_mut(&id) {
            if !c.lost {
                c.lose();
                self.events.push((id, "webglcontextlost"));
            }
        }
    }

    /// 重建丢失的上下文：同一套属性建新的（资源全部没了，游戏在 webglcontextrestored 里重建）
    pub fn restore(&mut self, canvases: &RefCell<GameCanvases>, id: u32) -> bool {
        let Some(old) = self.ctxs.get(&id) else { return false };
        if !old.lost {
            return false;
        }
        let (version, attrs) = (old.version, old.attrs);
        let a = serde_json::json!({ "alpha": attrs.alpha, "depth": attrs.depth, "stencil": attrs.stencil, "antialias": attrs.antialias,
            "premultipliedAlpha": attrs.premultiplied_alpha, "preserveDrawingBuffer": attrs.preserve_drawing_buffer });
        self.ctxs.remove(&id);
        self.current = None;
        // 画布的种类保留着（create 认同一种）
        let ok = self.create(canvases, id, version, &a).is_some();
        if ok {
            self.events.push((id, "webglcontextrestored"));
        }
        ok
    }

    /// 画布对象被回收
    pub fn free(&mut self, id: u32) {
        if self.ctxs.remove(&id).is_some() && self.current == Some(id) {
            self.current = None;
        }
    }

    pub fn has(&self, id: u32) -> bool {
        self.ctxs.contains_key(&id)
    }

    /// 诊断：某块画布用的是哪个后端
    pub fn backend(&self, id: u32) -> Option<&'static str> {
        self.ctxs.get(&id).map(|c| c.d.backend.name())
    }
}

fn attrs_json(c: &GlCtx) -> String {
    serde_json::json!({
        "alpha": c.attrs.alpha, "depth": c.attrs.depth, "stencil": c.attrs.stencil, "antialias": c.attrs.antialias,
        "premultipliedAlpha": c.attrs.premultiplied_alpha, "preserveDrawingBuffer": c.attrs.preserve_drawing_buffer,
        "powerPreference": "default", "failIfMajorPerformanceCaveat": false, "desynchronized": false, "xrCompatible": false,
        "__backend": c.d.backend.name(), "__validated": c.d.webgl_compat,
    })
    .to_string()
}

impl crate::game::GameEngine {
    /// 帧末（frame.rs 第 6 步之前）：主画布是 WebGL 且这一帧画过 → 读回；再派发丢失 / 恢复事件
    pub(in crate::game) fn webgl_frame_end(&mut self) {
        let main = self.canvases.borrow().main_id();
        let rotated = self.rotation.is_some();
        if self.webgl.borrow_mut().present(&self.canvases, main, rotated) {
            self.canvases.borrow_mut().main_dirty = true;
        }
        let events = std::mem::take(&mut self.webgl.borrow_mut().events);
        for (id, ev) in events {
            self.guards.watchdog.arm();
            let r = self.app.eval(&format!("typeof __gameGlEvent === 'function' && __gameGlEvent({id}, '{ev}')"));
            self.guards.watchdog.disarm();
            if let Err(e) = r {
                self.report_error("webgl", &e);
            }
            self.after_js();
        }
    }

    /// 每段 JS 之后：建上下文时的后端说明 / 回退原因进游戏日志
    pub(in crate::game) fn webgl_notes(&mut self) {
        let notes = std::mem::take(&mut self.webgl.borrow_mut().notes);
        for n in notes {
            self.host_log(&n);
        }
    }

    /// 宿主上屏（Android nativeBlit）：主画布是 WebGL 时直接 swap 到窗口 surface（window.rs）。
    /// `win` 是 ANativeWindow 指针；`cpu_locked` = 这块窗口被 CPU 锁过；`geometry(w, h)` 设缓冲尺寸。None = 走 CPU 上屏
    pub fn webgl_present_window(&mut self, win: usize, cpu_locked: bool, geometry: &mut dyn FnMut(u32, u32) -> bool) -> Option<i32> {
        let main = self.canvases.borrow().main_id()?;
        if !self.webgl.borrow().has(main) {
            return None;
        }
        let pending = self.webgl.borrow().rgba_pending || self.rotation.is_some();
        let overlay = self.overlay_visible();
        let webgl = self.webgl.clone();
        let frame = |e: &mut Self, layer: bool| {
            if layer {
                return Some(e.overlay_layer());
            }
            let (w, h, px) = e.frame_rgba();
            Some((w, h, px.to_vec()))
        };
        let r = webgl.borrow_mut().present_window(main, win, cpu_locked, pending, overlay, &mut |layer| frame(self, layer), geometry);
        self.webgl_notes();
        r
    }

    /// 宿主的 surface 要没了
    pub fn webgl_detach_window(&mut self) {
        self.webgl.borrow_mut().detach_window();
    }

    /// 诊断 / 测试：主画布用的 WebGL 后端
    pub fn webgl_backend(&self) -> Option<&'static str> {
        let main = self.canvases.borrow().main_id()?;
        self.webgl.borrow().backend(main)
    }
}
