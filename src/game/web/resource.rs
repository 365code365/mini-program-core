//! 高性能模式页面里的 URL → 内容（宿主的自定义协议处理器 / 桌面的 HTTP 服务都只转给这里）。
//!
//! | 路径 | 内容 |
//! |---|---|
//! | `/__wg.html`（启动页，放在根上：页面里的相对路径正好按包根解析）`/__wg/boot.js` `/__wg/entry.js` `/__wg/m/<键>.js` | 启动页（boot.rs） |
//! | `/__wg/file/<路径>` | `wxfile://<路径>`（下载的临时文件、用户文件、存下的文件；沙盒外的不认） |
//! | `/__wg/proxy?u=<网址>` | 远程资源经原生取回（见下） |
//! | 其余 `/<路径>` | 包内文件（路径里不许有 `..`） |
//!
//! 为什么远程图片 / 音频 / 字体要经 `/__wg/proxy`：页面的源是包（`wxgame://app`），直接加载 `https://cdn/…` 是跨源的 ——
//! 没有 CORS 头的图片画进 WebGL / `getImageData` 会抛 SecurityError（微信高性能模式文档 QA 4 的 isTrusted / 跨域问题）。
//! 经原生取回之后同源，游戏不用改服务器配置。

use super::WebGame;
use std::path::{Component, Path};

pub struct Resource {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}

impl Resource {
    fn text(s: String, mime: &'static str) -> Self {
        Self { bytes: s.into_bytes(), mime }
    }
}

pub fn mime_of(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "txt" | "csv" => "text/plain; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "m4a" | "aac" => "audio/aac",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// 包内相对路径是否安全（不许 `..`、不许绝对路径）
fn safe_rel(p: &str) -> bool {
    !p.is_empty() && Path::new(p).components().all(|c| matches!(c, Component::Normal(_)))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl WebGame {
    /// `path` 是 URL 的路径部分（含查询串，不含协议与主机）。认不出 / 不存在返回 None（宿主回 404）
    pub fn resource(&self, path: &str) -> Option<Resource> {
        let (path, query) = path.split_once('?').unwrap_or((path, ""));
        let path = percent_decode(path);
        match path.as_str() {
            "/__wg.html" | "/" => return Some(Resource::text(super::boot::boot_html(), "text/html; charset=utf-8")),
            "/__wg/boot.js" => return Some(Resource::text(self.boot_js(), "application/javascript; charset=utf-8")),
            "/__wg/entry.js" => return Some(Resource::text(self.entry_js(), "application/javascript; charset=utf-8")),
            "/__wg/proxy" => return proxy(query),
            "/__wg/worker.js" => {
                let entry = query.split('&').find_map(|kv| kv.strip_prefix("entry=")).map(percent_decode)?;
                return self.worker_js(&entry).map(|s| Resource::text(s, "application/javascript; charset=utf-8"));
            }
            _ => {}
        }
        if let Some(name) = path.strip_prefix("/__wg/seg/").and_then(|k| k.strip_suffix(".js")) {
            return super::boot::segment_js(name).map(|s| Resource::text(s, "application/javascript; charset=utf-8"));
        }
        if let Some(key) = path.strip_prefix("/__wg/m/").and_then(|k| k.strip_suffix(".js")) {
            return self.module_js(key).map(|s| Resource::text(s, "application/javascript; charset=utf-8"));
        }
        if let Some(rest) = path.strip_prefix("/__wg/file/") {
            let real = crate::wxfile::to_path(&format!("{}{rest}", crate::wxfile::SCHEME))?;
            let bytes = std::fs::read(&real).ok()?;
            return Some(Resource { bytes, mime: mime_of(rest) });
        }
        let rel = path.trim_start_matches('/');
        if !safe_rel(rel) {
            return None;
        }
        let bytes = std::fs::read(self.root.join(rel)).ok()?;
        // `?br=1`：WXWebAssembly.instantiate('x.wasm.br') 要解压后的字节（官方支持 br 压缩的 wasm，wg-extra.js）
        if query.split('&').any(|kv| kv == "br=1") && rel.ends_with(".br") {
            let mut out = Vec::new();
            brotli_decompressor::BrotliDecompress(&mut bytes.as_slice(), &mut out).ok()?;
            return Some(Resource { bytes: out, mime: mime_of(rel.trim_end_matches(".br")) });
        }
        Some(Resource { bytes, mime: mime_of(rel) })
    }
}

/// `/__wg/proxy?u=…` 不需要实例（也不该在拿着实例的锁时阻塞在网络上）：宿主先问这个，是就不加锁取
pub fn unlocked(path: &str) -> Option<Option<Resource>> {
    let (p, q) = path.split_once('?').unwrap_or((path, ""));
    (p == "/__wg/proxy").then(|| proxy(q))
}

/// 远程资源（只认 http / https）。阻塞取：宿主在后台线程调（iOS 的 WKURLSchemeHandler 回调可以晚点回）
fn proxy(query: &str) -> Option<Resource> {
    let url = query.split('&').find_map(|kv| kv.strip_prefix("u=")).map(percent_decode)?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return None;
    }
    let config = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(30))).max_redirects(5).build();
    let mut resp = ureq::Agent::new_with_config(config).get(&url).call().ok()?;
    let bytes = resp.body_mut().with_config().limit(64 << 20).read_to_vec().ok()?;
    let mime = mime_of(url.split(['?', '#']).next().unwrap_or(""));
    Some(Resource { bytes, mime })
}

#[cfg(test)]
pub(super) fn decode_for_test(s: &str) -> String {
    percent_decode(s)
}
