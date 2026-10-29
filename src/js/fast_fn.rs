//! 热路径原生函数：绕开 rquickjs 的 `Function::new`，直接注册成 QuickJS 的 C 函数。
//!
//! 为什么要有这一条：小游戏的 WebGL 每帧要调几百次 `__game_gl`（Cocos 的 farm001 约 350~600 次）。
//! 走 `register_raw` 时 rquickjs 每次调用要：经 RustFunction 这个类的 call 虚表、按参数个数建一个 `Vec`（系统分配器，带锁）、
//! 每个参数 `JS_DupValue` + `JS_DupContext` 包成 `Value`（调完再各释放一次）。一加 PLY110 上 simpleperf：这些包装占游戏线程
//! 4.8%，比 GL 调用本身（5.2%）还贵。
//!
//! 这里的做法：`JS_NewCFunctionData` 注册一个 `extern "C"` 跳板，闭包指针拆成两个 int32 放在函数数据里；
//! 参数**借用**调用方栈上的 JSValue（不 dup，也不在这里释放），按位拷一份 `Ctx` 包成 `ManuallyDrop<Value>` 放在栈上的数组里，
//! 以 `&[Value]` 交给闭包。整次调用只有一次 `JS_DupContext` / `JS_FreeContext`。
//!
//! 约束（闭包里照常写 rquickjs 代码就满足）：参数只能借用，要留下来就 `clone()`（那时才真的 dup）。
//! 闭包装箱后归 `JsRuntime` 所有（`FastFns`），比 QuickJS 运行时活得久，函数对象不会悬空。
use rquickjs::{qjs, Ctx, Result as JsResult, Value};
use std::cell::RefCell;
use std::ffi::{c_int, CString};
use std::mem::ManuallyDrop;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;

/// 快速原生函数：参数是借来的 JS 原值，返回 JS 原值
pub type FastFn = dyn for<'js> Fn(Ctx<'js>, &[Value<'js>]) -> JsResult<Value<'js>>;

/// 参数不超过这么多个就在栈上拼切片（`__game_gl` 是 12 个：画布 id、op、最多 10 个参数）
const STACK_ARGS: usize = 16;

/// 已注册闭包的所有者（外层 Box 给出稳定的细指针，塞进函数数据）
#[derive(Default)]
pub(crate) struct FastFns(RefCell<Vec<Box<Box<FastFn>>>>);

impl FastFns {
    /// 建一个 JS 函数对象（`length` 报 0：不让 QuickJS 按形参个数补 undefined 再拷一遍参数）
    pub(crate) fn make<'js>(&self, ctx: &Ctx<'js>, f: Box<FastFn>) -> JsResult<Value<'js>> {
        let boxed: Box<Box<FastFn>> = Box::new(f);
        let addr = &*boxed as *const Box<FastFn> as usize as u64;
        self.0.borrow_mut().push(boxed);
        // 地址拆成两个 int32（Android 的堆指针带标记字节，放 f64 会丢精度）
        let mut data = [qjs::JS_MKVAL(qjs::JS_TAG_INT, addr as u32 as i32), qjs::JS_MKVAL(qjs::JS_TAG_INT, (addr >> 32) as u32 as i32)];
        // SAFETY：ctx 有效；data 是两个不带引用计数的整数，QuickJS 拷进函数对象
        let v = unsafe { qjs::JS_NewCFunctionData(ctx.as_raw().as_ptr(), Some(trampoline), 0, 0, 2, data.as_mut_ptr()) };
        if unsafe { qjs::JS_IsException(v) } {
            return Err(rquickjs::Error::Exception);
        }
        // SAFETY：v 是刚建的、归我们所有的值
        Ok(unsafe { Value::from_raw(ctx.clone(), v) })
    }
}

