//! 着色器、程序、uniform、属性位置、uniform 块、变换反馈的 varyings。
//!
//! uniform 位置在 JS 里包成 WebGLUniformLocation（记着所属程序与名字），这里只收位置整数；
//! 「位置不属于当前程序」由 JS 判（INVALID_OPERATION），位置为 null 的调用 JS 直接吞掉（WebGL：静默忽略）。

use super::args::{cast, js, null, program, shader, undef, A};
use super::ctx::GlCtx;
use super::natives::{Env, Op};
use crate::js::binary::with_bytes;
use glow::HasContext;
use rquickjs::{Ctx, IntoJs, Result as JsResult, TypedArray, Value};

fn loc(i: i32) -> glow::NativeUniformLocation {
    glow::NativeUniformLocation(i as u32)
}

fn info<'js>(ctx: &Ctx<'js>, size: i32, ty: u32, name: String) -> JsResult<Value<'js>> {
    let o = rquickjs::Object::new(ctx.clone())?;
    o.set("size", size)?;
    o.set("type", ty)?;
    o.set("name", name)?;
    Ok(o.into_value())
}

/// 一种 GL 类型占几个属性位置（mat4 = 4）
fn attrib_slots(ty: u32) -> u32 {
    match ty {
        glow::FLOAT_MAT2 | glow::FLOAT_MAT2x3 | glow::FLOAT_MAT2x4 => 2,
        glow::FLOAT_MAT3 | glow::FLOAT_MAT3x2 | glow::FLOAT_MAT3x4 => 3,
        glow::FLOAT_MAT4 | glow::FLOAT_MAT4x2 | glow::FLOAT_MAT4x3 => 4,
        _ => 1,
    }
}

