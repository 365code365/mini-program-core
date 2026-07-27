//! `font-family` 的解析：CSS 字体栈 → 具体字体文件 → 专属 `TextRenderer`。
//!
//! 之前 `font-family` 是**整条被忽略**的：全局只有一个系统主字体（macOS 上是
//! 黑体系的 PingFang / Hiragino Sans GB），于是所有声明宋体/衬线的文字都画成黑体。
//! 对以「书卷感」为设计语言的应用（tea-app 的品牌名、导航文字、正文都指定
//! `"Songti SC","STSong",serif`）来说，这是肉眼第一眼就能看出来的差别。
//!
//! 解析规则与浏览器一致：**按字体栈从左到右取第一个「本机存在」的字族**，
//! 都不存在时用通用族（serif / sans-serif / monospace）兜底，
//! 通用族也认不出来就回退到系统默认字体（即保持旧行为）。
//!
//! 每个字族一份 `TextRenderer`（自带字形缓存），按「规范化后的字体栈字符串」缓存，
//! 所以同一条 `font-family` 只解析、只加载一次。缺字形时按 CJK 回退链继续找
//! （Times New Roman 没有汉字，浏览器也是逐字回退的）。

use crate::text::TextRenderer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

fn registry() -> &'static Mutex<HashMap<String, Option<Arc<TextRenderer>>>> {
    static REG: OnceLock<Mutex<HashMap<String, Option<Arc<TextRenderer>>>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 按 CSS `font-family` 取渲染器。返回 `None` 表示「用默认系统字体」。
pub fn renderer_for_family(list: Option<&str>) -> Option<Arc<TextRenderer>> {
    let list = list?.trim();
    if list.is_empty() {
        return None;
    }
    let key = list.to_ascii_lowercase();
    if let Ok(reg) = registry().lock() {
        if let Some(hit) = reg.get(&key) {
            return hit.clone();
        }
    }
    let loaded = load_family_stack(list);
    if let Ok(mut reg) = registry().lock() {
        reg.insert(key, loaded.clone());
    }
    loaded
}

/// 字体栈里的某一项能否在本机找到字体文件（供测试/诊断用）
pub fn family_font_path(name: &str) -> Option<&'static str> {
    candidates_for(&normalize(name))
        .iter()
        .copied()
        .find(|p| std::path::Path::new(p).exists())
}

fn normalize(name: &str) -> String {
    name.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_ascii_lowercase()
}

fn load_family_stack(list: &str) -> Option<Arc<TextRenderer>> {
    for item in list.split(',') {
        let name = normalize(item);
        if name.is_empty() {
            continue;
        }
        // 系统 UI 字体就是我们的默认主字体，没必要另开一份
        if matches!(
            name.as_str(),
            "sans-serif" | "system-ui" | "-apple-system" | "blinkmacsystemfont" | "helvetica neue"
        ) {
            return None;
        }
        for path in candidates_for(&name) {
            if !std::path::Path::new(path).exists() {
                continue;
            }
            match TextRenderer::from_file_with_fallback(path) {
                Ok(r) => {
                    if std::env::var_os("MINI_FONT_LOG").is_some() {
                        eprintln!("🔤 font-family `{}` -> {}", item.trim(), path);
                    }
                    return Some(Arc::new(r));
                }
                Err(e) => {
                    if std::env::var_os("MINI_FONT_LOG").is_some() {
                        eprintln!("🔤 字体 {path} 加载失败: {e}");
                    }
                }
            }
        }
    }
    None
}

