//! 文本渲染模块 - 支持系统字体、Emoji 和高清渲染

use crate::{Canvas, Color, Paint};
use fontdue::{Font, FontSettings, Metrics};
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// CSS `line-height: normal` 的等效系数（含中日韩字符的行）。
///
/// 浏览器的行盒高度取「该行实际用到的字体」的度量。在 `-apple-system, PingFang SC,
/// Hiragino Sans GB` 这类字体栈下实测（Chrome，headless，DSF=1）：
/// 含 CJK 的行稳定为 1.375 倍（16px→22px、24px→33px）。
pub const CJK_LINE_HEIGHT_FACTOR: f32 = 1.375;

/// CSS `line-height: normal` 的等效系数（纯西文/数字的行）。
///
/// 同一字体栈下这类行由系统 UI 字体（SF）供字，实测 24px→28px、16px→18px、12px→15px，
/// 与 SF 的 `new_line_size` 比例 1.1777 吻合。
pub const LATIN_LINE_HEIGHT_FACTOR: f32 = 1.1777;

/// 兼容旧调用点的默认系数（按含 CJK 处理，与小程序界面以中文为主的现实一致）。
pub const NORMAL_LINE_HEIGHT_FACTOR: f32 = CJK_LINE_HEIGHT_FACTOR;

/// 该字符是否会让行盒采用 CJK 字体度量（中日韩及全角标点）。
pub fn is_cjk_char(ch: char) -> bool {
    matches!(ch as u32,
        0x1100..=0x11FF |   // 谚文字母
        0x2E80..=0x2EFF |   // CJK 部首补充
        0x3000..=0x303F |   // CJK 符号和标点（含全角空格、。、）
        0x3040..=0x30FF |   // 平假名/片假名
        0x3130..=0x318F |   // 谚文兼容字母
        0x3400..=0x4DBF |   // CJK 扩展 A
        0x4E00..=0x9FFF |   // CJK 基本区
        0xA960..=0xA97F |
        0xAC00..=0xD7FF |   // 谚文音节
        0xF900..=0xFAFF |   // CJK 兼容表意
        0xFE30..=0xFE4F |   // CJK 兼容form
        0xFF00..=0xFFEF |   // 全角/半角形式
        0x20000..=0x2FA1F   // CJK 扩展 B+
    )
}

/// 文本是否包含使行盒采用 CJK 度量的字符。
pub fn text_has_cjk(text: &str) -> bool {
    text.chars().any(is_cjk_char)
}

/// 进程内共享的字体渲染器。
///
/// 加载系统字体要读取整套 TTC（中文字体 + Apple Color Emoji 合计上百 MB），实测单次
/// 约 1.3s。此前每个 `WxmlRenderer`、每个度量用的 Lazy 都各自加载一份，切换页面时会
/// 创建多个渲染器，于是每次切页都要花数秒——这是交互卡顿的主因。
/// 字体是只读的（内部字形缓存自带 Mutex），因此全进程共享一份即可。
static SHARED_FONTS: OnceLock<Option<Arc<TextRenderer>>> = OnceLock::new();

/// 取进程内共享的字体渲染器（首次调用时加载，之后为原子指针克隆）。
pub fn shared_fonts() -> Option<Arc<TextRenderer>> {
    SHARED_FONTS
        .get_or_init(|| {
            TextRenderer::load_system_font()
                .or_else(|_| {
                    TextRenderer::from_bytes(include_bytes!("../assets/ArialUnicode.ttf"))
                })
                .ok()
                .map(Arc::new)
        })
        .clone()
}

/// 文本渲染器 - 支持多字体回退（中文 + Emoji）
pub struct TextRenderer {
    /// 主字体（中文/英文）
    main_font: Font,
    /// 主字体的粗体字面（若字体集合内提供），用于替代描边式 faux-bold
    bold_font: Option<Font>,
    /// Emoji 字体
    emoji_font: Option<Font>,
    /// 符号字体（✕ ✓ ★ 等主字体缺失的字形）。
    /// 懒加载：多数页面不含这类符号，启动时不必解析 20MB+ 字体。
    symbol_font: OnceLock<Option<Font>>,
    /// 指定 `font-family` 时的兜底渲染器（系统默认字体）。
    /// 西文字体没有汉字、宋体没有某些符号，逐字回退到它，行为与浏览器一致。
    fallback: Option<Arc<TextRenderer>>,
    /// 简单的字形缓存 (char, size_u32, bold) -> (Metrics, Bitmap)
    /// 使用 Mutex 实现内部可变性，因为 draw 方法是 &self
    cache: Arc<Mutex<HashMap<(char, u32, bool), (Metrics, Vec<u8>)>>>,
}

