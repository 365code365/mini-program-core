//! WebGL 的原生入口：`__game_gl(画布 id, op, ...参数)` 一个分派函数（理由与 2D 的 `__game_2d` 相同）。
//!
//! op 名字表只有这里一份：`__game_gl_ops()` 把它交给 game-webgl.js，JS 按名字查编号 —— 加一个方法只改 Rust，
//! 两边不会对不上。名字与 WebGL 方法同名的，JS 直接转发；带 `_` 前缀的是 JS 外壳内部用的变体。

use super::args::{null, A};
use super::{ops_buffer, ops_fbo, ops_gl2, ops_shader, ops_state, ops_texture, WebGl};
use crate::game::canvas::GameCanvases;
use crate::game::image::ImageStore;
use crate::js::JsRuntime;
use rquickjs::{Ctx, IntoJs, Result as JsResult, Value};
use std::cell::RefCell;
use std::rc::Rc;

macro_rules! ops {
    ($($name:ident),* $(,)?) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Op { $($name),* }
        pub const NAMES: &[&str] = &[$(stringify!($name)),*];
        const ALL: &[Op] = &[$(Op::$name),*];
    };
}

ops! {
    // 状态（ops_state.rs）
    activeTexture, blendColor, blendEquation, blendEquationSeparate, blendFunc, blendFuncSeparate, clearColor, clearDepth,
    clearStencil, colorMask, cullFace, depthFunc, depthMask, depthRange, disable, enable, frontFace, hint, isEnabled, lineWidth,
    pixelStorei, polygonOffset, sampleCoverage, scissor, stencilFunc, stencilFuncSeparate, stencilMask, stencilMaskSeparate,
    stencilOp, stencilOpSeparate, viewport, clear, finish, flush, getError, getParameter, getSupportedExtensions, _getExtension,
    getShaderPrecisionFormat, _synthError, _info,
    // 缓冲与绘制（ops_buffer.rs）
    createBuffer, deleteBuffer, bindBuffer, _bufferDataSize, bufferData, bufferSubData, getBufferParameter,
    enableVertexAttribArray, disableVertexAttribArray, vertexAttribPointer, vertexAttribIPointer, vertexAttrib1f, vertexAttrib2f,
    vertexAttrib3f, vertexAttrib4f, _vertexAttribfv, vertexAttribI4i, vertexAttribI4ui, getVertexAttrib, getVertexAttribOffset,
    drawArrays, drawElements, drawArraysInstanced, drawElementsInstanced, drawRangeElements, vertexAttribDivisor,
    createVertexArray, deleteVertexArray, bindVertexArray, drawBuffers, copyBufferSubData, getBufferSubData,
    bindBufferBase, bindBufferRange,
    // 着色器与程序（ops_shader.rs）
    createShader, deleteShader, shaderSource, compileShader, getShaderParameter, getShaderInfoLog, attachShader,
    detachShader, createProgram, deleteProgram, linkProgram, useProgram, validateProgram, getProgramParameter,
    getProgramInfoLog, getActiveAttrib, getActiveUniform, getAttribLocation, bindAttribLocation, getUniformLocation,
    getUniform, uniform1f, uniform2f, uniform3f, uniform4f, uniform1i, uniform2i, uniform3i, uniform4i, uniform1ui, uniform2ui,
    uniform3ui, uniform4ui, _uniformfv, _uniformiv, _uniformuiv, _uniformMatrix, getAttachedShaders, getTranslatedShaderSource,
    getFragDataLocation, getUniformBlockIndex, getActiveUniformBlockParameter, getActiveUniformBlockName, uniformBlockBinding,
    getUniformIndices, getActiveUniforms, transformFeedbackVaryings, getTransformFeedbackVarying,
    // 纹理（ops_texture.rs）
    createTexture, deleteTexture, bindTexture, texParameterf, texParameteri, getTexParameter, generateMipmap,
    _texImage2D, _texImage2DSource, _texSubImage2D, _texSubImage2DSource, copyTexImage2D, copyTexSubImage2D,
    _compressedTexImage2D, _compressedTexSubImage2D, _texImage3D, _texSubImage3D, texStorage2D, texStorage3D,
    copyTexSubImage3D, _compressedTexImage3D, _compressedTexSubImage3D, _texImage3DSource, _texSubImage3DSource,
    // 帧缓冲（ops_fbo.rs）
    createFramebuffer, deleteFramebuffer, bindFramebuffer, framebufferTexture2D, framebufferRenderbuffer,
    checkFramebufferStatus, getFramebufferAttachmentParameter, createRenderbuffer, deleteRenderbuffer,
    bindRenderbuffer, renderbufferStorage, getRenderbufferParameter, _readPixels, blitFramebuffer,
    framebufferTextureLayer, invalidateFramebuffer, invalidateSubFramebuffer, readBuffer, renderbufferStorageMultisample,
    getInternalformatParameter, _clearBufferfv, _clearBufferiv, _clearBufferuiv, clearBufferfi,
    // WebGL2 其余（ops_gl2.rs）
    createSampler, deleteSampler, bindSampler, samplerParameteri, samplerParameterf, getSamplerParameter,
    createQuery, deleteQuery, beginQuery, endQuery, getQueryParameter, fenceSync, deleteSync,
    clientWaitSync, waitSync, getSyncParameter, createTransformFeedback, deleteTransformFeedback,
    bindTransformFeedback, beginTransformFeedback, endTransformFeedback, pauseTransformFeedback, resumeTransformFeedback,
    getIndexedParameter,
}

