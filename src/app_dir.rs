//! 示例小程序目录的定位。
//!
//! 所有示例小程序都收在仓库的 `sample/` 下（`sample/sample-app`、`sample/news-app`、
//! `sample/tea-app` …）。但命令行、脚本、文档里到处都写着**裸名字**（`sample-app`），
//! 所以这里统一做一次解析：既接受裸名字，也接受 `sample/xxx`，也接受任意绝对/相对路径。
//!
//! 把这件事收在一个地方，是为了避免"迁个目录要改十几处硬编码字符串"再次发生。

use std::path::{Path, PathBuf};

/// 示例小程序的收纳目录名
pub const SAMPLE_DIR: &str = "sample";

/// 仓库根目录（编译期写入，供开发期直接跑二进制时定位 `sample/`）
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 判断一个目录是不是小程序（含 `app.json`）
pub fn is_mini_app(dir: &Path) -> bool {
    dir.join("app.json").is_file()
}

/// 把「小程序名或路径」解析成真实目录。
///
/// 依次尝试：原样路径 → `sample/<名字>` → `<仓库根>/sample/<名字>` → `<仓库根>/<名字>`。
/// 全都不是小程序目录时，返回原样路径（让调用方按自己的方式报错）。
pub fn resolve(name_or_path: &str) -> PathBuf {
    let raw = PathBuf::from(name_or_path);
    let candidates = [
        raw.clone(),
        PathBuf::from(SAMPLE_DIR).join(name_or_path),
        repo_root().join(SAMPLE_DIR).join(name_or_path),
        repo_root().join(name_or_path),
    ];
    for c in candidates {
        if is_mini_app(&c) {
            return c;
        }
    }
    raw
}

/// 默认小程序（没给参数时用）
pub fn default_app() -> PathBuf {
    resolve("sample-app")
}

/// 列出 `sample/` 下所有小程序的名字（按字母序）。
///
/// 只看一层：`sample/<name>/app.json`。因此 `sample/_archive/...` 这类归档目录
/// 不会被当成可运行的示例。
pub fn list_apps() -> Vec<String> {
    let mut out = Vec::new();
    for base in [PathBuf::from(SAMPLE_DIR), repo_root().join(SAMPLE_DIR)] {
        let Ok(entries) = std::fs::read_dir(&base) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() && is_mini_app(&p) {
                if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                    if !out.iter().any(|x| x == n) {
                        out.push(n.to_string());
                    }
                }
            }
        }
        if !out.is_empty() {
            break;
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_bare_name_into_sample_dir() {
        let p = resolve("sample-app");
        assert!(is_mini_app(&p), "裸名字应能解析到 sample/sample-app：{:?}", p);
        assert!(p.ends_with("sample-app"));
    }

    #[test]
    fn resolves_prefixed_path() {
        let p = resolve("sample/news-app");
        assert!(is_mini_app(&p), "带 sample/ 前缀的路径也要能用：{:?}", p);
    }

    #[test]
    fn unknown_name_returns_input_untouched() {
        let p = resolve("definitely-not-an-app");
        assert_eq!(p, PathBuf::from("definitely-not-an-app"));
    }

    #[test]
    fn lists_bundled_apps() {
        let apps = list_apps();
        assert!(apps.contains(&"sample-app".to_string()), "应列出 sample-app：{:?}", apps);
        assert!(apps.contains(&"news-app".to_string()), "应列出 news-app：{:?}", apps);
        // 归档目录不含 app.json，不该被当成示例
        assert!(!apps.contains(&"_archive".to_string()));
    }
}
