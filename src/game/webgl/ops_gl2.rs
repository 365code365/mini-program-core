//! WebGL2 其余对象：采样器、查询、同步、变换反馈、getIndexedParameter。
//!
//! WebGLSync 背后是 GLsync 指针：不交给 JS，JS 拿到的是本上下文里的编号（ctx.syncs）。

use super::args::{js, null, query, sampler, undef, xfb, A};
use super::ctx::GlCtx;
use super::natives::{Env, Op};
use glow::HasContext;
use rquickjs::{IntoJs, Result as JsResult, Value};

pub fn call<'js>(c: &mut GlCtx, e: &Env<'_, 'js>, op: Op, a: &A<'_, 'js>) -> Option<JsResult<Value<'js>>> {
    use Op::*;
    let ctx = &e.ctx;
    unsafe {
        match op {
            createSampler => return Some(c.gl.create_sampler().map(|s| s.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteSampler => { if let Some(s) = sampler(a.u(0)) { c.gl.delete_sampler(s) } }
            bindSampler => c.gl.bind_sampler(a.u(0), sampler(a.u(1))),
            samplerParameteri => { if let Some(s) = sampler(a.u(0)) { c.gl.sampler_parameter_i32(s, a.u(1), a.i(2)) } }
            samplerParameterf => { if let Some(s) = sampler(a.u(0)) { c.gl.sampler_parameter_f32(s, a.u(1), a.f(2)) } }
            getSamplerParameter => {
                let Some(s) = sampler(a.u(0)) else { return Some(null(ctx)) };
                let p = a.u(1);
                return Some(match p {
                    glow::TEXTURE_MIN_LOD | glow::TEXTURE_MAX_LOD | 0x84FE => c.gl.get_sampler_parameter_f32(s, p).into_js(ctx),
                    _ => c.gl.get_sampler_parameter_i32(s, p).into_js(ctx),
                });
            }
            createQuery => return Some(c.gl.create_query().map(|q| q.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteQuery => { if let Some(q) = query(a.u(0)) { c.gl.delete_query(q) } }
            beginQuery => { if let Some(q) = query(a.u(1)) { c.gl.begin_query(a.u(0), q) } }
            endQuery => c.gl.end_query(a.u(0)),
            getQueryParameter => {
                let Some(q) = query(a.u(0)) else { return Some(null(ctx)) };
                let p = a.u(1);
                let v = c.gl.get_query_parameter_u32(q, p);
                return Some(if p == glow::QUERY_RESULT_AVAILABLE { (v != 0).into_js(ctx) } else { v.into_js(ctx) });
            }
            fenceSync => {
                return Some(match c.gl.fence_sync(a.u(0), a.u(1)) {
                    Ok(f) => {
                        c.next_sync += 1;
                        c.syncs.insert(c.next_sync, f);
                        js(ctx, c.next_sync)
                    }
                    Err(_) => null(ctx),
                });
            }
            deleteSync => { if let Some(f) = c.syncs.remove(&a.u(0)) { c.gl.delete_sync(f) } }
            clientWaitSync => {
                let Some(f) = c.syncs.get(&a.u(0)).copied() else {
                    c.error(glow::INVALID_VALUE);
                    return Some(glow::WAIT_FAILED.into_js(ctx));
                };
                // WebGL：超时只能是 0（MAX_CLIENT_WAIT_TIMEOUT_WEBGL = 0），不许阻塞引擎线程
                if a.l(2) > 0 {
                    c.error(glow::INVALID_OPERATION);
                    return Some(glow::WAIT_FAILED.into_js(ctx));
                }
                return Some(c.gl.client_wait_sync(f, a.u(1), 0).into_js(ctx));
            }
            waitSync => { if let Some(f) = c.syncs.get(&a.u(0)).copied() { c.gl.wait_sync(f, a.u(1), glow::TIMEOUT_IGNORED as u64) } }
            getSyncParameter => {
                let Some(f) = c.syncs.get(&a.u(0)).copied() else { return Some(null(ctx)) };
                return Some((c.gl.get_sync_parameter_i32(f, a.u(1)) as u32).into_js(ctx));
            }
            createTransformFeedback => return Some(c.gl.create_transform_feedback().map(|t| t.0.get()).map_or_else(|_| null(ctx), |n| js(ctx, n))),
            deleteTransformFeedback => { if let Some(t) = xfb(a.u(0)) { c.gl.delete_transform_feedback(t) } }
            bindTransformFeedback => c.gl.bind_transform_feedback(a.u(0), xfb(a.u(1))),
            beginTransformFeedback => c.gl.begin_transform_feedback(a.u(0)),
            endTransformFeedback => c.gl.end_transform_feedback(),
            pauseTransformFeedback => c.gl.pause_transform_feedback(),
            resumeTransformFeedback => c.gl.resume_transform_feedback(),
            getIndexedParameter => {
                let (p, i) = (a.u(0), a.u(1));
                return Some(match p {
                    glow::TRANSFORM_FEEDBACK_BUFFER_START | glow::TRANSFORM_FEEDBACK_BUFFER_SIZE | glow::UNIFORM_BUFFER_START | glow::UNIFORM_BUFFER_SIZE => (c.gl.get_parameter_indexed_i64(p, i) as f64).into_js(ctx),
                    // 绑定（名字，JS 换成对象）
                    _ => c.gl.get_parameter_indexed_i32(p, i).into_js(ctx),
                });
            }
            _ => return None,
        }
    }
    Some(undef(ctx))
}
