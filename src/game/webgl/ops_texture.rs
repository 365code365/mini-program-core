//! 纹理：对象、参数、texImage / texSubImage（ArrayBufferView、PBO 偏移、Image / 画布 / ImageData 来源）、压缩纹理、拷贝、存储。
//!
//! 来源是 Image / 画布 / ImageData 时（`_texImage2DSource` 一族）：参数 `(kind, idOrWidth, height, data)`，
//! kind 1 = Image（id）、2 = 画布（id，WebGL 画布由 JS 先 `__game_gl_sync`）、3 = ImageData（宽、高、字节）。

use super::args::{js, null, tex, undef, A};
use super::ctx::GlCtx;
use super::natives::{Env, Op};
use super::pixels::{self, Store};
use crate::js::binary::with_bytes;
use glow::HasContext;
use rquickjs::{IntoJs, Result as JsResult, Value};

pub fn call<'js>(c: &mut GlCtx, e: &Env<'_, 'js>, op: Op, a: &A<'_, 'js>) -> Option<JsResult<Value<'js>>> {
    use Op::*;
    let ctx = &e.ctx;
    unsafe {
        match op {
            createTexture => return Some(c.gl.create_texture().map(|t| t.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteTexture => { if let Some(t) = tex(a.u(0)) { c.gl.delete_texture(t) } }
            bindTexture => c.gl.bind_texture(a.u(0), tex(a.u(1))),
            texParameterf => c.gl.tex_parameter_f32(a.u(0), a.u(1), a.f(2)),
            texParameteri => c.gl.tex_parameter_i32(a.u(0), a.u(1), a.i(2)),
            getTexParameter => {
                let (t, p) = (a.u(0), a.u(1));
                return Some(match p {
                    0x84FE /* TEXTURE_MAX_ANISOTROPY_EXT */ | glow::TEXTURE_MIN_LOD | glow::TEXTURE_MAX_LOD => c.gl.get_tex_parameter_f32(t, p).into_js(ctx),
                    glow::TEXTURE_IMMUTABLE_FORMAT => (c.gl.get_tex_parameter_i32(t, p) != 0).into_js(ctx),
                    _ => c.gl.get_tex_parameter_i32(t, p).into_js(ctx),
                });
            }
            generateMipmap => c.gl.generate_mipmap(a.u(0)),
            _texImage2D => {
                let (t, lv, ifmt, w, h, b, f, ty) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.u(6), a.u(7));
                upload(c, a.v(8), w, h, 1, f, ty, |gl, d| gl.tex_image_2d(t, lv, ifmt, w, h, b, f, ty, d));
            }
            _texSubImage2D => {
                let (t, lv, x, y, w, h, f, ty) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.u(6), a.u(7));
                if a.v(8).is_none() {
                    c.error(glow::INVALID_VALUE);
                } else {
                    upload(c, a.v(8), w, h, 1, f, ty, |gl, d| gl.tex_sub_image_2d(t, lv, x, y, w, h, f, ty, d));
                }
            }
            _texImage3D => {
                let (t, lv, ifmt, w, h, d, b, f, ty) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6), a.u(7), a.u(8));
                upload(c, a.v(9), w, h, d, f, ty, |gl, px| gl.tex_image_3d(t, lv, ifmt, w, h, d, b, f, ty, px));
            }
            _texSubImage3D => {
                let (t, lv, x, y, z, w, h, d, f, ty) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6), a.i(7), a.u(8), a.u(9));
                upload(c, a.v(10), w, h, d, f, ty, |gl, px| gl.tex_sub_image_3d(t, lv, x, y, z, w, h, d, f, ty, px));
            }
            _texImage2DSource => {
                let (t, lv, ifmt, f, ty) = (a.u(0), a.i(1), a.i(2), a.u(3), a.u(4));
                if let Some((data, w, h)) = source(c, e, a, 5, f, ty) {
                    tight(c, |gl| gl.tex_image_2d(t, lv, ifmt, w as i32, h as i32, 0, f, ty, glow::PixelUnpackData::Slice(Some(&data))));
                }
            }
            _texSubImage2DSource => {
                let (t, lv, x, y, f, ty) = (a.u(0), a.i(1), a.i(2), a.i(3), a.u(4), a.u(5));
                if let Some((data, w, h)) = source(c, e, a, 6, f, ty) {
                    tight(c, |gl| gl.tex_sub_image_2d(t, lv, x, y, w as i32, h as i32, f, ty, glow::PixelUnpackData::Slice(Some(&data))));
                }
            }
            _texImage3DSource => {
                // (t, lv, ifmt, w, h, d, border, f, ty, kind…)：来源按行切成 d 层、每层 h 行
                let (t, lv, ifmt, w, h, d, b, f, ty) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6), a.u(7), a.u(8));
                if let Some((data, sw, sh)) = source(c, e, a, 9, f, ty) {
                    if sw as i32 != w || (sh as i64) < h as i64 * d as i64 {
                        c.error(glow::INVALID_OPERATION);
                    } else {
                        tight(c, |gl| gl.tex_image_3d(t, lv, ifmt, w, h, d, b, f, ty, glow::PixelUnpackData::Slice(Some(&data))));
                    }
                }
            }
            _texSubImage3DSource => {
                let (t, lv, x, y, z, w, h, d, f, ty) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6), a.i(7), a.u(8), a.u(9));
                if let Some((data, sw, sh)) = source(c, e, a, 10, f, ty) {
                    if sw as i32 != w || (sh as i64) < h as i64 * d as i64 {
                        c.error(glow::INVALID_OPERATION);
                    } else {
                        tight(c, |gl| gl.tex_sub_image_3d(t, lv, x, y, z, w, h, d, f, ty, glow::PixelUnpackData::Slice(Some(&data))));
                    }
                }
            }
            copyTexImage2D => {
                c.touch_default(false);
                c.gl.copy_tex_image_2d(a.u(0), a.i(1), a.u(2), a.i(3), a.i(4), a.i(5), a.i(6), a.i(7));
            }
            copyTexSubImage2D => {
                c.touch_default(false);
                c.gl.copy_tex_sub_image_2d(a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6), a.i(7));
            }
            copyTexSubImage3D => {
                c.touch_default(false);
                c.gl.copy_tex_sub_image_3d(a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6), a.i(7), a.i(8));
            }
            _compressedTexImage2D => {
                let (t, lv, ifmt, w, h, b) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5));
                let gl = &c.gl;
                if a.v(6).and_then(|v| with_bytes(v, |d| gl.compressed_tex_image_2d(t, lv, ifmt, w, h, b, d.len() as i32, d))).is_none() {
                    c.error(glow::INVALID_VALUE);
                }
            }
            _compressedTexSubImage2D => {
                let (t, lv, x, y, w, h, f) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.u(6));
                let gl = &c.gl;
                if a.v(7).and_then(|v| with_bytes(v, |d| gl.compressed_tex_sub_image_2d(t, lv, x, y, w, h, f, glow::CompressedPixelUnpackData::Slice(d)))).is_none() {
                    c.error(glow::INVALID_VALUE);
                }
            }
            _compressedTexImage3D => {
                let (t, lv, ifmt, w, h, d, b) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6));
                let gl = &c.gl;
                if a.v(7).and_then(|v| with_bytes(v, |px| gl.compressed_tex_image_3d(t, lv, ifmt, w, h, d, b, px.len() as i32, px))).is_none() {
                    c.error(glow::INVALID_VALUE);
                }
            }
            _compressedTexSubImage3D => {
                let (t, lv, x, y, z, w, h, d, f) = (a.u(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6), a.i(7), a.u(8));
                let gl = &c.gl;
                if a.v(9).and_then(|v| with_bytes(v, |px| gl.compressed_tex_sub_image_3d(t, lv, x, y, z, w, h, d, f, glow::CompressedPixelUnpackData::Slice(px)))).is_none() {
                    c.error(glow::INVALID_VALUE);
                }
            }
            texStorage2D => c.gl.tex_storage_2d(a.u(0), a.i(1), a.u(2), a.i(3), a.i(4)),
            texStorage3D => c.gl.tex_storage_3d(a.u(0), a.i(1), a.u(2), a.i(3), a.i(4), a.i(5)),
            _ => return None,
        }
    }
    Some(undef(ctx))
}

