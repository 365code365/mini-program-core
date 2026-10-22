//! 字体加载：系统默认字体（候选见 `text_system`）、按 `font-family` 加载、同集合里探测粗体字面、符号回退字体。
//!
//! 从 `text.rs` 按职责拆出（**纯搬迁**，原本私有的几个函数改成 `pub(super)` 供父模块调用，其余一字未改）：
//! 那个文件到了 667 行，加载与绘制 / 度量混在一起。
use super::{shared_fonts, TextRenderer};
use fontdue::{Font, FontSettings};
use std::path::Path;

impl TextRenderer {
    /// 从同一份字体数据里挑选粗体字面（TTC 字体集合常把 W3/W6 放在不同 face）。
    ///
    /// 判定方式：光栅化同一个汉字，比较墨水覆盖率，取显著更黑且步进宽度一致的字面。
    /// 找不到就返回 None，绘制端回退到 faux-bold。
    pub(super) fn detect_bold_face(font_data: &[u8], regular: &Font) -> Option<Font> {
        const PROBE: char = '国';
        const PROBE_SIZE: f32 = 32.0;
        let ink_ratio = |font: &Font| -> f32 {
            let (metrics, bitmap) = font.rasterize(PROBE, PROBE_SIZE);
            let area = (metrics.width * metrics.height).max(1) as u32;
            let ink: u32 = bitmap.iter().map(|v| *v as u32).sum();
            ink as f32 / (area * 255) as f32
        };
        let regular_ratio = ink_ratio(regular);
        let regular_advance = regular.metrics(PROBE, PROBE_SIZE).advance_width;
        let log = std::env::var_os("MINI_FONT_LOG").is_some();
        // 连续几个字面都比常规更细：字体集合是按粗细排的，再往后只会更细。
        // 这个早停不是可选的优化 —— macOS 的 `Songti.ttc` 有 63.8MB、5 个字面，
        // 而且第 0 个就是最黑的那个，探完全部只会得到 `None`：白解析 5 遍 ≈ 1.37s，
        // 全都发生在**第一帧的建树阶段**（启动页首帧因此要 3 秒才出来）。
        let mut lighter_in_a_row = 0;
        for index in 1..6u32 {
            let settings = FontSettings { scale: 40.0, collection_index: index, ..Default::default() };
            let Ok(candidate) = Font::from_bytes(font_data, settings) else { break };
            if candidate.lookup_glyph_index(PROBE) == 0 {
                continue;
            }
            let advance = candidate.metrics(PROBE, PROBE_SIZE).advance_width;
            let ratio = ink_ratio(&candidate);
            if log {
                eprintln!(
                    "🔤   字面 #{index}：字宽 {advance:.2}（常规 {regular_advance:.2}）\
                     墨占比 {ratio:.3}（常规 {regular_ratio:.3}）"
                );
            }
            // 粗体字面应与常规字面等宽（同一字族的等宽 CJK），否则可能是压缩/斜体字面
            if (advance - regular_advance).abs() > 0.6 {
                continue;
            }
            // 命中第一个「明显更黑且等宽」的字面即返回：继续扫描只会把同一个
            // 字体集合反复解析，白白拖慢启动
            if ratio > regular_ratio * 1.25 {
                return Some(candidate);
            }
            if ratio < regular_ratio {
                lighter_in_a_row += 1;
                if lighter_in_a_row >= 2 {
                    if log {
                        eprintln!("🔤   连续 2 个字面都比常规更细，认定本集合无更粗字面，停止探测");
                    }
                    break;
                }
            } else {
                lighter_in_a_row = 0;
            }
        }
        None
    }

    /// 从文件路径加载字体
    pub fn from_file(path: &str) -> Result<Self, String> {
        let font_data = std::fs::read(path)
            .map_err(|e| format!("Failed to read font file: {}", e))?;
        Self::from_bytes(&font_data)
    }