pub fn call<'js>(c: &mut GlCtx, e: &Env<'_, 'js>, op: Op, a: &A<'_, 'js>) -> Option<JsResult<Value<'js>>> {
    use Op::*;
    let ctx = &e.ctx;
    let gl = &c.gl;
    let p = || program(a.u(0));
    unsafe {
        match op {
            createShader => {
                let ty = a.u(0);
                if ty != glow::VERTEX_SHADER && ty != glow::FRAGMENT_SHADER {
                    c.error(glow::INVALID_ENUM);
                    return Some(null(ctx));
                }
                return Some(gl.create_shader(ty).map(|s| s.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n)));
            }
            deleteShader => { if let Some(s) = shader(a.u(0)) { gl.delete_shader(s) } }
            shaderSource => { if let Some(s) = shader(a.u(0)) { gl.shader_source(s, &a.s(1)) } }
            compileShader => { if let Some(s) = shader(a.u(0)) { gl.compile_shader(s) } }
            getShaderParameter => {
                let Some(s) = shader(a.u(0)) else { return Some(null(ctx)) };
                return Some(match a.u(1) {
                    glow::COMPILE_STATUS => gl.get_shader_compile_status(s).into_js(ctx),
                    // COMPLETION_STATUS_KHR（KHR_parallel_shader_compile）：同步编译，问就是好了
                    0x91B1 => true.into_js(ctx),
                    _ => {
                        c.error(glow::INVALID_ENUM);
                        null(ctx)
                    }
                });
            }
            getShaderInfoLog => return Some(shader(a.u(0)).map_or_else(|| null(ctx), |s| gl.get_shader_info_log(s).into_js(ctx))),
            attachShader => { if let (Some(p), Some(s)) = (p(), shader(a.u(1))) { gl.attach_shader(p, s) } }
            detachShader => { if let (Some(p), Some(s)) = (p(), shader(a.u(1))) { gl.detach_shader(p, s) } }
            createProgram => return Some(gl.create_program().map(|p| p.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteProgram => {
                if let Some(pp) = p() {
                    gl.delete_program(pp);
                    if let Some(t) = c.check.as_mut() { t.delete_program(a.u(0)); }
                }
            }
            linkProgram => {
                let Some(pp) = p() else { return Some(undef(ctx)) };
                gl.link_program(pp);
                if c.check.is_some() {
                    let mut locs = Vec::new();
                    for i in 0..gl.get_active_attributes(pp) {
                        if let Some(at) = gl.get_active_attribute(pp, i) {
                            if let Some(l) = gl.get_attrib_location(pp, &at.name) {
                                for k in 0..attrib_slots(at.atype) * at.size.max(1) as u32 {
                                    locs.push(l + k);
                                }
                            }
                        }
                    }
                    c.check.as_mut().unwrap().linked(a.u(0), locs);
                }
            }
            useProgram => {
                gl.use_program(p());
                if let Some(t) = c.check.as_mut() { t.use_program(a.u(0)); }
            }
            validateProgram => { if let Some(pp) = p() { gl.validate_program(pp) } }
            getProgramParameter => {
                let Some(pp) = p() else { return Some(null(ctx)) };
                let pn = a.u(1);
                let v = gl.get_program_parameter_i32(pp, pn);
                return Some(match pn {
                    glow::DELETE_STATUS | glow::LINK_STATUS | glow::VALIDATE_STATUS => (v != 0).into_js(ctx),
                    0x91B1 => true.into_js(ctx),
                    glow::ATTACHED_SHADERS | glow::ACTIVE_ATTRIBUTES | glow::ACTIVE_UNIFORMS => v.into_js(ctx),
                    glow::TRANSFORM_FEEDBACK_BUFFER_MODE | glow::TRANSFORM_FEEDBACK_VARYINGS | glow::ACTIVE_UNIFORM_BLOCKS if c.version == 2 => v.into_js(ctx),
                    _ => {
                        c.error(glow::INVALID_ENUM);
                        null(ctx)
                    }
                });
            }
            getProgramInfoLog => return Some(p().map_or_else(|| null(ctx), |pp| gl.get_program_info_log(pp).into_js(ctx))),
            getActiveAttrib => {
                let r = p().and_then(|pp| gl.get_active_attribute(pp, a.u(1)));
                return Some(match r {
                    Some(at) => info(ctx, at.size, at.atype, at.name),
                    None => {
                        c.error(glow::INVALID_VALUE);
                        null(ctx)
                    }
                });
            }
            getActiveUniform => {
                let r = p().and_then(|pp| gl.get_active_uniform(pp, a.u(1)));
                return Some(match r {
                    Some(u) => info(ctx, u.size, u.utype, u.name),
                    None => {
                        c.error(glow::INVALID_VALUE);
                        null(ctx)
                    }
                });
            }
            getAttribLocation => return Some(p().and_then(|pp| gl.get_attrib_location(pp, &a.s(1))).map(|l| l as i32).unwrap_or(-1).into_js(ctx)),
            bindAttribLocation => { if let Some(pp) = p() { gl.bind_attrib_location(pp, a.u(1), &a.s(2)) } }
            getUniformLocation => {
                return Some(match p().and_then(|pp| gl.get_uniform_location(pp, &a.s(1))) {
                    Some(l) => js(ctx, l.0 as i32),
                    None => null(ctx),
                });
            }
            getUniform => return Some(get_uniform(c, ctx, a)),
            uniform1f => gl.uniform_1_f32(Some(&loc(a.i(0))), a.f(1)),
            uniform2f => gl.uniform_2_f32(Some(&loc(a.i(0))), a.f(1), a.f(2)),
            uniform3f => gl.uniform_3_f32(Some(&loc(a.i(0))), a.f(1), a.f(2), a.f(3)),
            uniform4f => gl.uniform_4_f32(Some(&loc(a.i(0))), a.f(1), a.f(2), a.f(3), a.f(4)),
            uniform1i => gl.uniform_1_i32(Some(&loc(a.i(0))), a.i(1)),
            uniform2i => gl.uniform_2_i32(Some(&loc(a.i(0))), a.i(1), a.i(2)),
            uniform3i => gl.uniform_3_i32(Some(&loc(a.i(0))), a.i(1), a.i(2), a.i(3)),
            uniform4i => gl.uniform_4_i32(Some(&loc(a.i(0))), a.i(1), a.i(2), a.i(3), a.i(4)),
            uniform1ui => gl.uniform_1_u32(Some(&loc(a.i(0))), a.u(1)),
            uniform2ui => gl.uniform_2_u32(Some(&loc(a.i(0))), a.u(1), a.u(2)),
            uniform3ui => gl.uniform_3_u32(Some(&loc(a.i(0))), a.u(1), a.u(2), a.u(3)),
            uniform4ui => gl.uniform_4_u32(Some(&loc(a.i(0))), a.u(1), a.u(2), a.u(3), a.u(4)),
            _uniformfv | _uniformiv | _uniformuiv => {
                let (l, n) = (loc(a.i(0)), a.u(1) as usize);
                let ok = a.v(2).and_then(|v| with_bytes(v, |b| {
                    let count = b.len() / 4;
                    if n == 0 || count == 0 || count % n != 0 {
                        return false;
                    }
                    let l = Some(&l);
                    match op {
                        _uniformfv => {
                            let f = cast::<f32>(b);
                            match n { 1 => gl.uniform_1_f32_slice(l, &f), 2 => gl.uniform_2_f32_slice(l, &f), 3 => gl.uniform_3_f32_slice(l, &f), _ => gl.uniform_4_f32_slice(l, &f) }
                        }
                        _uniformiv => {
                            let f = cast::<i32>(b);
                            match n { 1 => gl.uniform_1_i32_slice(l, &f), 2 => gl.uniform_2_i32_slice(l, &f), 3 => gl.uniform_3_i32_slice(l, &f), _ => gl.uniform_4_i32_slice(l, &f) }
                        }
                        _ => {
                            let f = cast::<u32>(b);
                            match n { 1 => gl.uniform_1_u32_slice(l, &f), 2 => gl.uniform_2_u32_slice(l, &f), 3 => gl.uniform_3_u32_slice(l, &f), _ => gl.uniform_4_u32_slice(l, &f) }
                        }
                    }
                    true
                }));
                if ok != Some(true) {
                    c.error(glow::INVALID_VALUE);
                }
            }
            _uniformMatrix => {
                let (l, cols, rows, tr) = (loc(a.i(0)), a.u(1), a.u(2), a.b(3));
                if tr && c.version == 1 {
                    c.error(glow::INVALID_VALUE);
                    return Some(undef(ctx));
                }
                let ok = a.v(4).and_then(|v| with_bytes(v, |b| {
                    let f = cast::<f32>(b);
                    let n = (cols * rows) as usize;
                    if f.is_empty() || f.len() % n != 0 {
                        return false;
                    }
                    let l = Some(&l);
                    match (cols, rows) {
                        (2, 2) => gl.uniform_matrix_2_f32_slice(l, tr, &f),
                        (3, 3) => gl.uniform_matrix_3_f32_slice(l, tr, &f),
                        (4, 4) => gl.uniform_matrix_4_f32_slice(l, tr, &f),
                        (2, 3) => gl.uniform_matrix_2x3_f32_slice(l, tr, &f),
                        (2, 4) => gl.uniform_matrix_2x4_f32_slice(l, tr, &f),
                        (3, 2) => gl.uniform_matrix_3x2_f32_slice(l, tr, &f),
                        (3, 4) => gl.uniform_matrix_3x4_f32_slice(l, tr, &f),
                        (4, 2) => gl.uniform_matrix_4x2_f32_slice(l, tr, &f),
                        (4, 3) => gl.uniform_matrix_4x3_f32_slice(l, tr, &f),
                        _ => return false,
                    }
                    true
                }));
                if ok != Some(true) {
                    c.error(glow::INVALID_VALUE);
                }
            }
            getTranslatedShaderSource => return Some("".into_js(ctx)),
            getFragDataLocation => return Some(p().map(|pp| gl.get_frag_data_location(pp, &a.s(1))).unwrap_or(-1).into_js(ctx)),
            getUniformBlockIndex => return Some(p().and_then(|pp| gl.get_uniform_block_index(pp, &a.s(1))).unwrap_or(glow::INVALID_INDEX).into_js(ctx)),
            getActiveUniformBlockParameter => {
                let Some(pp) = p() else { return Some(null(ctx)) };
                let (i, pn) = (a.u(1), a.u(2));
                return Some(match pn {
                    glow::UNIFORM_BLOCK_ACTIVE_UNIFORM_INDICES => {
                        let n = gl.get_active_uniform_block_parameter_i32(pp, i, glow::UNIFORM_BLOCK_ACTIVE_UNIFORMS).max(0) as usize;
                        let mut v = vec![0i32; n];
                        gl.get_active_uniform_block_parameter_i32_slice(pp, i, pn, &mut v);
                        TypedArray::<u32>::new(ctx.clone(), v.into_iter().map(|x| x as u32).collect::<Vec<_>>()).map(|t| t.into_value())
                    }
                    glow::UNIFORM_BLOCK_REFERENCED_BY_VERTEX_SHADER | glow::UNIFORM_BLOCK_REFERENCED_BY_FRAGMENT_SHADER => (gl.get_active_uniform_block_parameter_i32(pp, i, pn) != 0).into_js(ctx),
                    _ => (gl.get_active_uniform_block_parameter_i32(pp, i, pn) as u32).into_js(ctx),
                });
            }
            getActiveUniformBlockName => return Some(p().map_or_else(|| null(ctx), |pp| gl.get_active_uniform_block_name(pp, a.u(1)).into_js(ctx))),
            uniformBlockBinding => { if let Some(pp) = p() { gl.uniform_block_binding(pp, a.u(1), a.u(2)) } }
            getUniformIndices => {
                let Some(pp) = p() else { return Some(null(ctx)) };
                let names: Vec<String> = a.v(1).and_then(|v| v.as_array()).map(|arr| arr.iter::<String>().filter_map(|x| x.ok()).collect()).unwrap_or_default();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                let r: Vec<u32> = gl.get_uniform_indices(pp, &refs).into_iter().map(|x| x.unwrap_or(glow::INVALID_INDEX)).collect();
                return Some(r.into_js(ctx));
            }
            getActiveUniforms => {
                let Some(pp) = p() else { return Some(null(ctx)) };
                let idx: Vec<u32> = a.v(1).and_then(|v| v.as_array()).map(|arr| arr.iter::<f64>().filter_map(|x| x.ok()).map(|x| x as u32).collect()).unwrap_or_default();
                let pn = a.u(2);
                let r = gl.get_active_uniforms_parameter(pp, &idx, pn);
                return Some(if pn == glow::UNIFORM_IS_ROW_MAJOR { r.into_iter().map(|x| x != 0).collect::<Vec<_>>().into_js(ctx) } else { r.into_js(ctx) });
            }
            transformFeedbackVaryings => {
                let Some(pp) = p() else { return Some(undef(ctx)) };
                let names: Vec<String> = a.v(1).and_then(|v| v.as_array()).map(|arr| arr.iter::<String>().filter_map(|x| x.ok()).collect()).unwrap_or_default();
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                gl.transform_feedback_varyings(pp, &refs, a.u(2));
            }
            getTransformFeedbackVarying => {
                return Some(match p().and_then(|pp| gl.get_transform_feedback_varying(pp, a.u(1))) {
                    Some(v) => info(ctx, v.size, v.tftype, v.name),
                    None => {
                        c.error(glow::INVALID_VALUE);
                        null(ctx)
                    }
                });
            }
            _ => return None,
        }
    }
    Some(undef(ctx))
}

/// getUniform(program, location, 名字)：按名字在活动 uniform 里找类型，再按类型取值、按 WebGL 的形状返回
#[allow(non_upper_case_globals)]
fn get_uniform<'js>(c: &mut GlCtx, ctx: &Ctx<'js>, a: &A<'_, 'js>) -> JsResult<Value<'js>> {
    let gl = &c.gl;
    let Some(pp) = program(a.u(0)) else { return null(ctx) };
    let l = loc(a.i(1));
    let name = a.s(2);
    let base = name.split('[').next().unwrap_or("").to_string();
    let ty = unsafe {
        (0..gl.get_active_uniforms(pp)).filter_map(|i| gl.get_active_uniform(pp, i)).find(|u| u.name == name || u.name.split('[').next() == Some(&base)).map(|u| u.utype)
    };
    let Some(ty) = ty else {
        c.error(glow::INVALID_OPERATION);
        return null(ctx);
    };
    use glow::*;
    let (n, kind) = match ty {
        FLOAT => (1, 'f'),
        FLOAT_VEC2 => (2, 'f'),
        FLOAT_VEC3 => (3, 'f'),
        FLOAT_VEC4 | FLOAT_MAT2 => (4, 'f'),
        FLOAT_MAT3 => (9, 'f'),
        FLOAT_MAT4 => (16, 'f'),
        FLOAT_MAT2x3 | FLOAT_MAT3x2 => (6, 'f'),
        FLOAT_MAT2x4 | FLOAT_MAT4x2 => (8, 'f'),
        FLOAT_MAT3x4 | FLOAT_MAT4x3 => (12, 'f'),
        INT_VEC2 => (2, 'i'),
        INT_VEC3 => (3, 'i'),
        INT_VEC4 => (4, 'i'),
        UNSIGNED_INT => (1, 'u'),
        UNSIGNED_INT_VEC2 => (2, 'u'),
        UNSIGNED_INT_VEC3 => (3, 'u'),
        UNSIGNED_INT_VEC4 => (4, 'u'),
        BOOL => (1, 'b'),
        BOOL_VEC2 => (2, 'b'),
        BOOL_VEC3 => (3, 'b'),
        BOOL_VEC4 => (4, 'b'),
        // INT 与全部采样器
        _ => (1, 'i'),
    };
    unsafe {
        match kind {
            'f' => {
                let mut v = vec![0f32; n];
                gl.get_uniform_f32(pp, &l, &mut v);
                if n == 1 { v[0].into_js(ctx) } else { TypedArray::<f32>::new(ctx.clone(), v).map(|t| t.into_value()) }
            }
            'u' => {
                let mut v = vec![0u32; n];
                gl.get_uniform_u32(pp, &l, &mut v);
                if n == 1 { v[0].into_js(ctx) } else { TypedArray::<u32>::new(ctx.clone(), v).map(|t| t.into_value()) }
            }
            'b' => {
                let mut v = vec![0i32; n];
                gl.get_uniform_i32(pp, &l, &mut v);
                if n == 1 { (v[0] != 0).into_js(ctx) } else { v.into_iter().map(|x| x != 0).collect::<Vec<_>>().into_js(ctx) }
            }
            _ => {
                let mut v = vec![0i32; n];
                gl.get_uniform_i32(pp, &l, &mut v);
                if n == 1 { v[0].into_js(ctx) } else { TypedArray::<i32>::new(ctx.clone(), v).map(|t| t.into_value()) }
            }
        }
    }
}
