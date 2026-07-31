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
//!
//! ## 已知限制：这是**进程级**设置
//! 一个进程只有一个数据目录，所以同一进程里跑两个 [`crate::host::MiniEngine`]（比如
//! 宿主想同时预热两个小程序）会共用同一份 storage 与图片缓存 —— 两个小程序的
//! storage 靠文件名（`<小程序名>.json`）隔离，但沙盒隔离要靠宿主自己给不同目录做不到。
//! 要真正支持多实例，得把它变成引擎的实例状态（`storage_file` / `image_net` 都要跟着改）。
//! 现网用法是「一个 App 一个容器」，所以先留着；测试里因此不能假设它在跑测期间不变。

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
    subdir_in(&data_dir(), name)
}

/// 指定根目录下的子目录 —— 与全局状态无关的那一半，可以确定性地测。
fn subdir_in(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
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
    fn subdir_is_under_the_given_root() {
        // 只测与全局无关的那一半。
        //
        // 从前写的是 `subdir("x").starts_with(data_dir())` —— 读了两次全局：
        // 一次在 `subdir()` 里、一次在断言里。而 `MiniEngine::new()` 会调
        // `set_data_dir()`，并行跑测试时这两次读之间全局就可能被改掉。
        // 实测 40 轮里挂 5 次，报的却是「路径不在数据目录下」，看着像功能坏了。
        let root = std::env::temp_dir().join("mini-render-data-dir-test");
        let s = subdir_in(&root, "mini-storage");
        assert!(s.starts_with(&root), "{} 应在 {} 下", s.display(), root.display());
        assert!(s.ends_with("mini-storage"));
        assert!(s.is_dir(), "子目录应被创建出来");
        std::fs::remove_dir_all(&root).ok();
    }
}
