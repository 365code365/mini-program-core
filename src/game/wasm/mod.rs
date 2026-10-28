//! WXWebAssembly（game-doc/04 G10，D-11）：wasmi 解释执行，全平台一份（iOS 不许 App 自己 JIT）。
//!
//! JS 外壳在 game-wasm.js：`WXWebAssembly.instantiate(包内路径, imports)` 与 Memory / Table / Global / Instance。
//! 这里按编号管 wasm 对象：模块、函数、内存、表、全局。Store 一个实例一份（同一个游戏里的模块共用，
//! 导出的内存 / 函数才能互相导入）。
//!
//! 内存的 `buffer` 是零拷贝的 ArrayBuffer（指向 wasm 线性内存）。线性内存增长会搬家：每次 wasm 跑完、
//! 每次进 JS 导入函数之前、每次取 `buffer` 时比一下地址与长度，变了就 detach 旧的（旧视图长度变 0，
//! 与标准里 grow 之后旧 buffer detached 一致），JS 永远碰不到已经释放的那块内存。

mod conv;
mod host;
mod natives;
mod objects;

use rquickjs::{ArrayBuffer, Ctx, Object};
use std::path::{Path, PathBuf};

pub use natives::register;

pub(crate) struct MemSlot {
    mem: wasmi::Memory,
    /// 交给 JS 的那块 buffer 当时的地址、长度（buffer 对象本身放在 JS 的 `__wasmBufs[编号]`：
    /// 不用 Persistent —— 运行时销毁时还挂着的 Persistent 会让 QuickJS 的 JS_FreeRuntime 断言失败直接 abort）
    buf: Option<(usize, usize)>,
}

pub struct Rt {
    engine: wasmi::Engine,
    /// 执行时被取走（嵌套调用用宿主回调的 Caller，见 host.rs）
    store: Option<wasmi::Store<()>>,
    root: PathBuf,
    modules: Vec<wasmi::Module>,
    funcs: Vec<wasmi::Func>,
    mems: Vec<MemSlot>,
    tables: Vec<wasmi::Table>,
    globals: Vec<wasmi::Global>,
    /// JS 导入函数的签名（编号 = JS 侧 __wasmHosts 的下标）
    hosts: Vec<wasmi::FuncType>,
}

impl Rt {
    pub fn new(root: PathBuf) -> Self {
        // SIMD 随 feature `simd` 默认开着（微信 8.0.25 起支持）
        let cfg = wasmi::Config::default();
        let engine = wasmi::Engine::new(&cfg);
        let store = wasmi::Store::new(&engine, ());
        Self { engine, store: Some(store), root, modules: Vec::new(), funcs: Vec::new(), mems: Vec::new(), tables: Vec::new(), globals: Vec::new(), hosts: Vec::new() }
    }

    /// 包内路径 → 字节（`.wasm.br` 先解 brotli）。只认包内：绝对路径、`..` 一律拒绝
    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let rel = path.trim_start_matches("./").trim_start_matches('/');
        let p = Path::new(rel);
        if p.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            return Err(format!("只接受代码包内的路径：{path}"));
        }
        if !(rel.ends_with(".wasm") || rel.ends_with(".wasm.br")) {
            return Err(format!("只接受 .wasm / .wasm.br：{path}"));
        }
        let raw = std::fs::read(self.root.join(p)).map_err(|e| format!("读不出 {path}：{e}"))?;
        if rel.ends_with(".br") {
            let mut out = Vec::new();
            brotli_decompressor::BrotliDecompress(&mut raw.as_slice(), &mut out).map_err(|e| format!("{path} brotli 解压失败：{e}"))?;
            return Ok(out);
        }
        Ok(raw)
    }

    fn func_id(&mut self, f: &wasmi::Func) -> u32 {
        self.funcs.push(*f);
        (self.funcs.len() - 1) as u32
    }

    fn mem_id(&mut self, m: wasmi::Memory) -> u32 {
        self.mems.push(MemSlot { mem: m, buf: None });
        (self.mems.len() - 1) as u32
    }
}

/// 在合适的 store 上下文里跑 `f`：宿主回调进行中用它的 Caller，否则用实例的 Store（跑完放回）
pub(crate) fn with_store<R>(rt: &std::cell::RefCell<Rt>, f: impl FnOnce(wasmi::StoreContextMut<'_, ()>) -> R) -> Result<R, String> {
    use wasmi::AsContextMut;
    if let Some(p) = host::caller() {
        // SAFETY: host.rs 只在宿主回调的作用域里给出这个指针
        let c = unsafe { &mut *p };
        return Ok(f(c.as_context_mut()));
    }
    let mut store = rt.borrow_mut().store.take().ok_or("wasm store 正在被占用")?;
    let r = f(store.as_context_mut());
    rt.borrow_mut().store = Some(store);
    Ok(r)
}

/// 内存搬家 / 长了：detach 交给 JS 的旧 buffer（下次取 buffer 时新建）
pub(crate) fn refresh_buffers(rt: &std::cell::RefCell<Rt>, ctx: &Ctx<'_>) {
    let mems: Vec<(usize, wasmi::Memory)> = rt.borrow().mems.iter().enumerate().filter(|(_, m)| m.buf.is_some()).map(|(i, m)| (i, m.mem)).collect();
    if mems.is_empty() {
        return;
    }
    let now: Vec<(usize, usize, usize)> = with_store(rt, |s| mems.iter().map(|(i, m)| (*i, m.data_ptr(&s) as usize, m.data_size(&s))).collect()).unwrap_or_default();
    let mut w = rt.borrow_mut();
    for (i, ptr, len) in now {
        let slot = &mut w.mems[i];
        if slot.buf.is_some_and(|b| b != (ptr, len)) {
            slot.buf = None;
            detach_js(ctx, i);
        }
    }
}

/// detach JS 那边登记的那块 buffer（旧视图从此长度 0，碰不到已经搬走的内存）
pub(crate) fn detach_js(ctx: &Ctx<'_>, id: usize) {
    let Ok(bufs) = ctx.globals().get::<_, Object>("__wasmBufs") else { return };
    if let Ok(v) = bufs.get::<_, rquickjs::Value>(id as u32) {
        if let Some(mut ab) = ArrayBuffer::from_value(v) {
            ab.detach();
        }
    }
    let _ = bufs.set(id as u32, rquickjs::Value::new_undefined(ctx.clone()));
}

/// 指向 wasm 线性内存、不拥有内存的 ArrayBuffer（释放归 wasmi 管）。
///
/// 不用 rquickjs 的 `ArrayBuffer::from_source`：它的释放回调在 detach 时跑一次、GC 回收时又跑一次
/// （QuickJS 的 finalizer 不看 detached 标志），两次 `Box::from_raw` 就是重复释放、进程 abort（实测）。
/// 这里不给释放回调，detach 与回收都不碰这块内存。
pub(crate) fn linear_buffer<'js>(ctx: &Ctx<'js>, ptr: *mut u8, len: usize) -> rquickjs::Result<rquickjs::Value<'js>> {
    // SAFETY: ptr/len 是当时线性内存的地址与长度；搬家之前 refresh_buffers 会把它 detach（之后 QuickJS 不再读写）
    unsafe {
        let v = rquickjs::qjs::JS_NewArrayBuffer(ctx.as_raw().as_ptr(), ptr, len as _, None, std::ptr::null_mut(), false);
        Ok(rquickjs::Value::from_raw(ctx.clone(), v))
    }
}
