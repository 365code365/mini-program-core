//! 判定缓存 `gpu-verdict.json`：「这个驱动上次在这里出过问题」（设计 §6.4；v2 修订 R15）。
//!
//! Auto 每次启动都要过自检，但有的驱动是「自检过不了」或「跑着跑着设备丢失」，
//! 每次都重新撞一遍既浪费启动时间、也会让用户每次都看到一次回退。所以把结论按
//! 驱动记下来，下次 Auto 直接走 CPU。
//!
//! - 键由调用方给（R15：`backend|vendor|device|name|driver|driver_info|引擎键`，
//!   引擎键里含着色器源码哈希与自检版本，这些都在以后的 GPU 模块里算）。
//!   驱动、着色器或自检逻辑一变就是新键，重新评估。
//! - 自检像素不一致：记一次就拒绝。设备丢失：同一个键累计 2 次才拒绝 ——
//!   单次可能只是系统级的 GPU 重置，不该一次就把这台机器打成 CPU。
//! - 最多 [`VERDICT_CAP`] 条，超出丢最久没更新的。
//! - 文件不存在 → 空；JSON 损坏或版本不对 → 空（下次写入时覆盖），并报一条 warn。
//!
//! ## 什么时候不读、不写（[`CacheMode`]）
//! - **注入故障时不写**（`MINI_GPU_FAULT` 已设置）：否则一次回退测试会让之后所有的
//!   Auto 运行都走 CPU，而且不看缓存文件根本想不到为什么。
//! - **debug 构建既不读也不写**（R15）：开发期改着色器时引擎键不一定跟着变，
//!   读到上一版留下的「拒绝」会让人以为是新改动出了问题。
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 最多留多少条（每条对应一个驱动 + 引擎版本）
pub const VERDICT_CAP: usize = 32;
/// 设备丢失累计这么多次才拒绝
pub const DEVICE_LOST_LIMIT: u32 = 2;
/// 文件格式版本：字段语义变了就 +1，旧文件按损坏处理
const FILE_VERSION: u32 = 1;

/// 要记的事件
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerdictEvent {
    SelftestMismatch,
    DeviceLost,
}

/// 读写策略
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheMode {
    /// 不读不写（debug 构建）
    Off,
    /// 只读（注入了故障）
    ReadOnly,
    ReadWrite,
}

