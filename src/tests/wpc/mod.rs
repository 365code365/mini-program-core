//! 组件对齐 WP-C（`.agents/tasks/component-gap-2026-11-03/plan.md`）：逻辑层、内置组件、事件链。
//! 每个用例现场写一个小程序（与 `tests/wx_compat.rs` 同一套写法），用 `MiniEngine` 跑起来按可判定的判据断言。
//! 放在库内单测里而不是新的 `tests/*.rs`：每个集成测试文件是一个 75MB 的独立二进制（steering 30）。
use crate::compositor::HostEnv;
use crate::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;
use std::time::Duration;

mod events;
mod gesture;
mod open_type;
mod pages;
mod park;

/// 三页（p 是 tab 首页、q 普通页、t tab 页）的小程序；`p_wxml` / `p_js` 是首页，`extra` 是额外文件（可覆盖 q 页）
pub(super) fn fixture(name: &str, p_wxml: &str, p_js: &str, extra: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join("mini-wpc-tests").join(name);
    std::fs::remove_dir_all(&root).ok();
    let app_json = r#"{"pages":["pages/p/p","pages/q/q","pages/t/t"],"tabBar":{"list":[{"pagePath":"pages/p/p","text":"P"},{"pagePath":"pages/t/t","text":"T"}]}}"#;
    let base: Vec<(&str, &str)> = vec![
        ("app.json", app_json),
        ("app.js", "App({ log: [] });"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", p_wxml),
        ("pages/p/p.wxss", ""),
        ("pages/p/p.js", p_js),
        ("pages/p/p.json", "{}"),
        ("pages/q/q.wxml", "<view>q</view>"),
        ("pages/q/q.wxss", ""),
        ("pages/q/q.js", "Page({});"),
        ("pages/q/q.json", "{}"),
        ("pages/t/t.wxml", "<view>t</view>"),
        ("pages/t/t.wxss", ""),
        ("pages/t/t.js", "Page({});"),
        ("pages/t/t.json", "{}"),
    ];
    for (rel, text) in base.iter().chain(extra.iter()) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

/// 小程序根目录与数据目录是进程级的（同 `tests/wx_compat.rs` 的 SERIAL）：这一组用例一个一个跑
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 引擎 + 锁（字段顺序即析构顺序：先关引擎再放锁）。这组用例的包根是自己现场写的临时目录，
/// 换了进程级的包根，所以拿包根的**写锁**（`data_dir::ROOT_TEST_LOCK`：拿读锁的用例都用 sample 里的包根，
/// 与它们并行跑会让对方 launch 时「页面不存在」）
pub(super) struct Wpc {
    e: MiniEngine,
    _root: std::sync::RwLockWriteGuard<'static, ()>,
    _s: std::sync::MutexGuard<'static, ()>,
}
impl std::ops::Deref for Wpc {
    type Target = MiniEngine;
    fn deref(&self) -> &MiniEngine {
        &self.e
    }
}
impl std::ops::DerefMut for Wpc {
    fn deref_mut(&mut self) -> &mut MiniEngine {
        &mut self.e
    }
}

pub(super) fn engine(root: &std::path::Path, opts: EngineOptions) -> Wpc {
    let s = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let g = crate::sys::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 1.0, HostEnv::default(), opts)
        .expect("创建引擎");
    // 定时器与动画跟着 pump 的时刻走（不睡墙上时钟），见 `MiniEngine::use_virtual_clock`
    e.use_virtual_clock();
    e.launch(None).expect("打开首页");
    e.pump(1);
    Wpc { e, _root: g, _s: s }
}

pub(super) fn js(e: &MiniEngine, code: &str) -> String {
    e.eval(code).unwrap_or_default()
}

thread_local! {
    /// 这个用例线程上 pump 给到的时刻（只增不减：虚拟时钟下时间不能往回走）
    static PUMP_MS: std::cell::Cell<u64> = const { std::cell::Cell::new(100) };
}

/// 按 16ms 一帧 pump，直到 `cond` 成立或引擎时钟过了 `ms`。虚拟时钟下不睡：定时器只认 pump 的时刻。
/// 网络、解码这类后台线程的活是真的：有活在飞时每帧让出 2ms 给它们（「在飞」是进程级的，别的用例留下的也算，
/// 所以只让、不延长循环 —— 从前按它延长，一个连接测试留在队列里的 close 事件让后面每次 settle 都多等 5 秒）
pub(super) fn pump_until(e: &mut MiniEngine, ms: u64, mut cond: impl FnMut(&MiniEngine) -> bool) -> bool {
    let mut passed = 0u64;
    while passed <= ms {
        let t = PUMP_MS.with(|c| {
            let t = c.get() + 16;
            c.set(t);
            t
        });
        e.pump(t);
        if cond(e) {
            return true;
        }
        if e.net_pending() {
            std::thread::sleep(Duration::from_millis(2));
        }
        passed += 16;
    }
    false
}

pub(super) fn log(e: &MiniEngine) -> Vec<String> {
    serde_json::from_str(&js(e, "JSON.stringify(getApp().log)")).unwrap_or_default()
}

pub(super) fn clear_log(e: &MiniEngine) {
    js(e, "getApp().log.length = 0");
}

/// 一次拖动：按下 → `steps` 步移动 → 抬起，每步之间出一帧
pub(super) fn drag(e: &mut MiniEngine, from: (f32, f32), to: (f32, f32), steps: u32) {
    e.pointer_down(from.0, from.1);
    e.pump(1000);
    for i in 1..=steps {
        let k = i as f32 / steps as f32;
        e.pointer_move(from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k);
        e.pump(1000 + i as u64 * 16);
        std::thread::sleep(Duration::from_millis(4));
    }
    e.pointer_up(to.0, to.1);
    e.pump(1000 + steps as u64 * 16 + 16);
}

/// 宿主事件（`poll_event`）全取出来
pub(super) fn host_events(e: &mut MiniEngine) -> Vec<serde_json::Value> {
    std::iter::from_fn(|| e.poll_event()).filter_map(|s| serde_json::from_str(&s).ok()).collect()
}

/// (x, y) 处的 RGB（逻辑像素，dpr 1）
pub(super) fn rgb_at(e: &mut MiniEngine, x: u32, y: u32) -> [u8; 3] {
    let (w, _) = e.pixel_size();
    let px = e.rgba();
    let i = ((y * w + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2]]
}
