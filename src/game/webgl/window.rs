//! 主画布是 WebGL 时直接上屏（design-webgl.md §5）：EGL 窗口 surface 建在宿主的 ANativeWindow 上，
//! 当主上下文的默认帧缓冲，帧末 `eglSwapBuffers`，不读回、不经 CPU。
//!
//! 一加 PLY110 实测（读回那条路）：1270×2770 每帧 32ms、427×931 每帧 8.6ms，都过不了 60fps —— 读回本身就是瓶颈。
//!
//! 什么时候还走读回：
//! - 覆盖层弹着 / 引擎内旋转：输出帧要在 CPU 上合成。读回之后由一个辅助上下文把合成好的 RGBA 当纹理画进同一块窗口 surface
//!   再 swap（不动游戏上下文的任何 GL 状态）；
//! - 窗口第一次接上的那一帧（窗口 surface 里还没东西）；
//! - 这块窗口已经被 CPU 锁过（`ANativeWindow_lock` 连上了 CPU 生产者，EGL 再也连不上）：一直读回，直到宿主换一块 surface；
//! - `preserveDrawingBuffer: true` 而 config 不支持 EGL_BUFFER_PRESERVED。

use super::egl;
use super::WebGl;
use glow::HasContext;
use khronos_egl as k;

pub struct Window {
    win: usize,
    surface: k::Surface,
    helper: Option<Helper>,
    /// 刚接上，还没画过：这一帧走读回 + 辅助上下文
    pub fresh: bool,
    /// 辅助上下文的 last 纹理里存着最近一帧游戏画面（覆盖层弹着时才存）
    have_last: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Blit {
    /// 合成好的整帧（读回那条路）
    Frame,
    /// 游戏刚画完：存一份进 last，再叠覆盖层
    OverlayOnNew,
    /// 游戏没画：先画 last，再叠覆盖层（覆盖层空 = 只画 last，收起覆盖层的那一帧）
    OverlayOnLast,
}

struct Helper {
    ctx: k::Context,
    gl: glow::Context,
    prog: glow::NativeProgram,
    /// 1 = 纹理行 0 在上（CPU 给的 RGBA），0 = GL 自己的（copyTexImage 来的 last）
    flip: glow::NativeUniformLocation,
    tex: glow::NativeTexture,
    last: glow::NativeTexture,
    buf: glow::NativeBuffer,
}

impl WebGl {
    /// 能不能直接上屏（窗口 surface 已经当着默认帧缓冲、不是刚接上）
    pub fn window_mode(&self) -> bool {
        self.window.as_ref().map(|w| !w.fresh).unwrap_or(false)
    }