/// 原生调用的环境（纹理来源要取图片 / 画布像素）
pub struct Env<'r, 'js> {
    pub ctx: Ctx<'js>,
    pub canvases: &'r RefCell<GameCanvases>,
    pub images: &'r RefCell<ImageStore>,
}

pub fn register(rt: &JsRuntime, gl: Rc<RefCell<WebGl>>, canvases: Rc<RefCell<GameCanvases>>, images: Rc<RefCell<ImageStore>>) -> Result<(), String> {
    rt.register_raw("__game_gl_ops", |ctx, _| NAMES.to_vec().into_js(&ctx))?;
    // (画布 id, 版本 1|2, 属性 JSON) → 实际属性 JSON 串；建不了返回 null（getContext 返回 null）
    let (g, c) = (gl.clone(), canvases.clone());
    rt.register_raw("__game_gl_create", move |ctx, args| {
        let a = A(&args.0);
        let attrs: serde_json::Value = serde_json::from_str(&a.s(2)).unwrap_or_default();
        match g.borrow_mut().create(&c, a.u(0), a.u(1).clamp(1, 2) as u8, &attrs) {
            Some(json) => json.into_js(&ctx),
            None => null(&ctx),
        }
    })?;
    let g = gl.clone();
    rt.register_raw("__game_gl_resize", move |ctx, args| {
        let a = A(&args.0);
        g.borrow_mut().resize(a.u(0), a.u(1), a.u(2));
        Ok(Value::new_undefined(ctx))
    })?;
    // 当作图片源用之前（drawImage / texImage2D / toDataURL）：把当前绘图缓冲读回画布后备缓冲
    let (g, c) = (gl.clone(), canvases.clone());
    rt.register_raw("__game_gl_sync", move |ctx, args| {
        g.borrow_mut().sync(&c, A(&args.0).u(0));
        Ok(Value::new_undefined(ctx))
    })?;
    let g = gl.clone();
    rt.register_raw("__game_gl_free", move |ctx, args| {
        g.borrow_mut().free(A(&args.0).u(0));
        Ok(Value::new_undefined(ctx))
    })?;
    // WEBGL_lose_context.loseContext：标丢失，帧末派发 webglcontextlost
    let g = gl.clone();
    rt.register_raw("__game_gl_lose", move |ctx, args| {
        g.borrow_mut().lose(A(&args.0).u(0));
        Ok(Value::new_undefined(ctx))
    })?;
    // WEBGL_lose_context.restoreContext / 丢失后游戏 preventDefault：重建上下文（旧对象全部作废，JS 换代）
    let (g, c) = (gl.clone(), canvases.clone());
    rt.register_raw("__game_gl_restore", move |ctx, args| g.borrow_mut().restore(&c, A(&args.0).u(0)).into_js(&ctx))?;
    rt.register_raw("__game_gl", move |ctx, args| {
        let a = A(&args.0);
        let id = a.u(0);
        let Some(&op) = ALL.get(a.u(1) as usize) else { return null(&ctx) };
        let rest = A(&args.0[2.min(args.0.len())..]);
        let mut w = gl.borrow_mut();
        let Some(c) = w.current(id) else { return null(&ctx) };
        if c.lost {
            return lost_result(&ctx, c, op);
        }
        let env = Env { ctx: ctx.clone(), canvases: &canvases, images: &images };
        // glow 对驱动没给的函数是 panic（「called glXxx but it was not loaded」）：不能让它穿过 QuickJS 的 C 栈帧，
        // 接住后按 INVALID_OPERATION 处理并记一行日志
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ops_state::call(c, &env, op, &rest)
                .or_else(|| ops_buffer::call(c, &env, op, &rest))
                .or_else(|| ops_shader::call(c, &env, op, &rest))
                .or_else(|| ops_texture::call(c, &env, op, &rest))
                .or_else(|| ops_fbo::call(c, &env, op, &rest))
                .or_else(|| ops_gl2::call(c, &env, op, &rest))
        }));
        match r {
            Ok(r) => r.unwrap_or_else(|| null(&ctx)),
            Err(p) => {
                let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                c.error(glow::INVALID_OPERATION);
                w.notes.push(format!("⚠️ WebGL {op:?}：驱动缺这个函数（{msg}），按 INVALID_OPERATION 处理"));
                null(&ctx)
            }
        }
    })?;
    Ok(())
}

/// 上下文丢失时：getError 报一次 CONTEXT_LOST_WEBGL，is* 报 false，其余 null（WebGL 规范 §5.15.2）
fn lost_result<'js>(ctx: &Ctx<'js>, c: &mut super::ctx::GlCtx, op: Op) -> JsResult<Value<'js>> {
    use Op::*;
    match op {
        getError => c.get_error().into_js(ctx),
        isEnabled => false.into_js(ctx),
        checkFramebufferStatus => 0x8CDDu32.into_js(ctx),
        getAttribLocation | getFragDataLocation => (-1i32).into_js(ctx),
        clientWaitSync => 0x911Du32.into_js(ctx), // WAIT_FAILED
        _ => null(ctx),
    }
}
