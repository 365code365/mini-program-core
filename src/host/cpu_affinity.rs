//! Android：把引擎线程绑到大核（去掉最慢的那一簇）。`MINI_CPU_PIN=0`（`debug.mini.cpu_pin 0`）关掉。
//!
//! 为什么要：引擎线程每个 vsync 醒一次、干几毫秒就睡，调度器按「利用率低」把它放在小核上。
//! 一加 PLY110（SM7635：0~3 号 A520 1.8GHz，4~6 号 A720 2.4GHz，7 号 A720 2.7GHz）滑 WXSS 页时
//! 抽样 400 次有 92% 在 0~3 号小核 —— 桌面 0.3ms 的补条带在那上面要 5~11ms，复杂页面一滑就掉帧。
//!
//! 先试过 ADPF（`PerformanceHintManager`，宿主那边的 FrameBoost.kt）：会话建成了
//! （`dumpsys performance_hint` 里有，目标 5.5ms），但 ColorOS 上线程照样 90% 在小核。
//! 所以再加这一刀：游戏引擎的渲染线程普遍这么做。代价只在「醒着」的时候（大核更耗电），
//! 空闲时线程在睡，绑在哪都不耗。
//!
//! 只排除**最慢的一簇**（按 `cpuinfo_max_freq`）：所有核一样快（模拟器、同构芯片）时什么都不做。
//! 只绑调用线程（`sched_setaffinity(0, …)` 的 0 在 Linux 上就是当前线程），不碰宿主的其它线程。
//!
//! ## 只在「要连续出帧」的时候绑（省电）
//! 绑核跟着两件事走：人在操作（手指按着、页面惯性 / 回弹、转场、侧滑），或者引擎在连续出帧
//! （上一帧离现在不到 [`CONTINUOUS_MS`]：CSS 动画、swiper 自动翻页、视频）。两样都停了
//! [`IDLE_MS`] 就放开（亲和性还原成全部核，调度器自己挑）：每秒一次的倒计时 setData、
//! 网络回包这类零星的活落到小核上。建引擎时先绑着（建树、排版、开 GPU 都重）。
//!
//! 为什么连续动画也要绑：试过只按交互绑 —— 综合场景页不碰屏幕时线程回到小核，页面上那个
//! 129×128 的转圈一帧要 13~14ms（画 8 + wgpu 编码提交 5），3 秒里 41 帧超 8ms、屏上重复 5.3%；
//! 绑着时 3 帧、0.8%。手机上「动画一卡一卡」比多耗那点电更不能接受。

/// 停止交互这么久之后放开大核
pub const IDLE_MS: u64 = 800;
/// 两帧隔得比这近就算「在连续出帧」（120Hz 两个周期多一点 ~ 60Hz 三个周期）
pub const CONTINUOUS_MS: u64 = 50;

/// 引擎线程上的绑核状态（只在引擎线程读写）
#[derive(Default)]
struct Gate {
    /// 大核掩码与全部核的掩码；None = 没有大小核之分 / 关掉了，什么都不做
    masks: Option<(u64, u64)>,
    pinned: bool,
    last_active: u64,
    /// 上一次出新画面的时刻（[`note_produced`]）
    last_produced: u64,
}

thread_local! {
    static GATE: std::cell::RefCell<Gate> = std::cell::RefCell::new(Gate::default());
}

/// 引擎每次 pump 报一次：`interacting` = 手指按着 / 惯性 / 转场 / 侧滑。交互开始时绑大核，
/// 停了 [`IDLE_MS`] 放开。没调过 [`pin_current_thread`]（桌面、iOS、关掉了）时什么都不做
pub fn note_frame(now_ms: u64, interacting: bool) {
    GATE.with(|g| {
        let mut g = g.borrow_mut();
        let Some((fast, all)) = g.masks else { return };
        if g.last_active == 0 {
            // 建引擎时绑上的：从第一次 pump 起算空闲（首页建树、排版那几帧还在大核上）
            g.last_active = now_ms;
        }
        let continuous = g.last_produced > 0 && now_ms.saturating_sub(g.last_produced) < CONTINUOUS_MS;
        if interacting || continuous {
            g.last_active = now_ms;
            if !g.pinned && set_affinity(fast).is_ok() {
                g.pinned = true;
            }
        } else if g.pinned && now_ms.saturating_sub(g.last_active) > IDLE_MS && set_affinity(all).is_ok() {
            g.pinned = false;
        }
    });
}