/// 字族名 → 候选字体文件（按优先级）。认不出的名字返回空表，交给栈里的下一项。
///
/// 只列 macOS 自带的字体：小程序没有 `@font-face` 的本地字体，
/// 远程字体（`wx.loadFontFace`）本引擎不支持，所以「本机有没有」就是唯一判据。
fn candidates_for(name: &str) -> &'static [&'static str] {
    const SONGTI: &[&str] = &[
        "/System/Library/Fonts/Supplemental/Songti.ttc",
        "/Library/Fonts/Songti.ttc",
        "/System/Library/Fonts/STSong.ttf",
    ];
    const TIMES: &[&str] = &[
        "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
        "/System/Library/Fonts/Times.ttc",
        "/Library/Fonts/Times New Roman.ttf",
    ];
    const GEORGIA: &[&str] = &["/System/Library/Fonts/Supplemental/Georgia.ttf"];
    const MENLO: &[&str] = &["/System/Library/Fonts/Menlo.ttc"];
    const COURIER: &[&str] = &[
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/System/Library/Fonts/Courier.ttc",
    ];
    const PINGFANG: &[&str] = &["/System/Library/Fonts/PingFang.ttc"];
    const HIRAGINO: &[&str] = &["/System/Library/Fonts/Hiragino Sans GB.ttc"];
    const HEITI: &[&str] = &["/System/Library/Fonts/STHeiti Light.ttc"];
    const ARIAL: &[&str] = &[
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
    ];
    const HELVETICA: &[&str] = &["/System/Library/Fonts/Helvetica.ttc"];
    const KAITI: &[&str] = &["/System/Library/Fonts/Supplemental/Kaiti.ttc"];
    const NONE: &[&str] = &[];

    match name {
        // ── 中文衬线（宋体系）：设计稿里的「书卷感」几乎都落在这一族 ──
        "songti sc" | "songti tc" | "songti" | "stsong" | "simsun" | "宋体" | "nsimsun"
        | "noto serif sc" | "noto serif cjk sc" | "source han serif sc" | "source han serif cn"
        | "serif" => SONGTI,
        // 楷体
        "kaiti sc" | "kaiti" | "stkaiti" | "楷体" => KAITI,
        // ── 西文衬线 ──
        "times new roman" | "times" => TIMES,
        "georgia" => GEORGIA,
        // ── 等宽 ──
        "monospace" | "menlo" | "monaco" | "sf mono" | "sfmono-regular" | "consolas" => MENLO,
        "courier new" | "courier" => COURIER,
        // ── 具体的黑体系（与默认主字体同族，但显式指定时也按它加载）──
        "pingfang sc" | "pingfang tc" | "pingfang" => PINGFANG,
        "hiragino sans gb" | "hiragino sans" => HIRAGINO,
        "heiti sc" | "stheiti" | "黑体" => HEITI,
        "arial" | "helvetica" => ARIAL_OR_HELVETICA,
        _ => NONE,
    }
}

/// Arial 缺失时退 Helvetica（两者度量接近，浏览器上的字体栈也常并列写）
const ARIAL_OR_HELVETICA: &[&str] = &[
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/Library/Fonts/Arial.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_sans_uses_default_renderer() {
        assert!(renderer_for_family(Some("sans-serif")).is_none());
        assert!(renderer_for_family(Some("-apple-system, system-ui")).is_none());
        assert!(renderer_for_family(None).is_none());
        assert!(renderer_for_family(Some("   ")).is_none());
    }

    /// 字体栈要跳过本机没有的字族，取到第一个存在的（浏览器行为）。
    /// `Playfair Display` 在 macOS 上不存在，应落到 `Times New Roman`。
    #[test]
    fn stack_skips_missing_families() {
        if family_font_path("Times New Roman").is_none() {
            return; // 非 macOS / 精简系统，跳过
        }
        assert_eq!(family_font_path("Playfair Display"), None);
        let r = renderer_for_family(Some("\"Playfair Display\", \"Times New Roman\", serif"));
        assert!(r.is_some(), "应解析到 Times New Roman");
    }

    /// 宋体族要真的解析到宋体文件，且同一条字体栈只加载一次（缓存命中同一个 Arc）
    #[test]
    fn songti_stack_resolves_and_caches() {
        if family_font_path("Songti SC").is_none() {
            return;
        }
        let a = renderer_for_family(Some("\"Songti SC\", \"STSong\", serif"));
        let b = renderer_for_family(Some("\"Songti SC\", \"STSong\", serif"));
        let (a, b) = (a.expect("宋体应可解析"), b.expect("宋体应可解析"));
        assert!(Arc::ptr_eq(&a, &b), "同一条 font-family 应命中缓存");
    }

    /// 宋体（衬线）与默认黑体的字形宽度不同 —— 说明确实换了字体，而不是名义上换了。
    #[test]
    fn serif_metrics_differ_from_default() {
        let (Some(def), Some(serif)) = (
            crate::text::shared_fonts(),
            renderer_for_family(Some("\"Songti SC\", serif")),
        ) else {
            return;
        };
        let w_def = def.measure_text_weighted("凤凰云岫", 40.0, 0.0, false);
        let w_serif = serif.measure_text_weighted("凤凰云岫", 40.0, 0.0, false);
        // 宽度可能接近（CJK 等宽），但字形位图必须不同
        let ink = |r: &TextRenderer| -> u32 {
            let mut c = crate::Canvas::new(64, 64);
            c.clear(crate::Color::WHITE);
            let paint = crate::Paint::new().with_color(crate::Color::BLACK);
            r.draw_text_weighted(&mut c, "岫", 4.0, 48.0, 40.0, 0.0, false, &paint);
            c.pixels().iter().map(|p| 255 - p.r as u32).sum()
        };
        assert!(w_def > 0.0 && w_serif > 0.0);
        assert_ne!(ink(&def), ink(&serif), "宋体与黑体的字形应当不同");
    }
}
