//! 小游戏的时间轴（game-doc/02 §4.3）：`performance.now()`、rAF 参数、定时器都在这一条轴上，0 = 实例创建。
//!
//! 为什么不复用 `frame_clock::Anchor`：它按毫秒整数取样，rAF 的时间戳要微秒（144Hz 一帧 6.944ms，
//! 按毫秒截断后 dt 在 6 与 7 之间跳，按 dt 积分的动画会抖）。
//!
//! 两种模式：
//! - **宿主时钟**（真机 / 交互窗体）：宿主每帧给 vsync 时刻（µs，任意原点的单调钟），第一次映射到「此刻相对创建的偏移」，
//!   之后严格按宿主给的间隔走；与墙上时钟偏差超过 250ms（宿主换钟、长时间卡住）重锚。帧与帧之间（输入回调、game.js）
//!   `performance.now()` 读墙上时钟。
//! - **固定时钟**（无头测试，`fixed_clock`）：宿主给的时刻就是游戏时间；帧与帧之间 `performance.now()` 停在最近一帧。
//!   于是同一个 `--hz` 跑两次，rAF 参数、定时器到点、像素逐帧相同。

use std::time::Instant;

const RESYNC_US: i64 = 250_000;

pub struct GameClock {
    origin: Instant,
    fixed: bool,
    /// 宿主时刻 → 游戏时刻的偏移（µs）：game = host + offset
    offset: Option<i64>,
    /// 最近一帧的游戏时刻（µs）
    frame_us: u64,
}

impl GameClock {
    pub fn new(fixed: bool) -> Self {
        Self { origin: Instant::now(), fixed, offset: None, frame_us: 0 }
    }

    fn wall_us(&self) -> u64 {
        self.origin.elapsed().as_micros() as u64
    }

    /// 一帧开始：宿主给的 vsync 时刻（µs；0 = 宿主不给，取墙上时钟）→ 这一帧的游戏时刻（µs，单调不减）
    pub fn begin_frame(&mut self, host_us: u64) -> u64 {
        let t = if host_us == 0 {
            self.wall_us()
        } else if self.fixed {
            host_us
        } else {
            let wall = self.wall_us() as i64;
            let off = *self.offset.get_or_insert(wall - host_us as i64);
            let mapped = host_us as i64 + off;
            // 宿主时刻比墙上时钟超前（不可能是 vsync）或落后太多：重锚
            if mapped > wall + 2_000 || wall - mapped > RESYNC_US {
                self.offset = Some(wall - host_us as i64);
                wall.max(0) as u64
            } else {
                mapped.max(0) as u64
            }
        };
        self.frame_us = t.max(self.frame_us);
        self.frame_us
    }

    /// 「此刻」对应的 Instant：固定时钟停在最近一帧（创建时是 0），宿主时钟就是墙上时钟。
    /// 帧与帧之间（game.js 顶层、输入回调）登记的定时器按它起算
    pub fn current_instant(&self) -> Instant {
        if self.fixed { self.instant_at(self.frame_us) } else { Instant::now() }
    }

    /// `performance.now()`（毫秒，带小数）
    pub fn now_ms(&self) -> f64 {
        let us = if self.fixed { self.frame_us } else { self.wall_us().max(self.frame_us) };
        us as f64 / 1000.0
    }

    /// 宿主时刻（µs，与 `begin_frame` 同一时钟；0 = 不知道）→ 游戏时间轴上的毫秒（输入事件的 `timeStamp`）。
    /// 宿主时钟模式下用帧映射同一个偏移，并夹到「此刻」以内（事件不可能来自未来）；还没映射过（第一帧之前）用此刻
    pub fn host_to_game_ms(&self, host_us: u64) -> f64 {
        let now = self.now_ms();
        if host_us == 0 {
            return now;
        }
        let us = if self.fixed {
            host_us as i64
        } else {
            match self.offset {
                Some(off) => host_us as i64 + off,
                None => return now,
            }
        };
        let ms = us.max(0) as f64 / 1000.0;
        // 固定时钟（无头）不夹：脚本里给的就是游戏时刻，可以落在「下一帧」
        if self.fixed { ms } else { ms.min(now) }
    }

    /// 游戏时刻对应的 Instant（定时器时钟用）
    pub fn instant_at(&self, us: u64) -> Instant {
        self.origin + std::time::Duration::from_micros(us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 固定时钟_宿主时刻就是游戏时刻_帧间停住() {
        let mut c = GameClock::new(true);
        assert_eq!(c.begin_frame(6_944), 6_944);
        assert_eq!(c.now_ms(), 6.944);
        std::thread::sleep(std::time::Duration::from_millis(3));
        assert_eq!(c.now_ms(), 6.944, "帧与帧之间停在最近一帧");
        assert_eq!(c.begin_frame(13_889), 13_889);
    }

    #[test]
    fn 宿主时钟_按宿主间隔走_单调() {
        let mut c = GameClock::new(false);
        let base = 9_000_000_000u64; // 宿主的单调钟原点任意
        let t0 = c.begin_frame(base);
        // vsync 回调总是在 vsync 之后才到：真等一下，宿主时刻才不会跑到墙上时钟前面
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t1 = c.begin_frame(base + 8_333);
        assert_eq!(t1 - t0, 8_333, "间隔严格按宿主给的");
        // 宿主时刻倒退：游戏时刻不倒退
        assert!(c.begin_frame(base) >= t1);
    }
}
