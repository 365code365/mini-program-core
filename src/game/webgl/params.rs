//! getParameter：pname → 返回类型（WebGL 规范 §5.14.3 的表，WebGL2 §3.7.2 的补充）。
//!
//! 对象绑定（ARRAY_BUFFER_BINDING 这类）在这里只返回 GL 名字，JS 外壳按 pname 的种类换成包装对象；
//! VENDOR / RENDERER / VERSION / SHADING_LANGUAGE_VERSION 是 WebGL 自己的固定串，也在 JS 里。

use super::ctx::GlCtx;
use glow::HasContext;
use rquickjs::{Ctx, IntoJs, Result as JsResult, TypedArray, Value};

enum K {
    I,
    U,
    B,
    F,
    Fa(usize),
    Ia(usize),
    Ba(usize),
    L,
    Str,
}

fn kind(p: u32, v2: bool) -> Option<K> {
    use K::*;
    Some(match p {
        0x84E0 | 0x0D52..=0x0D57 | 0x80CA | 0x80C8 | 0x883D | 0x8009 | 0x80CB | 0x80C9 | 0x0B45 | 0x0B74 | 0x0B46 | 0x8192 | 0x8B9B
        | 0x8B9A | 0x8B4D | 0x851C | 0x8DFD | 0x84E8 | 0x8872 | 0x0D33 | 0x8DFC | 0x8869 | 0x8B4C | 0x8DFB | 0x0D05 | 0x0CF5
        | 0x80A8 | 0x80A9 | 0x8800..=0x8803 | 0x8CA3 | 0x0B91 | 0x0B92 | 0x0B94..=0x0B97 | 0x0D50 | 0x8B8B => I,
        // 对象绑定：返回 GL 名字
        0x8894 | 0x8895 | 0x8B8D | 0x8CA6 | 0x8CA7 | 0x8069 | 0x8514 | 0x85B5 => I,
        0x8CA4 | 0x8CA5 | 0x0B93 | 0x0B98 => U,
        0x0BE2 | 0x0B44 | 0x0B71 | 0x0B72 | 0x0BD0 | 0x8037 | 0x809E | 0x80A0 | 0x80AB | 0x0C11 | 0x0B90 => B,
        0x0B73 | 0x0B21 | 0x8038 | 0x2A00 | 0x80AA | 0x84FF => F,
        0x846E | 0x846D | 0x0B70 => Fa(2),
        0x8005 | 0x0C22 => Fa(4),
        0x0D3A => Ia(2),
        0x0C10 | 0x0BA2 => Ia(4),
        0x0C23 => Ba(4),
        // WEBGL_draw_buffers / WebGL2
        0x8824 | 0x8CDF | 0x8825..=0x8834 => I,
        0x9245 | 0x9246 => Str,
        _ if !v2 => return None,
        0x8F36 | 0x8F37 | 0x8073 | 0x88FF | 0x8A2E | 0x80E9 | 0x80E8 | 0x9125 | 0x8A2D | 0x8B49 | 0x8905 | 0x8D57 | 0x8C8A
        | 0x8C8B | 0x8C80 | 0x8A2F | 0x8B4B | 0x9122 | 0x8A2B | 0x8B4A | 0x8904 | 0x0D02 | 0x0D03 | 0x0D04 | 0x88ED | 0x88EF
        | 0x0C02 | 0x8CAA | 0x8919 | 0x8C1D | 0x806A | 0x8E25 | 0x8C8F | 0x8A28 | 0x8A34 | 0x806E | 0x0CF2 | 0x806D | 0x0CF4
        | 0x0CF3 => I,
        0x8A33 | 0x8A31 | 0x8D6B | 0x9111 | 0x8A30 => L,
        0x8C89 | 0x8E24 | 0x8E23 => B,
        0x84FD => F,
        0x9247 => L,
        _ => return None,
    })
}

pub fn get<'js>(c: &mut GlCtx, ctx: &Ctx<'js>, p: u32) -> JsResult<Value<'js>> {
    // WebGL 自己的
    match p {
        0x9240 => return c.unpack.flip_y.into_js(ctx),
        0x9241 => return c.unpack.premultiply.into_js(ctx),
        0x9243 => return c.unpack.colorspace.into_js(ctx),
        // 客户端等待的上限：0（clientWaitSync 只能轮询，与 Chromium 一致）
        0x9247 => return 0.into_js(ctx),
        0x9245 | 0x9246 if !c.has_enabled("WEBGL_debug_renderer_info") => {
            c.error(glow::INVALID_ENUM);
            return Ok(Value::new_null(ctx.clone()));
        }
        0x84FF if !c.has_enabled("EXT_texture_filter_anisotropic") => {
            c.error(glow::INVALID_ENUM);
            return Ok(Value::new_null(ctx.clone()));
        }
        0x8B8B if c.version == 1 && !c.has_enabled("OES_standard_derivatives") => {
            c.error(glow::INVALID_ENUM);
            return Ok(Value::new_null(ctx.clone()));
        }
        0x8824 | 0x8CDF | 0x8825..=0x8834 if c.version == 1 && !c.has_enabled("WEBGL_draw_buffers") => {
            c.error(glow::INVALID_ENUM);
            return Ok(Value::new_null(ctx.clone()));
        }
        0x85B5 if c.version == 1 && !c.has_enabled("OES_vertex_array_object") => {
            c.error(glow::INVALID_ENUM);
            return Ok(Value::new_null(ctx.clone()));
        }
        // 压缩纹理格式：只列开了的扩展带来的（ANGLE 的 WebGL 上下文本来就这么报）
        0x86A3 => {
            let gl = &c.gl;
            let n = unsafe { gl.get_parameter_i32(0x86A2) }.max(0) as usize;
            let mut v = vec![0i32; n];
            if n > 0 {
                unsafe { gl.get_parameter_i32_slice(0x86A3, &mut v) };
            }
            let u: Vec<u32> = v.into_iter().map(|x| x as u32).collect();
            return Ok(TypedArray::<u32>::new(ctx.clone(), u)?.into_value());
        }
        _ => {}
    }
    let Some(k) = kind(p, c.version == 2) else {
        c.error(glow::INVALID_ENUM);
        return Ok(Value::new_null(ctx.clone()));
    };
    let gl = &c.gl;
    unsafe {
        match k {
            K::I => gl.get_parameter_i32(p).into_js(ctx),
            K::U => (gl.get_parameter_i32(p) as u32).into_js(ctx),
            K::B => (gl.get_parameter_i32(p) != 0).into_js(ctx),
            K::F => gl.get_parameter_f32(p).into_js(ctx),
            K::L => (gl.get_parameter_i64(p) as f64).into_js(ctx),
            K::Str => {
                let s = gl.get_parameter_string(if p == 0x9245 { glow::VENDOR } else { glow::RENDERER });
                s.into_js(ctx)
            }
            K::Fa(n) => {
                let mut v = vec![0f32; n];
                gl.get_parameter_f32_slice(p, &mut v);
                Ok(TypedArray::<f32>::new(ctx.clone(), v)?.into_value())
            }
            K::Ia(n) => {
                let mut v = vec![0i32; n];
                gl.get_parameter_i32_slice(p, &mut v);
                Ok(TypedArray::<i32>::new(ctx.clone(), v)?.into_value())
            }
            K::Ba(n) => {
                let mut v = vec![0i32; n];
                gl.get_parameter_i32_slice(p, &mut v);
                v.into_iter().map(|x| x != 0).collect::<Vec<_>>().into_js(ctx)
            }
        }
    }
}