impl TextRenderer {
    /// 从字体数据创建
    pub fn from_bytes(font_data: &[u8]) -> Result<Self, String> {
        let settings = FontSettings {
            scale: 40.0,
            ..Default::default()
        };
        let font = Font::from_bytes(font_data, settings)
            .map_err(|e| e.to_string())?;
        Ok(Self { 
            main_font: font,
            bold_font: None,
            emoji_font: None,
            symbol_font: OnceLock::new(),
            fallback: None,
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// 从同一份字体数据里挑选粗体字面（TTC 字体集合常把 W3/W6 放在不同 face）。
    ///
    /// 判定方式：光栅化同一个汉字，比较墨水覆盖率，取显著更黑且步进宽度一致的字面。
    /// 找不到就返回 None，绘制端回退到 faux-bold。
    fn detect_bold_face(font_data: &[u8], regular: &Font) -> Option<Font> {
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
    
    /// 判断主字体是否有该字符的字形（无字形时 fontdue 返回索引 0）
    fn main_has_glyph(&self, ch: char) -> bool {
        self.main_font.lookup_glyph_index(ch) != 0
    }

    /// 为某字符选择字体：主字体缺字形时依次回退 Emoji 字体、符号字体。
    ///
    /// 修复：中文字体（如 Hiragino Sans GB）缺少 ✕ ✓ ★ 等符号字形，直接用主字体会
    /// 画成豆腐块（□）。回退到系统符号字体后与浏览器表现一致。
    /// 惰性加载符号字体：补齐中文字体缺失的 ✕ ✓ ★ 等字形。
    ///
    /// 按覆盖度排序（实测 Apple Symbols 反而缺 U+2715/U+2713，故不作首选）；
    /// 系统都不可用时回退到随包字体，保证跨平台一致。
    fn symbol_font(&self) -> Option<&Font> {
        self.symbol_font
            .get_or_init(|| {
                let candidates = [
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
                    include_bytes!("../assets/ArialUnicode.ttf").as_slice(),
                    settings,
                )
                .ok()
            })
            .as_ref()
    }

    /// 是否有真实粗体字面可用（无则调用方回退 faux-bold）。
    pub fn has_bold_face(&self) -> bool {
        self.bold_font.is_some()
    }

    /// 按字重选择字体：粗体优先用真实粗体字面（主字体集合内的 W6 等），
    /// 该字面缺字形时回退到常规选择链。
    fn font_for_weight(&self, ch: char, bold: bool) -> &Font {
        if bold {
            if let Some(font) = &self.bold_font {
                if font.lookup_glyph_index(ch) != 0 {
                    return font;
                }
            }
        }
        self.font_for(ch)
    }

    fn font_for(&self, ch: char) -> &Font {
        // Emoji 优先用彩色 Emoji 字体，但必须确认其确有该字形：
        // is_emoji 的区间（如 0x2700..=0x27BF）包含 ✕ ✓ 这类纯符号，
        // Apple Color Emoji 并不覆盖，若无条件返回会渲染成豆腐块。
        if Self::is_emoji(ch) {
            if let Some(font) = &self.emoji_font {
                if font.lookup_glyph_index(ch) != 0 {
                    return font;
                }
            }
        }
        if self.main_has_glyph(ch) {
            return &self.main_font;
        }
        // 指定字族缺字形时回退到系统默认字体（浏览器也是逐字回退的）：
        // `font-family: "Times New Roman", serif` 里的西文字体没有汉字，
        // 不回退的话中文全变豆腐块。
        if let Some(base) = &self.fallback {
            if base.main_has_glyph(ch) {
                return &base.main_font;
            }
        }
        if let Some(font) = self.symbol_font() {
            if font.lookup_glyph_index(ch) != 0 {
                return font;
            }
        }
        if let Some(font) = &self.emoji_font {
            if font.lookup_glyph_index(ch) != 0 {
                return font;
            }
        }
        if let Some(base) = &self.fallback {
            return &base.main_font;
        }
        &self.main_font
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
        r.bold_font = Self::detect_bold_face(&data, &r.main_font);
        if log {
            eprintln!(
                "🔤 加载 {path}（{:.1}MB）：读盘 {:.0}ms 解析常规字面 {:.0}ms 探测粗体字面 {:.0}ms",
                data.len() as f32 / 1048576.0,
                t_read.as_secs_f32() * 1000.0,
                (t_parse - t_read).as_secs_f32() * 1000.0,
                (t0.elapsed() - t_parse).as_secs_f32() * 1000.0,
            );
        }
        // 只在该字体缺汉字时才挂回退（宋体自带汉字，不必多占一份）
        if !r.main_has_glyph('国') {
            r.fallback = shared_fonts();
        }
        Ok(r)
    }

    /// 加载系统字体（macOS）- 包含 Emoji 支持
    pub fn load_system_font() -> Result<Self, String> {
        // 主字体路径（中文优先）
        let main_font_paths = [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
            "/System/Library/Fonts/STHeiti Light.ttc",
        ];
        
        // Emoji 字体路径
        let emoji_font_paths = [
            "/System/Library/Fonts/Apple Color Emoji.ttc",
            "/System/Library/Fonts/AppleColorEmoji.ttf",
        ];
        
        // 加载主字体，并在同一字体集合里寻找真正的粗体字面
        let mut renderer: Option<TextRenderer> = None;
        for path in &main_font_paths {
            if !Path::new(path).exists() {
                continue;
            }
            let Ok(data) = std::fs::read(path) else { continue };
            match Self::from_bytes(&data) {
                Ok(mut r) => {
                    println!("✅ Main font: {}", path);
                    r.bold_font = Self::detect_bold_face(&data, &r.main_font);
                    if r.bold_font.is_some() {
                        println!("✅ Bold face: {}", path);
                    }
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

    /// 判断字符是否为 Emoji
    fn is_emoji(ch: char) -> bool {
        let code = ch as u32;
        // Emoji 范围（简化版）
        matches!(code,
            0x1F300..=0x1F9FF |  // Misc Symbols, Emoticons, etc.
            0x2600..=0x26FF |    // Misc Symbols
            0x2700..=0x27BF |    // Dingbats
            0xFE00..=0xFE0F |    // Variation Selectors
            0x1F000..=0x1F02F |  // Mahjong, Domino
            0x1F0A0..=0x1F0FF |  // Playing Cards
            0x1F100..=0x1F1FF |  // Enclosed Alphanumerics
            0x1F200..=0x1F2FF |  // Enclosed Ideographic
            0x1FA00..=0x1FAFF |  // Chess, Extended-A
            0x231A..=0x231B |    // Watch, Hourglass
            0x23E9..=0x23FA |    // Media controls
            0x25AA..=0x25FE |    // Squares
            0x2934..=0x2935 |
            0x2B05..=0x2B07 |
            0x2B1B..=0x2B1C |
            0x2B50 | 0x2B55 |
            0x3030 | 0x303D |
            0x3297 | 0x3299
        )
    }

    /// 渲染文本到画布
    pub fn draw_text(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, paint: &Paint) {
        self.draw_text_with_spacing(canvas, text, x, y, size, 0.0, paint);
    }
    
    /// 渲染文本到画布（带字间距）
    pub fn draw_text_with_spacing(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, letter_spacing: f32, paint: &Paint) {
        self.draw_text_weighted(canvas, text, x, y, size, letter_spacing, false, paint);
    }

    /// 彩色 emoji 位图（仅对 emoji 区间的码点尝试，避免数字/`#`/`*` 被 keycap 字形抢走）
    fn color_emoji(ch: char, size: f32) -> Option<std::sync::Arc<crate::emoji::EmojiBitmap>> {
        if !Self::is_emoji(ch) {
            return None;
        }
        crate::emoji::glyph_bitmap(ch, size)
    }

    /// emoji 的前进宽度（无彩色字形时返回 None，交给常规字体链）
    fn emoji_advance(ch: char, size: f32) -> Option<f32> {
        if !Self::is_emoji(ch) {
            return None;
        }
        crate::emoji::advance(ch, size)
    }

    /// 渲染文本到画布（带字间距 + 字重）。bold 为真且存在真实粗体字面时用该字面绘制。
    pub fn draw_text_weighted(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, letter_spacing: f32, bold: bool, paint: &Paint) {
        let bold = bold && self.bold_font.is_some();
        let mut cursor_x = x;
        let size_key = (size * 10.0) as u32; // 将 size 转换为整数 key，保留1位小数精度
        
        // 批量获取锁，避免循环中频繁锁竞争
        // 注意：这里为了简化，我们会在需要时获取锁。更好的做法可能是先收集所有需要的 glyph，然后一次性 rasterize。
        // 但考虑到 font.rasterize 是耗时操作，不应该在锁内做。
        
        for ch in text.chars() {
            // 变体选择符 / ZWJ 等零宽码点不绘制也不占位
            if crate::emoji::is_zero_width(ch) {
                continue;
            }
            // 彩色 emoji：走位图 blit（轮廓光栅化画不出位图字形，会变豆腐块）
            if let Some(bitmap) = Self::color_emoji(ch, size) {
                let draw_x = cursor_x + bitmap.left;
                let draw_y = y - bitmap.bottom - bitmap.draw_h;
                canvas.draw_image(
                    &bitmap.rgba,
                    bitmap.width,
                    bitmap.height,
                    draw_x,
                    draw_y,
                    bitmap.draw_w,
                    bitmap.draw_h,
                    "scaleToFill",
                    0.0,
                );
                cursor_x += bitmap.advance + letter_spacing;
                continue;
            }
            // 先尝试从缓存获取（快速路径）
            let cached_data = {
                let cache = self.cache.lock().unwrap();
                cache.get(&(ch, size_key, bold)).cloned()
            };
            
            let (metrics, bitmap) = if let Some(data) = cached_data {
                data
            } else {
                // 缓存未命中，执行光栅化（按字重与字形可用性选择字体，含符号回退）
                let font = self.font_for_weight(ch, bold);
                
                let (metrics, bitmap) = font.rasterize(ch, size);
                
                // 存入缓存
                let mut cache = self.cache.lock().unwrap();
                cache.insert((ch, size_key, bold), (metrics.clone(), bitmap.clone()));
                (metrics, bitmap)
            };
            
            if metrics.width == 0 || metrics.height == 0 {
                cursor_x += metrics.advance_width + letter_spacing;
                continue;
            }

            let glyph_x = cursor_x + metrics.xmin as f32;
            let glyph_y = y - metrics.height as f32 - metrics.ymin as f32;

            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let coverage = bitmap[gy * metrics.width + gx] as f32 / 255.0;
                    
                    if coverage > 0.001 {
                        let px = (glyph_x + gx as f32).round() as i32;
                        let py = (glyph_y + gy as f32).round() as i32;

                        if px >= 0 && py >= 0 && px < canvas.width() as i32 && py < canvas.height() as i32 {
                            // 优化：移除 gamma 校正，直接使用 linear alpha
                            // let gamma_coverage = coverage.powf(0.8);
                            let alpha = (paint.color.a as f32 * coverage) as u8;
                            
                            if alpha > 0 {
                                let color = Color::new(paint.color.r, paint.color.g, paint.color.b, alpha);
                                
                                canvas.set_pixel(px, py, color);
                            }
                        }
                    }
                }
            }

            cursor_x += metrics.advance_width + letter_spacing;
        }
    }

    /// 测量文本宽度
    pub fn measure_text(&self, text: &str, size: f32) -> f32 {
        self.measure_text_with_spacing(text, size, 0.0)
    }
    
    /// 测量文本宽度（带字间距）
    pub fn measure_text_with_spacing(&self, text: &str, size: f32, letter_spacing: f32) -> f32 {
        self.measure_text_weighted(text, size, letter_spacing, false)
    }

    /// 测量文本宽度（带字间距 + 字重）：粗体用真实粗体字面度量，避免测量与绘制不一致。
    /// 文本宽度（含 `letter-spacing`）。
    ///
    /// 字间距**每个字符后面都要加一份，末字符也算**：CSS 把 letter-spacing 加进
    /// 每个字形的前进宽度，行盒因此带一段行尾空隙（Chrome 实测 `letter-spacing:10px`
    /// 的 4 字符串正好宽 40px，不是 30px）。
    ///
    /// 从前这里按 `(n-1)` 份算，比绘制与断行用的 `n` 份窄了一份，于是**盒子按内容宽
    /// 定好之后，断行算法又认为同样的文字装不下**，最后一个字被挤到第二行：
    /// 带 letter-spacing 的标题盒子凭空高出一行（tea-app 首页
    /// `.brand-cn` 正是如此，把同一行的 `.brand-en` 挤出定高导航栏而整行消失）。
    pub fn measure_text_weighted(&self, text: &str, size: f32, letter_spacing: f32, bold: bool) -> f32 {
        let bold = bold && self.bold_font.is_some();
        let mut width = 0.0;
        for ch in text.chars() {
            if crate::emoji::is_zero_width(ch) {
                continue;
            }
            width += self.measure_char_weighted(ch, size, bold) + letter_spacing;
        }
        width
    }
    
    /// 测量单个字符宽度
    pub fn measure_char(&self, ch: char, size: f32) -> f32 {
        self.measure_char_weighted(ch, size, false)
    }

    /// 测量单个字符宽度（带字重）。彩色 emoji 用位图字体的前进宽度，
    /// 与绘制路径保持一致，否则文本会与背景框错位。
    pub fn measure_char_weighted(&self, ch: char, size: f32, bold: bool) -> f32 {
        if crate::emoji::is_zero_width(ch) {
            return 0.0;
        }
        if let Some(advance) = Self::emoji_advance(ch, size) {
            return advance;
        }
        let bold = bold && self.bold_font.is_some();
        self.font_for_weight(ch, bold).metrics(ch, size).advance_width
    }
    
    /// 测量文本高度
    pub fn measure_height(&self, size: f32) -> f32 {
        let metrics = self.main_font.metrics('M', size);
        metrics.height as f32
    }

    /// CSS `line-height: normal` 的等效行高（默认按含 CJK 处理）。
    ///
    /// 不能直接用字体表里的 `new_line_size`：实测同机上它随字体剧烈波动
    /// （Hiragino Sans GB 1.50、Arial Unicode 1.34、SF 1.18），而浏览器是按
    /// 「该行实际用到的字体」取度量。因此这里用实测校准的系数，并以字体的
    /// ascent-descent 作为下限防止裁字。
    pub fn natural_line_height(&self, size: f32) -> f32 {
        self.line_height_with_factor(size, CJK_LINE_HEIGHT_FACTOR)
    }

    /// 按文本内容选择 `line-height: normal` 行高：含 CJK 用 1.375，纯西文用 1.1777。
    ///
    /// 这与浏览器行为一致——同一段 CSS 下，`¥199` 这类纯西文行比中文行更矮，
    /// 若统一按 CJK 系数计算会让价格、数字等卡片整体偏高并逐行累积漂移。
    pub fn natural_line_height_for(&self, text: &str, size: f32) -> f32 {
        let factor = if text_has_cjk(text) {
            CJK_LINE_HEIGHT_FACTOR
        } else {
            LATIN_LINE_HEIGHT_FACTOR
        };
        self.line_height_with_factor(size, factor)
    }

    fn line_height_with_factor(&self, size: f32, factor: f32) -> f32 {
        // 至少一个字号高，避免异常字体度量导致行高塌陷
        (size * factor).max(size)
    }
    
    /// 自动换行绘制文本
    pub fn draw_text_wrapped(&self, canvas: &mut Canvas, text: &str, x: f32, y: f32, size: f32, max_width: f32, paint: &Paint) {
        if max_width <= 0.0 {
            self.draw_text(canvas, text, x, y, size, paint);
            return;
        }
        
        let line_height = size * 1.5; // 行高
        let mut current_y = y;
        let mut line_start = 0;
        let chars: Vec<char> = text.chars().collect();
        let mut current_width = 0.0;
        
        for (i, ch) in chars.iter().enumerate() {
            let metrics = self.font_for(*ch).metrics(*ch, size);
            let char_width = metrics.advance_width;
            
            // 检查是否需要换行
            if current_width + char_width > max_width && i > line_start {
                // 绘制当前行
                let line: String = chars[line_start..i].iter().collect();
                self.draw_text(canvas, &line, x, current_y, size, paint);
                
                // 移动到下一行
                current_y += line_height;
                line_start = i;
                current_width = char_width;
            } else {
                current_width += char_width;
            }
        }
        
        // 绘制最后一行
        if line_start < chars.len() {
            let line: String = chars[line_start..].iter().collect();
            self.draw_text(canvas, &line, x, current_y, size, paint);
        }
    }
    
    /// 计算换行后的文本高度
    pub fn measure_wrapped_height(&self, text: &str, size: f32, max_width: f32) -> f32 {
        if max_width <= 0.0 || text.is_empty() {
            return size * 1.5;
        }
        
        let line_height = size * 1.5;
        let mut line_count = 1;
        let mut current_width = 0.0;
        
        for ch in text.chars() {
            let metrics = self.font_for(ch).metrics(ch, size);
            let char_width = metrics.advance_width;
            
            if current_width + char_width > max_width && current_width > 0.0 {
                line_count += 1;
                current_width = char_width;
            } else {
                current_width += char_width;
            }
        }
        
        line_count as f32 * line_height
    }
}
