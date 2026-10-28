//! JS 值 ↔ wasm 值（WebAssembly JS API §ToWebAssemblyValue / ToJSValue）。
//!
//! i32：ToInt32；i64：只认 BigInt（number 抛 TypeError，与标准一致）；f32 / f64：ToNumber；v128 不过边界（TypeError）。
//! 引用类型在 JS 那边先换成编号：funcref = 函数编号（Rt.funcs），externref = JS 侧表里的编号，null = null。

use rquickjs::{BigInt, Ctx, Exception, IntoJs, Result as JsResult, Value};
use wasmi::{Nullable, Val, ValType};

pub fn ty_name(t: &ValType) -> &'static str {
    match t {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
        ValType::V128 => "v128",
        ValType::FuncRef => "anyfunc",
        ValType::ExternRef => "externref",
    }
}

pub fn ty_from(s: &str) -> Option<ValType> {
    Some(match s {
        "i32" => ValType::I32,
        "i64" => ValType::I64,
        "f32" => ValType::F32,
        "f64" => ValType::F64,
        "anyfunc" | "funcref" => ValType::FuncRef,
        "externref" => ValType::ExternRef,
        _ => return None,
    })
}

fn num(v: &Value) -> f64 {
    if let Some(n) = v.as_number() {
        n
    } else if let Some(b) = v.as_bool() {
        b as i32 as f64
    } else if v.is_undefined() {
        f64::NAN
    } else {
        0.0
    }
}

fn to_i32(x: f64) -> i32 {
    if x.is_finite() { (x.trunc() as i64 % (1i64 << 32)) as i32 } else { 0 }
}

/// JS → wasm。`func`：funcref 的编号 → Func；`ext`：externref 的编号 → ExternRef
pub fn to_val<'js>(ctx: &Ctx<'js>, v: &Value<'js>, t: &ValType, func: &dyn Fn(u32) -> Option<wasmi::Func>, ext: &mut dyn FnMut(u32) -> wasmi::ExternRef) -> JsResult<Val> {
    Ok(match t {
        ValType::I32 => Val::I32(to_i32(num(v))),
        ValType::I64 => match v.as_big_int() {
            Some(b) => Val::I64(b.clone().to_i64()?),
            None => return Err(Exception::throw_type(ctx, "i64 参数 / 返回值必须是 BigInt")),
        },
        ValType::F32 => Val::F32((num(v) as f32).into()),
        ValType::F64 => Val::F64(num(v).into()),
        ValType::V128 => return Err(Exception::throw_type(ctx, "v128 不能跨 JS 边界")),
        ValType::FuncRef => match v.as_number() {
            None => Val::FuncRef(Nullable::Null),
            Some(id) => match func(id as u32) {
                Some(f) => Val::FuncRef(Nullable::Val(f)),
                None => return Err(Exception::throw_type(ctx, "funcref 只能是 wasm 导出的函数或 null")),
            },
        },
        ValType::ExternRef => match v.as_number() {
            None => Val::ExternRef(Nullable::Null),
            Some(id) => Val::ExternRef(Nullable::Val(ext(id as u32))),
        },
    })
}

/// wasm → JS（引用类型给编号，JS 再换成对象）。`func`：Func → 编号；`ext`：ExternRef → 编号
pub fn from_val<'js>(ctx: &Ctx<'js>, v: &Val, func: &mut dyn FnMut(&wasmi::Func) -> u32, ext: &dyn Fn(&wasmi::ExternRef) -> u32) -> JsResult<Value<'js>> {
    Ok(match v {
        Val::I32(x) => x.into_js(ctx)?,
        Val::I64(x) => BigInt::from_i64(ctx.clone(), *x)?.into_value(),
        Val::F32(x) => (f32::from(*x) as f64).into_js(ctx)?,
        Val::F64(x) => f64::from(*x).into_js(ctx)?,
        Val::V128(_) => return Err(Exception::throw_type(ctx, "v128 不能跨 JS 边界")),
        Val::FuncRef(Nullable::Val(f)) => func(f).into_js(ctx)?,
        Val::ExternRef(Nullable::Val(e)) => ext(e).into_js(ctx)?,
        _ => Value::new_null(ctx.clone()),
    })
}

/// 某个类型的默认值（结果缓冲的占位）
pub fn zero(t: &ValType) -> Val {
    Val::default_for_ty(*t)
}
