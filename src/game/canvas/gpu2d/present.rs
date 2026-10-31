//! GPU 2D 主画布直接上屏：EGL 窗口 surface 建在宿主给的原生窗口上（Android 的 ANativeWindow、iOS 的 CAMetalLayer），
//! 主画布的纹理铺满、翻 y、压黑底（与 CPU 的 `to_rgba_opaque` 同一个约定），覆盖层（Toast / Modal，预乘）叠在上面，swap。
//! 与 WebGL 的直接上屏（webgl/window.rs）同一个宿主约定：`geometry(w, h)` 设 / 核对缓冲尺寸，返回 -1 让宿主改图层尺寸。

use super::{gl, Gpu2d};
use crate::game::webgl::egl;
use glow::HasContext;
use khronos_egl as k;

pub struct Win {
    win: usize,
    pub(super) surface: k::Surface,
    overlay: Option<glow::NativeTexture>,
}

impl Gpu2d {
    /// 主画布 `main` 上屏到 `win`。`overlay` = 覆盖层（预乘 RGBA，合成尺寸）。
    /// 返回 None = 这块窗口用不了（建不了 surface），调用方把主画布降回 CPU
    pub fn present_window(&mut self, main: u32, win: usize, overlay: Option<(u32, u32, Vec<u8>)>, geometry: &mut dyn FnMut(u32, u32) -> bool) -> Option<i32> {
        let (w, h, tex) = self.surfs.get(&main).map(|s| (s.w, s.h, s.tex))?;
        if !self.cur() {
            return None;
        }
        if self.window.as_ref().map(|x| x.win) != Some(win) {
            self.detach_window();
            if !geometry(w, h) {
                return Some(-1);
            }
            // SAFETY: win 是宿主挂上的原生窗口 / 图层，detach 之前有效
            let s = unsafe { self.d.egl.create_window_surface(self.d.dpy, self.cfg, win as k::NativeWindowType, None) }.ok()?;
            if self.d.egl.make_current(self.d.dpy, Some(s), Some(s), Some(self.ctx)).is_err() {
                let _ = self.d.egl.destroy_surface(self.d.dpy, s);
                return None;
            }
            egl::set_owner(egl::OWNER_2D);
            self.window = Some(Win { win, surface: s, overlay: None });
        } else if !geometry(w, h) {
            return Some(-1);
        }
        self.flush();
        let s = self.window.as_ref()?.surface;
        let gl = &self.gl;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, w as i32, h as i32);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl::set_filter(gl, glow::NEAREST);
            self.prog.blit(gl, true, 1);
            if let Some((ow, oh, px)) = overlay.filter(|o| o.0 > 0 && o.1 > 0) {
                let t = match self.window.as_ref().and_then(|x| x.overlay) {
                    Some(t) => t,
                    None => gl::new_texture(gl, glow::LINEAR).ok()?,
                };
                if let Some(x) = self.window.as_mut() {
                    x.overlay = Some(t);
                }
                gl.bind_texture(glow::TEXTURE_2D, Some(t));
                gl::upload_rgba(gl, ow, oh, &px);
                gl.enable(glow::BLEND);
                gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
                self.prog.blit(gl, true, 0);
            }
        }
        self.draws += 1;
        // 下一批画回 FBO 时要重新绑（flush 每次都绑），这里不用恢复
        self.key = None;
        match self.d.egl.swap_buffers(self.d.dpy, s) {
            Ok(()) => Some(1),
            Err(e) => {
                if e == k::Error::ContextLost {
                    self.lost = true;
                }
                Some(-2)
            }
        }
    }

    /// 宿主的窗口 / 图层要没了：surface 拆掉，回到 pbuffer
    pub fn detach_window(&mut self) {
        let Some(w) = self.window.take() else { return };
        let d = self.d;
        if !self.lost && d.egl.make_current(d.dpy, Some(self.pb), Some(self.pb), Some(self.ctx)).is_ok() {
            egl::set_owner(egl::OWNER_2D);
            if let Some(t) = w.overlay {
                unsafe { self.gl.delete_texture(t) };
            }
        }
        let _ = d.egl.destroy_surface(d.dpy, w.surface);
    }

    pub fn window_attached(&self) -> bool {
        self.window.is_some()
    }
}
