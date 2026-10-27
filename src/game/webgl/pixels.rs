//! 像素搬运：读回上屏、texImage 的尺寸核算与来源转换（design-webgl.md §2、§3）。
//!
//! 尺寸一律自己算：传给 GL 的是 JS 缓冲的指针，GL 不知道它有多长 —— 数据比 宽×高×每像素 短就是越界读
//! （readPixels 是越界写）。WebGL 规定这种情况报 INVALID_OPERATION，这里先判再下发。

use super::ctx::GlCtx;
use crate::game::canvas::Slot;
use glow::HasContext;

/// 每像素分量数（format）
pub fn components(format: u32) -> Option<usize> {
    Some(match format {
        glow::ALPHA | glow::LUMINANCE | glow::RED | glow::RED_INTEGER | glow::DEPTH_COMPONENT | glow::DEPTH_STENCIL => 1,
        glow::LUMINANCE_ALPHA | glow::RG | glow::RG_INTEGER => 2,
        glow::RGB | glow::RGB_INTEGER | 0x8C40 /* SRGB_EXT */ => 3,
        glow::RGBA | glow::RGBA_INTEGER | 0x8C42 /* SRGB_ALPHA_EXT */ => 4,
        _ => return None,
    })
}

/// 每像素字节数（format × type；打包类型整像素算）
pub fn bytes_per_pixel(format: u32, ty: u32) -> Option<usize> {
    let packed = match ty {
        glow::UNSIGNED_SHORT_5_6_5 | glow::UNSIGNED_SHORT_4_4_4_4 | glow::UNSIGNED_SHORT_5_5_5_1 => Some(2),
        glow::UNSIGNED_INT_2_10_10_10_REV | glow::UNSIGNED_INT_10F_11F_11F_REV | glow::UNSIGNED_INT_5_9_9_9_REV | glow::UNSIGNED_INT_24_8 => Some(4),
        glow::FLOAT_32_UNSIGNED_INT_24_8_REV => Some(8),
        _ => None,
    };
    if packed.is_some() {
        return packed;
    }
    let size = match ty {
        glow::UNSIGNED_BYTE | glow::BYTE => 1,
        glow::UNSIGNED_SHORT | glow::SHORT | glow::HALF_FLOAT | 0x8D61 => 2,
        glow::UNSIGNED_INT | glow::INT | glow::FLOAT => 4,
        _ => return None,
    };
    Some(components(format)? * size)
}

/// 像素存储参数（解包或打包那一侧）
#[derive(Clone, Copy)]
pub struct Store {
    pub alignment: i32,
    pub row_length: i32,
    pub image_height: i32,
    pub skip_pixels: i32,
    pub skip_rows: i32,
    pub skip_images: i32,
}

impl Store {
    pub fn packed() -> Self {
        Self { alignment: 1, row_length: 0, image_height: 0, skip_pixels: 0, skip_rows: 0, skip_images: 0 }
    }
}

/// GL 会从数据里读（或往里写）多少字节：ES 3.0 §3.8.3 的算法（最后一行不补齐）
pub fn image_bytes(w: i32, h: i32, d: i32, bpp: usize, s: &Store) -> Option<usize> {
    if w < 0 || h < 0 || d < 0 {
        return None;
    }
    if w == 0 || h == 0 || d == 0 {
        return Some(0);
    }
    let a = s.alignment.max(1) as usize;
    let row_len = if s.row_length > 0 { s.row_length } else { w } as usize;
    let stride = (row_len * bpp).div_ceil(a) * a;
    let img_h = if s.image_height > 0 { s.image_height } else { h } as usize;
    let rows = (s.skip_images.max(0) as usize + d as usize - 1) * img_h + s.skip_rows.max(0) as usize + h as usize - 1;
    rows.checked_mul(stride)?.checked_add((s.skip_pixels.max(0) as usize + w as usize) * bpp)
}

