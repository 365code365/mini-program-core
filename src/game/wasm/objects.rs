//! WXWebAssembly.Memory / Table / Global 的原生入口（JS 外壳 game-wasm.js）。

use super::conv::{from_val, to_val, ty_from, ty_name};
use super::{refresh_buffers, with_store, Rt};
use crate::js::JsRuntime;
use rquickjs::{Exception, IntoJs, Value};
use std::cell::RefCell;
use std::rc::Rc;
use wasmi::{Mutability, Nullable, Ref, RefType, ValType};

fn num(v: Option<&Value>) -> f64 {
    v.and_then(|x| x.as_number()).unwrap_or(f64::NAN)
}
fn opt(v: Option<&Value>) -> Option<u64> {
    v.and_then(|x| x.as_number()).filter(|n| n.is_finite() && *n >= 0.0).map(|n| n as u64)
}
fn text(v: Option<&Value>) -> String {
    v.and_then(|x| x.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default()
}

pub fn register(rt: &JsRuntime, w: Rc<RefCell<Rt>>) -> Result<(), String> {
    // Memory(initial, maximum)：页（64KiB）
    let r = w.clone();
    rt.register_raw("__wasm_mem_new", move |ctx, a| {
        let (min, max) = (opt(a.0.first()).unwrap_or(0), opt(a.0.get(1)));
        if min > 65536 || max.is_some_and(|m| m < min || m > 65536) {
            return Err(Exception::throw_range(&ctx, "WXWebAssembly.Memory：initial / maximum 超出范围"));
        }
        let ty = wasmi::MemoryType::new(min as u32, max.map(|m| m as u32));
        let m = with_store(&r, |s| wasmi::Memory::new(s, ty)).map_err(|e| Exception::throw_message(&ctx, &e))?.map_err(|e| Exception::throw_range(&ctx, &e.to_string()))?;
        r.borrow_mut().mem_id(m).into_js(&ctx)
    })?;
    // buffer：指向线性内存的 ArrayBuffer（同一块没搬家就返回同一个对象）
    let r = w.clone();
    rt.register_raw("__wasm_mem_buffer", move |ctx, a| {
        refresh_buffers(&r, &ctx);
        let id = num(a.0.first()) as usize;
        let mem = r.borrow().mems.get(id).map(|m| m.mem).ok_or_else(|| Exception::throw_type(&ctx, "内存不存在"))?;
        let bufs: rquickjs::Object = ctx.globals().get("__wasmBufs")?;
        if r.borrow().mems[id].buf.is_some() {
            return bufs.get(id as u32);
        }
        let (ptr, len) = with_store(&r, |s| (mem.data_ptr(&s) as usize, mem.data_size(&s))).map_err(|e| Exception::throw_message(&ctx, &e))?;
        let ab = super::linear_buffer(&ctx, ptr as *mut u8, len)?;
        bufs.set(id as u32, ab.clone())?;
        r.borrow_mut().mems[id].buf = Some((ptr, len));
        Ok(ab)
    })?;
    let r = w.clone();
    rt.register_raw("__wasm_mem_grow", move |ctx, a| {
        let id = num(a.0.first()) as usize;
        let delta = opt(a.0.get(1)).ok_or_else(|| Exception::throw_type(&ctx, "grow 的参数要是非负整数"))?;
        let mem = r.borrow().mems.get(id).map(|m| m.mem).ok_or_else(|| Exception::throw_type(&ctx, "内存不存在"))?;
        let old = with_store(&r, |s| mem.grow(s, delta)).map_err(|e| Exception::throw_message(&ctx, &e))?.map_err(|_| Exception::throw_range(&ctx, "WXWebAssembly.Memory.grow：超过 maximum"))?;
        // 标准：grow（哪怕 0）之后旧 buffer 一律 detached
        if r.borrow_mut().mems[id].buf.take().is_some() {
            super::detach_js(&ctx, id);
        }
        (old as f64).into_js(&ctx)
    })?;
    // Table(element, initial, maximum)
    let r = w.clone();
    rt.register_raw("__wasm_table_new", move |ctx, a| {
        let el = match text(a.0.first()).as_str() { "externref" => RefType::Extern, _ => RefType::Func };
        let (min, max) = (opt(a.0.get(1)).unwrap_or(0), opt(a.0.get(2)));
        if max.is_some_and(|m| m < min) || min > 10_000_000 {
            return Err(Exception::throw_range(&ctx, "WXWebAssembly.Table：initial / maximum 超出范围"));
        }
        let ty = wasmi::TableType::new(el, min as u32, max.map(|m| m as u32));
        let t = with_store(&r, |s| wasmi::Table::new(s, ty, Ref::null(el))).map_err(|e| Exception::throw_message(&ctx, &e))?.map_err(|e| Exception::throw_range(&ctx, &e.to_string()))?;
        let mut w = r.borrow_mut();
        w.tables.push(t);
        ((w.tables.len() - 1) as u32).into_js(&ctx)
    })?;
    // (表, 下标) → 函数编号 / externref 编号 / null；越界 RangeError
    let r = w.clone();
    rt.register_raw("__wasm_table_get", move |ctx, a| {
        let t = r.borrow().tables.get(num(a.0.first()) as usize).copied().ok_or_else(|| Exception::throw_type(&ctx, "表不存在"))?;
        let i = opt(a.0.get(1)).ok_or_else(|| Exception::throw_type(&ctx, "下标要是非负整数"))?;
        let v = with_store(&r, |s| t.get(&s, i)).map_err(|e| Exception::throw_message(&ctx, &e))?.ok_or_else(|| Exception::throw_range(&ctx, "Table.get：下标越界"))?;
        match v {
            Ref::Func(Nullable::Val(f)) => r.borrow_mut().func_id(&f).into_js(&ctx),
            Ref::Extern(Nullable::Val(e)) => with_store(&r, |s| e.data(&s).downcast_ref::<u32>().copied().unwrap_or(0)).unwrap_or(0).into_js(&ctx),
            _ => Ok(Value::new_null(ctx)),
        }
    })?;
    // (表, 下标, 值编号 | null)
    let r = w.clone();
    rt.register_raw("__wasm_table_set", move |ctx, a| {
        let t = r.borrow().tables.get(num(a.0.first()) as usize).copied().ok_or_else(|| Exception::throw_type(&ctx, "表不存在"))?;
        let i = opt(a.0.get(1)).ok_or_else(|| Exception::throw_type(&ctx, "下标要是非负整数"))?;
        let el = with_store(&r, |s| t.ty(&s).element()).map_err(|e| Exception::throw_message(&ctx, &e))?;
        let v = a.0.get(2).and_then(|x| x.as_number()).map(|x| x as u32);
        let rf = match (el, v) {
            (RefType::Func, Some(id)) => Ref::Func(Nullable::Val(r.borrow().funcs.get(id as usize).copied().ok_or_else(|| Exception::throw_type(&ctx, "只能放 wasm 导出的函数"))?)),
            (RefType::Extern, Some(id)) => Ref::Extern(Nullable::Val(with_store(&r, |s| wasmi::ExternRef::new(s, id)).map_err(|e| Exception::throw_message(&ctx, &e))?)),
            (ty, None) => Ref::null(ty),
        };
        with_store(&r, |s| t.set(s, i, rf)).map_err(|e| Exception::throw_message(&ctx, &e))?.map_err(|_| Exception::throw_range(&ctx, "Table.set：下标越界"))?;
        Ok(Value::new_undefined(ctx))
    })?;
    let r = w.clone();
    rt.register_raw("__wasm_table_size", move |ctx, a| {
        let t = r.borrow().tables.get(num(a.0.first()) as usize).copied().ok_or_else(|| Exception::throw_type(&ctx, "表不存在"))?;
        (with_store(&r, |s| t.size(&s)).unwrap_or(0) as f64).into_js(&ctx)
    })?;
    let r = w.clone();
    rt.register_raw("__wasm_table_grow", move |ctx, a| {
        let t = r.borrow().tables.get(num(a.0.first()) as usize).copied().ok_or_else(|| Exception::throw_type(&ctx, "表不存在"))?;
        let d = opt(a.0.get(1)).ok_or_else(|| Exception::throw_type(&ctx, "grow 的参数要是非负整数"))?;
        let el = with_store(&r, |s| t.ty(&s).element()).map_err(|e| Exception::throw_message(&ctx, &e))?;
        let old = with_store(&r, |s| t.grow(s, d, Ref::null(el))).map_err(|e| Exception::throw_message(&ctx, &e))?.map_err(|_| Exception::throw_range(&ctx, "Table.grow：超过 maximum"))?;
        (old as f64).into_js(&ctx)
    })?;
    // Global({value, mutable}, 初值)
    let r = w.clone();
    rt.register_raw("__wasm_global_new", move |ctx, a| {
        let t = ty_from(&text(a.0.first())).ok_or_else(|| Exception::throw_type(&ctx, "WXWebAssembly.Global：value 只能是 i32 / i64 / f32 / f64 / anyfunc / externref"))?;
        let m = if a.0.get(1).and_then(|x| x.as_bool()).unwrap_or(false) { Mutability::Var } else { Mutability::Const };
        let init = a.0.get(2).cloned().unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        // 没给初值：数值类型 0（i64 是 0n），引用类型 null
        let v = if init.is_undefined() { wasmi::Val::default_for_ty(t) } else {
            let fr = |id: u32| r.borrow().funcs.get(id as usize).copied();
            let mut ext = |id: u32| with_store(&r, |s| wasmi::ExternRef::new(s, id)).expect("store");
            to_val(&ctx, &init, &t, &fr, &mut ext)?
        };
        let g = with_store(&r, |s| wasmi::Global::new(s, v, m)).map_err(|e| Exception::throw_message(&ctx, &e))?;
        let mut w = r.borrow_mut();
        w.globals.push(g);
        ((w.globals.len() - 1) as u32).into_js(&ctx)
    })?;
    let r = w.clone();
    rt.register_raw("__wasm_global_type", move |ctx, a| {
        let g = r.borrow().globals.get(num(a.0.first()) as usize).copied().ok_or_else(|| Exception::throw_type(&ctx, "全局不存在"))?;
        let t = with_store(&r, |s| g.ty(&s)).map_err(|e| Exception::throw_message(&ctx, &e))?;
        vec![ty_name(&t.content()).to_string(), if t.mutability().is_mut() { "1".into() } else { "0".into() }].into_js(&ctx)
    })?;
    let r = w.clone();
    rt.register_raw("__wasm_global_get", move |ctx, a| {
        let g = r.borrow().globals.get(num(a.0.first()) as usize).copied().ok_or_else(|| Exception::throw_type(&ctx, "全局不存在"))?;
        let v = with_store(&r, |s| g.get(&s)).map_err(|e| Exception::throw_message(&ctx, &e))?;
        let ext = |e: &wasmi::ExternRef| with_store(&r, |s| e.data(&s).downcast_ref::<u32>().copied().unwrap_or(0)).unwrap_or(0);
        from_val(&ctx, &v, &mut |f| r.borrow_mut().func_id(f), &ext)
    })?;
    let r = w;
    rt.register_raw("__wasm_global_set", move |ctx, a| {
        let g = r.borrow().globals.get(num(a.0.first()) as usize).copied().ok_or_else(|| Exception::throw_type(&ctx, "全局不存在"))?;
        let t: ValType = with_store(&r, |s| g.ty(&s).content()).map_err(|e| Exception::throw_message(&ctx, &e))?;
        let v = a.0.get(1).cloned().unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        let fr = |id: u32| r.borrow().funcs.get(id as usize).copied();
        let mut ext = |id: u32| with_store(&r, |s| wasmi::ExternRef::new(s, id)).expect("store");
        let v = to_val(&ctx, &v, &t, &fr, &mut ext)?;
        with_store(&r, |s| g.set(s, v)).map_err(|e| Exception::throw_message(&ctx, &e))?.map_err(|_| Exception::throw_type(&ctx, "Global 不可变"))?;
        Ok(Value::new_undefined(ctx))
    })?;
    Ok(())
}
