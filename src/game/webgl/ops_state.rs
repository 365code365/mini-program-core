//! 全局状态、清屏、错误、参数查询、扩展。

use super::args::{js, undef, A};
use super::ctx::GlCtx;
use super::natives::{Env, Op};
use glow::HasContext;
use rquickjs::{IntoJs, Result as JsResult, Value};

pub fn call<'js>(c: &mut GlCtx, e: &Env<'_, 'js>, op: Op, a: &A<'_, 'js>) -> Option<JsResult<Value<'js>>> {
    use Op::*;
    let ctx = &e.ctx;
    let gl = &c.gl;
    unsafe {
        match op {
            activeTexture => gl.active_texture(a.u(0)),
            blendColor => gl.blend_color(a.f(0), a.f(1), a.f(2), a.f(3)),
            blendEquation => gl.blend_equation(a.u(0)),
            blendEquationSeparate => gl.blend_equation_separate(a.u(0), a.u(1)),
            blendFunc => {
                if constant_color_conflict(a.u(0), a.u(1)) {
                    c.error(glow::INVALID_OPERATION);
                } else {
                    gl.blend_func(a.u(0), a.u(1));
                }
            }
            blendFuncSeparate => {
                if constant_color_conflict(a.u(0), a.u(1)) {
                    c.error(glow::INVALID_OPERATION);
                } else {
                    gl.blend_func_separate(a.u(0), a.u(1), a.u(2), a.u(3));
                }
            }
            clearColor => gl.clear_color(a.f(0), a.f(1), a.f(2), a.f(3)),
            clearDepth => gl.clear_depth_f32(a.f(0)),
            clearStencil => gl.clear_stencil(a.i(0)),
            colorMask => gl.color_mask(a.b(0), a.b(1), a.b(2), a.b(3)),
            cullFace => gl.cull_face(a.u(0)),
            depthFunc => gl.depth_func(a.u(0)),
            depthMask => gl.depth_mask(a.b(0)),
            depthRange => {
                // WebGL：zNear > zFar 是 INVALID_OPERATION（GL 会夹）
                if a.f(0) > a.f(1) {
                    c.error(glow::INVALID_OPERATION);
                } else {
                    gl.depth_range_f32(a.f(0), a.f(1));
                }
            }
            disable => gl.disable(a.u(0)),
            enable => gl.enable(a.u(0)),
            frontFace => gl.front_face(a.u(0)),
            hint => gl.hint(a.u(0), a.u(1)),
            isEnabled => return Some(gl.is_enabled(a.u(0)).into_js(ctx)),
            lineWidth => {
                let w = a.f(0);
                if !(w > 0.0) || !w.is_finite() {
                    c.error(glow::INVALID_VALUE);
                } else {
                    gl.line_width(w);
                }
            }
            pixelStorei => pixel_store(c, a.u(0), a.i(1)),
            polygonOffset => gl.polygon_offset(a.f(0), a.f(1)),
            sampleCoverage => gl.sample_coverage(a.f(0), a.b(1)),
            scissor => {
                if a.i(2) < 0 || a.i(3) < 0 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    gl.scissor(a.i(0), a.i(1), a.i(2), a.i(3));
                }
            }
            stencilFunc => gl.stencil_func(a.u(0), a.i(1), a.u(2)),
            stencilFuncSeparate => gl.stencil_func_separate(a.u(0), a.u(1), a.i(2), a.u(3)),
            stencilMask => gl.stencil_mask(a.u(0)),
            stencilMaskSeparate => gl.stencil_mask_separate(a.u(0), a.u(1)),
            stencilOp => gl.stencil_op(a.u(0), a.u(1), a.u(2)),
            stencilOpSeparate => gl.stencil_op_separate(a.u(0), a.u(1), a.u(2), a.u(3)),
            viewport => {
                if a.i(2) < 0 || a.i(3) < 0 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    gl.viewport(a.i(0), a.i(1), a.i(2), a.i(3));
                }
            }
            clear => {
                let mask = a.u(0);
                if mask & !(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT) != 0 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    c.touch_default(true);
                    c.gl.clear(mask);
                }
            }
            finish => gl.finish(),
            flush => gl.flush(),
            getError => return Some(c.get_error().into_js(ctx)),
            getParameter => return Some(super::params::get(c, ctx, a.u(0))),
            getSupportedExtensions => return Some(c.supported_extensions().into_js(ctx)),
            _getExtension => return Some(c.enable_extension(&a.s(0)).into_js(ctx)),
            getShaderPrecisionFormat => {
                return Some(match gl.get_shader_precision_format(a.u(0), a.u(1)) {
                    Some(f) => js(ctx, vec![f.range_min, f.range_max, f.precision]),
                    // 桌面 GL 不支持查询时 glow 返回 None：按 highp float 的典型值（与 Chromium 的兜底一致）
                    None => match a.u(1) {
                        glow::LOW_INT | glow::MEDIUM_INT | glow::HIGH_INT => js(ctx, vec![31, 30, 0]),
                        _ => js(ctx, vec![127, 127, 23]),
                    },
                });
            }
            _synthError => c.error(a.u(0)),
            _info => {
                let v = serde_json::json!({
                    "vendor": gl.get_parameter_string(glow::VENDOR), "renderer": gl.get_parameter_string(glow::RENDERER),
                    "version": gl.get_parameter_string(glow::VERSION), "backend": c.d.backend.name(), "validated": c.d.webgl_compat,
                });
                return Some(v.to_string().into_js(ctx));
            }
            _ => return None,
        }
    }
    Some(undef(ctx))
}

