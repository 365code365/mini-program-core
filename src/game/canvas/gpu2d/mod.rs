//! 小游戏的 2D 画布走 GPU（`MINI_2D=gpu|cpu|auto`，创建选项 `canvas2d`）。
//!
//! ## 做法
//! 每块 GPU 画布是一张 FBO 纹理（RGBA8、预乘），在一个专用的 EGL 上下文（与 WebGL 同一个 display：iOS / macOS 上是
//! ANGLE 的 Metal，Android 上 ANGLE / 系统 GLES，无头是 SwiftShader）里画。**光栅器不另写一份**：
//!
//! | 调用 | GPU 上怎么画 |
//! |---|---|
//! | fillRect（纯色、轴对齐） | 一个四边形，覆盖率在片元里按 CPU 抗锯齿矩形同一个算法算（gl.rs）；连续的合成一个 draw call |
//! | clearRect（无裁剪） | 整数像素框里写 0 |
//! | drawImage（无阴影） | 纹理四边形（图片按解码结果缓存成纹理；源是 CPU 画布按内容代数缓存；源是 GPU 画布直接采它的 FBO） |
//! | 路径填充 / 描边、文字、strokeRect、旋转的 fillRect、带阴影的 drawImage | CPU 照原样画进一张**透明临时层**（裁剪到保守外接框），被画到的那块当纹理按合成模式叠上去（route.rs） |
//! | putImageData | 那块直接写进纹理 |
//! | getImageData、toDataURL、画布当 WebGL 纹理源 / CPU 画布的 drawImage 源、multiply 这类要读目标色的合成 | 读回 CPU 后备缓冲，在 CPU 上做 |
//!
//! 「临时层再叠」与「直接画在目标上」对 source-over 是同一个结果（结合律），对其余能用混合函数表达的 Porter-Duff 模式，
//! 临时层按 source-over 画出「源 × 覆盖率」，GPU 用该模式的混合函数叠上去，也是同一个式子（fx_composite.rs）。
//! 两边差的只是舍入：CPU 存非预乘，GPU 存预乘。
//!
//! ## 权威副本
//! 每块 GPU 画布同时有 CPU 后备缓冲：[`Auth`] 记哪一边是新的。GPU 调用之前 CPU 那边新 → 整张传上去；
//! CPU 调用 / 读像素之前 GPU 那边新 → 整张读回。游戏稳态只走 GPU 那几条时两边不来回搬。
//!
//! ## 回退（steering 10：CPU 是参考实现）
//! 建不了上下文 / 编不过着色器 → 日志一行原因，这个进程里 2D 全走 CPU；上屏建不了窗口 surface → 主画布降回 CPU；
//! 运行中 make current 失败（上下文丢了）→ 所有 GPU 画布降回 CPU（丢失前没读回的内容没了，日志写明）。

mod draw;
mod gl;
pub mod present;
pub mod route;
mod share;

use crate::game::image::loader::Decoded;
use crate::game::webgl::egl::{self, Display, Want};
use crate::renderer::components::GpuBlend;
use crate::Color;
use glow::HasContext;
use khronos_egl as k;
use std::collections::HashMap;
use std::sync::{Arc, Weak};

pub use super::Auth;

struct Surf {
    fbo: glow::NativeFramebuffer,
    tex: glow::NativeTexture,
    w: u32,
    h: u32,
}

/// 一批顶点共用的状态（变了就先把攒着的画掉）
#[derive(Clone, Copy, PartialEq)]
struct Key {
    target: u32,
    tex: Option<glow::NativeTexture>,
    filter: u32,
    blend: GpuBlend,
    clip: Option<usize>,
    /// 渐变（种类 4 / 5）的编号：0 = 没有；同一组参数连续用同一个编号，能合批
    grad: u32,
}

struct ImgTex {
    tex: glow::NativeTexture,
    src: Weak<Decoded>,
}

