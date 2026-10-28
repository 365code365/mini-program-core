//! 真 Worker（game-doc/02 §9、04 G6.5，GT-W）：`wx.createWorker` 起一条线程跑独立的 QuickJS 运行时（thread.rs）。
//!
//! - 路径：game.json 必须配 `workers`（目录，或 `{path, isSubpackage}`），脚本路径相对包根且在这个目录里；
//! - 同时最多一个（官方限制）。还活着时再 create：先 terminate 旧的再建新的、日志一行（与小程序那份一致；真机行为待取证 GT-W3）；
//! - 消息：结构化克隆（game-worker-codec.js）编码成 JSON 骨架 + ArrayBuffer 字节，经 mpsc 跨线程；
//!   主线程这边在 pump 的第 1 步之后取（frame.rs），onMessage 的参数是消息本身；
//! - terminate：设 stop（QuickJS 中断回调读它，死循环也停）+ 发 Stop，线程自己退出，不在引擎线程上 join；
//! - Worker 里没接住的异常 → `worker.onError({message, stack})` 并打日志；线程 panic / testOnProcessKilled → onProcessKilled，Worker 作废。

mod thread;

use super::GameEngine;
use crate::game::config::GameConfig;
use rquickjs::IntoJs;
use serde_json::json;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

/// 活着的 Worker 线程数（进程内，GT-W4）
pub static LIVE: AtomicUsize = AtomicUsize::new(0);

pub struct Packet {
    pub json: String,
    pub bufs: Vec<Vec<u8>>,
}

pub enum ToWorker {
    Msg(Packet),
    Stop,
    /// testOnProcessKilled：Worker 线程报 onProcessKilled 后退出
    Kill,
}

pub enum FromWorker {
    Msg(Packet),
    /// (级别, 文本)
    Log(String, String),
    Error { message: String, stack: String },
    Killed(String),
    Exited,
}

struct Active {
    to: Sender<ToWorker>,
    from: Receiver<FromWorker>,
    stop: Arc<AtomicBool>,
}

pub struct Workers {
    root: PathBuf,
    dir: Option<String>,
    active: Option<Active>,
    /// 主线程这边收到、还没交给 JS 的消息（__game_worker_take 取）
    inbox: Option<Packet>,
    logs: Vec<String>,
}

pub fn collect_js(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_js(root, &p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("js") {
            if let Ok(rel) = p.strip_prefix(root) {
                out.push((rel.to_string_lossy().replace('\\', "/").trim_end_matches(".js").to_string(), p));
            }
        }
    }
}

impl Workers {
    pub fn new(root: PathBuf, cfg: &GameConfig) -> Self {
        let dir = cfg.workers.as_ref().map(|d| d.trim_start_matches("./").trim_matches('/').to_string());
        Self { root, dir, active: None, inbox: None, logs: Vec::new() }
    }

    fn terminate(&mut self) {
        if let Some(a) = self.active.take() {
            a.stop.store(true, Ordering::SeqCst);
            let _ = a.to.send(ToWorker::Stop);
        }
    }

    fn create(&mut self, path: &str) -> Result<(), String> {
        let Some(dir) = self.dir.clone() else { return Err("createWorker:fail game.json 没有配置 workers".into()) };
        let p = path.trim_start_matches("./").trim_start_matches('/');
        let key = p.trim_end_matches(".js").to_string();
        if !(key.starts_with(&format!("{dir}/")) || dir.is_empty()) {
            return Err(format!("createWorker:fail 脚本 {path} 不在 workers 目录 {dir} 里"));
        }
        if !self.root.join(format!("{key}.js")).is_file() {
            return Err(format!("createWorker:fail 脚本不存在：{path}"));
        }
        if self.active.is_some() {
            self.logs.push("⚠ 同时只能有一个 Worker：先 terminate 旧的再建新的".into());
            self.terminate();
        }
        let (to, rx) = channel();
        let (tx, from) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        thread::spawn(thread::Spec { root: self.root.clone(), dir, entry: key }, rx, tx, stop.clone())
            .map_err(|e| format!("createWorker:fail 起不了线程：{e}"))?;
        self.active = Some(Active { to, from, stop });
        Ok(())
    }

