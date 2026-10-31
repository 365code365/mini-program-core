//! GPU 2D 画布当 WebGL 纹理源（`texImage2D / texSubImage2D(…, canvas)`）时在 GPU 上直接拷，不读回 CPU。
//!
//! 前提：两边的上下文都建在 display 级纹理共享组里（EGL_ANGLE_display_texture_share_group，egl.rs）——
//! 纹理名在它们之间通用。拷贝在 2D 的上下文里做：把 WebGL 那张纹理挂到本上下文的帧缓冲上，2D 画布的纹理铺满
//! 画进去（按 UNPACK_FLIP_Y 翻、按 UNPACK_PREMULTIPLY_ALPHA 决定要不要去预乘：WebGL 默认要非预乘），然后 glFlush。
//! 同一个线程、同一个 display，flush 之后 WebGL 上下文看到的就是新内容（ANGLE 的 Metal / Vulkan 后端都是一条队列）。
//!
//! 不走这里（照旧读回 + 上传）：没有共享组（Android 系统驱动）、格式不是 RGBA / UNSIGNED_BYTE、mip 级别不是 0
//! （GLES2 的帧缓冲只能挂 0 级）、texSubImage2D 的目标不是本路径定义过尺寸的纹理（越界该报 INVALID_VALUE，
//! 这边量不到纹理尺寸）。

use super::{gl, Gpu2d};
use glow::HasContext;

impl Gpu2d {
    /// 画布 `sid` → WebGL 纹理 `tex` 的 0 级。`at` = None：texImage2D（按画布尺寸重新定义存储）；Some((x, y))：texSubImage2D。
    /// 返回画布尺寸；None = 这条路走不了，调用方读回走 CPU
    pub fn copy_to(&mut self, sid: u32, tex: u32, at: Option<(i32, i32)>, flip: bool, premul: bool) -> Option<(u32, u32)> {
        let (src, w, h) = self.surfs.get(&sid).map(|s| (s.tex, s.w, s.h))?;
        let t = glow::NativeTexture(std::num::NonZeroU32::new(tex)?);
        if !self.shared || !self.cur() {
            return None;
        }
        self.flush();
        let gl = &self.gl;
        let ok = unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(t));
            if at.is_none() {
                gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w as i32, h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
            }
            let fbo = match self.copy_fbo {
                Some(f) => f,
                None => match gl.create_framebuffer() {
                    Ok(f) => f,
                    Err(_) => return None,
                },
            };
            self.copy_fbo = Some(fbo);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(t), 0);
            let ok = gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
            if ok {
                let (x, y) = at.unwrap_or((0, 0));
                gl.viewport(x, y, w as i32, h as i32);
                gl.disable(glow::SCISSOR_TEST);
                gl.disable(glow::BLEND);
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(src));
                gl::set_filter(gl, glow::NEAREST);
                self.prog.blit(gl, flip, if premul { 0 } else { 2 });
            }
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, None, 0);
            gl.flush();
            ok
        };
        // 下一批画回自己的 FBO 时重新绑
        self.key = None;
        self.draws += 1;
        if ok {
            self.share_copies += 1;
            self.period[5] += 1;
            Some((w, h))
        } else {
            None
        }
    }
}