    /// nativeBlit：主画布是 WebGL 时接管上屏。`cpu_locked` = 这块窗口被 CPU 锁过；`rgba` 取引擎合成好的输出帧。
    /// 返回 None = 不归 WebGL 管（走 CPU 上屏）
    /// `pixels(false)` = 合成好的整帧 RGBA，`pixels(true)` = 只有覆盖层（预乘，overlay_layer.rs）
    pub fn present_window(&mut self, main: u32, win: usize, cpu_locked: bool, rgba_pending: bool, overlay: bool, pixels: &mut dyn FnMut(bool) -> Option<(u32, u32, Vec<u8>)>, geometry: &mut dyn FnMut(u32, u32) -> bool) -> Option<i32> {
        let c = self.ctxs.get(&main)?;
        if c.lost || cpu_locked || win == 0 || self.window_blocked == win {
            return None;
        }
        if self.window.as_ref().map(|w| w.win) != Some(win) {
            self.detach_window();
            let c = self.ctxs.get(&main)?;
            if c.attrs.preserve_drawing_buffer && !preserve_ok(c) {
                self.window_blocked = win;
                return None;
            }
            if !geometry(c.w, c.h) {
                return None;
            }
            // SAFETY: win 是宿主挂上的 ANativeWindow（ffi 持有一个引用，detach 之前有效）
            let s = match unsafe { c.d.egl.create_window_surface(c.d.dpy, c.config(), win as k::NativeWindowType, None) } {
                Ok(s) => s,
                Err(e) => {
                    self.window_blocked = win;
                    self.notes.push(format!("⚠️ WebGL 建窗口 surface 失败（{e}），这块 surface 走读回上屏"));
                    return None;
                }
            };
            if c.attrs.preserve_drawing_buffer {
                let _ = c.d.egl.surface_attrib(c.d.dpy, s, k::SWAP_BEHAVIOR, k::BUFFER_PRESERVED);
            }
            self.notes.push(format!("🧊 WebGL 直接上屏：EGL 窗口 surface {}×{}", c.w, c.h));
            self.window = Some(Window { win, surface: s, helper: None, fresh: true, have_last: false });
            self.current = None;
            let c = self.ctxs.get_mut(&main)?;
            c.bind_window(Some(s));
            self.current = Some(main);
        }
        let fresh = self.window.as_ref()?.fresh;
        if fresh || rgba_pending {
            let (w, h, px) = pixels(false)?;
            if !geometry(w, h) {
                return Some(-1);
            }
            let r = self.blit(main, w, h, &px, Blit::Frame);
            if let Some(win) = self.window.as_mut() {
                win.fresh = false;
                win.have_last = false;
            }
            self.game_frame_new = false;
            return Some(r);
        }
        let c = self.ctxs.get(&main)?;
        if !geometry(c.w, c.h) {
            return Some(-1);
        }
        let new_frame = std::mem::take(&mut self.game_frame_new);
        let have_last = self.window.as_ref()?.have_last;
        if overlay || (!new_frame && have_last) {
            // 覆盖层在 GPU 上叠：游戏这一帧画过就先存一份（覆盖层动、游戏不动的帧要拿它重画），没画就用存的那份
            let (w, h, px) = if overlay { pixels(true)? } else { (0, 0, Vec::new()) };
            let how = if new_frame { Blit::OverlayOnNew } else if have_last { Blit::OverlayOnLast } else { return Some(0) };
            let r = self.blit(main, w, h, &px, how);
            if let Some(win) = self.window.as_mut() {
                win.have_last = overlay;
            }
            return Some(r);
        }
        if !new_frame {
            // 游戏这一帧没画：窗口的后台缓冲内容未定义，不 swap
            return Some(0);
        }
        let s = self.window.as_ref()?.surface;
        match c.d.egl.swap_buffers(c.d.dpy, s) {
            Ok(()) => Some(1),
            Err(e) => {
                self.notes.push(format!("⚠️ eglSwapBuffers：{e}"));
                if e == k::Error::ContextLost {
                    self.lose(main);
                }
                Some(-2)
            }
        }
    }

    /// 宿主的 surface 要没了：窗口 surface 拆掉，主上下文回到 pbuffer
    pub fn detach_window(&mut self) {
        let Some(w) = self.window.take() else { return };
        let ids: Vec<u32> = self.ctxs.keys().copied().collect();
        let mut d = None;
        for id in ids {
            if let Some(c) = self.ctxs.get_mut(&id) {
                d = Some(c.d);
                c.bind_window(None);
            }
        }
        self.current = None;
        if let Some(d) = d {
            if let Some(h) = w.helper {
                let _ = d.egl.destroy_context(d.dpy, h.ctx);
            }
            let _ = d.egl.destroy_surface(d.dpy, w.surface);
        }
        self.window_blocked = 0;
    }

    /// 辅助上下文往窗口 surface 上画再 swap（不碰游戏上下文的 GL 状态）
    fn blit(&mut self, main: u32, w: u32, h: u32, px: &[u8], how: Blit) -> i32 {
        let Some(c) = self.ctxs.get(&main) else { return 0 };
        let (d, cfg, cw, ch) = (c.d, c.config(), c.w, c.h);
        let Some(win) = self.window.as_mut() else { return 0 };
        if win.helper.is_none() {
            match helper(d, cfg) {
                Ok(h) => win.helper = Some(h),
                Err(e) => {
                    self.notes.push(format!("⚠️ WebGL 上屏辅助上下文：{e}"));
                    return -2;
                }
            }
        }
        let hp = win.helper.as_ref().unwrap();
        let s = win.surface;
        if d.egl.make_current(d.dpy, Some(s), Some(s), Some(hp.ctx)).is_err() {
            return -2;
        }
        unsafe {
            let gl = &hp.gl;
            gl.disable(glow::BLEND);
            gl.use_program(Some(hp.prog));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(hp.buf));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            match how {
                Blit::Frame => {
                    gl.viewport(0, 0, w as i32, h as i32);
                    gl.uniform_1_f32(Some(&hp.flip), 1.0);
                    upload(gl, hp.tex, w, h, px);
                    gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                }
                Blit::OverlayOnNew | Blit::OverlayOnLast => {
                    gl.viewport(0, 0, cw as i32, ch as i32);
                    gl.bind_texture(glow::TEXTURE_2D, Some(hp.last));
                    if how == Blit::OverlayOnNew {
                        // 后台缓冲（游戏刚画完的这一帧）→ last；GL 坐标原点在左下，画回去时不翻
                        gl.copy_tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA, 0, 0, cw as i32, ch as i32, 0);
                    } else {
                        gl.uniform_1_f32(Some(&hp.flip), 0.0);
                        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    }
                    if !px.is_empty() {
                        gl.enable(glow::BLEND);
                        gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
                        gl.uniform_1_f32(Some(&hp.flip), 1.0);
                        upload(gl, hp.tex, w, h, px);
                        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                    }
                }
            }
        }
        let r = d.egl.swap_buffers(d.dpy, s);
        // 游戏上下文换回来（下一帧接着往窗口 surface 上画）
        self.current = None;
        let _ = self.current(main);
        if r.is_ok() { 1 } else { -2 }
    }
}