fn store(c: &GlCtx) -> Store {
    let u = &c.unpack;
    Store { alignment: u.alignment, row_length: u.row_length, image_height: u.image_height, skip_pixels: u.skip_pixels, skip_rows: u.skip_rows, skip_images: u.skip_images }
}

/// ArrayBufferView / PBO 偏移 / null 三种数据：先核尺寸，要 FLIP_Y / 预乘的重排成紧密的再下发
fn upload(c: &mut GlCtx, data: Option<&Value<'_>>, w: i32, h: i32, d: i32, f: u32, ty: u32, call: impl FnOnce(&glow::Context, glow::PixelUnpackData)) {
    let Some(bpp) = pixels::bytes_per_pixel(f, ty) else {
        c.error(glow::INVALID_ENUM);
        return;
    };
    let st = store(c);
    let mut call = Some(call);
    let mut call = move |gl: &glow::Context, d: glow::PixelUnpackData| (call.take().unwrap())(gl, d);
    match data {
        // WebGL2：绑着 PIXEL_UNPACK_BUFFER 时最后一个参数是偏移
        Some(v) if v.is_number() => {
            let off = v.as_number().unwrap_or(0.0);
            if off < 0.0 {
                c.error(glow::INVALID_VALUE);
                return;
            }
            call(&c.gl, glow::PixelUnpackData::BufferOffset(off as u32));
        }
        Some(v) => {
            let Some(need) = pixels::image_bytes(w, h, d, bpp, &st) else {
                c.error(glow::INVALID_VALUE);
                return;
            };
            let flip = c.unpack.flip_y;
            let premul = c.unpack.premultiply && ty == glow::UNSIGNED_BYTE && matches!(f, glow::RGBA | glow::LUMINANCE_ALPHA);
            if (flip || premul) && d > 1 {
                // WebGL2 §5.35：3D 纹理的 ArrayBufferView 上传不支持这两个开关
                c.error(glow::INVALID_OPERATION);
                return;
            }
            let mut short = false;
            let mut repacked = None;
            with_bytes(v, |b| {
                if b.len() < need {
                    short = true;
                } else if flip || premul {
                    let alpha_at = if f == glow::RGBA { 3 } else { 1 };
                    repacked = Some(pixels::repack_view(b, w, h, bpp, &st, flip, premul.then_some(alpha_at)));
                } else {
                    call(&c.gl, glow::PixelUnpackData::Slice(Some(b)));
                }
            });
            if short {
                c.error(glow::INVALID_OPERATION);
            } else if let Some(r) = repacked {
                tight(c, |gl| call(gl, glow::PixelUnpackData::Slice(Some(&r))));
            }
        }
        None if c.d.webgl_compat => call(&c.gl, glow::PixelUnpackData::Slice(None)),
        None => {
            // 系统驱动：null 数据的纹理内容未定义，WebGL 要求是 0
            let n = (w.max(0) as usize) * (h.max(0) as usize) * (d.max(0) as usize) * bpp;
            let z = vec![0u8; n];
            tight(c, |gl| call(gl, glow::PixelUnpackData::Slice(Some(&z))));
        }
    }
}

