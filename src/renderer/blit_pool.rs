//! 变换贴回（`compose::blit_transformed`）的常驻帮手线程。
//!
//! 为什么不每次 `std::thread::scope` 现起：静心播放页每帧两次大块贴回，一加 PLY110 上
//! 起线程 + 退出（`clone` / `munmap` / 栈回收）占引擎线程约 3%，新线程还常被排到小核上。
//! 这里起一次、一直留着，派活只是一次加锁 + 唤醒。
//!
//! 用法与 `scope` 相同：[`run`] 让调用线程与帮手们一起执行同一个闭包，**返回前**等所有
//! 已经拿到这个闭包的帮手都执行完 —— 所以闭包可以借用调用方栈上的东西。闭包自己负责分活
//! （贴回那边是「谁空了谁领下一条行带」），帮手醒得晚、活已被领光也没关系：
//! 调用线程做完自己那份就收回闭包，没来得及拿到的帮手直接跳过这一次，不用等它们醒。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};

/// 派出去的活：借用调用方的闭包，生命周期由 [`run`] 的等待保证
#[derive(Clone, Copy)]
struct Job(*const (dyn Fn() + Sync));
// SAFETY：指针只在 run() 返回前被解引用（见 run 里的等待），闭包本身是 Sync 的
unsafe impl Send for Job {}

struct State {
    /// 当前这次派活；None = 没有活或已收回
    job: Option<Job>,
    /// 每派一次加一，帮手据此知道「这是新的一次」
    seq: u64,
    /// 正在执行闭包的帮手数
    running: usize,
}

struct Pool {
    state: Mutex<State>,
    /// 有新活
    wake: Condvar,
    /// 有帮手执行完
    done: Condvar,
    /// 真起来了几个帮手
    helpers: AtomicUsize,
}

fn pool(helpers: usize) -> &'static Pool {
    static POOL: OnceLock<&'static Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let p: &'static Pool = Box::leak(Box::new(Pool {
            state: Mutex::new(State { job: None, seq: 0, running: 0 }),
            wake: Condvar::new(),
            done: Condvar::new(),
            helpers: AtomicUsize::new(0),
        }));
        for _ in 0..helpers {
            if std::thread::Builder::new().name("mini-blit".into()).spawn(move || helper(p)).is_ok() {
                p.helpers.fetch_add(1, Ordering::Relaxed);
            }
        }
        p
    })
}

fn helper(p: &'static Pool) {
    boost_self();
    let mut seen = 0u64;
    let mut st = p.state.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        while st.job.is_none() || st.seq == seen {
            st = p.wake.wait(st).unwrap_or_else(|e| e.into_inner());
        }
        seen = st.seq;
        let job = st.job.expect("上面刚判过");
        st.running += 1;
        drop(st);
        // SAFETY：running 计上了，run() 收回闭包后要等 running 归零才返回
        let f = unsafe { &*job.0 };
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        st = p.state.lock().unwrap_or_else(|e| e.into_inner());
        st.running -= 1;
        if st.running == 0 {
            p.done.notify_all();
        }
    }
}

/// 帮手线程起来先做的事：放到与引擎线程同一档的核上。
///
/// Android：引擎线程绑在大核上（`host/cpu_affinity.rs`；ColorOS 上 ADPF 会话不管用），帮手不绑的话
/// 调度器按「利用率低」把它们放在小核上 —— 一加 PLY110 静心播放页实测引擎线程领了 40% 的行带、
/// 三个帮手各 20%，整块贴回要等小核上的那几条。帮手只在贴回时醒着，一直绑着不多耗电。
/// Apple：新线程默认 QoS 低一档，会被排到能效核上，提到 user-interactive
fn boost_self() {
    #[cfg(target_os = "android")]
    crate::host::cpu_affinity::pin_helper_thread();
    #[cfg(target_vendor = "apple")]
    // SAFETY：只改调用线程自己的 QoS
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
    }
}

/// 调用线程与最多 `threads - 1` 个帮手一起执行 `f`，返回时 `f` 已无人在用。
/// `threads` 只决定第一次调用时起几个帮手（之后固定）；≤1 或帮手起不来时就只在调用线程上跑
pub(crate) fn run(threads: usize, f: &(dyn Fn() + Sync)) {
    if threads <= 1 {
        f();
        return;
    }
    let p = pool(threads - 1);
    if p.helpers.load(Ordering::Relaxed) == 0 {
        f();
        return;
    }
    // 平时只有引擎线程在派活；两处同时来（并行跑的单测、桌面多窗）时后来的那个自己跑
    let mut st = p.state.lock().unwrap_or_else(|e| e.into_inner());
    if st.job.is_some() || st.running > 0 {
        drop(st);
        f();
        return;
    }
    // SAFETY：只是抹掉生命周期；下面等 running 归零之后才返回，之后没人再碰这个指针
    let job: *const (dyn Fn() + Sync) = unsafe { std::mem::transmute::<&(dyn Fn() + Sync), &'static (dyn Fn() + Sync)>(f) };
    st.job = Some(Job(job));
    st.seq = st.seq.wrapping_add(1);
    drop(st);
    p.wake.notify_all();
    // 调用线程这份 panic 了也要先收回闭包、等帮手做完，再接着往上抛：
    // 否则帮手还在用的闭包借着的栈已经被展开掉了
    let mine = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let mut st = p.state.lock().unwrap_or_else(|e| e.into_inner());
    st.job = None; // 还没醒的帮手不再拿这一次
    while st.running > 0 {
        st = p.done.wait(st).unwrap_or_else(|e| e.into_inner());
    }
    drop(st);
    if let Err(e) = mine {
        std::panic::resume_unwind(e);
    }
}
