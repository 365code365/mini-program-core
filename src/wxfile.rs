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

/// 宿主可以直接写的临时目录（`<根>/tmp`，不存在就建）。宿主把选到的图、拍的照片写进这里，
/// 回包时给 `file://<这里>/<名>` —— 引擎只换成虚拟路径，不再拷一遍（见 [`import`]）
pub fn temp_dir() -> PathBuf {
    let dir = root().join("tmp");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// 真实路径 → 虚拟路径：落在当前小程序的 wxfile 根目录下才给。
///
/// 两边都先 `canonicalize`：Android 的 `/data/data/<包>` 是 `/data/user/0/<包>` 的软链、
/// macOS 的 `/var` 是 `/private/var`，宿主与引擎各拿到其中一种写法时前缀对不上
pub fn from_path(path: &Path) -> Option<String> {
    let root = root();
    let root = root.canonicalize().unwrap_or(root);
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let rel = path.strip_prefix(&root).ok()?;
    if rel.as_os_str().is_empty() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(format!("{SCHEME}{}", rel.to_str()?.replace('\\', "/")))
}

/// 宿主交来的文件（选图、拍照、选文件）变成小程序能用的虚拟路径：已在 wxfile 根目录下的
/// （宿主写进了 [`temp_dir`]）只换名字；别处的拷进 `tmp/`，扩展名沿用。
///
/// 拷贝而不是硬链接 / 挪走：桌面上选的是用户自己的照片，链过来之后小程序那边的改动会改到原图，
/// 挪走更是把用户的文件拿走了。移动端宿主写进 temp_dir 就没有这次拷贝
pub fn import(path: &Path) -> std::io::Result<String> {
    if !path.is_file() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, path.display().to_string()));
    }
    if let Some(uri) = from_path(path) {
        return Ok(uri);
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let (uri, dst) = new_temp(&ext);
    if let Some(dir) = dst.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::copy(path, &dst)?;
    Ok(uri)
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
