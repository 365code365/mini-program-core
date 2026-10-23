//! `MINI_DAMAGE_LOG` 的分段计时：一帧重画里各段各花了多少（取数据 / 画条带 / 画布改高后重画 /
//! 覆盖层 / tabBar / 合成），接在 🩹 行尾。
//!
//! 真机上「进页面第一帧 40~50ms」只有一个总数时，不知道该砍哪一段；桌面上的 `sample`
//! 又量不到手机上的比例（CPU、内存带宽差得远）。关着日志时不取时钟。
use std::fmt::Write;
use std::time::Instant;

pub(crate) struct Laps {
    last: Option<Instant>,
    out: String,
}

impl Laps {
    pub fn new(on: bool) -> Self {
        Self { last: on.then(Instant::now), out: String::new() }
    }

    /// 记一段：从上一次 `lap`（或开始）到现在
    pub fn lap(&mut self, what: &str) {
        let Some(t) = self.last else { return };
        let now = Instant::now();
        let ms = now.duration_since(t).as_secs_f32() * 1000.0;
        // 不到 0.1ms 的段不写，行别太长
        if ms >= 0.1 {
            let _ = write!(self.out, " {what} {ms:.1}");
        }
        self.last = Some(now);
    }

    /// 没开日志、或一段都没记到：None
    pub fn finish(self) -> Option<String> {
        (!self.out.is_empty()).then(|| format!("  〔{} 〕", self.out))
    }
}

/// 一次 pump 里重画之前那几段（换页留图 / 建页 / 事件回调 / 失效分析）的计时，接在 `⏱ [SDK] pump` 行尾。
/// 关着日志时 `on` 是假，`start` 不取时钟
#[derive(Default)]
pub(crate) struct PumpNotes {
    on: bool,
    out: String,
}
impl PumpNotes {
    pub fn new(on: bool) -> Self {
        Self { on, out: String::new() }
    }
    /// 开始计一段（关着日志时 None）
    pub fn start(&self) -> Option<Instant> {
        self.on.then(Instant::now)
    }
    /// 记下从 `start` 到现在这一段（不到 0.5ms 不记）
    pub fn add(&mut self, what: &str, t: Option<Instant>) {
        let Some(t) = t else { return };
        let ms = t.elapsed().as_secs_f32() * 1000.0;
        if ms >= 0.5 {
            let _ = write!(self.out, "；{what} {ms:.1}");
        }
    }
    /// 原样接一段说明（开着日志才记）
    pub fn note(&mut self, s: &str) {
        if self.on {
            self.out.push_str(s);
        }
    }
    /// 取走这一次 pump 记下的（同时清空）
    pub fn take_line(&mut self) -> String {
        std::mem::take(&mut self.out)
    }
}
