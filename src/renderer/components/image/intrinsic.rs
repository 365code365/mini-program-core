//! `<image>` 的固有尺寸：`mode="widthFix"` / `"heightFix"` 排版要用（宽定高随、高定宽随）。
//!
//! 建树时就要知道，而图片多半还没画过（解码缓存里没有）：本地文件只读文件头拿宽高
//! （`image::image_dimensions`，不解码），记一张小表，整页重建时不再重复读盘；
//! 远程图只查缓存 —— 没下完返回 `None`，按默认高度先排，到了之后整页重排（engine_frame.rs 已经这么做）。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn headers() -> &'static Mutex<HashMap<String, Option<(u32, u32)>>> {
    static T: OnceLock<Mutex<HashMap<String, Option<(u32, u32)>>>> = OnceLock::new();
    T.get_or_init(Default::default)
}

/// 图片原始像素尺寸。不知道（远程图在下载、文件读不了）返回 `None`
pub fn intrinsic_size(src: &str) -> Option<(u32, u32)> {
    let src = src.trim();
    if src.is_empty() {
        return None;
    }
    if let super::super::ImageLoad::Ready { width, height } = super::super::probe_load(src) {
        return (width > 0 && height > 0).then_some((width, height));
    }
    if src.starts_with("http://") || src.starts_with("https://") || src.starts_with("data:") {
        return None;
    }
    if let Ok(t) = headers().lock() {
        if let Some(v) = t.get(src) {
            return *v;
        }
    }
    let v = crate::assets::resolve(src)
        .and_then(|p| image::image_dimensions(&p).ok())
        .filter(|(w, h)| *w > 0 && *h > 0);
    if let Ok(mut t) = headers().lock() {
        // 只记本地包内文件，量级是几十到几百条；防止异常页面无限增长
        if t.len() > 4096 {
            t.clear();
        }
        t.insert(src.to_string(), v);
    }
    v
}