/// 主画布 / 当作源的画布：默认帧缓冲读回后备缓冲（行翻转；预乘的还原成直通 alpha；alpha:false 补成不透明）
pub fn read_back(c: &mut GlCtx, slot: &mut Slot) {
    if slot.w == 0 || slot.h == 0 {
        return;
    }
    // 上屏之后还没再画过（preserveDrawingBuffer=false）：后备缓冲里就是上屏的那一帧，保持不动
    if c.needs_clear && !c.drew {
        return;
    }
    let (w, h) = (c.w.min(slot.w), c.h.min(slot.h));
    let n = (w * h * 4) as usize;
    let mut buf = std::mem::take(&mut c.scratch);
    buf.resize(n, 0);
    let gl = &c.gl;
    let es3 = c.version == 2;
    unsafe {
        let read_target = if es3 { glow::READ_FRAMEBUFFER } else { glow::FRAMEBUFFER };
        let fb = gl.get_parameter_i32(if es3 { glow::READ_FRAMEBUFFER_BINDING } else { glow::FRAMEBUFFER_BINDING });
        let align = gl.get_parameter_i32(glow::PACK_ALIGNMENT);
        let (row, sp, sr, pbo) = if es3 {
            (gl.get_parameter_i32(glow::PACK_ROW_LENGTH), gl.get_parameter_i32(glow::PACK_SKIP_PIXELS), gl.get_parameter_i32(glow::PACK_SKIP_ROWS), gl.get_parameter_i32(glow::PIXEL_PACK_BUFFER_BINDING))
        } else {
            (0, 0, 0, 0)
        };
        if fb != 0 { gl.bind_framebuffer(read_target, None); }
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 4);
        if es3 {
            gl.pixel_store_i32(glow::PACK_ROW_LENGTH, 0);
            gl.pixel_store_i32(glow::PACK_SKIP_PIXELS, 0);
            gl.pixel_store_i32(glow::PACK_SKIP_ROWS, 0);
            if pbo != 0 { gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None); }
        }
        gl.read_pixels(0, 0, w as i32, h as i32, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut buf)));
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, align);
        if es3 {
            gl.pixel_store_i32(glow::PACK_ROW_LENGTH, row);
            gl.pixel_store_i32(glow::PACK_SKIP_PIXELS, sp);
            gl.pixel_store_i32(glow::PACK_SKIP_ROWS, sr);
            if pbo != 0 { gl.bind_buffer(glow::PIXEL_PACK_BUFFER, super::args::buf(pbo as u32)); }
        }
        if fb != 0 { gl.bind_framebuffer(read_target, super::args::fbo(fb as u32)); }
    }
    let (opaque, premul) = (!c.attrs.alpha, c.attrs.premultiplied_alpha);
    let canvas = slot.ctx.get_canvas();
    if let Ok(mut cv) = canvas.lock() {
        let cw = cv.width() as usize;
        let px = cv.pixels_mut();
        for y in 0..h as usize {
            let src = &buf[(h as usize - 1 - y) * w as usize * 4..][..w as usize * 4];
            let dst = &mut px[y * cw..y * cw + w as usize];
            for (d, s) in dst.iter_mut().zip(src.chunks_exact(4)) {
                let a = if opaque { 255 } else { s[3] };
                let un = |v: u8| if !premul || a == 255 { v } else if a == 0 { 0 } else { ((v as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8 };
                *d = crate::Color::new(un(s[0]), un(s[1]), un(s[2]), a);
            }
        }
    }
    c.scratch = buf;
}

/// DOM 一类的来源（Image / 2D 画布 / ImageData）：直通 alpha 的 RGBA8 → 按 format/type 排好、紧密排列（对齐 1）。
/// 认不得的组合返回 None（INVALID_OPERATION）
pub fn convert_source(rgba: &[u8], w: u32, h: u32, flip_y: bool, premultiply: bool, format: u32, ty: u32) -> Option<Vec<u8>> {
    let bpp = bytes_per_pixel(format, ty)?;
    let mut out = Vec::with_capacity(w as usize * h as usize * bpp);
    for y in 0..h as usize {
        let sy = if flip_y { h as usize - 1 - y } else { y };
        let row = &rgba[sy * w as usize * 4..][..w as usize * 4];
        for p in row.chunks_exact(4) {
            let (mut r, mut g, mut b, a) = (p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32);
            if premultiply && a != 255 {
                r = (r * a + 127) / 255;
                g = (g * a + 127) / 255;
                b = (b * a + 127) / 255;
            }
            let c = [r, g, b, a];
            match ty {
                glow::UNSIGNED_BYTE => match format {
                    glow::RGBA | 0x8C42 => out.extend_from_slice(&[r as u8, g as u8, b as u8, a as u8]),
                    glow::RGB | 0x8C40 => out.extend_from_slice(&[r as u8, g as u8, b as u8]),
                    glow::LUMINANCE | glow::RED => out.push(r as u8),
                    glow::ALPHA => out.push(a as u8),
                    glow::LUMINANCE_ALPHA => out.extend_from_slice(&[r as u8, a as u8]),
                    glow::RG => out.extend_from_slice(&[r as u8, g as u8]),
                    _ => return None,
                },
                glow::UNSIGNED_SHORT_5_6_5 => out.extend_from_slice(&(((r >> 3) << 11 | (g >> 2) << 5 | (b >> 3)) as u16).to_le_bytes()),
                glow::UNSIGNED_SHORT_4_4_4_4 => out.extend_from_slice(&(((r >> 4) << 12 | (g >> 4) << 8 | (b >> 4) << 4 | (a >> 4)) as u16).to_le_bytes()),
                glow::UNSIGNED_SHORT_5_5_5_1 => out.extend_from_slice(&(((r >> 3) << 11 | (g >> 3) << 6 | (b >> 3) << 1 | (a >> 7)) as u16).to_le_bytes()),
                glow::FLOAT | glow::HALF_FLOAT | 0x8D61 => {
                    let pick: &[usize] = match format {
                        glow::RGBA => &[0, 1, 2, 3],
                        glow::RGB => &[0, 1, 2],
                        glow::LUMINANCE | glow::RED => &[0],
                        glow::ALPHA => &[3],
                        glow::LUMINANCE_ALPHA => &[0, 3],
                        glow::RG => &[0, 1],
                        _ => return None,
                    };
                    for &i in pick {
                        let v = c[i] as f32 / 255.0;
                        if ty == glow::FLOAT { out.extend_from_slice(&v.to_le_bytes()) } else { out.extend_from_slice(&f16(v).to_le_bytes()) }
                    }
                }
                _ => return None,
            }
        }
    }
    Some(out)
}

/// f32 → IEEE half（0~1 的颜色值够用：正数、无溢出）
fn f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let exp = ((x >> 23) & 0xff) as i32 - 127 + 15;
    let man = x & 0x7f_ffff;
    if v == 0.0 {
        return sign;
    }
    if exp <= 0 {
        // 次正规数
        let m = (man | 0x80_0000) >> (1 - exp + 13);
        return sign | m as u16;
    }
    if exp >= 31 {
        return sign | 0x7c00;
    }
    // 四舍五入到最近
    let r = sign | ((exp as u16) << 10) | (man >> 13) as u16;
    if man & 0x1000 != 0 { r + 1 } else { r }
}

