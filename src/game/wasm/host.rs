//! wasm ↔ JS 的重入（G10.3）：wasm 调 JS 导入函数、JS 在导入函数里再调 wasm 导出（Emscripten 的 invoke_* 天天这么做）。
//!
//! wasmi 的宿主函数闭包必须是 `'static + Send + Sync`，拿不到当前这次原生调用的 QuickJS `Ctx`；
//! 重入时 Store 正被外层调用借着，只能用宿主函数收到的 `Caller` 当 store 上下文。两样都放线程局部，
//! 只在一次原生调用（或一次宿主回调）的作用域里有效，出作用域就复位（`Scope` 的 Drop）。

use std::cell::Cell;

thread_local! {
    /// 当前这次原生调用的「调 JS 导入函数」回调（擦掉了生命周期，作用域内有效）
    static HOST: Cell<Option<*const (dyn Fn(u32, &[wasmi::Val], &mut [wasmi::Val]) -> Result<(), wasmi::Error> + 'static)>> = const { Cell::new(None) };
    /// 宿主回调进行中：wasm 那边的 store 上下文（JS 在回调里再调 wasm 时用它）
    static CALLER: Cell<Option<*mut wasmi::Caller<'static, ()>>> = const { Cell::new(None) };
}

/// 作用域守卫：进入时设、离开时恢复成进入前的值（支持嵌套）
pub struct Scope {
    host: Option<Option<*const (dyn Fn(u32, &[wasmi::Val], &mut [wasmi::Val]) -> Result<(), wasmi::Error> + 'static)>>,
    caller: Option<Option<*mut wasmi::Caller<'static, ()>>>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(h) = self.host {
            HOST.with(|c| c.set(h));
        }
        if let Some(p) = self.caller {
            CALLER.with(|c| c.set(p));
        }
    }
}

/// 在 `f` 的作用域里登记「调 JS 导入函数」的回调
pub fn with_host<R>(cb: &dyn Fn(u32, &[wasmi::Val], &mut [wasmi::Val]) -> Result<(), wasmi::Error>, f: impl FnOnce() -> R) -> R {
    // SAFETY: 只擦生命周期；Scope 在 f 返回（或 panic 展开）时恢复旧值，cb 在这之前一直活着
    let p: *const (dyn Fn(u32, &[wasmi::Val], &mut [wasmi::Val]) -> Result<(), wasmi::Error> + 'static) = unsafe { std::mem::transmute(cb as *const _) };
    let old = HOST.with(|c| c.replace(Some(p)));
    let _s = Scope { host: Some(old), caller: None };
    f()
}

/// 宿主函数的入口：wasm 调到编号 `hid` 的 JS 导入函数
pub fn call_host(caller: &mut wasmi::Caller<'_, ()>, hid: u32, params: &[wasmi::Val], results: &mut [wasmi::Val]) -> Result<(), wasmi::Error> {
    let Some(cb) = HOST.with(|c| c.get()) else {
        return Err(wasmi::Error::new("wasm 在 JS 调用之外执行到了导入函数"));
    };
    // SAFETY: caller 在本函数返回前有效；Scope 返回时复位
    let p: *mut wasmi::Caller<'static, ()> = unsafe { std::mem::transmute(caller as *mut wasmi::Caller<'_, ()>) };
    let old = CALLER.with(|c| c.replace(Some(p)));
    let _s = Scope { host: None, caller: Some(old) };
    // SAFETY: 见 with_host
    unsafe { (*cb)(hid, params, results) }
}

/// 宿主回调进行中的 store 上下文（没有 = 不在回调里，用实例自己的 Store）
pub fn caller() -> Option<*mut wasmi::Caller<'static, ()>> {
    CALLER.with(|c| c.get())
}