/// 临时把解包参数复位成紧密排列（对齐 1、无跳过、无 PBO），调用完还原成游戏设的
fn tight(c: &GlCtx, f: impl FnOnce(&glow::Context)) {
    let gl = &c.gl;
    let es3 = c.version == 2;
    unsafe {
        let pbo = if es3 { gl.get_parameter_i32(glow::PIXEL_UNPACK_BUFFER_BINDING) } else { 0 };
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        if es3 {
            for p in [glow::UNPACK_ROW_LENGTH, glow::UNPACK_IMAGE_HEIGHT, glow::UNPACK_SKIP_PIXELS, glow::UNPACK_SKIP_ROWS, glow::UNPACK_SKIP_IMAGES] {
                gl.pixel_store_i32(p, 0);
            }
            if pbo != 0 { gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None); }
        }
        f(gl);
        let u = &c.unpack;
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, u.alignment);
        if es3 {
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, u.row_length);
            gl.pixel_store_i32(glow::UNPACK_IMAGE_HEIGHT, u.image_height);
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, u.skip_pixels);
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, u.skip_rows);
            gl.pixel_store_i32(glow::UNPACK_SKIP_IMAGES, u.skip_images);
            if pbo != 0 { gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, super::args::buf(pbo as u32)); }
        }
    }
}

/// Image / 画布 / ImageData → 按 format/type 排好的紧密数据与宽高
fn source(c: &mut GlCtx, e: &Env<'_, '_>, a: &A<'_, '_>, at: usize, f: u32, ty: u32) -> Option<(Vec<u8>, u32, u32)> {
    let (flip, premul) = (c.unpack.flip_y, c.unpack.premultiply);
    let r = match a.i(at) {
        1 => {
            let img = e.images.borrow_mut().get(a.u(at + 1));
            img.and_then(|d| pixels::convert_source(&d.rgba, d.w, d.h, flip, premul, f, ty).map(|v| (v, d.w, d.h)))
        }
        2 => {
            let mut cs = e.canvases.borrow_mut();
            let slot = cs.get(a.u(at + 1))?;
            let canvas = slot.ctx.get_canvas();
            let cv = canvas.lock().ok()?;
            let (w, h) = (cv.width(), cv.height());
            pixels::convert_source(crate::canvas::color_bytes(cv.pixels()), w, h, flip, premul, f, ty).map(|v| (v, w, h))
        }
        3 => {
            let (w, h) = (a.u(at + 1), a.u(at + 2));
            a.v(at + 3).and_then(|v| with_bytes(v, |b| {
                if b.len() < (w * h * 4) as usize {
                    return None;
                }
                pixels::convert_source(b, w, h, flip, premul, f, ty).map(|v| (v, w, h))
            })).flatten()
        }
        _ => None,
    };
    if r.is_none() {
        c.error(glow::INVALID_OPERATION);
    }
    r
}
