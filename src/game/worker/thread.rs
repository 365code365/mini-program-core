//! Worker 线程（game-doc/02 §9）：自己的 QuickJS 运行时（在本线程上建、注册原生函数 —— 闭包是 `Rc`，不能跨线程，06 §29），
//! 自己的事件循环：有消息就派发，有到点的定时器就跑，都没有就阻塞在 channel 上等（带下一个定时器的超时），不轮询。
//! 结束：主线程设 stop 标志（QuickJS 的中断回调读它，死循环也能停下）并发 Stop；本线程退出时 LIVE 减一（GT-W4 数线程用）。

use super::{FromWorker, Packet, ToWorker, LIVE};
use crate::js::JsRuntime;
use rquickjs::{Array, ArrayBuffer, IntoJs, Value as JsValue};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

const RUNTIME_JS: &str = include_str!("runtime.js");
/// Worker 运行时的 JS 堆上限（主运行时默认 1024MB；Worker 只做计算，给一半）
const WORKER_MEMORY_MB: usize = 512;

pub struct Spec {
    pub root: PathBuf,
    /// game.json 的 workers 目录（相对包根，不带首尾斜杠）
    pub dir: String,
    /// 入口模块键（相对包根、去 .js）
    pub entry: String,
}

struct LiveGuard;
impl Drop for LiveGuard {
    fn drop(&mut self) {
        LIVE.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn spawn(spec: Spec, rx: Receiver<ToWorker>, tx: Sender<FromWorker>, stop: Arc<AtomicBool>) -> std::io::Result<std::thread::JoinHandle<()>> {
    LIVE.fetch_add(1, Ordering::SeqCst);
    std::thread::Builder::new().name("mini-game-worker".into()).spawn(move || {
        let _live = LiveGuard;
        let tx2 = tx.clone();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(spec, rx, tx, stop)));
        if r.is_err() {
            let _ = tx2.send(FromWorker::Killed("Worker 线程内部错误（panic）".into()));
        }
        let _ = tx2.send(FromWorker::Exited);
    })
}

/// 把 JS 数组里的 ArrayBuffer 拷成字节；`transfer` 里的下标拷完 detach
fn take_bufs(v: Option<&JsValue>, transfer: Option<&JsValue>) -> Vec<Vec<u8>> {
    let tr: Vec<usize> = transfer
        .and_then(|t| t.as_array())
        .map(|a| a.iter::<f64>().flatten().map(|x| x as usize).collect())
        .unwrap_or_default();
    let Some(arr) = v.and_then(|x| x.as_array()) else { return Vec::new() };
    let mut out = Vec::new();
    for (i, item) in arr.iter::<JsValue>().flatten().enumerate() {
        out.push(crate::js::binary::bytes_of(&item).unwrap_or_default());
        if tr.contains(&i) {
            if let Some(mut ab) = ArrayBuffer::from_value(item) {
                ab.detach();
            }
        }
    }
    out
}

/// 消息交给 JS：[json, [ArrayBuffer…]]
pub fn packet_to_js<'js>(ctx: &rquickjs::Ctx<'js>, p: Packet) -> rquickjs::Result<JsValue<'js>> {
    let arr = Array::new(ctx.clone())?;
    arr.set(0, p.json)?;
    let bufs = Array::new(ctx.clone())?;
    for (i, b) in p.bufs.into_iter().enumerate() {
        bufs.set(i, crate::js::binary::array_buffer(ctx, b)?)?;
    }
    arr.set(1, bufs)?;
    Ok(arr.into_value())
}

/// `__xxx_post(json, bufs, transfer)` 的参数 → 一条消息
pub fn packet_from_js(a: &rquickjs::function::Rest<JsValue<'_>>) -> Packet {
    let json = a.0.first().and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_else(|| "null".into());
    Packet { json, bufs: take_bufs(a.0.get(1), a.0.get(2)) }
}

