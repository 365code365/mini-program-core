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
//! 缓存分两层，**都是必须的**：
//! - 按「规范化后的字体栈字符串」缓存解析结果，省掉重复的字族匹配；
//! - 按**解析出来的字体文件路径**缓存 `TextRenderer`，因为不同的字体栈经常落到
//!   同一个文件上。tea-app 就有三条不同的栈（`"Source Han Serif SC",…`、
//!   `"Songti SC",…`、`"Yunxiu Serif",…`）全部解析到 `Songti.ttc` —— 那是个
//!   **63.8MB** 的 ttc，解析一次要 1.5~2.4s。只有栈级缓存时它会被解析三遍，
//!   而且全都发生在**第一帧的建树阶段**：启动页实测「建树+样式 3093ms」，
//!   首屏三秒不出帧，页面里靠 `bindload` 驱动的倒计时看起来像根本没启动。
//!
//! 缺字形时按 CJK 回退链继续找（Times New Roman 没有汉字，浏览器也是逐字回退的）。

use crate::text::TextRenderer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

fn registry() -> &'static Mutex<HashMap<String, Option<Arc<TextRenderer>>>> {
    static REG: OnceLock<Mutex<HashMap<String, Option<Arc<TextRenderer>>>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 字体**文件**级缓存：路径 → (渲染器, 最近使用序号)。
/// 多条字体栈落到同一个文件时共享一份。
///
/// **必须封顶**：一个解析好的 CJK 字面在 fontdue 里要 150~300MB
/// （见 `doc/引擎测试说明.md` 的内存分项），所以「声明了几种字体就常驻几百 MB」
/// 是站不住的。这里只保留最近用到的若干个文件，超了按最久未用逐出。
/// 逐出只是从表里去掉，正在用它的节点手上还有 `Arc`，那一帧照常画完。
type PathCache = HashMap<&'static str, (Option<Arc<TextRenderer>>, u64)>;

fn by_path() -> &'static Mutex<PathCache> {
    static REG: OnceLock<Mutex<PathCache>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 同时保留多少个**自定义**字体文件（系统主字体是另一份进程级单例，不在此列）。
/// 可用 `MINI_FONT_CACHE_MAX` 覆盖。
fn max_cached_fonts() -> usize {
    static MAX: OnceLock<usize> = OnceLock::new();
    *MAX.get_or_init(|| {
        std::env::var("MINI_FONT_CACHE_MAX")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(2)
    })
}

fn next_tick() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TICK: AtomicU64 = AtomicU64::new(0);
    TICK.fetch_add(1, Ordering::Relaxed)
}

/// 加载一个字体文件（同一个文件只解析一次）。`None` = 这个文件加载失败。
fn renderer_for_path(path: &'static str) -> Option<Arc<TextRenderer>> {
    if let Ok(mut cache) = by_path().lock() {
        if let Some(hit) = cache.get_mut(path) {
            hit.1 = next_tick();
            return hit.0.clone();
        }
    }
    let loaded = match TextRenderer::from_file_with_fallback(path) {
        Ok(r) => Some(Arc::new(r)),
        Err(e) => {
            if std::env::var_os("MINI_FONT_LOG").is_some() {
                eprintln!("🔤 字体 {path} 加载失败: {e}");
            }
            None
        }
    };
    if let Ok(mut cache) = by_path().lock() {
        cache.insert(path, (loaded.clone(), next_tick()));
        evict_beyond_cap(&mut cache);
    }
    loaded
}

/// 只统计**真的解析成功**的条目：加载失败的记录几乎不占内存，
/// 留着还能避免反复去读一个不存在/坏掉的文件。
fn evict_beyond_cap(cache: &mut PathCache) {
    let cap = max_cached_fonts();
    loop {
        let loaded: Vec<(&'static str, u64)> = cache
            .iter()
            .filter(|(_, (r, _))| r.is_some())
            .map(|(p, (_, t))| (*p, *t))
            .collect();
        if loaded.len() <= cap {
            return;
        }
        let Some((oldest, _)) = loaded.iter().min_by_key(|(_, t)| *t).copied() else { return };
        cache.remove(oldest);
        if std::env::var_os("MINI_FONT_LOG").is_some() {
            eprintln!("🔤 自定义字体缓存超过 {cap} 个，逐出 {oldest}");
        }
    }
}

/// 已加载的自定义字体文件（诊断用）
pub fn loaded_font_files() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = by_path()
        .lock()
        .map(|c| {
            c.iter()
                .filter(|(_, (r, _))| r.is_some())
                .map(|(p, _)| *p)
                .collect()
        })
        .unwrap_or_default();
    v.sort_unstable();
    v
}

/// 释放自定义字体缓存（宿主收到系统内存告警、或切换小程序时调用）。
///
/// 不动系统主字体：它是所有文字的兜底，放掉下一帧就要重新解析 300MB。
pub fn clear_caches() {
    if let Ok(mut c) = by_path().lock() {
        c.clear();
    }
    if let Ok(mut r) = registry().lock() {
        r.clear();
    }
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
            if let Some(r) = renderer_for_path(path) {
                if std::env::var_os("MINI_FONT_LOG").is_some() {
                    eprintln!("🔤 font-family `{}` -> {}", item.trim(), path);
                }
                return Some(r);
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