impl CacheMode {
    /// 本次运行该用哪种：debug 构建 → `Off`；注入了故障 → `ReadOnly`；否则读写
    pub fn for_run(fault_injected: bool) -> Self {
        if cfg!(debug_assertions) {
            CacheMode::Off
        } else if fault_injected {
            CacheMode::ReadOnly
        } else {
            CacheMode::ReadWrite
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    key: String,
    #[serde(default)]
    selftest_mismatch: bool,
    #[serde(default)]
    device_lost: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct VerdictFile {
    version: u32,
    /// 从旧到新：更新过的条目挪到末尾，超出上限时从头丢
    entries: Vec<Entry>,
}

/// 内存里的判定缓存
#[derive(Clone, Debug)]
pub struct VerdictCache {
    path: PathBuf,
    mode: CacheMode,
    entries: Vec<Entry>,
}

/// 默认位置：数据目录下的 `gpu/verdict.json`（开发机上是 `target/gpu/verdict.json`，
/// `clean-target.sh` 会顺手清掉；移动端落在宿主给的沙盒目录）
pub fn default_path() -> PathBuf {
    crate::data_dir::subdir("gpu").join("verdict.json")
}

impl VerdictCache {
    /// 读缓存。第二个返回值是要打的 warn（文件损坏 / 版本不对）；文件不存在不算问题。
    pub fn load(path: &Path, mode: CacheMode) -> (Self, Option<String>) {
        let mut cache = Self { path: path.to_path_buf(), mode, entries: Vec::new() };
        if mode == CacheMode::Off {
            return (cache, None);
        }
        let Ok(text) = std::fs::read_to_string(path) else { return (cache, None) };
        match serde_json::from_str::<VerdictFile>(&text) {
            Ok(f) if f.version == FILE_VERSION => {
                cache.entries = f.entries;
                // 手改过的文件可能超过上限：只留最新的那些
                let extra = cache.entries.len().saturating_sub(VERDICT_CAP);
                cache.entries.drain(..extra);
                (cache, None)
            }
            Ok(f) => {
                let w = format!("⚠️ GPU 判定缓存版本 {} 不对（应为 {FILE_VERSION}），按空处理：{}", f.version, path.display());
                (cache, Some(w))
            }
            Err(e) => {
                let w = format!("⚠️ GPU 判定缓存已损坏，按空处理：{}（{e}）", path.display());
                (cache, Some(w))
            }
        }
    }

    pub fn mode(&self) -> CacheMode {
        self.mode
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 这个键该不该被拒绝；拒绝时给出原因（拼进「缓存判定：该驱动上次{原因}」）
    pub fn rejection(&self, key: &str) -> Option<String> {
        let e = self.entries.iter().find(|e| e.key == key)?;
        if e.selftest_mismatch {
            Some("自检像素不一致".into())
        } else if e.device_lost >= DEVICE_LOST_LIMIT {
            Some(format!("设备丢失（累计 {} 次）", e.device_lost))
        } else {
            None
        }
    }

    /// 记一次事件（只改内存，[`save`](Self::save) 才落盘）。只读 / 关闭时什么也不做。
    pub fn record(&mut self, key: &str, event: VerdictEvent) {
        if self.mode != CacheMode::ReadWrite {
            return;
        }
        let mut entry = match self.entries.iter().position(|e| e.key == key) {
            Some(i) => self.entries.remove(i),
            None => Entry { key: key.to_string(), ..Entry::default() },
        };
        match event {
            VerdictEvent::SelftestMismatch => entry.selftest_mismatch = true,
            VerdictEvent::DeviceLost => entry.device_lost = entry.device_lost.saturating_add(1),
        }
        self.entries.push(entry);
        let extra = self.entries.len().saturating_sub(VERDICT_CAP);
        self.entries.drain(..extra);
    }

    /// 落盘。只读 / 关闭时不写（返回 `Ok`）。失败由调用方 warn 一次后忽略。
    ///
    /// 先写临时文件再改名：写到一半被杀掉时留下的是旧文件，而不是半截 JSON。
    pub fn save(&self) -> Result<(), String> {
        if self.mode != CacheMode::ReadWrite {
            return Ok(());
        }
        let file = VerdictFile { version: FILE_VERSION, entries: self.entries.clone() };
        let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("建目录 {} 失败：{e}", dir.display()))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| format!("写 {} 失败：{e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| format!("改名到 {} 失败：{e}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个用例一个独立文件：测试并行跑，共用一个文件会互相读到对方写的条目
    fn temp_path(slot: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mini-verdict-{}-{slot}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir.join("verdict.json")
    }

    fn cleanup(p: &Path) {
        if let Some(d) = p.parent() {
            std::fs::remove_dir_all(d).ok();
        }
    }

    #[test]
    fn round_trip_and_rules() {
        let p = temp_path("rt");
        let (mut c, w) = VerdictCache::load(&p, CacheMode::ReadWrite);
        assert!(c.is_empty() && w.is_none(), "文件不存在应视为空、不报 warn");
        c.record("a", VerdictEvent::SelftestMismatch);
        c.record("b", VerdictEvent::DeviceLost);
        c.save().unwrap();

        let (mut c, w) = VerdictCache::load(&p, CacheMode::ReadWrite);
        assert!(w.is_none());
        assert_eq!(c.len(), 2);
        // 自检不一致：一次就拒绝
        assert_eq!(c.rejection("a").as_deref(), Some("自检像素不一致"));
        // 设备丢失：一次还不拒绝，两次才拒绝
        assert_eq!(c.rejection("b"), None);
        c.record("b", VerdictEvent::DeviceLost);
        assert_eq!(c.rejection("b").as_deref(), Some("设备丢失（累计 2 次）"));
        assert_eq!(c.rejection("没见过"), None);
        cleanup(&p);
    }

    #[test]
    fn corrupt_or_wrong_version_is_empty_with_a_warning() {
        let p = temp_path("bad");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "{ 这不是 json").unwrap();
        let (c, w) = VerdictCache::load(&p, CacheMode::ReadWrite);
        assert!(c.is_empty());
        assert!(w.unwrap().contains("损坏"));
        // 损坏的文件下次写入时被覆盖
        let mut c = c;
        c.record("k", VerdictEvent::SelftestMismatch);
        c.save().unwrap();
        assert_eq!(VerdictCache::load(&p, CacheMode::ReadWrite).0.len(), 1);

        std::fs::write(&p, r#"{"version":99,"entries":[{"key":"k","selftest_mismatch":true}]}"#).unwrap();
        let (c, w) = VerdictCache::load(&p, CacheMode::ReadWrite);
        assert!(c.is_empty());
        assert!(w.unwrap().contains("版本 99"));
        cleanup(&p);
    }

    #[test]
    fn keeps_at_most_32_dropping_the_stalest() {
        let p = temp_path("cap");
        let (mut c, _) = VerdictCache::load(&p, CacheMode::ReadWrite);
        for i in 0..40 {
            c.record(&format!("k{i}"), VerdictEvent::SelftestMismatch);
        }
        assert_eq!(c.len(), VERDICT_CAP);
        assert_eq!(c.rejection("k7"), None, "最旧的 8 条应已丢掉");
        assert!(c.rejection("k8").is_some());
        // 更新过的条目挪到最新：再塞一条时丢的是 k9 而不是 k8
        c.record("k8", VerdictEvent::DeviceLost);
        c.record("k40", VerdictEvent::SelftestMismatch);
        assert!(c.rejection("k8").is_some());
        assert_eq!(c.rejection("k9"), None);
        c.save().unwrap();
        assert_eq!(VerdictCache::load(&p, CacheMode::ReadWrite).0.len(), VERDICT_CAP);
        cleanup(&p);
    }

    #[test]
    fn fault_injected_run_reads_but_never_writes() {
        let p = temp_path("ro");
        let (mut c, _) = VerdictCache::load(&p, CacheMode::ReadOnly);
        c.record("k", VerdictEvent::SelftestMismatch);
        assert!(c.is_empty(), "只读模式不该记");
        c.save().unwrap();
        assert!(!p.exists(), "注入故障时不该写文件");

        // 已有的结论照样能读到
        let (mut rw, _) = VerdictCache::load(&p, CacheMode::ReadWrite);
        rw.record("k", VerdictEvent::SelftestMismatch);
        rw.save().unwrap();
        let (ro, _) = VerdictCache::load(&p, CacheMode::ReadOnly);
        assert!(ro.rejection("k").is_some());
        cleanup(&p);
    }

    #[test]
    fn off_mode_neither_reads_nor_writes() {
        let p = temp_path("off");
        let (mut rw, _) = VerdictCache::load(&p, CacheMode::ReadWrite);
        rw.record("k", VerdictEvent::SelftestMismatch);
        rw.save().unwrap();
        let before = std::fs::read_to_string(&p).unwrap();

        let (mut off, w) = VerdictCache::load(&p, CacheMode::Off);
        assert!(off.is_empty() && w.is_none(), "关闭时不该读");
        off.record("x", VerdictEvent::SelftestMismatch);
        off.save().unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), before, "关闭时不该写");
        cleanup(&p);
    }

    #[test]
    fn mode_for_run_follows_build_and_fault() {
        if cfg!(debug_assertions) {
            assert_eq!(CacheMode::for_run(false), CacheMode::Off);
            assert_eq!(CacheMode::for_run(true), CacheMode::Off);
        } else {
            assert_eq!(CacheMode::for_run(false), CacheMode::ReadWrite);
            assert_eq!(CacheMode::for_run(true), CacheMode::ReadOnly);
        }
    }
}