unsafe extern "C" fn trampoline(
    ctx: *mut qjs::JSContext,
    _this: qjs::JSValue,
    argc: c_int,
    argv: *mut qjs::JSValue,
    _magic: c_int,
    data: *mut qjs::JSValue,
) -> qjs::JSValue {
    let lo = qjs::JS_VALUE_GET_INT(*data) as u32 as u64;
    let hi = qjs::JS_VALUE_GET_INT(*data.add(1)) as u32 as u64;
    // SAFETY：make 里放进去的地址，闭包归 FastFns 所有、比运行时活得久
    let f: &FastFn = &**((hi << 32 | lo) as usize as *const Box<FastFn>);
    let raw_ctx = NonNull::new_unchecked(ctx);
    let n = argc.max(0) as usize;
    let raw: &[qjs::JSValue] = if n == 0 || argv.is_null() { &[] } else { std::slice::from_raw_parts(argv, n) };
    // glow 对驱动缺的函数是 panic：不能穿过 QuickJS 的 C 栈帧，接住转成 JS 的 InternalError
    let r = catch_unwind(AssertUnwindSafe(|| {
        let c = Ctx::from_raw(raw_ctx);
        if n <= STACK_ARGS {
            let mut views: [ManuallyDrop<Value>; STACK_ARGS] = std::array::from_fn(|i| borrow(&c, raw.get(i).copied().unwrap_or(qjs::JS_MKVAL(qjs::JS_TAG_UNDEFINED, 0))));
            call(f, c, &mut views[..n])
        } else {
            let mut views: Vec<ManuallyDrop<Value>> = raw.iter().map(|&v| borrow(&c, v)).collect();
            call(f, c, &mut views)
        }
    }));
    match r {
        Ok(Ok(v)) => {
            let out = v.as_raw();
            let vc = v.ctx().as_raw().as_ptr();
            // 值的所有权交给 QuickJS（作返回值），只放掉它里面那份 Ctx 引用
            std::mem::forget(v);
            qjs::JS_FreeContext(vc);
            out
        }
        Ok(Err(rquickjs::Error::Exception)) => qjs::JS_EXCEPTION,
        Ok(Err(e)) => throw(ctx, false, &e.to_string()),
        Err(p) => {
            let msg = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            throw(ctx, true, &format!("原生函数 panic：{msg}"))
        }
    }
}

/// 借用一个参数：按位拷一份 Ctx（不加引用计数），整个包成 ManuallyDrop —— 既不 dup 也不 free
#[inline(always)]
unsafe fn borrow<'js>(c: &Ctx<'js>, v: qjs::JSValue) -> ManuallyDrop<Value<'js>> {
    ManuallyDrop::new(Value::from_raw(std::ptr::read(c), v))
}

#[inline(always)]
fn call<'js>(f: &FastFn, c: Ctx<'js>, views: &mut [ManuallyDrop<Value<'js>>]) -> JsResult<Value<'js>> {
    // SAFETY：ManuallyDrop<T> 是 repr(transparent)，切片布局相同；闭包只拿到共享引用，挪不走也放不掉
    let args: &[Value<'js>] = unsafe { std::slice::from_raw_parts(views.as_ptr() as *const Value<'js>, views.len()) };
    f(c, args)
}

/// 与 rquickjs 的 Error::throw 同口径：转换类错误报 TypeError，panic 报 InternalError
unsafe fn throw(ctx: *mut qjs::JSContext, internal: bool, msg: &str) -> qjs::JSValue {
    let m = CString::new(msg.replace('\0', "")).unwrap_or_default();
    let fmt = c"%s".as_ptr();
    if internal {
        qjs::JS_ThrowInternalError(ctx, fmt, m.as_ptr())
    } else {
        qjs::JS_ThrowTypeError(ctx, fmt, m.as_ptr())
    }
}

#[cfg(test)]
mod tests {
    use crate::js::JsRuntime;
    use rquickjs::IntoJs;

    #[test]
    fn 参数借用_返回值_异常() {
        let rt = JsRuntime::new().unwrap();
        rt.register_fast("__f", |ctx, a| {
            if a.len() == 1 && a[0].is_string() {
                return Err(rquickjs::Exception::throw_range(&ctx, "坏参数"));
            }
            // 返回一个参数本身：借来的值 clone 才 dup，返回给 QuickJS 后引用计数要平
            if a.len() == 2 {
                return Ok(a[1].clone());
            }
            let s: f64 = a.iter().filter_map(|v| v.as_number()).sum();
            (s + a.len() as f64 * 1000.0).into_js(&ctx)
        })
        .unwrap();
        assert_eq!(rt.eval("__f(1, 2, 3)").unwrap(), "3006");
        assert_eq!(rt.eval("__f()").unwrap(), "0");
        // 超过栈上数组的参数个数走 Vec
        assert_eq!(rt.eval("__f(1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1)").unwrap(), "20020");
        assert_eq!(rt.eval("var o = {k: 7}; var r = __f(0, o); (r === o) + ':' + r.k").unwrap(), "true:7");
        assert_eq!(rt.eval("try { __f('x'); 'no' } catch (e) { e instanceof RangeError ? e.message : 'bad' }").unwrap(), "坏参数");
        // 大量调用、对象参数：引用计数不平的话 GC / 运行时销毁时 QuickJS 会断言或泄漏
        assert_eq!(rt.eval("var t = 0; for (var i = 0; i < 20000; i++) { var x = {i: i}; if (__f(x, x) === x) t++; } t").unwrap(), "20000");
        rt.collect_garbage();
    }

    #[test]
    fn panic_变成_internal_error() {
        let rt = JsRuntime::new().unwrap();
        rt.register_fast("__p", |_ctx, _a| panic!("驱动缺函数")).unwrap();
        let r = rt.eval("try { __p(1); 'no' } catch (e) { (e instanceof InternalError) + ':' + e.message }").unwrap();
        assert!(r.starts_with("true:") && r.contains("驱动缺函数"), "{r}");
    }
}