/// 2D 画布当纹理源时用到的「源」
pub enum Src<'a> {
    Image(&'a Arc<Decoded>),
    /// CPU 画布：内容代数（变了才重传）与像素
    Cpu { id: u32, gen: u64, rgba: &'a [u8], w: u32, h: u32 },
    /// 另一块 GPU 画布
    Gpu(u32),
}

pub struct Gpu2d {
    d: &'static Display,
    ctx: k::Context,
    pb: k::Surface,
    cfg: k::Config,
    gl: glow::Context,
    prog: gl::Prog,
    surfs: HashMap<u32, Surf>,
    verts: Vec<f32>,
    key: Option<Key>,
    /// 临时层纹理轮着用（同一张刚交给 GPU 又改写，驱动要么等、要么拷一份）
    tiles: Vec<(glow::NativeTexture, u32, u32)>,
    tile_next: usize,
    images: HashMap<usize, ImgTex>,
    canvas_src: HashMap<u32, (glow::NativeTexture, u64)>,
    clip: Option<(Arc<Vec<u8>>, glow::NativeTexture)>,
    /// 当前渐变 uniform 对应的参数与编号
    grad: Option<(gl::Grad, u32)>,
    grad_serial: u32,
    scratch: Vec<u8>,
    window: Option<present::Win>,
    pub lost: bool,
    /// 在 display 级纹理共享组里（share.rs：WebGL 拿 2D 画布当纹理时 GPU 上直接拷）
    pub shared: bool,
    /// share.rs 拷进 WebGL 纹理用的帧缓冲
    copy_fbo: Option<glow::NativeFramebuffer>,
    pub backend: &'static str,
    /// 诊断：本进程画过的 draw call、传过的临时层、读回次数
    pub draws: u64,
    pub tile_uploads: u64,
    pub readbacks: u64,
    /// GPU 上直接拷给 WebGL 纹理的次数（share.rs）
    pub share_copies: u64,
    /// 慢帧归因（`MINI_GAME_SLOW_MS`）每 2 秒一行的计数：(draw call, 临时层字节, 裁剪蒙版次数, 裁剪蒙版字节, 读回, 直拷, 帧,
    /// 提交批次的耗时 ns, 顶点字节)
    pub period: [u64; 9],
}

const TILE_RING: usize = 8;

impl Gpu2d {
    /// 按 WebGL 同一套后端顺序建上下文（`pref` = MINI_WEBGL 的取值，默认 auto）
    pub fn new(pref: &str, headless: bool) -> Result<Self, String> {
        let mut why = Vec::new();
        for b in egl::order(pref, headless) {
            let d = match egl::display(b) {
                Ok(d) => d,
                Err(e) => {
                    why.push(e);
                    continue;
                }
            };
            match Self::on(d) {
                Ok(g) => return Ok(g),
                Err(e) => why.push(format!("{}: {e}", b.name())),
            }
        }
        Err(if why.is_empty() { "没有可用的 EGL 后端".into() } else { why.join("；") })
    }

    fn on(d: &'static Display) -> Result<Self, String> {
        let want = Want { alpha: true, depth: false, stencil: false, antialias: false, es3: false };
        let (cfg, _) = egl::choose_config(d, &want).ok_or("没有合用的 EGL config")?;
        let (ctx, shared) = egl::create_context_ex(d, cfg, false, false, true)?;
        let pb = egl::create_pbuffer(d, cfg, 1, 1)?;
        d.egl.make_current(d.dpy, Some(pb), Some(pb), Some(ctx)).map_err(|e| format!("eglMakeCurrent: {e}"))?;
        egl::set_owner(egl::OWNER_2D);
        let gl = unsafe { glow::Context::from_loader_function(|s| egl::proc_address(d, s)) };
        let prog = unsafe { gl::Prog::new(&gl)? };
        Ok(Self { d, ctx, pb, cfg, gl, prog, surfs: HashMap::new(), verts: Vec::new(), key: None, tiles: Vec::new(), tile_next: 0,
            images: HashMap::new(), canvas_src: HashMap::new(), clip: None, grad: None, grad_serial: 0, scratch: Vec::new(), window: None, lost: false, shared, copy_fbo: None,
            backend: d.backend.name(), draws: 0, tile_uploads: 0, readbacks: 0, share_copies: 0, period: [0; 9] })
    }

    /// 让本上下文成为 current（WebGL 切走过就切回来）。失败 = 上下文没了
    fn cur(&mut self) -> bool {
        if self.lost {
            return false;
        }
        if egl::owner() != egl::OWNER_2D {
            let s = self.window.as_ref().map(|w| w.surface).unwrap_or(self.pb);
            if self.d.egl.make_current(self.d.dpy, Some(s), Some(s), Some(self.ctx)).is_err() {
                self.lost = true;
                egl::set_owner(0);
                return false;
            }
            egl::set_owner(egl::OWNER_2D);
        }
        true
    }

    /// 确保画布 `id` 有一块 w×h 的纹理（尺寸变了重建、清成透明）。返回 false = 上下文没了 / 建不了
    pub fn ensure(&mut self, id: u32, w: u32, h: u32) -> bool {
        if !self.cur() {
            return false;
        }
        let (w, h) = (w.max(1), h.max(1));
        if self.surfs.get(&id).map(|s| (s.w, s.h)) == Some((w, h)) {
            return true;
        }
        self.flush();
        self.free(id);
        let gl = &self.gl;
        let r = unsafe {
            (|| -> Result<Surf, String> {
                let tex = gl::new_texture(gl, glow::NEAREST)?;
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w as i32, h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
                let fbo = gl.create_framebuffer()?;
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
                gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
                if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                    return Err(format!("帧缓冲不完整（{w}×{h}）"));
                }
                gl.viewport(0, 0, w as i32, h as i32);
                gl.disable(glow::SCISSOR_TEST);
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                gl.clear(glow::COLOR_BUFFER_BIT);
                Ok(Surf { fbo, tex, w, h })
            })()
        };
        match r {
            Ok(s) => {
                self.surfs.insert(id, s);
                true
            }
            Err(_) => false,
        }
    }

    pub fn has(&self, id: u32) -> bool {
        self.surfs.contains_key(&id)
    }

    pub fn free(&mut self, id: u32) {
        if let Some(s) = self.surfs.remove(&id) {
            if self.cur() {
                self.flush();
                unsafe {
                    self.gl.delete_framebuffer(s.fbo);
                    self.gl.delete_texture(s.tex);
                }
            }
        }
        if let Some((t, _)) = self.canvas_src.remove(&id) {
            if self.cur() {
                unsafe { self.gl.delete_texture(t) };
            }
        }
    }

    fn begin(&mut self, key: Key) {
        if self.key != Some(key) || self.verts.len() >= gl::MAX_QUADS * 4 * gl::FLOATS {
            self.flush();
            self.key = Some(key);
        }
    }

    /// 渐变的编号（参数与上一次相同就沿用，连续的渐变绘制能合成一批）
    fn grad_key(&mut self, g: Option<&gl::Grad>) -> u32 {
        let Some(g) = g else { return 0 };
        if let Some((cur, k)) = &self.grad {
            if cur == g {
                return *k;
            }
        }
        self.flush();
        self.grad_serial = self.grad_serial.wrapping_add(1).max(1);
        unsafe { self.prog.set_grad(&self.gl, g) };
        self.grad = Some((g.clone(), self.grad_serial));
        self.grad_serial
    }

    /// 把攒着的顶点画掉
    pub fn flush(&mut self) {
        let Some(key) = self.key else { return };
        if self.verts.is_empty() || !self.cur() {
            self.verts.clear();
            return;
        }
        let Some(s) = self.surfs.get(&key.target) else {
            self.verts.clear();
            return;
        };
        let gl = &self.gl;
        let t0 = std::time::Instant::now();
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(s.fbo));
            gl.viewport(0, 0, s.w as i32, s.h as i32);
            gl.disable(glow::SCISSOR_TEST);
            gl.enable(glow::BLEND);
            let (fa, fb) = gl::blend_func(key.blend);
            gl.blend_func(fa, fb);
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, self.clip.as_ref().filter(|_| key.clip.is_some()).map(|c| c.1));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, key.tex);
            if key.tex.is_some() {
                gl::set_filter(gl, key.filter);
            }
            self.prog.draw(gl, &self.verts, (s.w, s.h), key.clip.is_some());
        }
        self.draws += 1;
        self.period[0] += 1;
        self.period[7] += t0.elapsed().as_nanos() as u64;
        self.period[8] += (self.verts.len() * 4) as u64;
        self.verts.clear();
    }

    /// 裁剪蒙版（整块画布大小、行 0 在上）传成纹理，同一个 Arc 只传一次
    fn clip_key(&mut self, clip: Option<&Arc<Vec<u8>>>, w: u32, h: u32) -> Option<usize> {
        let m = clip?;
        if m.len() < (w as usize) * (h as usize) {
            return None;
        }
        let k = Arc::as_ptr(m) as usize;
        if self.clip.as_ref().map(|c| Arc::ptr_eq(&c.0, m)) == Some(true) {
            return Some(k);
        }
        self.flush();
        let gl = &self.gl;
        let t = match self.clip.take() {
            Some((_, t)) => t,
            None => unsafe { gl::new_texture(gl, glow::NEAREST).ok()? },
        };
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(t));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::ALPHA as i32, w as i32, h as i32, 0, glow::ALPHA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(&m[..(w * h) as usize])));
        }
        self.clip = Some((m.clone(), t));
        self.period[2] += 1;
        self.period[3] += (w * h) as u64;
        Some(k)
    }

    fn size(&self, id: u32) -> Option<(u32, u32)> {
        self.surfs.get(&id).map(|s| (s.w, s.h))
    }

    /// 轴对齐抗锯齿矩形 [l, r) × [t, b)（设备 px），颜色非预乘（a 已乘 globalAlpha，0~255 取整过）
    /// `grad` 给了：颜色取渐变（种类 4），`c` 不用，`galpha` 是 globalAlpha（浮点，与 CPU 的 fx 合成一致）
    #[allow(clippy::too_many_arguments)]
    pub fn rect(&mut self, id: u32, (l, t, r, b): (f32, f32, f32, f32), c: Color, grad: Option<&gl::Grad>, galpha: f32, blend: GpuBlend, clip: Option<&Arc<Vec<u8>>>) {
        let Some((w, h)) = self.size(id) else { return };
        if !self.cur() {
            return;
        }
        let clip = self.clip_key(clip, w, h);
        let gk = self.grad_key(grad);
        self.begin(Key { target: id, tex: None, filter: glow::NEAREST, blend, clip, grad: gk });
        // 清形状外像素的模式铺满整块画布（框外覆盖率 0、源为 0）；其余只画矩形外扩 1px（光栅器左右各留 1px）
        let (x0, y0, x1, y1) = if blend.clears_outside() {
            (0.0, 0.0, w as f32, h as f32)
        } else {
            ((l - 1.0).floor().max(0.0), (t - 1.0).floor().max(0.0), (r + 1.0).ceil().min(w as f32), (b + 1.0).ceil().min(h as f32))
        };
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let col = if gk != 0 { [1.0, 1.0, 1.0, galpha] } else { [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, c.a as f32 / 255.0] };
        gl::push_quad(&mut self.verts, [(x0, y0), (x1, y0), (x1, y1), (x0, y1)], [(0.0, 0.0); 4], col, [l, r, t, b], &gl::NO_EDGE, if gk != 0 { 4.0 } else { 0.0 });
    }

    /// 整数像素框写成透明（clearRect）
    pub fn clear(&mut self, id: u32, (x0, y0, x1, y1): (u32, u32, u32, u32)) {
        if x1 <= x0 || y1 <= y0 || !self.cur() || !self.has(id) {
            return;
        }
        self.begin(Key { target: id, tex: None, filter: glow::NEAREST, blend: GpuBlend::Copy, clip: None, grad: 0 });
        let (a, b, c, d) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32);
        gl::push_quad(&mut self.verts, [(a, b), (c, b), (c, d), (a, d)], [(0.0, 0.0); 4], [0.0; 4], [a, c, b, d], &gl::NO_EDGE, 0.0);
    }

    /// 整张读回：预乘 RGBA，行 0 = 画布顶行
    pub fn read(&mut self, id: u32, out: &mut [Color]) -> bool {
        let Some(&Surf { fbo, w, h, .. }) = self.surfs.get(&id) else { return false };
        if !self.cur() || out.len() < (w * h) as usize {
            return false;
        }
        self.flush();
        self.scratch.resize((w * h * 4) as usize, 0);
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            self.gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            self.gl.read_pixels(0, 0, w as i32, h as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut self.scratch)));
        }
        self.readbacks += 1;
        self.period[4] += 1;
        for (o, p) in out.iter_mut().zip(self.scratch.chunks_exact(4)) {
            *o = unpremul(p);
        }
        true
    }

    /// 整张写入（CPU 后备缓冲 → 纹理，预乘）
    pub fn write(&mut self, id: u32, px: &[Color]) -> bool {
        let Some(&Surf { tex, w, h, .. }) = self.surfs.get(&id) else { return false };
        if !self.cur() || px.len() < (w * h) as usize {
            return false;
        }
        self.flush();
        self.scratch.clear();
        self.scratch.extend(px.iter().flat_map(|c| premul(*c)));
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            self.gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, w as i32, h as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(&self.scratch)));
        }
        true
    }

    /// 帧末：本上下文正好是 current 时把攒着的画掉、提交，顺手丢掉已回收图片的纹理。
    ///
    /// **不为这个切上下文**：ANGLE 的 Metal 后端切走一个上下文时要等它的命令缓冲排进队列（unMakeCurrent →
    /// flush + waitUntilScheduled），主画布是 WebGL 时每帧在这里切一次 2D、上屏前再切回去，那一下等待落进了
    /// WebGL 的 swap：iPhone 上 present 从 0.1ms 变成 6ms，热起来 10~14ms、掉到 50fps（farm001 / 萌宠小岛）。
    /// 攒着的顶点也不会丢：用 2D 像素的每条路（上屏、拷给 WebGL、读回）都先 flush
    pub fn end_frame(&mut self) {
        if egl::owner() != egl::OWNER_2D || self.lost {
            return;
        }
        self.flush();
        if self.images.len() > 64 {
            let dead: Vec<usize> = self.images.iter().filter(|(_, t)| t.src.strong_count() == 0).map(|(k, _)| *k).collect();
            for k in dead {
                if let Some(t) = self.images.remove(&k) {
                    unsafe { self.gl.delete_texture(t.tex) };
                }
            }
        }
        unsafe { self.gl.flush() };
    }
}

fn premul(c: Color) -> [u8; 4] {
    let a = c.a as u32;
    let m = |v: u8| ((v as u32 * a + 127) / 255) as u8;
    [m(c.r), m(c.g), m(c.b), c.a]
}

fn unpremul(p: &[u8]) -> Color {
    let a = p[3] as u32;
    if a == 0 {
        return Color::TRANSPARENT;
    }
    let u = |v: u8| ((v as u32 * 255 + a / 2) / a).min(255) as u8;
    Color::new(u(p[0]), u(p[1]), u(p[2]), p[3])
}

impl Drop for Gpu2d {
    fn drop(&mut self) {
        let d = self.d;
        if let Some(w) = self.window.take() {
            let _ = d.egl.destroy_surface(d.dpy, w.surface);
        }
        let _ = d.egl.make_current(d.dpy, None, None, None);
        egl::set_owner(0);
        let _ = d.egl.destroy_surface(d.dpy, self.pb);
        let _ = d.egl.destroy_context(d.dpy, self.ctx);
    }
}