    pub fn alive(&self) -> bool {
        self.active.is_some()
    }
}

impl Drop for Workers {
    /// 实例销毁时 Worker 跟着停（不然线程会一直等在 channel 上）
    fn drop(&mut self) {
        self.terminate();
    }
}

/// JS 入口：`__game_worker(op, path)`（create / terminate，返回错误文本或空串）、`__game_worker_post(json, bufs, transfer)`、
/// `__game_worker_take()`（[json, bufs] 或 null）
pub fn register(rt: &crate::js::JsRuntime, w: Rc<RefCell<Workers>>) -> Result<(), String> {
    let w1 = w.clone();
    rt.register_raw("__game_worker", move |ctx, a| {
        let s = |i: usize| a.0.get(i).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        let mut w = w1.borrow_mut();
        let r = match s(0).as_str() {
            "create" => w.create(&s(1)).err().unwrap_or_default(),
            "terminate" => {
                w.terminate();
                String::new()
            }
            "kill" => {
                if let Some(a) = &w.active {
                    let _ = a.to.send(ToWorker::Kill);
                }
                String::new()
            }
            _ => String::new(),
        };
        r.into_js(&ctx)
    })?;
    let w2 = w.clone();
    rt.register_raw("__game_worker_post", move |ctx, a| {
        let p = thread::packet_from_js(&a);
        if let Some(act) = &w2.borrow().active {
            let _ = act.to.send(ToWorker::Msg(p));
        }
        Ok(rquickjs::Value::new_undefined(ctx.clone()))
    })?;
    rt.register_raw("__game_worker_take", move |ctx, _| match w.borrow_mut().inbox.take() {
        Some(p) => thread::packet_to_js(&ctx, p),
        None => Ok(rquickjs::Value::new_null(ctx.clone())),
    })
}

impl GameEngine {
    /// pump 里调：Worker 发来的消息、日志、错误交给 JS
    pub(super) fn dispatch_workers(&mut self) {
        let mut items = Vec::new();
        {
            let mut w = self.workers.borrow_mut();
            items.extend(std::mem::take(&mut w.logs).into_iter().map(|t| FromWorker::Log("warn".into(), t)));
            if let Some(a) = &w.active {
                while let Ok(m) = a.from.try_recv() {
                    items.push(m);
                }
            }
        }
        for m in items {
            let js = match m {
                FromWorker::Msg(p) => {
                    self.workers.borrow_mut().inbox = Some(p);
                    "__gameWorkerDeliver()".to_string()
                }
                FromWorker::Log(level, text) => {
                    // 日志行原样进宿主日志（T:EV 之类的机器可读行要能被解析），警告 / 错误加个前缀
                    let pre = match level.as_str() { "warn" => "[worker][warn] ", "error" => "[worker][error] ", _ => "" };
                    self.host_log(&format!("{pre}{text}"));
                    continue;
                }
                FromWorker::Error { message, stack } => {
                    self.host_log(&format!("[worker][error] {message}\n{stack}"));
                    format!("__gameWorkerEvent('error', {})", json!({ "message": message, "stack": stack }))
                }
                FromWorker::Killed(why) => {
                    self.host_log(&format!("⚠ Worker 被结束：{why}"));
                    self.workers.borrow_mut().terminate();
                    "__gameWorkerEvent('processKilled', {})".to_string()
                }
                FromWorker::Exited => continue,
            };
            self.guards.watchdog.arm();
            let r = self.app.eval(&format!("typeof __gameWorkerEvent === 'function' && {js}"));
            self.guards.watchdog.disarm();
            if let Err(e) = r {
                self.report_error(super::error_kind(&e, "worker"), &e);
            }
        }
        self.after_js();
    }

    /// 活着的 Worker 线程数（GT-W4）
    pub fn live_worker_threads() -> usize {
        LIVE.load(Ordering::SeqCst)
    }

    pub(super) fn worker_alive(&self) -> bool {
        self.workers.borrow().alive()
    }
}