/// ArrayBufferView 来源要 FLIP_Y / 预乘时：按当前解包参数读出、改好、排成紧密的（对齐 1）
pub fn repack_view(data: &[u8], w: i32, h: i32, bpp: usize, s: &Store, flip_y: bool, premultiply_rgba8: Option<usize>) -> Vec<u8> {
    let a = s.alignment.max(1) as usize;
    let row_len = if s.row_length > 0 { s.row_length } else { w } as usize;
    let stride = (row_len * bpp).div_ceil(a) * a;
    let row_bytes = w as usize * bpp;
    let mut out = vec![0u8; row_bytes * h as usize];
    for y in 0..h as usize {
        let sy = if flip_y { h as usize - 1 - y } else { y };
        let off = (s.skip_rows.max(0) as usize + sy) * stride + s.skip_pixels.max(0) as usize * bpp;
        out[y * row_bytes..(y + 1) * row_bytes].copy_from_slice(&data[off..off + row_bytes]);
    }
    if let Some(alpha_at) = premultiply_rgba8 {
        for p in out.chunks_exact_mut(bpp) {
            let a = p[alpha_at] as u32;
            for (i, v) in p.iter_mut().enumerate() {
                if i != alpha_at {
                    *v = ((*v as u32 * a + 127) / 255) as u8;
                }
            }
        }
    }
    out
}
