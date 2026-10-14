//! `MINI_GPU_FAULT` 的语法（设计 §10；v2 修订 R4、R14）。
//!
//! 这里只做**纯解析**：注入点在以后的 GPU 合成器里，解析结果由宿主入口经构造参数
//! 传下去（不在注入点再读环境变量，见 [`super::pref::HostEnv`]）。
//!
//! | 写法 | 含义 |
//! |---|---|
//! | `init` | `request_device` 之后当作失败 |
//! | `selftest` | 启动自检强制判不一致 |
//! | `tint` | 自检**之后**的帧 R 通道 +1（饱和），用来证明比较链路没有空转 |
//! | `lost@N` | 第 N 帧调 `device.destroy()`，走真实的设备丢失回调 |
//! | `surface-lost@N` | 第 N 帧 acquire 当作 `Lost`，且重建 surface 也当作失败 |
//! | `timeout@N` | 第 N 帧 acquire 当作 `Timeout`（本帧不上屏，下一帧必须补上） |
//!
//! N 是 `u32` 帧号，从选定 GPU 的那一帧起计。不区分大小写，两端空白忽略。
//! 认不出来的写法由入口 warn 一次后忽略注入 —— 不能因为一个拼错的诊断开关就拒绝启动。
use std::fmt;

/// 一次故障注入
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaultSpec {
    Init,
    Selftest,
    Tint,
    Lost(u32),
    SurfaceLost(u32),
    Timeout(u32),
}

impl FaultSpec {
    /// 带帧号的注入在第几帧生效；不带帧号的返回 `None`
    pub fn frame(self) -> Option<u32> {
        match self {
            FaultSpec::Lost(n) | FaultSpec::SurfaceLost(n) | FaultSpec::Timeout(n) => Some(n),
            FaultSpec::Init | FaultSpec::Selftest | FaultSpec::Tint => None,
        }
    }
}

impl fmt::Display for FaultSpec {
    /// 与输入同一种写法（日志里原样回显，方便复制去复现）
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FaultSpec::Init => f.write_str("init"),
            FaultSpec::Selftest => f.write_str("selftest"),
            FaultSpec::Tint => f.write_str("tint"),
            FaultSpec::Lost(n) => write!(f, "lost@{n}"),
            FaultSpec::SurfaceLost(n) => write!(f, "surface-lost@{n}"),
            FaultSpec::Timeout(n) => write!(f, "timeout@{n}"),
        }
    }
}

/// 解析一条注入。失败时返回给人看的原因（入口拼进 warn 里）。
pub fn parse(s: &str) -> Result<FaultSpec, String> {
    let s = s.trim().to_ascii_lowercase();
    let (name, frame) = match s.split_once('@') {
        Some((name, n)) => (name, Some(n)),
        None => (s.as_str(), None),
    };
    let need_frame = |make: fn(u32) -> FaultSpec| match frame {
        None => Err(format!("{name} 要带帧号，写成 {name}@N")),
        Some(n) => n
            .parse::<u32>()
            .map(make)
            .map_err(|_| format!("帧号 {n:?} 不是非负整数")),
    };
    let no_frame = |spec: FaultSpec| match frame {
        None => Ok(spec),
        Some(_) => Err(format!("{name} 不带帧号")),
    };
    match name {
        "init" => no_frame(FaultSpec::Init),
        "selftest" => no_frame(FaultSpec::Selftest),
        "tint" => no_frame(FaultSpec::Tint),
        "lost" => need_frame(FaultSpec::Lost),
        "surface-lost" => need_frame(FaultSpec::SurfaceLost),
        "timeout" => need_frame(FaultSpec::Timeout),
        _ => Err(format!(
            "认不出 {name:?}（可选 init / selftest / tint / lost@N / surface-lost@N / timeout@N）"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_form() {
        assert_eq!(parse("init"), Ok(FaultSpec::Init));
        assert_eq!(parse("selftest"), Ok(FaultSpec::Selftest));
        assert_eq!(parse("tint"), Ok(FaultSpec::Tint));
        assert_eq!(parse("lost@120"), Ok(FaultSpec::Lost(120)));
        assert_eq!(parse("surface-lost@0"), Ok(FaultSpec::SurfaceLost(0)));
        assert_eq!(parse("timeout@7"), Ok(FaultSpec::Timeout(7)));
        // 不区分大小写、两端空白忽略
        assert_eq!(parse("  LOST@30 "), Ok(FaultSpec::Lost(30)));
        assert_eq!(parse("Surface-Lost@4294967295"), Ok(FaultSpec::SurfaceLost(u32::MAX)));
    }

    #[test]
    fn rejects_malformed() {
        for bad in [
            "", "lost", "lost@", "lost@-1", "lost@1.5", "lost@x", "lost@4294967296",
            "timeout", "surface-lost", "init@3", "tint@1", "selftest@0", "boom", "lost@@1",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} 不该被接受");
        }
        // 原因要说清楚怎么改
        assert!(parse("lost").unwrap_err().contains("lost@N"));
        assert!(parse("boom").unwrap_err().contains("timeout@N"));
    }

    #[test]
    fn display_round_trips_and_frame_is_exposed() {
        for f in [
            FaultSpec::Init,
            FaultSpec::Selftest,
            FaultSpec::Tint,
            FaultSpec::Lost(5),
            FaultSpec::SurfaceLost(6),
            FaultSpec::Timeout(7),
        ] {
            assert_eq!(parse(&f.to_string()), Ok(f));
        }
        assert_eq!(FaultSpec::Timeout(9).frame(), Some(9));
        assert_eq!(FaultSpec::Tint.frame(), None);
    }
}
