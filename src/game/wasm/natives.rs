//! WXWebAssembly 的原生入口（JS 外壳 game-wasm.js）。对象一律按编号来回：
//! `__wasm_compile(路径)` → 模块描述 JSON；`__wasm_instantiate(模块, 导入数组)` → 导出 JSON；
//! `__wasm_call(函数, ...参数)`；内存 / 表 / 全局各几个。错误：编译失败抛 `CompileError: …`、
//! 链接失败抛 `LinkError: …`、执行陷入抛 `RuntimeError: …`（JS 外壳换成对应的错误类）；JS 导入函数抛的原样传出去。

use super::conv::{from_val, to_val, ty_name, zero};
use super::{host, refresh_buffers, with_store, Rt};
use crate::js::JsRuntime;
use rquickjs::{Array, Ctx, Exception, Function, IntoJs, Persistent, Result as JsResult, Value};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;
use wasmi::{Extern, ExternType, Val, ValType};

thread_local! {
    /// JS 导入函数抛的异常：wasm 那边先以宿主错误中止，回到最外层再原样抛给 JS
    static PENDING: RefCell<Option<Persistent<Value<'static>>>> = const { RefCell::new(None) };
}

fn num(v: Option<&Value>) -> f64 {
    v.and_then(|x| x.as_number()).unwrap_or(0.0)
}
fn text(v: Option<&Value>) -> String {
    v.and_then(|x| x.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default()
}
fn err<'js>(ctx: &Ctx<'js>, kind: &str, e: impl std::fmt::Display) -> rquickjs::Error {
    Exception::throw_message(ctx, &format!("{kind}: {e}"))
}

fn func_sig(t: &wasmi::FuncType) -> serde_json::Value {
    json!({ "params": t.params().iter().map(ty_name).collect::<Vec<_>>(), "results": t.results().iter().map(ty_name).collect::<Vec<_>>() })
}

fn extern_desc(t: &ExternType) -> serde_json::Value {
    match t {
        ExternType::Func(f) => json!({ "kind": "function", "type": func_sig(f) }),
        ExternType::Memory(m) => json!({ "kind": "memory", "min": m.minimum(), "max": m.maximum() }),
        ExternType::Table(t) => json!({ "kind": "table", "element": ty_name(&ValType::from(t.element())), "min": t.minimum(), "max": t.maximum() }),
        ExternType::Global(g) => json!({ "kind": "global", "value": ty_name(&g.content()), "mutable": g.mutability().is_mut() }),
    }
}

/// JS 导入函数的回调（wasm → JS）：参数转 JS、调 `__wasmHostCall(编号, 参数数组)`、结果转回 wasm
fn host_cb<'a, 'js: 'a>(ctx: &'a Ctx<'js>, rt: &'a Rc<RefCell<Rt>>) -> Box<dyn Fn(u32, &[Val], &mut [Val]) -> Result<(), wasmi::Error> + 'a> {
    Box::new(move |hid, params, results| {
        refresh_buffers(rt, ctx);
        let mut run = || -> JsResult<()> {
            let args = Array::new(ctx.clone())?;
            for (i, p) in params.iter().enumerate() {
                let ext = |e: &wasmi::ExternRef| with_store(rt, |s| e.data(&s).downcast_ref::<u32>().copied().unwrap_or(0)).unwrap_or(0);
                let v = from_val(ctx, p, &mut |f| rt.borrow_mut().func_id(f), &ext)?;
                args.set(i, v)?;
            }
            let call: Function = ctx.globals().get("__wasmHostCall")?;
            let r: Value = call.call((hid, args))?;
            let tys: Vec<ValType> = rt.borrow().hosts.get(hid as usize).map(|t| t.results().to_vec()).unwrap_or_default();
            let vals: Vec<Value> = if tys.len() > 1 {
                let a: Array = r.into_array().ok_or_else(|| Exception::throw_type(ctx, "多返回值的导入函数要返回数组"))?;
                a.iter::<Value>().collect::<JsResult<_>>()?
            } else {
                vec![r]
            };
            for (i, t) in tys.iter().enumerate() {
                let v = vals.get(i).cloned().unwrap_or_else(|| Value::new_undefined(ctx.clone()));
                let fr = |id: u32| rt.borrow().funcs.get(id as usize).copied();
                let mut ext = |id: u32| with_store(rt, |s| wasmi::ExternRef::new(s, id)).expect("store");
                results[i] = to_val(ctx, &v, t, &fr, &mut ext)?;
            }
            Ok(())
        };
        match run() {
            Ok(()) => Ok(()),
            Err(_) => {
                let e = ctx.catch();
                let p = Persistent::save(ctx, e);
                // SAFETY: 只在同一个运行时、同一个线程上恢复（最外层原生调用里）
                let p: Persistent<Value<'static>> = unsafe { std::mem::transmute(p) };
                PENDING.with(|x| *x.borrow_mut() = Some(p));
                Err(wasmi::Error::new("JS 导入函数抛出异常"))
            }
        }
    })
}