/// 引擎这一次 pump 出了新画面（[`note_frame`] 据此判断「在连续出帧」）
pub fn note_produced(now_ms: u64) {
    GATE.with(|g| {
        let mut g = g.borrow_mut();
        if g.masks.is_some() {
            g.last_produced = now_ms;
        }
    });
}

/// 在引擎线程上调：绑大核，返回日志行（没绑时说明原因）。之后由 [`note_frame`] 按交互收放
pub fn pin_current_thread() -> String {
    if std::env::var("MINI_CPU_PIN").is_ok_and(|v| v == "0") {
        return "⚡ 引擎线程不绑核（MINI_CPU_PIN=0）".into();
    }
    let freqs = cpu_max_freqs();
    let Some(mask) = fast_mask(&freqs) else {
        return format!("⚡ 引擎线程不绑核：各核一样快或读不到频率（{} 个核）", freqs.len());
    };
    let cpus: Vec<usize> = (0..64).filter(|i| mask & (1u64 << i) != 0).collect();
    let all = (0..freqs.len().min(64)).fold(0u64, |m, i| m | (1u64 << i));
    match set_affinity(mask) {
        Ok(()) => {
            // 时钟从 0 起：第一次 note_frame 不交互时，过了 IDLE_MS 就放开
            GATE.with(|g| *g.borrow_mut() = Gate { masks: Some((mask, all)), pinned: true, last_active: 0, last_produced: 0 });
            format!("⚡ 引擎线程绑大核 {cpus:?}（共 {} 个核，排除最慢一簇；交互时绑、停 {IDLE_MS}ms 放开）", freqs.len())
        }
        Err(e) => format!("⚡ 引擎线程绑核失败（{e}），按系统调度走"),
    }
}

/// 每个核的最高频率（kHz），读不到的记 0
fn cpu_max_freqs() -> Vec<u64> {
    (0..64)
        .map_while(|i| {
            let base = format!("/sys/devices/system/cpu/cpu{i}");
            if !std::path::Path::new(&base).exists() {
                return None;
            }
            let f = std::fs::read_to_string(format!("{base}/cpufreq/cpuinfo_max_freq"))
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            Some(f)
        })
        .collect()
}

/// 比最慢一簇快的那些核；全一样快、或读不到频率时 None
pub(crate) fn fast_mask(freqs: &[u64]) -> Option<u64> {
    let known: Vec<u64> = freqs.iter().copied().filter(|f| *f > 0).collect();
    let min = *known.iter().min()?;
    let mask = freqs
        .iter()
        .enumerate()
        .filter(|(i, f)| *i < 64 && **f > min)
        .fold(0u64, |m, (i, _)| m | (1u64 << i));
    (mask != 0).then_some(mask)
}

#[cfg(target_os = "android")]
fn set_affinity(mask: u64) -> Result<(), String> {
    extern "C" {
        fn sched_setaffinity(pid: i32, size: usize, mask: *const u64) -> i32;
    }
    // bionic 的 cpu_set_t 是 1024 位；给满 128 字节，低 64 位放掩码
    let mut set = [0u64; 16];
    set[0] = mask;
    // SAFETY: 指针指向 128 字节的栈数组，调用期间有效；pid 0 = 当前线程
    let r = unsafe { sched_setaffinity(0, std::mem::size_of_val(&set), set.as_ptr()) };
    if r == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().to_string())
    }
}

#[cfg(not(target_os = "android"))]
fn set_affinity(_mask: u64) -> Result<(), String> {
    Err("只在 Android 上绑核".into())
}

#[cfg(test)]
mod tests {
    use super::fast_mask;

    #[test]
    fn excludes_only_the_slowest_cluster() {
        // SM7635：4 小 + 3 中 + 1 大
        let f = [1804800, 1804800, 1804800, 1804800, 2400000, 2400000, 2400000, 2707200];
        assert_eq!(fast_mask(&f), Some(0b1111_0000));
    }

    #[test]
    fn homogeneous_or_unknown_is_left_alone() {
        assert_eq!(fast_mask(&[2000000; 8]), None);
        assert_eq!(fast_mask(&[0; 4]), None);
        assert_eq!(fast_mask(&[]), None);
        // 读不到的核（0）不算「最慢」，也不进掩码
        assert_eq!(fast_mask(&[0, 1000, 2000]), Some(0b100));
    }
}
