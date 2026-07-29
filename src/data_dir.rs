//! 引擎的**可写数据目录**：`wx.setStorageSync` 的落盘、远程图片的磁盘缓存都放这里。
//!
//! 为什么必须由宿主指定：以前这两处都写死在 `CARGO_MANIFEST_DIR/target/` 下 ——
//! 那是**编译这台机器**的绝对路径。开发期跑 `cargo run` 没问题，一旦把
//! `libmini_render` 装进 iOS/Android/鸿蒙 的 App 里，那个路径根本不存在：
//! - storage 静默退化成纯内存 → 「首次进入拉数据存起来、之后走缓存」的应用
//!   每次冷启都走首次分支，缓存逻辑等于从没执行过；
//! - 图片磁盘缓存也一直落空，每次冷启都重新下载。
//! 两个都不报错，只是「悄悄不生效」，这类问题在真机上极难定位。
//!
//! 所以宿主在创建引擎**之前**必须调一次 [`set_data_dir`]，传自己的沙盒目录：
//! - iOS：`NSSearchPathForDirectoriesInDomains(NSCachesDirectory, ...)` 或 Application Support
//! - Android：`context.getFilesDir()` / `getCacheDir()`
//! - 鸿蒙：`context.filesDir` / `context.cacheDir`
//! - 桌面：应用自己的配置/缓存目录
//!
//! 没设置时回落到开发期的老行为（仓库的 `target/`），所以现有脚本与测试不受影响。

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

fn slot() -> &'static RwLock<Option<PathBuf>> {
    static SLOT: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
    SLOT.get_or_init(|| RwLock::new(None))
}

/// 指定可写数据目录（宿主的沙盒路径）。目录不存在时会被创建。
pub fn set_data_dir<P: AsRef<Path>>(dir: P) {
    let dir = dir.as_ref().to_path_buf();
    std::fs::create_dir_all(&dir).ok();
    if let Ok(mut w) = slot().write() {
        *w = Some(dir);
    }
}

/// 当前可写数据目录。宿主没设时用开发期的 `<仓库>/target`。
pub fn data_dir() -> PathBuf {
    if let Ok(r) = slot().read() {
        if let Some(dir) = r.as_ref() {
            return dir.clone();
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")
}

/// 数据目录下的一个子目录（不存在则创建）。
pub fn subdir(name: &str) -> PathBuf {
    let dir = data_dir().join(name);
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_the_repo_target_dir_for_dev() {
        // 不设置时必须能拿到一个绝对路径（开发期脚本依赖这个回落）
        let d = data_dir();
        assert!(d.is_absolute(), "数据目录应为绝对路径: {}", d.display());
    }

    #[test]
    fn subdir_is_under_data_dir() {
        let s = subdir("mini-storage");
        assert!(s.starts_with(data_dir()));
        assert!(s.ends_with("mini-storage"));
    }
}