fn run(spec: Spec, rx: Receiver<ToWorker>, tx: Sender<FromWorker>, stop: Arc<AtomicBool>) {
    let rt = match JsRuntime::new() {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(FromWorker::Killed(format!("Worker 运行时建不起来：{e}")));
            return;
        }
    };
    rt.raw_runtime().set_memory_limit(WORKER_MEMORY_MB * 1024 * 1024);
    let s = stop.clone();
    rt.raw_runtime().set_interrupt_handler(Some(Box::new(move || s.load(Ordering::Relaxed))));
    let t0 = Instant::now();
    let inbox: Rc<RefCell<Option<Packet>>> = Rc::new(RefCell::new(None));
    let killed = Rc::new(RefCell::new(false));
    let reg = || -> Result<(), String> {
        rt.register_raw("__w_now", move |ctx, _| (t0.elapsed().as_secs_f64() * 1000.0).into_js(&ctx))?;
        let t = tx.clone();
        rt.register_raw("__w_print", move |ctx, a| {
            let s = |i: usize| a.0.get(i).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
            let _ = t.send(FromWorker::Log(s(0), s(1)));
            JsValue::new_undefined(ctx.clone()).into_js(&ctx)
        })?;
        let t = tx.clone();
        rt.register_raw("__w_error", move |ctx, a| {
            let s = |i: usize| a.0.get(i).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
            let _ = t.send(FromWorker::Error { message: s(0), stack: s(1) });
            JsValue::new_undefined(ctx.clone()).into_js(&ctx)
        })?;
        let t = tx.clone();
        rt.register_raw("__w_post", move |ctx, a| {
            let _ = t.send(FromWorker::Msg(packet_from_js(&a)));
            JsValue::new_undefined(ctx.clone()).into_js(&ctx)
        })?;
        let ib = inbox.clone();
        rt.register_raw("__w_take", move |ctx, _| match ib.borrow_mut().take() {
            Some(p) => packet_to_js(&ctx, p),
            None => Ok(JsValue::new_null(ctx.clone())),
        })?;
        let k = killed.clone();
        rt.register_raw("__w_kill", move |ctx, _| {
            *k.borrow_mut() = true;
            JsValue::new_undefined(ctx.clone()).into_js(&ctx)
        })?;
        // WXWebAssembly 在 Worker 里也能用（基础库 2.15 起；.wasm 放在 Worker 目录外，路径相对包根，G10.4）
        #[cfg(feature = "wasm")]
        crate::game::wasm::register(&rt, Rc::new(RefCell::new(crate::game::wasm::Rt::new(spec.root.clone()))))?;
        Ok(())
    };
    if let Err(e) = reg() {
        let _ = tx.send(FromWorker::Killed(format!("Worker 原生函数注册失败：{e}")));
        return;
    }
    let report = |e: String| {
        let (message, stack) = crate::game::split_exception(&e);
        let _ = tx.send(FromWorker::Error { message, stack });
    };
    let wasm_js = if cfg!(feature = "wasm") { Some(crate::js::game_prelude_source("game-wasm")) } else { None };
    for src in [crate::js::prelude_source("module"), Some(crate::js::game_prelude_source("game-worker-codec")), Some(RUNTIME_JS), wasm_js].into_iter().flatten() {
        if let Err(e) = rt.eval(src) {
            let _ = tx.send(FromWorker::Killed(format!("Worker 环境初始化失败：{e}")));
            return;
        }
    }
    // Worker 目录里的 .js 全部注册成模块（键相对包根，与主包同一套 require 解析）
    let mut files = Vec::new();
    super::collect_js(&spec.root, &spec.root.join(&spec.dir), &mut files);
    files.sort();
    for (key, path) in files {
        let Ok(src) = std::fs::read_to_string(&path) else { continue };
        if let Err(e) = rt.eval(&format!("__defineModule({key:?}, function(module, exports, require) {{{src}\n}});")) {
            report(e);
        }
    }
    if let Err(e) = rt.eval(&format!("__require('', {:?})", spec.entry)) {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        report(e);
    }
    rt.pump_jobs();
    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if *killed.borrow() {
            let _ = tx.send(FromWorker::Killed("testOnProcessKilled".into()));
            return;
        }
        let due = rt.call_num("__wNextDue", &[]).unwrap_or(-1.0);
        let got = if due < 0.0 {
            rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            rx.recv_timeout(Duration::from_secs_f64(due / 1000.0))
        };
        let r = match got {
            Ok(ToWorker::Msg(p)) => {
                *inbox.borrow_mut() = Some(p);
                rt.call_num("__wDeliver", &[]).map(|_| ())
            }
            Ok(ToWorker::Kill) => {
                let _ = tx.send(FromWorker::Killed("testOnProcessKilled".into()));
                return;
            }
            Ok(ToWorker::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => rt.call_num("__wRunTimers", &[]).map(|_| ()),
        };
        if let Err(e) = r {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            report(e);
        }
        rt.pump_jobs();
    }
}