/// WebGL §6.13：同一次 blendFunc 里 CONSTANT_COLOR 系与 CONSTANT_ALPHA 系不能一起用
fn constant_color_conflict(src: u32, dst: u32) -> bool {
    let color = |f: u32| f == glow::CONSTANT_COLOR || f == glow::ONE_MINUS_CONSTANT_COLOR;
    let alpha = |f: u32| f == glow::CONSTANT_ALPHA || f == glow::ONE_MINUS_CONSTANT_ALPHA;
    (color(src) && alpha(dst)) || (alpha(src) && color(dst))
}

/// pixelStorei：WebGL 独有的三个只记不下发；对齐只认 1/2/4/8；其余下发并记一份给尺寸核算
fn pixel_store(c: &mut GlCtx, p: u32, v: i32) {
    match p {
        0x9240 => c.unpack.flip_y = v != 0,
        0x9241 => c.unpack.premultiply = v != 0,
        0x9243 => {
            if v as u32 == 0 || v as u32 == 0x9244 {
                c.unpack.colorspace = v as u32;
            } else {
                c.error(glow::INVALID_ENUM);
            }
        }
        glow::UNPACK_ALIGNMENT | glow::PACK_ALIGNMENT => {
            if ![1, 2, 4, 8].contains(&v) {
                c.error(glow::INVALID_VALUE);
                return;
            }
            if p == glow::UNPACK_ALIGNMENT { c.unpack.alignment = v } else { c.pack_alignment = v }
            unsafe { c.gl.pixel_store_i32(p, v) };
        }
        _ if c.version == 2 => {
            if v < 0 {
                c.error(glow::INVALID_VALUE);
                return;
            }
            match p {
                glow::UNPACK_ROW_LENGTH => c.unpack.row_length = v,
                glow::UNPACK_IMAGE_HEIGHT => c.unpack.image_height = v,
                glow::UNPACK_SKIP_PIXELS => c.unpack.skip_pixels = v,
                glow::UNPACK_SKIP_ROWS => c.unpack.skip_rows = v,
                glow::UNPACK_SKIP_IMAGES => c.unpack.skip_images = v,
                glow::PACK_ROW_LENGTH | glow::PACK_SKIP_PIXELS | glow::PACK_SKIP_ROWS => {}
                _ => {
                    c.error(glow::INVALID_ENUM);
                    return;
                }
            }
            unsafe { c.gl.pixel_store_i32(p, v) };
        }
        _ => c.error(glow::INVALID_ENUM),
    }
}

