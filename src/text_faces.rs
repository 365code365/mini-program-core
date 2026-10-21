//! 动态字体：WXSS 的 `@font-face` 与 `wx.loadFontFace`（官方 api/ui/font/wx.loadFontFace.html）。
//!
//! 字族名 → 字体文件的登记表，`text_family` 解析 `font-family` 栈时先查它。字体来源：
//! - 包内路径 / `wxfile://`：按 `assets::resolve` 找到文件，当场登记；
//! - `https://`：后台线程下载到 `wxfile://tmp/…`，下完登记，并记一笔「字体变了」——
//!   宿主据 [`take_changed`] 让布局缓存作废重排（文字宽度变了，原来的排版不能再用）。
//!
//! 只认 TrueType / OpenType（`ttf` / `otf` / `ttc`）：Skyline 同样只支持 TTF，WOFF / WOFF2 明确 fail，
//! 不假装加载成功。`wx.loadFontFace` 的结果按调用 id 排队，`MiniApp::update` 每帧取走回调逻辑层。

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

fn faces() -> &'static Mutex<HashMap<String, &'static str>> {
    static F: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    F.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 等着回给逻辑层的 loadFontFace 结果：(调用 id, 字族, Ok / Err(原因))
fn done() -> &'static Mutex<Vec<(u64, String, Result<(), String>)>> {
    static D: OnceLock<Mutex<Vec<(u64, String, Result<(), String>)>>> = OnceLock::new();
    D.get_or_init(|| Mutex::new(Vec::new()))
}

static CHANGED: AtomicBool = AtomicBool::new(false);
static PENDING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn normalize(name: &str) -> String {
    name.trim().trim_matches('"').trim_matches('\'').trim().to_ascii_lowercase()
}

/// 登记过的字族对应的字体文件（`name` 已规范化：小写、去引号）
pub fn face_path(name: &str) -> Option<&'static str> {
    faces().lock().ok()?.get(name).copied()
}

/// 有字族新登记成功（读后清零）：宿主据此让布局缓存作废重排
pub fn take_changed() -> bool {
    CHANGED.swap(false, Ordering::Relaxed)
}

/// 还有字体在下载（宿主据此续帧，下完要有一帧去取结果）
pub fn has_pending() -> bool {
    PENDING.load(Ordering::Relaxed) > 0 || done().lock().map(|d| !d.is_empty()).unwrap_or(false)
}

/// 取走已经有结论的 loadFontFace（id 为 0 的是 `@font-face` 自己登记的，不回调）
pub fn take_completed() -> Vec<(u64, String, Result<(), String>)> {
    done().lock().map(|mut d| std::mem::take(&mut *d)).unwrap_or_default()
}

/// 文件头认格式：TrueType / OpenType / TTC 之外一律拒绝
fn check_format(bytes: &[u8]) -> Result<(), String> {
    match bytes.get(..4) {
        Some([0x00, 0x01, 0x00, 0x00]) | Some(b"true") | Some(b"OTTO") | Some(b"ttcf") => Ok(()),
        Some(b"wOFF") | Some(b"wOF2") => Err("unsupported font format: woff / woff2（只支持 TTF / OTF）".into()),
        _ => Err("unsupported font format".into()),
    }
}

/// 把一个本地字体文件登记成 `family`
pub fn register_file(family: &str, path: &Path) -> Result<(), String> {
    let mut head = [0u8; 4];
    {
        use std::io::Read;
        let mut f = std::fs::File::open(path).map_err(|e| format!("{e}: {}", path.display()))?;
        f.read_exact(&mut head).map_err(|e| e.to_string())?;
    }
    check_format(&head)?;
    let leaked: &'static str = Box::leak(path.to_string_lossy().into_owned().into_boxed_str());
    faces().lock().map_err(|e| e.to_string())?.insert(normalize(family), leaked);
    crate::text_family::forget_resolved_stacks();
    CHANGED.store(true, Ordering::Relaxed);
    if std::env::var_os("MINI_FONT_LOG").is_some() {
        eprintln!("🔤 登记字族 `{family}` -> {leaked}");
    }
    Ok(())
}

/// 登记一个字族的来源（`url(...)` 里的那串）。本地的当场登记、结果直接进回调队列；
/// 远程的后台下载，下完再进队列。`id` 为 0 表示不需要回调（`@font-face`）
pub fn load(id: u64, family: &str, src: &str) {
    let family = family.trim().to_string();
    if family.is_empty() || src.trim().is_empty() {
        push(id, family, Err("parameter error: family / source is required".into()));
        return;
    }
    let src = src.trim().to_string();
    if src.starts_with("http://") || src.starts_with("https://") {
        PENDING.fetch_add(1, Ordering::Relaxed);
        std::thread::spawn(move || {
            let r = download(&src).and_then(|p| register_file(&family, &p));
            push(id, family, r);
            PENDING.fetch_sub(1, Ordering::Relaxed);
        });
        return;
    }
    let r = match crate::assets::resolve(&src) {
        Some(p) => register_file(&family, &p),
        None => Err(format!("file not found: {src}")),
    };
    push(id, family, r);
}

fn push(id: u64, family: String, r: Result<(), String>) {
    if let Err(e) = &r {
        eprintln!("⚠️ 字体 `{family}` 加载失败：{e}");
    }
    if let Ok(mut d) = done().lock() {
        d.push((id, family, r));
    }
}

fn download(url: &str) -> Result<std::path::PathBuf, String> {
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(60))).build(),
    );
    let mut resp = agent.get(url).call().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let bytes = resp.body_mut().with_config().limit(64 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())?;
    check_format(&bytes)?;
    let ext = crate::wxfile::ext_of_url(url);
    let (_, path) = crate::wxfile::new_temp(if ext.is_empty() { "ttf" } else { &ext });
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_check_accepts_truetype_rejects_woff() {
        assert!(check_format(&[0, 1, 0, 0]).is_ok());
        assert!(check_format(b"OTTO").is_ok());
        assert!(check_format(b"wOF2").unwrap_err().contains("woff"));
        assert!(check_format(b"<htm").is_err());
    }
}
