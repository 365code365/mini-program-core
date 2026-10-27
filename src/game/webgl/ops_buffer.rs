//! 缓冲、顶点属性、绘制、VAO。
//!
//! 绘制前：不是 ANGLE 的上下文先过 validate.rs 的越界检查；画到默认帧缓冲时 `touch_default`（待清的先清、标记要上屏）。

use super::args::{buf, cast, js, null, undef, vao, A};
use super::ctx::GlCtx;
use super::natives::{Env, Op};
use crate::js::binary::{with_bytes, with_bytes_mut};
use glow::HasContext;
use rquickjs::{IntoJs, Result as JsResult, TypedArray, Value};

pub fn call<'js>(c: &mut GlCtx, e: &Env<'_, 'js>, op: Op, a: &A<'_, 'js>) -> Option<JsResult<Value<'js>>> {
    use Op::*;
    let ctx = &e.ctx;
    unsafe {
        match op {
            createBuffer => return Some(c.gl.create_buffer().map(|b| b.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteBuffer => {
                if let Some(b) = buf(a.u(0)) {
                    c.gl.delete_buffer(b);
                    if let Some(t) = c.check.as_mut() { t.delete_buffer(a.u(0)); }
                }
            }
            bindBuffer => {
                c.gl.bind_buffer(a.u(0), buf(a.u(1)));
                if let Some(t) = c.check.as_mut() { t.bind_buffer(a.u(0), a.u(1)); }
            }
            _bufferDataSize => {
                let size = a.l(1);
                if size < 0 || size > i32::MAX as i64 {
                    c.error(glow::INVALID_VALUE);
                } else if c.d.webgl_compat {
                    c.gl.buffer_data_size(a.u(0), size as i32, a.u(2));
                } else {
                    // 系统驱动不保证清零：WebGL 要求新缓冲的内容是 0
                    c.gl.buffer_data_u8_slice(a.u(0), &vec![0u8; size as usize], a.u(2));
                }
                if let Some(t) = c.check.as_mut() { t.buffer_data(a.u(0), size, None); }
            }
            bufferData => {
                let (target, usage) = (a.u(0), a.u(2));
                let gl = &c.gl;
                let n = a.v(1).and_then(|v| with_bytes(v, |b| {
                    gl.buffer_data_u8_slice(target, b, usage);
                    b.len()
                }));
                match n {
                    Some(n) => {
                        if let Some(t) = c.check.as_mut() {
                            a.v(1).and_then(|v| with_bytes(v, |b| t.buffer_data(target, n as i64, Some(b))));
                        }
                    }
                    None => c.error(glow::INVALID_VALUE),
                }
            }
            bufferSubData => {
                let (target, off) = (a.u(0), a.l(1));
                if off < 0 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    let gl = &c.gl;
                    let ok = a.v(2).and_then(|v| with_bytes(v, |b| gl.buffer_sub_data_u8_slice(target, off as i32, b))).is_some();
                    if !ok {
                        c.error(glow::INVALID_VALUE);
                    } else if let Some(t) = c.check.as_mut() {
                        a.v(2).and_then(|v| with_bytes(v, |b| t.buffer_sub_data(target, off, b)));
                    }
                }
            }
            getBufferParameter => return Some(c.gl.get_buffer_parameter_i32(a.u(0), a.u(1)).into_js(ctx)),
            enableVertexAttribArray => {
                c.gl.enable_vertex_attrib_array(a.u(0));
                if let Some(t) = c.check.as_mut() { t.enable_attrib(a.u(0), true); }
            }
            disableVertexAttribArray => {
                c.gl.disable_vertex_attrib_array(a.u(0));
                if let Some(t) = c.check.as_mut() { t.enable_attrib(a.u(0), false); }
            }
            vertexAttribPointer => {
                let (i, size, ty, norm, stride, off) = (a.u(0), a.i(1), a.u(2), a.b(3), a.i(4), a.l(5));
                if off < 0 || stride < 0 || stride > 255 || off > i32::MAX as i64 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    c.gl.vertex_attrib_pointer_f32(i, size, ty, norm, stride, off as i32);
                    if let Some(t) = c.check.as_mut() { t.attrib_pointer(i, size, ty, stride, off); }
                }
            }
            vertexAttribIPointer => {
                let (i, size, ty, stride, off) = (a.u(0), a.i(1), a.u(2), a.i(3), a.l(4));
                if off < 0 || stride < 0 || stride > 255 || off > i32::MAX as i64 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    c.gl.vertex_attrib_pointer_i32(i, size, ty, stride, off as i32);
                    if let Some(t) = c.check.as_mut() { t.attrib_pointer(i, size, ty, stride, off); }
                }
            }
            vertexAttrib1f => c.gl.vertex_attrib_1_f32(a.u(0), a.f(1)),
            vertexAttrib2f => c.gl.vertex_attrib_2_f32(a.u(0), a.f(1), a.f(2)),
            vertexAttrib3f => c.gl.vertex_attrib_3_f32(a.u(0), a.f(1), a.f(2), a.f(3)),
            vertexAttrib4f => c.gl.vertex_attrib_4_f32(a.u(0), a.f(1), a.f(2), a.f(3), a.f(4)),
            _vertexAttribfv => {
                let (i, n) = (a.u(0), a.u(1) as usize);
                let gl = &c.gl;
                let ok = a.v(2).and_then(|v| with_bytes(v, |b| {
                    let f = cast::<f32>(b);
                    if f.len() < n {
                        return false;
                    }
                    match n {
                        1 => gl.vertex_attrib_1_f32_slice(i, &f[..1]),
                        2 => gl.vertex_attrib_2_f32_slice(i, &f[..2]),
                        3 => gl.vertex_attrib_3_f32_slice(i, &f[..3]),
                        _ => gl.vertex_attrib_4_f32_slice(i, &f[..4]),
                    }
                    true
                }));
                if ok != Some(true) {
                    c.error(glow::INVALID_VALUE);
                }
            }
            vertexAttribI4i => c.gl.vertex_attrib_4_i32(a.u(0), a.i(1), a.i(2), a.i(3), a.i(4)),
            vertexAttribI4ui => c.gl.vertex_attrib_4_u32(a.u(0), a.u(1), a.u(2), a.u(3), a.u(4)),
            getVertexAttrib => {
                let (i, p) = (a.u(0), a.u(1));
                let mut v = [0f32; 4];
                c.gl.get_vertex_attrib_parameter_f32_slice(i, p, &mut v);
                return Some(match p {
                    glow::CURRENT_VERTEX_ATTRIB => TypedArray::<f32>::new(ctx.clone(), v.to_vec()).map(|t| t.into_value()),
                    glow::VERTEX_ATTRIB_ARRAY_ENABLED | glow::VERTEX_ATTRIB_ARRAY_NORMALIZED | glow::VERTEX_ATTRIB_ARRAY_INTEGER => (v[0] != 0.0).into_js(ctx),
                    glow::VERTEX_ATTRIB_ARRAY_SIZE | glow::VERTEX_ATTRIB_ARRAY_STRIDE | glow::VERTEX_ATTRIB_ARRAY_TYPE
                    | glow::VERTEX_ATTRIB_ARRAY_DIVISOR | glow::VERTEX_ATTRIB_ARRAY_BUFFER_BINDING => (v[0] as i64 as u32).into_js(ctx),
                    _ => {
                        c.error(glow::INVALID_ENUM);
                        null(ctx)
                    }
                });
            }
            drawArrays | drawArraysInstanced => {
                let (mode, first, count) = (a.u(0), a.i(1), a.i(2));
                let inst = if op == drawArrays { 1 } else { a.i(3) };
                if first < 0 || count < 0 || inst < 0 {
                    c.error(glow::INVALID_VALUE);
                } else if c.check.as_mut().map(|t| t.draw_arrays(first, count, inst)) == Some(false) {
                    c.error(glow::INVALID_OPERATION);
                } else {
                    c.touch_default(true);
                    if op == drawArrays { c.gl.draw_arrays(mode, first, count) } else { c.gl.draw_arrays_instanced(mode, first, count, inst) }
                }
            }
            drawElements | drawElementsInstanced | drawRangeElements => {
                // drawRangeElements(mode, start, end, count, type, offset)：start/end 只是提示，按 drawElements 下发
                let base = if op == drawRangeElements { 2 } else { 0 };
                let (mode, count, ty, off) = (a.u(0), a.i(base + 1), a.u(base + 2), a.l(base + 3));
                let inst = if op == drawElementsInstanced { a.i(4) } else { 1 };
                if count < 0 || off < 0 || inst < 0 || off > i32::MAX as i64 {
                    c.error(glow::INVALID_VALUE);
                } else if ty == glow::UNSIGNED_INT && c.version == 1 && !c.has_enabled("OES_element_index_uint") {
                    c.error(glow::INVALID_ENUM);
                } else if c.check.as_mut().map(|t| t.draw_elements(count, ty, off, inst)) == Some(false) {
                    c.error(glow::INVALID_OPERATION);
                } else {
                    c.touch_default(true);
                    if op == drawElementsInstanced { c.gl.draw_elements_instanced(mode, count, ty, off as i32, inst) } else { c.gl.draw_elements(mode, count, ty, off as i32) }
                }
            }
            vertexAttribDivisor => {
                c.gl.vertex_attrib_divisor(a.u(0), a.u(1));
                if let Some(t) = c.check.as_mut() { t.divisor(a.u(0), a.u(1)); }
            }
            createVertexArray => return Some(c.gl.create_vertex_array().map(|v| v.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteVertexArray => {
                if let Some(v) = vao(a.u(0)) {
                    c.gl.delete_vertex_array(v);
                    if let Some(t) = c.check.as_mut() { t.delete_vao(a.u(0)); }
                }
            }
            bindVertexArray => {
                c.gl.bind_vertex_array(vao(a.u(0)));
                if let Some(t) = c.check.as_mut() { t.bind_vao(a.u(0)); }
            }
            drawBuffers => {
                let list: Vec<u32> = a.v(0).and_then(|v| v.as_array()).map(|arr| arr.iter::<f64>().filter_map(|x| x.ok()).map(|x| x as i64 as u32).collect()).unwrap_or_default();
                c.gl.draw_buffers(&list);
            }
            copyBufferSubData => {
                let (r, w, ro, wo, n) = (a.u(0), a.u(1), a.l(2), a.l(3), a.l(4));
                if ro < 0 || wo < 0 || n < 0 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    c.gl.copy_buffer_sub_data(r, w, ro as i32, wo as i32, n as i32);
                    // 系统驱动路径：索引缓冲的影子跟着变 —— 简单起见作废（下次 drawElements 当越界拒掉之前会按 0 判）
                }
            }
            getBufferSubData => {
                let (target, off) = (a.u(0), a.l(1));
                if off < 0 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    let size = c.gl.get_buffer_parameter_i32(target, glow::BUFFER_SIZE) as i64;
                    let gl = &c.gl;
                    let ok = a.v(2).and_then(|v| with_bytes_mut(v, |b| {
                        if off + b.len() as i64 > size {
                            return false;
                        }
                        // GLES 3.0 没有 glGetBufferSubData（那是桌面 GL）：映射只读区间拷出来
                        if b.is_empty() {
                            return true;
                        }
                        let p = gl.map_buffer_range(target, off as i32, b.len() as i32, glow::MAP_READ_BIT);
                        if p.is_null() {
                            return false;
                        }
                        std::ptr::copy_nonoverlapping(p, b.as_mut_ptr(), b.len());
                        gl.unmap_buffer(target);
                        true
                    }));
                    if ok != Some(true) {
                        c.error(glow::INVALID_VALUE);
                    }
                }
            }
            bindBufferBase => c.gl.bind_buffer_base(a.u(0), a.u(1), buf(a.u(2))),
            bindBufferRange => {
                let (off, size) = (a.l(3), a.l(4));
                if off < 0 || size < 0 {
                    c.error(glow::INVALID_VALUE);
                } else {
                    c.gl.bind_buffer_range(a.u(0), a.u(1), buf(a.u(2)), off as i32, size as i32);
                }
            }
            _ => return None,
        }
    }
    Some(undef(ctx))
}