    /// 为某字符选择字体：主字体缺字形时依次回退 Emoji 字体、符号字体。
    ///
    /// 修复：中文字体（如 Hiragino Sans GB）缺少 ✕ ✓ ★ 等符号字形，直接用主字体会
    /// 画成豆腐块（□）。回退到系统符号字体后与浏览器表现一致。
    /// 惰性加载符号字体：补齐中文字体缺失的 ✕ ✓ ★ 等字形。
    ///
    /// 按覆盖度排序（实测 Apple Symbols 反而缺 U+2715/U+2713，故不作首选）；
    /// 系统都不可用时回退到随包字体，保证跨平台一致。
    pub(super) fn symbol_font(&self) -> Option<&Font> {
        self.symbol_font
            .get_or_init(|| {
                // Android：主字体换成系统中文字体后（见 text_system），✔ ✖ ① 这类符号要靠这里补；
                // 不给系统的符号字体就会落到内置 ArialUnicode —— 第一次碰到符号时现场解析 23MB、
                // 约 5 万个字形的几何，手机上是一次秒级的卡顿外加几百 MB 内存
                let candidates = [
                    "/system/fonts/NotoSansSymbols-Regular-Subsetted.ttf",
                    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                    "/Library/Fonts/Arial Unicode.ttf",
                    "/System/Library/Fonts/Menlo.ttc",
                    "/System/Library/Fonts/Apple Symbols.ttf",
                ];
                let settings = FontSettings { scale: 40.0, ..Default::default() };
                for path in candidates {
                    if !Path::new(path).exists() {
                        continue;
                    }
                    if let Ok(data) = std::fs::read(path) {
                        if let Ok(font) = Font::from_bytes(data.as_slice(), settings.clone()) {
                            return Some(font);
                        }
                    }
                }
                Font::from_bytes(
                    include_bytes!("../../assets/ArialUnicode.ttf").as_slice(),
                    settings,
                )
                .ok()
            })
            .as_ref()
    }

    /// 按 `font-family` 加载具体字体：同集合里找粗体字面，缺字形时回退系统默认字体。
    ///
    /// 与 `from_file` 的区别就是这两条回退 —— 直接用 `from_file` 的话，
    /// 宋体没有的符号、西文字体没有的汉字都会画成豆腐块。
    pub fn from_file_with_fallback(path: &str) -> Result<Self, String> {
        let log = std::env::var_os("MINI_FONT_LOG").is_some();
        let t0 = std::time::Instant::now();
        let data = std::fs::read(path).map_err(|e| format!("读取字体 {path} 失败: {e}"))?;
        let t_read = t0.elapsed();
        let mut r = Self::from_bytes(&data)?;
        let t_parse = t0.elapsed();
        // 粗体字面留到第一次真的要画粗体时再探测（见 `bold_font` 字段注释）
        r.bold_src = Some(path.to_string());
        if log {
            eprintln!(
                "🔤 加载 {path}（{:.1}MB）：读盘 {:.0}ms 解析常规字面 {:.0}ms（粗体字面懒加载）",
                data.len() as f32 / 1048576.0,
                t_read.as_secs_f32() * 1000.0,
                (t_parse - t_read).as_secs_f32() * 1000.0,
            );
        }
        // 只在该字体缺汉字时才挂回退（宋体自带汉字，不必多占一份）
        if !r.main_has_glyph('国') {
            r.fallback = shared_fonts();
        }
        Ok(r)
    }

    /// 加载宿主系统的默认中文字体（候选按平台见 `text_system`：macOS / Android `fonts.xml` / iOS）
    pub fn load_system_font() -> Result<Self, String> {
        // Emoji 字体路径
        let emoji_font_paths = [
            "/System/Library/Fonts/Apple Color Emoji.ttc",
            "/System/Library/Fonts/AppleColorEmoji.ttf",
        ];

        // 加载主字体，并在同一字体集合里寻找真正的粗体字面
        let mut renderer: Option<TextRenderer> = None;
        for c in crate::text_system::main_font_candidates() {
            // 断掉的符号链接 `exists()` 也是 false（ColorOS 的 SysFont-*.ttf 就是这样）
            if !Path::new(&c.path).exists() {
                continue;
            }
            let Ok(data) = std::fs::read(&c.path) else { continue };
            match Self::from_bytes_index(&data, c.index) {
                Ok(mut r) => {
                    if c.need_cjk && !r.main_has_glyph('国') {
                        continue;
                    }
                    println!("✅ Main font: {}", c.path);
                    // 粗体字面懒加载：多数页面首屏并不需要它，而它要多占一份完整字体
                    r.bold_src = Some(c.path);
                    renderer = Some(r);
                    break;
                }
                Err(_) => continue,
            }
        }

        let mut renderer = renderer.ok_or("No main font found")?;
        
        // Emoji 字体：默认不加载。
        //
        // Apple Color Emoji 是 183MB 的彩色位图字体（sbix），fontdue 只支持轮廓字形，
        // 无法光栅化它的位图 —— 加载后既画不出 emoji，又让每次初始化多花约 1.5 秒。
        // 需要时可设 MINI_LOAD_EMOJI_FONT=1 显式开启（例如换用支持位图的光栅化后端后）。
        if std::env::var_os("MINI_LOAD_EMOJI_FONT").is_some() {
            for path in &emoji_font_paths {
                if Path::new(path).exists() {
                    if let Ok(data) = std::fs::read(path) {
                        let settings = FontSettings {
                            scale: 40.0,
                            ..Default::default()
                        };
                        if let Ok(font) = Font::from_bytes(data.as_slice(), settings) {
                            println!("✅ Emoji font: {}", path);
                            renderer.emoji_font = Some(font);
                            break;
                        }
                    }
                }
            }
        }

        Ok(renderer)
    }
}
