//! 原生调用的参数与 GL 名字换算。WebGL 的整数参数按 WebIDL 的 ToInt32 / ToUint32（取整后按 2^32 回绕），
//! GLboolean 按 JS 的真假值。

use rquickjs::{Ctx, IntoJs, Result as JsResult, Value};
use std::num::NonZeroU32;

pub struct A<'a, 'js>(pub &'a [Value<'js>]);

impl<'a, 'js> A<'a, 'js> {
    /// WebIDL 的数值参数 = ToNumber（"2" → 2、[2] → 2、"foo" → NaN → 0）。数字直接取（热路径），别的才走 JS 的强制转换
    pub fn n(&self, i: usize) -> f64 {
        match self.0.get(i) {
            Some(v) => match v.as_number() {
                Some(x) => x,
                None if v.is_bool() => v.as_bool().unwrap_or(false) as i32 as f64,
                None if v.is_null() || v.is_undefined() => if v.is_undefined() { f64::NAN } else { 0.0 },
                None => v.get::<rquickjs::convert::Coerced<f64>>().map(|c| c.0).unwrap_or(f64::NAN),
            },
            None => 0.0,
        }
    }
    pub fn f(&self, i: usize) -> f32 {
        self.n(i) as f32
    }
    /// GLclampf / GLfloat 里 undefined 是 NaN（ToNumber）；整数参数里 NaN 一律按 0
    pub fn i(&self, i: usize) -> i32 {
        let x = self.n(i);
        if x.is_finite() { (x.trunc() as i64) as i32 } else { 0 }
    }
    pub fn u(&self, i: usize) -> u32 {
        self.i(i) as u32
    }
    /// GLintptr / GLsizeiptr（偏移、尺寸）：64 位，负数原样留着给调用方判
    pub fn l(&self, i: usize) -> i64 {
        let x = self.n(i);
        if x.is_finite() { x.trunc() as i64 } else { 0 }
    }
    pub fn b(&self, i: usize) -> bool {
        match self.0.get(i) {
            Some(v) if v.is_bool() => v.as_bool().unwrap_or(false),
            Some(v) if v.is_number() => v.as_number().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(false),
            Some(v) => !(v.is_null() || v.is_undefined()),
            None => false,
        }
    }
    pub fn s(&self, i: usize) -> String {
        self.0.get(i).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default()
    }
    pub fn v(&self, i: usize) -> Option<&Value<'js>> {
        self.0.get(i).filter(|v| !(v.is_null() || v.is_undefined()))
    }
}

pub fn nz(id: u32) -> Option<NonZeroU32> {
    NonZeroU32::new(id)
}
pub fn buf(id: u32) -> Option<glow::NativeBuffer> {
    nz(id).map(glow::NativeBuffer)
}
pub fn tex(id: u32) -> Option<glow::NativeTexture> {
    nz(id).map(glow::NativeTexture)
}
pub fn fbo(id: u32) -> Option<glow::NativeFramebuffer> {
    nz(id).map(glow::NativeFramebuffer)
}
pub fn rbo(id: u32) -> Option<glow::NativeRenderbuffer> {
    nz(id).map(glow::NativeRenderbuffer)
}
pub fn shader(id: u32) -> Option<glow::NativeShader> {
    nz(id).map(glow::NativeShader)
}
pub fn program(id: u32) -> Option<glow::NativeProgram> {
    nz(id).map(glow::NativeProgram)
}
pub fn vao(id: u32) -> Option<glow::NativeVertexArray> {
    nz(id).map(glow::NativeVertexArray)
}
pub fn sampler(id: u32) -> Option<glow::NativeSampler> {
    nz(id).map(glow::NativeSampler)
}
pub fn query(id: u32) -> Option<glow::NativeQuery> {
    nz(id).map(glow::NativeQuery)
}
pub fn xfb(id: u32) -> Option<glow::NativeTransformFeedback> {
    nz(id).map(glow::NativeTransformFeedback)
}

pub fn undef<'js>(ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
    Ok(Value::new_undefined(ctx.clone()))
}
pub fn null<'js>(ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
    Ok(Value::new_null(ctx.clone()))
}
pub fn js<'js, T: IntoJs<'js>>(ctx: &Ctx<'js>, v: T) -> JsResult<Value<'js>> {
    v.into_js(ctx)
}

/// 字节 → f32 / i32 / u32 切片（JS 的 Float32Array 等按 4 字节对齐；没对齐就拷一份）
pub fn cast<T: Copy + Default>(b: &[u8]) -> std::borrow::Cow<'_, [T]> {
    let sz = std::mem::size_of::<T>();
    let n = b.len() / sz;
    if (b.as_ptr() as usize) % std::mem::align_of::<T>() == 0 {
        // SAFETY: 对齐已判、长度按元素取整；f32 / i32 / u32 任意位模式都合法
        std::borrow::Cow::Borrowed(unsafe { std::slice::from_raw_parts(b.as_ptr() as *const T, n) })
    } else {
        let mut v = vec![T::default(); n];
        // SAFETY: 目标是 n 个 T 的缓冲，源至少 n*sz 字节
        unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), v.as_mut_ptr() as *mut u8, n * sz) };
        std::borrow::Cow::Owned(v)
    }
}
