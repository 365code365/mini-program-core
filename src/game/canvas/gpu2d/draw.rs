//! GPU 2D 的纹理四边形：CPU 画好的临时层（tile）、drawImage（image）、源纹理的缓存（src_tex）。
//! 从 mod.rs 纯搬迁出来（单文件 500 行上限），逻辑未改。

use super::{gl, Gpu2d, ImgTex, Key, Src, TILE_RING};
use crate::renderer::components::GpuBlend;
use glow::HasContext;
use std::sync::Arc;

impl Gpu2d {
    /// CPU 画好的一块（非预乘 RGBA，框 [x0, x1) × [y0, y1)）按 `blend` 叠上去；`clears_outside` 的模式铺满整块画布。
    /// `grad` 给了：这一块是白色画的覆盖率，颜色取渐变（种类 5），`alpha` 是 globalAlpha
    #[allow(clippy::too_many_arguments)]
    pub fn tile(&mut self, id: u32, (x0, y0, x1, y1): (usize, usize, usize, usize), rgba: &[u8], grad: Option<&gl::Grad>, alpha: f32, blend: GpuBlend, clip: Option<&Arc<Vec<u8>>>) {
        let Some((w, h)) = self.size(id) else { return };
        if x1 <= x0 || y1 <= y0 || !self.cur() {
            return;
        }
        let clip = self.clip_key(clip, w, h);
        self.flush();
        if self.tiles.len() < TILE_RING {
            match unsafe { gl::new_texture(&self.gl, glow::NEAREST) } {
                Ok(t) => self.tiles.push((t, 0, 0)),
                Err(_) => return,
            }
        }
        let i = self.tile_next % self.tiles.len();
        self.tile_next += 1;
        let (tw, th) = ((x1 - x0) as u32, (y1 - y0) as u32);
        let (t, mut cw, mut ch) = self.tiles[i];
        unsafe {
            self.gl.bind_texture(glow::TEXTURE_2D, Some(t));
            if tw > cw || th > ch {
                // 只增不缩，按 256 取整：同一类绘制（每帧的 HUD 文字）之后都走 texSubImage
                (cw, ch) = (tw.max(cw).next_multiple_of(256), th.max(ch).next_multiple_of(256));
                self.gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, cw as i32, ch as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
                self.tiles[i] = (t, cw, ch);
            }
            self.gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            self.gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, tw as i32, th as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(rgba)));
        }
        let (um, vm) = (tw as f32 / cw as f32, th as f32 / ch as f32);
        self.tile_uploads += 1;
        self.period[1] += rgba.len() as u64;
        let gk = self.grad_key(grad);
        self.begin(Key { target: id, tex: Some(t), filter: glow::NEAREST, blend, clip, grad: gk });
        let (fx0, fy0, fx1, fy1) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32);
        let (qw, qh) = (fx1 - fx0, fy1 - fy0);
        let (p, uv) = if blend.clears_outside() {
            let (cw, ch) = (w as f32, h as f32);
            ([(0.0, 0.0), (cw, 0.0), (cw, ch), (0.0, ch)], [(-fx0 / qw, -fy0 / qh), ((cw - fx0) / qw, -fy0 / qh), ((cw - fx0) / qw, (ch - fy0) / qh), (-fx0 / qw, (ch - fy0) / qh)])
        } else {
            ([(fx0, fy0), (fx1, fy0), (fx1, fy1), (fx0, fy1)], [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)])
        };
        let uv = uv.map(|(u, v)| (u * um, v * vm));
        let a = if gk != 0 { alpha } else { 1.0 };
        gl::push_quad(&mut self.verts, p, uv, [1.0, 1.0, 1.0, a], [0.0, um, 0.0, vm], &gl::NO_EDGE, if gk != 0 { 5.0 } else { 3.0 });
        // 下一张临时层要换纹理：这一批现在就画掉（纹理内容在 draw call 时才被读）
        self.flush();
    }

    /// 源的纹理（图片 / CPU 画布按需传，GPU 画布直接用它的 FBO 纹理）与它是否预乘
    fn src_tex(&mut self, src: &Src) -> Option<(glow::NativeTexture, bool)> {
        match src {
            Src::Gpu(sid) => self.surfs.get(sid).map(|s| (s.tex, true)),
            Src::Image(d) => {
                let key = Arc::as_ptr(d) as usize;
                if let Some(t) = self.images.get(&key).filter(|t| t.src.upgrade().is_some_and(|x| Arc::ptr_eq(&x, d))) {
                    return Some((t.tex, false));
                }
                self.flush();
                let tex = unsafe { gl::new_texture(&self.gl, glow::LINEAR).ok()? };
                unsafe { gl::upload_rgba(&self.gl, d.w, d.h, &d.rgba) };
                if let Some(old) = self.images.insert(key, ImgTex { tex, src: Arc::downgrade(d) }) {
                    unsafe { self.gl.delete_texture(old.tex) };
                }
                Some((tex, false))
            }
            Src::Cpu { id, gen, rgba, w, h } => {
                if let Some((t, g)) = self.canvas_src.get(id) {
                    if g == gen {
                        return Some((*t, false));
                    }
                }
                self.flush();
                let t = match self.canvas_src.get(id) {
                    Some((t, _)) => *t,
                    None => unsafe { gl::new_texture(&self.gl, glow::LINEAR).ok()? },
                };
                unsafe {
                    self.gl.bind_texture(glow::TEXTURE_2D, Some(t));
                    gl::upload_rgba(&self.gl, *w, *h, rgba);
                }
                self.canvas_src.insert(*id, (t, *gen));
                Some((t, false))
            }
        }
    }

    /// drawImage：`quad` 是目标四角（设备 px，左上、右上、右下、左下），`uv` 同序，`inset` = 内缩半纹素的源矩形 (u0, u1, v0, v1)。
    /// `aa`：矩阵有旋转 / 斜切，四条边做抗锯齿（与 CPU 的 image2d::sample_affine 同一个条件）
    #[allow(clippy::too_many_arguments)]
    pub fn image(&mut self, id: u32, src: &Src, quad: [(f32, f32); 4], uv: [(f32, f32); 4], inset: [f32; 4], alpha: f32, smooth: bool, aa: bool, blend: GpuBlend, clip: Option<&Arc<Vec<u8>>>) {
        let Some((w, h)) = self.size(id) else { return };
        if !self.cur() {
            return;
        }
        let Some((tex, premul)) = self.src_tex(src) else { return };
        let clip = self.clip_key(clip, w, h);
        self.begin(Key { target: id, tex: Some(tex), filter: if smooth { glow::LINEAR } else { glow::NEAREST }, blend, clip, grad: 0 });
        let (quad, uv, e) = if aa { gl::aa_quad(quad, uv) } else { (quad, uv, gl::NO_EDGE) };
        gl::push_quad(&mut self.verts, quad, uv, [1.0, 1.0, 1.0, alpha], inset, &e, if premul { 2.0 } else { 1.0 });
    }
}
