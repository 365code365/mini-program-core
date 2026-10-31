//! 2D 调用分到 GPU 还是 CPU（gpu2d/mod.rs 的表）、两边像素的搬运、CPU 临时层。
//!
//! natives.rs 的分派函数对 GPU 画布先问 [`GameCanvases::gpu_op`]：接了就完事；不接（返回 false）就照 CPU 那条路走 ——
//! 之前 [`GameCanvases::sync_cpu`] 把 GPU 上新画的读回来，之后 [`GameCanvases::cpu_wrote`] 标「CPU 那边新」。

use super::super::{Auth, GameCanvases, Mode2d, Slot};
use super::{Gpu2d, Src};
use crate::game::image::ImageStore;
use crate::renderer::components::{Canvas2DContext, GpuBlend, ImageRects};
use crate::{Canvas, Color};
use rquickjs::Value;
use std::cell::RefCell;

thread_local! {
    /// 透明临时层（按尺寸缓存；不变式：全透明、无裁剪）与取块用的缓冲
    static LAYERS: RefCell<Vec<Canvas>> = const { RefCell::new(Vec::new()) };
    static TILE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn take_layer(w: u32, h: u32) -> Canvas {
    let hit = LAYERS.with(|m| {
        let mut m = m.borrow_mut();
        let i = m.iter().position(|c| c.width() == w && c.height() == h)?;
        Some(m.remove(i))
    });
    hit.unwrap_or_else(|| {
        let mut c = Canvas::new(w, h);
        c.clear(Color::TRANSPARENT);
        c
    })
}

fn give_layer(c: Canvas) {
    LAYERS.with(|m| {
        let mut m = m.borrow_mut();
        m.insert(0, c);
        m.truncate(2);
    });
}

fn num(v: Option<&Value>) -> f32 {
    v.and_then(|x| x.as_number()).unwrap_or(f64::NAN) as f32
}

fn finite(xs: &[f32]) -> bool {
    xs.iter().all(|x| x.is_finite())
}

/// 设备坐标外接框 → 光栅器可能写到的整数像素框（外扩 2px，与画布求交），同 fx_mask::pixel_region
fn pixel_region(b: (f32, f32, f32, f32), w: u32, h: u32) -> (usize, usize, usize, usize) {
    let c = |v: f32, hi: u32| (v.max(0.0) as u32).min(hi) as usize;
    let (x0, y0) = (c((b.0 - 2.0).floor(), w), c((b.1 - 2.0).floor(), h));
    let (x1, y1) = (c((b.2 + 2.0).ceil(), w), c((b.3 + 2.0).ceil(), h));
    (x0, y0, x1.max(x0), y1.max(y0))
}

impl GameCanvases {
    /// `getContext('2d')` 第一次拿到时定后端
    pub fn choose_2d(&mut self, id: u32) {
        let main = self.is_main(id);
        let want = match self.mode2d {
            Mode2d::Cpu => false,
            Mode2d::Gpu => true,
            Mode2d::Auto => main && !self.cpu_output && cfg!(any(target_os = "ios", target_os = "android")),
        };
        if !want || self.gpu2d_failed {
            return;
        }
        if self.gpu2d.is_none() {
            match Gpu2d::new(&self.gl_pref, self.cpu_output) {
                Ok(g) => {
                    self.notes.push(format!("🎨 2D 画布后端 gpu（{}）{}", g.backend, if self.mode2d == Mode2d::Gpu { "：MINI_2D=gpu，全部 2D 画布" } else { "：主画布" }));
                    self.gpu2d = Some(Box::new(g));
                }
                Err(e) => {
                    self.gpu2d_failed = true;
                    self.notes.push(format!("ℹ️ 2D 画布后端 cpu（GPU 建不了：{e}）"));
                    return;
                }
            }
        }
        if let Some(s) = self.get(id) {
            s.gpu = Some(Auth::Both);
        }
    }

    pub fn is_gpu(&self, id: u32) -> bool {
        self.slots.get(id as usize).and_then(|s| s.as_ref()).is_some_and(|s| s.gpu.is_some())
    }

    /// 主画布是 GPU 画布
    pub fn main_is_gpu(&self) -> bool {
        self.main.is_some_and(|m| self.is_gpu(m))
    }

    /// CPU 要读 / 画这块画布之前：GPU 那边新就读回来。返回 true = 动过 GL（2D 的上下文成了 current，
    /// WebGL 那边的调用方要切回自己的）
    pub fn sync_cpu(&mut self, id: u32) -> bool {
        let Some(slot) = self.slots.get_mut(id as usize).and_then(|s| s.as_mut()) else { return false };
        if slot.gpu != Some(Auth::Gpu) {
            return false;
        }
        let ok = match self.gpu2d.as_mut() {
            Some(g) => {
                let canvas = slot.ctx.get_canvas();
                let mut c = canvas.lock().unwrap();
                g.read(id, c.pixels_mut())
            }
            None => false,
        };
        if ok {
            slot.gpu = Some(Auth::Both);
        } else {
            self.lost_all("读回失败");
        }
        true
    }

    /// CPU 刚改过这块画布（回退的那几条调用）
    pub fn cpu_wrote(&mut self, id: u32) {
        if let Some(s) = self.get(id) {
            s.gen += 1;
            if s.gpu.is_some() {
                s.gpu = Some(Auth::Cpu);
            }
        }
    }

    /// GPU 要画之前：纹理在、CPU 那边新就传上去。false = GPU 用不了（已经降回 CPU）
    fn sync_gpu(&mut self, id: u32) -> bool {
        let Some(slot) = self.slots.get_mut(id as usize).and_then(|s| s.as_mut()) else { return false };
        // 稳态（每个 fillRect / drawImage 都走）：已经是「GPU 那边新」= 纹理在、尺寸对（改尺寸会降成 Both、丢失会整个降成 None），
        // 不锁后备缓冲、不查表。从前每次都锁 + 两次 HashMap，真机 2000 个 fillRect 一帧多 1.3ms
        if slot.gpu == Some(Auth::Gpu) {
            slot.gen += 1;
            return true;
        }
        let Some(g) = self.gpu2d.as_mut() else { return false };
        let canvas = slot.ctx.get_canvas();
        let c = canvas.lock().unwrap();
        let fresh = !g.has(id);
        if !g.ensure(id, c.width(), c.height()) {
            drop(c);
            self.lost_all("建画布纹理失败");
            return false;
        }
        // 新建的纹理是空的：CPU 那边有内容（降级过又升回来、或之前画过）就整张传上去
        if slot.gpu == Some(Auth::Cpu) || (fresh && slot.gpu == Some(Auth::Gpu)) {
            g.write(id, c.pixels());
        }
        drop(c);
        slot.gpu = Some(Auth::Gpu);
        slot.gen += 1;
        true
    }

    /// 一块画布降回 CPU（上屏用不了 GPU 时）：GPU 上新画的先读回
    pub fn demote(&mut self, id: u32, why: &str) {
        if !self.is_gpu(id) {
            return;
        }
        self.sync_cpu(id);
        if let Some(s) = self.get(id) {
            s.gpu = None;
        }
        if let Some(g) = self.gpu2d.as_mut() {
            g.free(id);
        }
        self.notes.push(format!("ℹ️ 2D 画布 #{id} 回到 CPU：{why}"));
    }

    /// 上屏前：主画布在 GPU 上要有最新内容（CPU 那边新就传上去）
    pub fn gpu_ready_main(&mut self) {
        if let Some(m) = self.main.filter(|&m| self.is_gpu(m)) {
            if self.get(m).is_some_and(|s| s.gpu != Some(Auth::Gpu)) {
                self.sync_gpu(m);
            }
        }
    }

    /// GPU 上下文没了 / 出错：所有 GPU 画布降回 CPU（没读回的内容丢了），本实例不再用 GPU
    fn lost_all(&mut self, why: &str) {
        self.notes.push(format!("⚠️ GPU 2D 画布回退 CPU：{why}"));
        for s in self.slots.iter_mut().flatten() {
            s.gpu = None;
        }
        self.gpu2d = None;
        self.gpu2d_failed = true;
    }

    /// 帧末（frame.rs 第 6 步）
    pub fn gpu_frame_end(&mut self) {
        if let Some(g) = self.gpu2d.as_mut() {
            g.end_frame();
            if g.lost {
                self.lost_all("上下文丢失");
            }
        }
    }

    /// GPU 画布上的一条绘制调用。true = GPU 接了（像素已画好），false = 走 CPU
    pub fn gpu_op(&mut self, id: u32, op: i32, a: &[Value]) -> bool {
        let n = |i: usize| num(a.get(i));
        match op {
            1 => {
                let (x, y, w, h) = (n(0), n(1), n(2), n(3));
                if !finite(&[x, y, w, h]) {
                    return true;
                }
                if self.fill_rect_fast(id, x, y, w, h) {
                    return true;
                }
                let Some(b) = self.get(id).map(|s| s.ctx.gpu_rect_bounds(x, y, w, h, false)) else { return true };
                self.layer(id, b, |g| g.fill_rect(x, y, w, h))
            }
            2 => {
                let (x, y, w, h) = (n(0), n(1), n(2), n(3));
                if !finite(&[x, y, w, h]) {
                    return true;
                }
                let Some(b) = self.get(id).map(|s| s.ctx.gpu_rect_bounds(x, y, w, h, true)) else { return true };
                self.layer(id, b, |g| g.stroke_rect(x, y, w, h))
            }
            3 => {
                let (x, y, w, h) = (n(0), n(1), n(2), n(3));
                !finite(&[x, y, w, h]) || self.clear_rect(id, x, y, w, h)
            }
            14 | 15 => {
                let stroke = op == 15;
                let Some(b) = self.get(id).map(|s| s.ctx.gpu_path_bounds(stroke)) else { return true };
                let Some(b) = b else { return true };
                self.layer(id, b, |g| if stroke { g.stroke() } else { g.fill() })
            }
            25 | 26 => {
                let t = a.first().and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
                let (x, y) = (n(1), n(2));
                let mw = a.get(3).filter(|v| !v.is_undefined()).map(|v| v.as_number().unwrap_or(f64::NAN) as f32);
                if !(x.is_finite() && y.is_finite()) || t.is_empty() {
                    return true;
                }
                let stroke = op == 26;
                let Some(slot) = self.get(id) else { return true };
                let font = slot.font.clone();
                let b = slot.ctx.gpu_text_bounds(font.size, t.chars().count(), mw, stroke)(x, y);
                self.layer(id, b, |g| g.fill_text_font(&font, &t, x, y, mw, stroke))
            }
            _ => false,
        }
    }

    /// fillRect 的 GPU 直画：纯色、轴对齐、无阴影
    fn fill_rect_fast(&mut self, id: u32, x: f32, y: f32, w: f32, h: f32) -> bool {
        let Some(s) = self.get(id) else { return false };
        let g = &s.ctx;
        let m = g.gpu_transform();
        let (Some(c), Some(blend)) = (g.gpu_solid_fill(), g.gpu_blend()) else { return false };
        if m[1] != 0.0 || m[2] != 0.0 || g.gpu_shadow().is_some() || (g.gpu_clip().is_some() && blend.clears_outside()) {
            return false;
        }
        let alpha = g.gpu_alpha();
        // 与 CPU 的 apply_alpha 同一个取整（截断）
        let c = if alpha >= 1.0 { c } else { Color::new(c.r, c.g, c.b, (c.a as f32 * alpha) as u8) };
        let (ax, ay) = g.gpu_tp(x, y);
        let (bx, by) = g.gpu_tp(x + w, y + h);
        let r = (ax.min(bx), ay.min(by), ax.max(bx), ay.max(by));
        let clip = g.gpu_clip().cloned();
        if !self.sync_gpu(id) {
            return false;
        }
        if let Some(gp) = self.gpu2d.as_mut() {
            gp.rect(id, r, c, blend, clip.as_ref());
        }
        true
    }

    /// clearRect：无裁剪时 GPU 直接写 0（整数框与 CPU 的 clear_rect 同一个算法）
    fn clear_rect(&mut self, id: u32, x: f32, y: f32, w: f32, h: f32) -> bool {
        let Some(s) = self.get(id) else { return true };
        let g = &s.ctx;
        if g.gpu_clip().is_some() {
            return false;
        }
        let (tx0, ty0) = g.gpu_tp(x, y);
        let (tx1, ty1) = g.gpu_tp(x + w, y + h);
        let (x, y) = (tx0.min(tx1), ty0.min(ty1));
        let (cw, ch) = (s.ctx.width as f32, s.ctx.height as f32);
        let (x0, y0) = (x.max(0.0) as i32, y.max(0.0) as i32);
        let x1 = (x + (tx1 - tx0).abs()).min(cw) as i32;
        let y1 = (y + (ty1 - ty0).abs()).min(ch) as i32;
        if x1 <= x0 || y1 <= y0 {
            return true;
        }
        if !self.sync_gpu(id) {
            return false;
        }
        if let Some(gp) = self.gpu2d.as_mut() {
            gp.clear(id, (x0 as u32, y0 as u32, x1 as u32, y1 as u32));
        }
        true
    }

    /// CPU 照原样把这一笔画进透明临时层（裁剪到 `bounds` 外扩 2px），被画到的那块交给 GPU 按合成模式叠上去
    fn layer(&mut self, id: u32, bounds: (f32, f32, f32, f32), draw: impl FnOnce(&mut Canvas2DContext)) -> bool {
        let Some(s) = self.get(id) else { return true };
        let g = &s.ctx;
        let Some(blend) = g.gpu_blend() else { return false };
        let shadow = g.gpu_shadow();
        let clip = g.gpu_clip().cloned();
        if (shadow.is_some() && blend != GpuBlend::SourceOver) || (clip.is_some() && blend.clears_outside()) {
            return false;
        }
        let mut b = bounds;
        if let Some((blur, ox, oy)) = shadow {
            // 阴影：形状平移后的框并进来，再按模糊半径外扩（三遍盒式模糊，σ = blur / 2，摊开不超过 2×blur）
            let p = blur * 2.0 + 2.0;
            b = (b.0.min(b.0 + ox) - p, b.1.min(b.1 + oy) - p, b.2.max(b.2 + ox) + p, b.3.max(b.3 + oy) + p);
        }
        let (w, h) = (s.ctx.width, s.ctx.height);
        let region = pixel_region(b, w, h);
        if region.2 <= region.0 || region.3 <= region.1 {
            // 什么也画不到；清形状外像素的模式这时仍要清整块画布，交给 CPU 照它的语义做
            return !blend.clears_outside();
        }
        if !self.sync_gpu(id) {
            return false;
        }
        let Some(s) = self.get(id) else { return true };
        let mut layer = take_layer(w, h);
        let (x0, y0, x1, y1) = region;
        layer.clip_rect(crate::Rect::new(x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32));
        s.ctx.swap_backing(&mut layer);
        // 裁剪交给 GPU 乘（gl.rs 文件头）：这里不经 draw_op，CPU 不再按蒙版插值一遍
        s.ctx.with_source_over(draw);
        s.ctx.swap_backing(&mut layer);
        layer.reset_clip();
        let stride = w as usize;
        TILE.with(|t| {
            let mut t = t.borrow_mut();
            t.clear();
            let px = layer.pixels_mut();
            for y in y0..y1 {
                let row = &mut px[y * stride + x0..y * stride + x1];
                t.extend_from_slice(crate::canvas::color_bytes(row));
                row.fill(Color::TRANSPARENT);
            }
            if let Some(gp) = self.gpu2d.as_mut() {
                gp.tile(id, region, &t, blend, clip.as_ref());
            }
        });
        give_layer(layer);
        true
    }

    /// drawImage 到 GPU 画布。false = 走 CPU（阴影、清形状外像素的模式、画到自己身上、要读目标色的模式）
    pub fn gpu_draw_image(&mut self, images: &mut ImageStore, id: u32, a: &[Value]) -> bool {
        let n = |i: usize| num(a.get(i));
        let (kind, sid) = (n(0) as i32, n(1) as u32);
        let smooth = a.get(2).and_then(|v| v.as_bool()).unwrap_or(true);
        let argc = n(3) as i32;
        if kind == 2 && sid == id {
            return false;
        }
        let img = if kind == 1 { images.get(sid) } else { None };
        let (iw, ih) = match (kind, &img) {
            (1, Some(d)) => (d.w, d.h),
            (1, None) => return true,
            (2, _) => match self.get(sid) {
                Some(s) if s.w > 0 && s.h > 0 => (s.ctx.width, s.ctx.height),
                _ => return true,
            },
            _ => return true,
        };
        let (iwf, ihf) = (iw as f32, ih as f32);
        let r = match argc {
            3 => ImageRects { sx: 0.0, sy: 0.0, sw: iwf, sh: ihf, dx: n(4), dy: n(5), dw: iwf, dh: ihf },
            5 => ImageRects { sx: 0.0, sy: 0.0, sw: iwf, sh: ihf, dx: n(4), dy: n(5), dw: n(6), dh: n(7) },
            9 => ImageRects { sx: n(4), sy: n(5), sw: n(6), sh: n(7), dx: n(8), dy: n(9), dw: n(10), dh: n(11) },
            _ => return true,
        };
        if !finite(&[r.sx, r.sy, r.sw, r.sh, r.dx, r.dy, r.dw, r.dh]) {
            return true;
        }
        let Some(r) = r.normalize(iw, ih) else { return true };
        let Some(s) = self.get(id) else { return true };
        let g = &s.ctx;
        let Some(blend) = g.gpu_blend() else { return false };
        if g.gpu_shadow().is_some() || blend.clears_outside() {
            return false;
        }
        let alpha = g.gpu_alpha();
        if alpha <= 0.0 {
            return true;
        }
        let clip = g.gpu_clip().cloned();
        let quad = [g.gpu_tp(r.dx, r.dy), g.gpu_tp(r.dx + r.dw, r.dy), g.gpu_tp(r.dx + r.dw, r.dy + r.dh), g.gpu_tp(r.dx, r.dy + r.dh)];
        let (u0, u1, v0, v1) = (r.sx / iwf, (r.sx + r.sw) / iwf, r.sy / ihf, (r.sy + r.sh) / ihf);
        let uv = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)];
        let half = |lo: f32, hi: f32, n: f32| {
            let (a, b) = (lo + 0.5 / n, hi - 0.5 / n);
            if a <= b { (a, b) } else { ((lo + hi) / 2.0, (lo + hi) / 2.0) }
        };
        let ((iu0, iu1), (iv0, iv1)) = (half(u0, u1, iwf), half(v0, v1, ihf));
        if !self.sync_gpu(id) {
            return false;
        }
        let src_gpu = kind == 2 && self.slots.get(sid as usize).and_then(|s| s.as_ref()).is_some_and(|s| matches!(s.gpu, Some(Auth::Gpu | Auth::Both)));
        if src_gpu && !self.sync_gpu(sid) {
            return false;
        }
        let GameCanvases { slots, gpu2d, .. } = self;
        let Some(gp) = gpu2d.as_mut() else { return false };
        let inset = [iu0, iu1, iv0, iv1];
        match (&img, src_gpu) {
            (Some(d), _) => gp.image(id, &Src::Image(d), quad, uv, inset, alpha, smooth, blend, clip.as_ref()),
            (None, true) => gp.image(id, &Src::Gpu(sid), quad, uv, inset, alpha, smooth, blend, clip.as_ref()),
            (None, false) => {
                let Some(ss) = slots.get(sid as usize).and_then(|s| s.as_ref()) else { return true };
                let canvas = ss.ctx.get_canvas();
                let c = canvas.lock().unwrap();
                let src = Src::Cpu { id: sid, gen: ss.gen, rgba: crate::canvas::color_bytes(c.pixels()), w: c.width(), h: c.height() };
                gp.image(id, &src, quad, uv, inset, alpha, smooth, blend, clip.as_ref());
            }
        }
        true
    }
}

impl Slot {
    /// 诊断：这块画布走哪个后端
    pub fn backend_2d(&self) -> &'static str {
        if self.gpu.is_some() { "gpu" } else { "cpu" }
    }
}
