//! `wx.getFileSystemManager()` 的原生侧：路径权限 + 文件操作，JS 经 `__native_fs(op, json)` 同步调用。
//!
//! 三类路径，权限与微信一致（官方「文件系统」一节）：
//!
//! | 路径 | 例子 | 读 | 写 |
//! |---|---|---|---|
//! | 用户文件 | `wxfile://usr/…`（`wx.env.USER_DATA_PATH`） | ✓ | ✓ |
//! | 临时 / 已保存文件 | `wxfile://tmp_…`、`wxfile://store/…` | ✓ | ✗（只能由 downloadFile / saveFile 产生） |
//! | 代码包文件 | `/images/a.png`、`images/a.png` | ✓ | ✗ |
//!
//! 真实位置见 `crate::wxfile`（`<数据目录>/wxfile/<小程序名>/`）；代码包文件按小程序根目录解析，
//! 路径里不许有 `..`。
//!
//! 返回值一律是 JSON：成功 `{"ok":true,…}`，失败 `{"ok":false,"err":"no such file or directory, open <路径>"}`，
//! JS 侧拼成 `<api>:fail <err>`（同步版 throw 同样的消息）。二进制内容直接过桥（`__native_fs_raw`，见 extra.rs）。

pub mod codec;
/// 二进制直通、readCompressedFile、readZipEntry、用户文件配额
pub mod extra;
pub mod image;
mod ops;
/// `wx.getVideoInfo`：读 MP4 / MOV 的盒子结构
pub mod video_info;
pub mod zip;

use std::path::{Component, Path, PathBuf};

pub use ops::call;

/// 已保存文件（`wx.saveFile`）所在的虚拟目录
pub const STORE: &str = "wxfile://store";
/// 用户文件根（`wx.env.USER_DATA_PATH`）
pub const USR: &str = "wxfile://usr";

/// 一条路径解析出来的真实位置与权限
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub real: PathBuf,
    pub writable: bool,
}

/// 小程序路径 → 真实路径。`None`：路径非法（跳出沙盒 / 空）
pub fn resolve(path: &str) -> Option<Resolved> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    if path.starts_with(crate::wxfile::SCHEME) {
        let trimmed = path.trim_end_matches('/');
        let real = if trimmed.len() < crate::wxfile::SCHEME.len() {
            crate::wxfile::root()
        } else {
            crate::wxfile::to_path(trimmed)?
        };
        let writable = path == USR || path.starts_with(&format!("{USR}/"));
        return Some(Resolved { real, writable });
    }
    if path.starts_with("file://") {
        // 沙盒里的 file://（见 wxfile::from_file_uri），沙盒外的不认
        return resolve(&crate::wxfile::from_file_uri(path)?);
    }
    if path.contains("://") {
        return None;
    }
    let rel = path.trim_start_matches('/');
    let p = Path::new(rel);
    if rel.is_empty() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    let root = crate::assets::app_root()?;
    Some(Resolved { real: root.join(p), writable: false })
}

/// 真实路径 → `wxfile://` 虚拟路径（readdir / getSavedFileList 回给页面用）
pub fn to_virtual(real: &Path) -> Option<String> {
    let rel = real.strip_prefix(crate::wxfile::root()).ok()?;
    Some(format!("{}{}", crate::wxfile::SCHEME, rel.to_string_lossy().replace('\\', "/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_table() {
        let u = resolve("wxfile://usr/a/b.txt").unwrap();
        assert!(u.writable);
        assert!(u.real.ends_with("usr/a/b.txt"));
        assert!(resolve(USR).unwrap().writable);
        assert!(!resolve("wxfile://tmp/x.png").unwrap().writable);
        assert!(!resolve("wxfile://store/x.png").unwrap().writable);
        assert!(resolve("wxfile://usr/../../etc").is_none());
        assert!(resolve("../secret").is_none());
        assert!(resolve("https://a.com/x").is_none());
        assert!(resolve("").is_none());
    }
}