/// wasm 执行出错：JS 导入函数抛的原样抛；其余是陷入 / 链接错误
fn rethrow<'js>(ctx: &Ctx<'js>, kind: &str, e: wasmi::Error) -> rquickjs::Error {
    if let Some(p) = PENDING.with(|x| x.borrow_mut().take()) {
        if let Ok(v) = p.restore(ctx) {
            return ctx.throw(v);
        }
    }
    err(ctx, kind, e)
}

pub fn register(rt: &JsRuntime, w: Rc<RefCell<Rt>>) -> Result<(), String> {
    let r = w.clone();
    rt.register_raw("__wasm_compile", move |ctx, a| {
        let bytes = r.borrow().read(&text(a.0.first())).map_err(|e| err(&ctx, "CompileError", e))?;
        let m = wasmi::Module::new(&r.borrow().engine, &bytes).map_err(|e| err(&ctx, "CompileError", e))?;
        let imports: Vec<_> = m.imports().map(|i| { let mut d = extern_desc(i.ty()); d["module"] = json!(i.module()); d["name"] = json!(i.name()); d }).collect();
        let exports: Vec<_> = m.exports().map(|e| { let mut d = extern_desc(e.ty()); d["name"] = json!(e.name()); d }).collect();
        let mut w = r.borrow_mut();
        w.modules.push(m);
        json!({ "id": w.modules.len() - 1, "imports": imports, "exports": exports }).to_string().into_js(&ctx)
    })?;
    // (模块, [[种类, 编号], …]) 与模块的导入一一对应。种类：h = JS 函数（__wasmHosts 编号）、f = wasm 函数、m / t / g = 内存 / 表 / 全局
    let r = w.clone();
    rt.register_raw("__wasm_instantiate", move |ctx, a| {
        let mid = num(a.0.first()) as usize;
        let list: Vec<(String, u32)> = a.0.get(1).and_then(|v| v.as_array()).map(|arr| {
            arr.iter::<Array>().filter_map(|x| x.ok()).map(|p| (p.get::<String>(0).unwrap_or_default(), p.get::<f64>(1).unwrap_or(0.0) as u32)).collect()
        }).unwrap_or_default();
        let module = r.borrow().modules.get(mid).cloned().ok_or_else(|| err(&ctx, "LinkError", "模块不存在"))?;
        let cb = host_cb(&ctx, &r);
        let res = host::with_host(&*cb, || {
            with_store(&r, |mut s| -> Result<Vec<(String, Extern)>, wasmi::Error> {
                let mut linker = wasmi::Linker::<()>::new(s.engine());
                for (imp, (kind, id)) in module.imports().zip(list.iter()) {
                    let ext: Extern = match kind.as_str() {
                        "h" => {
                            let ExternType::Func(ft) = imp.ty() else { return Err(wasmi::Error::new(format!("{}.{} 不是函数导入", imp.module(), imp.name()))) };
                            let hid = *id;
                            {
                                let mut w = r.borrow_mut();
                                let n = w.hosts.len().max(hid as usize + 1);
                                w.hosts.resize(n, ft.clone());
                                w.hosts[hid as usize] = ft.clone();
                            }
                            wasmi::Func::new(&mut s, ft.clone(), move |mut c, p, out| host::call_host(&mut c, hid, p, out)).into()
                        }
                        "f" => r.borrow().funcs.get(*id as usize).copied().ok_or_else(|| wasmi::Error::new("函数不存在"))?.into(),
                        "m" => r.borrow().mems.get(*id as usize).map(|m| m.mem).ok_or_else(|| wasmi::Error::new("内存不存在"))?.into(),
                        "t" => r.borrow().tables.get(*id as usize).copied().ok_or_else(|| wasmi::Error::new("表不存在"))?.into(),
                        "g" => r.borrow().globals.get(*id as usize).copied().ok_or_else(|| wasmi::Error::new("全局不存在"))?.into(),
                        _ => return Err(wasmi::Error::new(format!("导入 {}.{} 缺失", imp.module(), imp.name()))),
                    };
                    linker.define(imp.module(), imp.name(), ext)?;
                }
                let inst = linker.instantiate_and_start(&mut s, &module)?;
                Ok(inst.exports(&s).map(|e| (e.name().to_string(), e.into_extern())).collect())
            })
        });
        let exports = match res {
            Ok(Ok(e)) => e,
            Ok(Err(e)) => return Err(rethrow(&ctx, if e.to_string().contains("trap") { "RuntimeError" } else { "LinkError" }, e)),
            Err(e) => return Err(err(&ctx, "LinkError", e)),
        };
        refresh_buffers(&r, &ctx);
        let mut out = Vec::new();
        let mut w = r.borrow_mut();
        for (name, e) in exports {
            let (kind, id) = match e {
                Extern::Func(f) => ("function", w.func_id(&f)),
                Extern::Memory(m) => ("memory", w.mem_id(m)),
                Extern::Table(t) => { w.tables.push(t); ("table", (w.tables.len() - 1) as u32) }
                Extern::Global(g) => { w.globals.push(g); ("global", (w.globals.len() - 1) as u32) }
            };
            out.push(json!({ "name": name, "kind": kind, "id": id }));
        }
        serde_json::Value::Array(out).to_string().into_js(&ctx)
    })?;
    let r = w.clone();
    rt.register_raw("__wasm_func_type", move |ctx, a| {
        let f = r.borrow().funcs.get(num(a.0.first()) as usize).copied().ok_or_else(|| err(&ctx, "TypeError", "函数不存在"))?;
        let t = with_store(&r, |s| f.ty(&s)).map_err(|e| err(&ctx, "Error", e))?;
        func_sig(&t).to_string().into_js(&ctx)
    })?;
    let r = w.clone();
    rt.register_raw("__wasm_call", move |ctx, a| {
        let f = r.borrow().funcs.get(num(a.0.first()) as usize).copied().ok_or_else(|| err(&ctx, "TypeError", "函数不存在"))?;
        let ty = with_store(&r, |s| f.ty(&s)).map_err(|e| err(&ctx, "Error", e))?;
        let mut args = Vec::with_capacity(ty.params().len());
        for (i, t) in ty.params().iter().enumerate() {
            let v = a.0.get(i + 1).cloned().unwrap_or_else(|| Value::new_undefined(ctx.clone()));
            let fr = |id: u32| r.borrow().funcs.get(id as usize).copied();
            let mut ext = |id: u32| with_store(&r, |s| wasmi::ExternRef::new(s, id)).expect("store");
            args.push(to_val(&ctx, &v, t, &fr, &mut ext)?);
        }
        let mut res: Vec<Val> = ty.results().iter().map(zero).collect();
        if host::caller().is_none() {
            PENDING.with(|x| x.borrow_mut().take());
        }
        let cb = host_cb(&ctx, &r);
        let out = host::with_host(&*cb, || with_store(&r, |s| f.call(s, &args, &mut res)));
        drop(cb);
        refresh_buffers(&r, &ctx);
        match out {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(rethrow(&ctx, "RuntimeError", e)),
            Err(e) => return Err(err(&ctx, "RuntimeError", e)),
        }
        let ext = |e: &wasmi::ExternRef| with_store(&r, |s| e.data(&s).downcast_ref::<u32>().copied().unwrap_or(0)).unwrap_or(0);
        let mut vals = Vec::new();
        for v in &res {
            vals.push(from_val(&ctx, v, &mut |f| r.borrow_mut().func_id(f), &ext)?);
        }
        match vals.len() {
            0 => Ok(Value::new_undefined(ctx.clone())),
            1 => Ok(vals.pop().unwrap()),
            _ => vals.into_js(&ctx),
        }
    })?;
    super::objects::register(rt, w)
}
