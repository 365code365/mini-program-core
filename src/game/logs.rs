//! 游戏实例的 `console` 输出：每帧取走 `__print_buffer`，**限流**（game-doc/09 §2.4）。
//!
//! 游戏常在每帧里打日志（调试遗留）。一帧几千行时，把它们序列化出 JS、再逐行交给宿主，
//! 本身就会让这一帧超时；宿主侧 logcat 量大了还会整段静默（避坑清单记过 ColorOS 的这个现象）。
//! 所以每帧最多交出 [`MAX_LINES`] 行、单行截到 [`MAX_LINE_BYTES`]，其余只计数，
//! 下一次交出时打一行「丢弃 N 行」。`MINI_GAME_LOG_ALL=1`（或创建选项 `logAll`）关掉限流。

use crate::runtime::MiniApp;

pub const MAX_LINES: usize = 500;
pub const MAX_LINE_BYTES: usize = 4096;

pub struct LogPump {
    unlimited: bool,
    dropped: usize,
}

impl LogPump {
    pub fn new(log_all: bool) -> Self {
        Self { unlimited: log_all || std::env::var("MINI_GAME_LOG_ALL").is_ok(), dropped: 0 }
    }

    /// 取走本帧的日志行（已限流、已截断）。缓冲为空时只做一次长度判断，不分配
    pub fn drain(&mut self, app: &MiniApp) -> Vec<String> {
        let cap = if self.unlimited { usize::MAX } else { MAX_LINES };
        // 在 JS 侧就截掉多余的行：不让 10 000 行先被 JSON 序列化出来再丢
        let js = format!(
            "(function(){{var a=__print_buffer;if(!a.length)return '';var n=a.length;var o=a.splice(0,{cap});a.length=0;return JSON.stringify([n,o]);}})()",
            cap = if cap == usize::MAX { "a.length".to_string() } else { cap.to_string() }
        );
        let raw = match app.eval(&js) {
            Ok(s) if !s.is_empty() => s,
            _ => return self.flush_dropped(),
        };
        let Ok(serde_json::Value::Array(v)) = serde_json::from_str::<serde_json::Value>(&raw) else { return self.flush_dropped() };
        let total = v.first().and_then(|n| n.as_u64()).unwrap_or(0) as usize;
        let lines = v.get(1).and_then(|l| l.as_array()).cloned().unwrap_or_default();
        let mut out = self.flush_dropped();
        out.extend(lines.iter().map(|l| truncate(l.as_str().map(str::to_string).unwrap_or_else(|| l.to_string()))));
        self.dropped += total.saturating_sub(lines.len());
        out
    }

    fn flush_dropped(&mut self) -> Vec<String> {
        if self.dropped == 0 {
            return Vec::new();
        }
        let n = std::mem::take(&mut self.dropped);
        vec![format!("…丢弃 {n} 行日志（每帧最多 {MAX_LINES} 行，MINI_GAME_LOG_ALL=1 关闭限流）")]
    }
}

fn truncate(mut s: String) -> String {
    if s.len() > MAX_LINE_BYTES {
        let mut cut = MAX_LINE_BYTES;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        let extra = s.len() - cut;
        s.truncate(cut);
        s.push_str(&format!("…（截断 {extra} 字节）"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn 截断不切坏多字节字符() {
        let s = "中".repeat(2000); // 6000 字节
        let t = truncate(s);
        assert!(t.starts_with(&"中".repeat(1365)));
        assert!(t.contains("截断"));
        assert_eq!(truncate("short".into()), "short");
    }
}
