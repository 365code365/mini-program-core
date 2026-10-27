//! 运行时守护（game-doc/02 §3.3、04 G1.7）：看门狗、内存上限、未处理的 Promise 拒绝。
//!
//! 目前只给小游戏用（`GameEngine` 在建实例时装）。小程序不装：给它加上限是行为变化，要单独征求同意。
//!
//! - **看门狗**：QuickJS 执行中会周期性地问一次 interrupt handler；我们在每次「进入 JS」前设一个截止时刻，
//!   超时就让它中断，抛 `InternalError: interrupted`。不自动杀进程：错误交给 `wx.onError` 与宿主，下一帧照常。
//! - **内存上限**：`set_memory_limit`，超了 JS 侧抛 out of memory，进程不崩。
//! - **未处理拒绝**：QuickJS 在「拒绝时没有处理者」与「之后又挂上了处理者」两个时刻各回调一次；
//!   我们先记下来，到本轮微任务跑完再看哪些仍然没人处理，再派发给 `wx.onUnhandledRejection`
//!   （与浏览器的「微任务检查点之后才算未处理」一致：`p.catch()` 晚一行挂上的不算）。

use rquickjs::{Ctx, Persistent, Value};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 截止时刻：相对 `origin` 的微秒数，0 = 不限
#[derive(Clone)]
pub struct Watchdog {
    origin: Instant,
    deadline_us: Arc<AtomicU64>,
    budget_us: u64,
}

impl Watchdog {
    pub fn new(budget_ms: u64) -> Self {
        Self { origin: Instant::now(), deadline_us: Arc::new(AtomicU64::new(0)), budget_us: budget_ms.max(1) * 1000 }
    }
    /// 进入一段 JS 之前调：从此刻起给它一个预算
    /// 单次进 JS 的上限（毫秒）
    pub fn budget_ms(&self) -> u64 {
        self.budget_us / 1000
    }

    pub fn arm(&self) {
        let now = self.origin.elapsed().as_micros() as u64;
        self.deadline_us.store(now + self.budget_us, Ordering::Relaxed);
    }
    /// 出来之后调（不调也行：下一次 arm 会覆盖）
    pub fn disarm(&self) {
        self.deadline_us.store(0, Ordering::Relaxed);
    }
    fn expired(&self) -> bool {
        let d = self.deadline_us.load(Ordering::Relaxed);
        d != 0 && self.origin.elapsed().as_micros() as u64 > d
    }
}

/// 还没派发的拒绝：(promise, reason)
type Pending = Rc<RefCell<Vec<(Persistent<Value<'static>>, Persistent<Value<'static>>)>>>;

pub struct Guards {
    pub watchdog: Watchdog,
    pending: Pending,
}

impl super::JsRuntime {
    /// 装上看门狗、内存上限、拒绝跟踪。`memory_limit` 为 0 表示不限
    pub fn install_guards(&self, watchdog_ms: u64, memory_limit: usize) -> Guards {
        let watchdog = Watchdog::new(watchdog_ms);
        let w = watchdog.clone();
        self.raw_runtime().set_interrupt_handler(Some(Box::new(move || w.expired())));
        if memory_limit > 0 {
            self.raw_runtime().set_memory_limit(memory_limit);
        }
        let pending: Pending = Rc::new(RefCell::new(Vec::new()));
        let p = pending.clone();
        self.raw_runtime().set_host_promise_rejection_tracker(Some(Box::new(
            move |ctx: Ctx<'_>, promise: Value<'_>, reason: Value<'_>, is_handled: bool| {
                let mut list = p.borrow_mut();
                if is_handled {
                    // 之后又挂上了处理者：从待派发里拿掉（按对象身份比）
                    list.retain(|(pp, _)| pp.clone().restore(&ctx).map(|v: Value| v != promise).unwrap_or(true));
                } else {
                    list.push((Persistent::save(&ctx, promise), Persistent::save(&ctx, reason)));
                }
            },
        )));
        Guards { watchdog, pending }
    }

    /// 本轮微任务跑完之后调：把仍然没人处理的拒绝交给 `__gameDispatchRejection(reason, promise)`。
    /// 返回派发了几个
    pub fn flush_rejections(&self, guards: &Guards) -> usize {
        let items: Vec<_> = guards.pending.borrow_mut().drain(..).collect();
        if items.is_empty() {
            return 0;
        }
        let n = items.len();
        self.raw_context().with(|ctx| {
            let f: Option<rquickjs::Function> = ctx.globals().get("__gameDispatchRejection").ok();
            for (promise, reason) in items {
                let (Ok(promise), Ok(reason)) = (promise.restore(&ctx), reason.restore(&ctx)) else { continue };
                // 没有监听者时也打一行：静默吞掉的拒绝是最难查的那类问题
                let text = describe(&reason);
                let _ = ctx.globals().get::<_, rquickjs::Function>("__native_print").map(|p| p.call::<_, ()>((format!("[WARN] 未处理的 Promise 拒绝：{text}"),)));
                if let Some(f) = &f {
                    let _ = f.call::<_, Value>((reason, promise));
                }
            }
        });
        self.pump_jobs();
        n
    }
}

fn describe(v: &Value) -> String {
    if let Some(s) = v.as_string() {
        return s.to_string().unwrap_or_default();
    }
    if let Some(o) = v.as_object() {
        if let Ok(m) = o.get::<_, String>("message") {
            return m;
        }
    }
    format!("{v:?}")
}
