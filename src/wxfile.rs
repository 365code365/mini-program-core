//! 小程序的本地文件路径 `wxfile://`（`wx.downloadFile` 的 `tempFilePath`、`wx.env.USER_DATA_PATH`）。
//!
//! 微信里下载、选图、录音的产物都是 `wxfile://tmp_…` / `wxfile://usr/…` 这样的虚拟路径，
//! 能直接塞给 `<image src>`、`uploadFile({filePath})`。这里把它映射到
//! `<数据目录>/wxfile/<小程序名>/`（数据目录同 storage，见 `crate::data_dir`），
//! 并让 `assets::resolve` 认得它 —— 于是图片、视频、上传都能用下载回来的文件。
//!
//! 路径里不许有 `..`（不能借虚拟路径读到沙盒外的文件）。

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const SCHEME: &str = "wxfile://";

/// 当前小程序的 wxfile 根目录
pub fn root() -> PathBuf {
    let name = crate::assets::app_root()
        .and_then(|r| r.file_name().and_then(|s| s.to_str()).map(str::to_string))
        .unwrap_or_else(|| "unknown".into());
    crate::data_dir::subdir("wxfile").join(name)
}

/// `wxfile://tmp/a.png` → 真实路径；不是 wxfile 路径、或想跳出根目录时给 None
pub fn to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix(SCHEME)?;
    let rel = Path::new(rest);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(root().join(rel))
}

/// 新的临时文件：(虚拟路径, 真实路径)。扩展名沿用来源（图片解码按内容识别，不靠它）
pub fn new_temp(ext: &str) -> (String, PathBuf) {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let ext = ext.trim_start_matches('.');
    let name = if ext.is_empty() { format!("tmp_{stamp}_{n}") } else { format!("tmp_{stamp}_{n}.{ext}") };
    let uri = format!("{SCHEME}tmp/{name}");
    let path = root().join("tmp").join(&name);
    (uri, path)
}

/// URL 路径里的扩展名（`https://x/a.png?v=1` → `png`），认不出给空串
pub fn ext_of_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or("");
    let last = path.rsplit('/').next().unwrap_or("");
    match last.rsplit_once('.') {
        Some((_, e)) if !e.is_empty() && e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()) => e.to_ascii_lowercase(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escaping_paths() {
        assert!(to_path("wxfile://tmp/a.png").is_some());
        assert!(to_path("wxfile://../etc/passwd").is_none());
        assert!(to_path("wxfile://tmp/../../x").is_none());
        assert!(to_path("/tmp/a.png").is_none());
    }

    #[test]
    fn url_extension() {
        assert_eq!(ext_of_url("https://a.com/x/y.PNG?v=1"), "png");
        assert_eq!(ext_of_url("https://a.com/x/y"), "");
        assert_eq!(ext_of_url("https://a.com/x.y/z"), "");
    }
}
