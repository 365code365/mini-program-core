//! 小程序资源根目录。
//!
//! WXML 里的图片路径是**小程序包内绝对路径**（`/assets/x.jpg`），解析它需要知道
//! 当前运行的小程序根目录。之前图片加载把 `sample-app` 写死在候选路径里，换成
//! 别的小程序（news-app）就全部加载失败 —— 宿主必须在启动时登记一次根目录。

use std::path::{Path, PathBuf};
use std::sync::RwLock;

static APP_ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

/// 登记当前小程序根目录（宿主启动时调用一次）。
pub fn set_app_root<P: Into<PathBuf>>(root: P) {
    if let Ok(mut guard) = APP_ROOT.write() {
        *guard = Some(root.into());
    }
}

/// 当前小程序根目录。
pub fn app_root() -> Option<PathBuf> {
    APP_ROOT.read().ok().and_then(|g| g.clone())
}

/// 把小程序内路径解析成存在的真实文件路径。
///
/// 顺序：小程序根目录 → 根目录下 `assets/` → 进程工作目录 → 内置 sample-app（兜底）。
pub fn resolve(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim_start_matches('/');
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(root) = app_root() {
        candidates.push(root.join(trimmed));
        candidates.push(root.join("assets").join(trimmed));
    }
    candidates.push(PathBuf::from(path));
    candidates.push(PathBuf::from(trimmed));
    candidates.push(Path::new("assets").join(trimmed));
    candidates.push(Path::new("sample-app").join(trimmed));

    candidates.into_iter().find(|p| p.is_file())
}