/// RGBA 传进纹理（行 0 在上：着色器按 flip 翻）
unsafe fn upload(gl: &glow::Context, t: glow::NativeTexture, w: u32, h: u32, px: &[u8]) {
    gl.bind_texture(glow::TEXTURE_2D, Some(t));
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w as i32, h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(px)));
}

fn preserve_ok(c: &super::ctx::GlCtx) -> bool {
    c.d.egl.get_config_attrib(c.d.dpy, c.config(), k::SURFACE_TYPE).map(|v| v & k::SWAP_BEHAVIOR_PRESERVED_BIT != 0).unwrap_or(false)
}

fn helper(d: &'static egl::Display, cfg: k::Config) -> Result<Helper, String> {
    let ctx = egl::create_context(d, cfg, false, false)?;
    // 先在一块 1×1 的 pbuffer 上建资源（窗口 surface 由调用方随后绑上）
    let pb = egl::create_pbuffer(d, cfg, 1, 1)?;
    d.egl.make_current(d.dpy, Some(pb), Some(pb), Some(ctx)).map_err(|e| e.to_string())?;
    let gl = unsafe { glow::Context::from_loader_function(|s| egl::proc_address(d, s)) };
    let r = unsafe {
        let sh = |t: u32, src: &str| -> Result<glow::NativeShader, String> {
            let s = gl.create_shader(t)?;
            gl.shader_source(s, src);
            gl.compile_shader(s);
            if gl.get_shader_compile_status(s) { Ok(s) } else { Err(gl.get_shader_info_log(s)) }
        };
        let vs = sh(glow::VERTEX_SHADER, "attribute vec2 p; uniform float flip; varying vec2 uv; void main(){ uv = vec2(p.x * 0.5 + 0.5, mix(p.y * 0.5 + 0.5, 0.5 - p.y * 0.5, flip)); gl_Position = vec4(p, 0.0, 1.0); }")?;
        let fs = sh(glow::FRAGMENT_SHADER, "precision mediump float; varying vec2 uv; uniform sampler2D t; void main(){ gl_FragColor = texture2D(t, uv); }")?;
        let prog = gl.create_program()?;
        gl.attach_shader(prog, vs);
        gl.attach_shader(prog, fs);
        gl.bind_attrib_location(prog, 0, "p");
        gl.link_program(prog);
        gl.use_program(Some(prog));
        let flip = gl.get_uniform_location(prog, "flip").ok_or("flip uniform")?;
        // 覆盖层是合成尺寸（≤ 2 倍逻辑尺寸），拉到画布尺寸用线性；last 与窗口同尺寸，最近邻
        let mk = |filter: u32| -> Result<glow::NativeTexture, String> {
            let t = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(t));
            for (p, v) in [(glow::TEXTURE_MIN_FILTER, filter), (glow::TEXTURE_MAG_FILTER, filter), (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE), (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE)] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
            }
            Ok(t)
        };
        let tex = mk(glow::LINEAR)?;
        let last = mk(glow::NEAREST)?;
        let buf = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buf));
        let q: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, std::slice::from_raw_parts(q.as_ptr() as *const u8, 32), glow::STATIC_DRAW);
        Ok::<_, String>((prog, flip, tex, last, buf))
    };
    let _ = d.egl.make_current(d.dpy, None, None, None);
    let _ = d.egl.destroy_surface(d.dpy, pb);
    let (prog, flip, tex, last, buf) = r?;
    Ok(Helper { ctx, gl, prog, flip, tex, last, buf })
}
