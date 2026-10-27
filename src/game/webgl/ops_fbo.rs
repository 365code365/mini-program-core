//! 帧缓冲、渲染缓冲、readPixels、blit、clearBuffer*。
//!
//! `bindFramebuffer` 记下画 / 读的是不是默认帧缓冲：上屏、待清、读回都只认默认的那块（ctx.rs `touch_default`）。

use super::args::{cast, fbo, js, null, rbo, tex, undef, A};
use super::ctx::GlCtx;
use super::natives::{Env, Op};
use super::pixels::{self, Store};
use crate::js::binary::{with_bytes, with_bytes_mut};
use glow::HasContext;
use rquickjs::{IntoJs, Result as JsResult, Value};

pub fn call<'js>(c: &mut GlCtx, e: &Env<'_, 'js>, op: Op, a: &A<'_, 'js>) -> Option<JsResult<Value<'js>>> {
    use Op::*;
    let ctx = &e.ctx;
    unsafe {
        match op {
            createFramebuffer => return Some(c.gl.create_framebuffer().map(|f| f.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteFramebuffer => {
                if let Some(f) = fbo(a.u(0)) {
                    // 删的是当前绑着的：GL 自动回到默认帧缓冲
                    let cur_draw = c.gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING) as u32;
                    let cur_read = if c.version == 2 { c.gl.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING) as u32 } else { cur_draw };
                    c.gl.delete_framebuffer(f);
                    if cur_draw == a.u(0) { c.draw_fb_default = true; }
                    if cur_read == a.u(0) { c.read_fb_default = true; }
                }
            }
            bindFramebuffer => {
                let (t, id) = (a.u(0), a.u(1));
                c.gl.bind_framebuffer(t, fbo(id));
                if t == glow::FRAMEBUFFER || t == glow::DRAW_FRAMEBUFFER { c.draw_fb_default = id == 0; }
                if t == glow::FRAMEBUFFER || t == glow::READ_FRAMEBUFFER { c.read_fb_default = id == 0; }
            }
            framebufferTexture2D => c.gl.framebuffer_texture_2d(a.u(0), a.u(1), a.u(2), tex(a.u(3)), a.i(4)),
            framebufferRenderbuffer => c.gl.framebuffer_renderbuffer(a.u(0), a.u(1), a.u(2), rbo(a.u(3))),
            framebufferTextureLayer => c.gl.framebuffer_texture_layer(a.u(0), a.u(1), tex(a.u(2)), a.i(3), a.i(4)),
            checkFramebufferStatus => return Some(c.gl.check_framebuffer_status(a.u(0)).into_js(ctx)),
            getFramebufferAttachmentParameter => {
                let (t, at, p) = (a.u(0), a.u(1), a.u(2));
                let v = c.gl.get_framebuffer_attachment_parameter_i32(t, at, p);
                return Some(v.into_js(ctx));
            }
            createRenderbuffer => return Some(c.gl.create_renderbuffer().map(|r| r.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteRenderbuffer => { if let Some(r) = rbo(a.u(0)) { c.gl.delete_renderbuffer(r) } }
            bindRenderbuffer => c.gl.bind_renderbuffer(a.u(0), rbo(a.u(1))),
            renderbufferStorage => {
                // WebGL1 的 DEPTH_STENCIL（0x84F9）= GL 的 DEPTH24_STENCIL8
                let f = if a.u(1) == glow::DEPTH_STENCIL { glow::DEPTH24_STENCIL8 } else { a.u(1) };
                c.gl.renderbuffer_storage(a.u(0), f, a.i(2), a.i(3));
            }
            renderbufferStorageMultisample => c.gl.renderbuffer_storage_multisample(a.u(0), a.i(1), a.u(2), a.i(3), a.i(4)),
            getRenderbufferParameter => return Some(c.gl.get_renderbuffer_parameter_i32(a.u(0), a.u(1)).into_js(ctx)),
            _readPixels => read_pixels(c, a),
            blitFramebuffer => {
                c.touch_default(false);
                c.touch_default(true);
                c.gl.blit_framebuffer(a.i(0), a.i(1), a.i(2), a.i(3), a.i(4), a.i(5), a.i(6), a.i(7), a.u(8), a.u(9));
            }
            invalidateFramebuffer | invalidateSubFramebuffer => {
                let list: Vec<u32> = a.v(1).and_then(|v| v.as_array()).map(|arr| arr.iter::<f64>().filter_map(|x| x.ok()).map(|x| x as i64 as u32).collect()).unwrap_or_default();
                if op == invalidateFramebuffer {
                    c.gl.invalidate_framebuffer(a.u(0), &list);
                } else {
                    c.gl.invalidate_sub_framebuffer(a.u(0), &list, a.i(2), a.i(3), a.i(4), a.i(5));
                }
            }
            readBuffer => c.gl.read_buffer(a.u(0)),
            getInternalformatParameter => {
                let (t, f, p) = (a.u(0), a.u(1), a.u(2));
                // SAMPLES：先问个数再取
                let n = c.gl.get_parameter_i32(0x9380 /* NUM_SAMPLE_COUNTS */);
                let _ = n;
                let mut cnt = [0i32];
                c.gl.get_internal_format_i32_slice(t, f, 0x9380, &mut cnt);
                let mut v = vec![0i32; cnt[0].max(0) as usize];
                if !v.is_empty() {
                    c.gl.get_internal_format_i32_slice(t, f, p, &mut v);
                }
                return Some(rquickjs::TypedArray::<i32>::new(ctx.clone(), v).map(|t| t.into_value()));
            }
            _clearBufferfv | _clearBufferiv | _clearBufferuiv => {
                let (buffer, idx) = (a.u(0), a.u(1));
                let need = if buffer == glow::COLOR { 4 } else { 1 };
                if buffer == glow::COLOR || buffer == glow::DEPTH || buffer == glow::STENCIL {
                    c.touch_default(true);
                }
                let gl = &c.gl;
                let ok = a.v(2).and_then(|v| with_bytes(v, |b| {
                    if b.len() / 4 < need {
                        return false;
                    }
                    match op {
                        _clearBufferfv => gl.clear_buffer_f32_slice(buffer, idx, &cast::<f32>(b)[..need]),
                        _clearBufferiv => gl.clear_buffer_i32_slice(buffer, idx, &cast::<i32>(b)[..need]),
                        _ => gl.clear_buffer_u32_slice(buffer, idx, &cast::<u32>(b)[..need]),
                    }
                    true
                }));
                if ok != Some(true) {
                    c.error(glow::INVALID_VALUE);
                }
            }
            clearBufferfi => {
                c.touch_default(true);
                c.gl.clear_buffer_depth_stencil(a.u(0), a.u(1), a.f(2), a.i(3));
            }
            _ => return None,
        }
    }
    Some(undef(ctx))
}

/// readPixels(x, y, w, h, format, type, 目标 | PBO 偏移)：目标比要写的短 → INVALID_OPERATION（GL 不知道 JS 缓冲多长）
fn read_pixels(c: &mut GlCtx, a: &A<'_, '_>) {
    let (x, y, w, h, f, ty) = (a.i(0), a.i(1), a.i(2), a.i(3), a.u(4), a.u(5));
    if w < 0 || h < 0 {
        c.error(glow::INVALID_VALUE);
        return;
    }
    c.touch_default(false);
    match a.v(6) {
        Some(v) if v.is_number() => unsafe {
            c.gl.read_pixels(x, y, w, h, f, ty, glow::PixelPackData::BufferOffset(v.as_number().unwrap_or(0.0).max(0.0) as u32));
        },
        Some(v) => {
            let Some(bpp) = pixels::bytes_per_pixel(f, ty) else {
                c.error(glow::INVALID_ENUM);
                return;
            };
            let st = Store { alignment: c.pack_alignment, ..Store::packed() };
            let Some(need) = pixels::image_bytes(w, h, 1, bpp, &st) else {
                c.error(glow::INVALID_VALUE);
                return;
            };
            let gl = &c.gl;
            let ok = with_bytes_mut(v, |b| {
                if b.len() < need {
                    return false;
                }
                unsafe { gl.read_pixels(x, y, w, h, f, ty, glow::PixelPackData::Slice(Some(b))) };
                true
            });
            if ok != Some(true) {
                c.error(glow::INVALID_OPERATION);
            }
        }
        None => c.error(glow::INVALID_VALUE),
    }
}
