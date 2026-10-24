//! 文本渲染模块 - 支持系统字体、Emoji 和高清渲染

//! ## 内存
//! 上游 fontdue 加载时把**每个字形的几何**一次性展开（CJK 字面一个 300~400MB）。现在用的是
//! `vendor/fontdue`：字形第一次用到才展开、展开代码与上游逐行相同（光栅化逐位一致），
//! 字体文件 mmap 进来、同一个 TTC 的几个字面共用（text/load.rs）。实测见 doc/引擎测试说明.md「五之二」。

use crate::{Canvas, Paint};
use fontdue::{Font, FontSettings, Metrics};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// 字体加载（系统字体 / font-family / 粗体字面 / 符号回退），见 text/load.rs
mod load;
/// 「这段字在这个宽度下排几行」的记忆表（建树 / 排版反复问同一段字）
mod wrap_memo;
/// 字体占了多少内存（memory_report）
pub mod mem;

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
                    TextRenderer::from_static(include_bytes!("../assets/ArialUnicode.ttf"))
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
    /// 主字体的粗体字面（若字体集合内提供），用于替代描边式 faux-bold。
    ///
    /// **懒加载**：解析一个 CJK 字面在 fontdue 里要 200~380MB（见
    /// `doc/引擎测试说明.md` 的内存分项），而这一份只有真的要画粗体时才用得上。
    /// 以前在加载阶段就探测，等于开机先付一份内存，还会让「常规 + 探测候选」
    /// 两份字体同时在内存里（启动峰值因此接近翻倍）。
    bold_font: OnceLock<Option<Font>>,
    /// 粗体字面的来源字体文件；`None` 表示这份渲染器没有可探测的文件（如内嵌字节）
    bold_src: Option<String>,
    /// Emoji 字体
    emoji_font: Option<Font>,
    /// 符号字体（✕ ✓ ★ 等主字体缺失的字形）。
    /// 懒加载：多数页面不含这类符号，启动时不必解析 20MB+ 字体。
    symbol_font: OnceLock<Option<Font>>,
    /// 指定 `font-family` 时的兜底渲染器（系统默认字体）。
    /// 西文字体没有汉字、宋体没有某些符号，逐字回退到它，行为与浏览器一致。
    fallback: Option<Arc<TextRenderer>>,
    /// 简单的字形缓存 (char, size_u32, bold) -> (Metrics, Bitmap)
    /// 使用 Mutex 实现内部可变性，因为 draw 方法是 &self。
    /// 位图用 Arc 共享：命中时只加引用计数，不必每帧每字深拷贝一份。
    cache: Arc<Mutex<HashMap<(char, u32, bool), (Metrics, Arc<[u8]>)>>>,
    /// 断行行数的记忆表（见 text/wrap_memo.rs）
    wrap_memo: wrap_memo::WrapMemo,
    /// 整段宽度的记忆表（[`measure_text_weighted`](Self::measure_text_weighted)，同一套键）
    width_memo: wrap_memo::Memo<f32>,
}

impl TextRenderer {
    /// 从字体数据创建
    pub fn from_bytes(font_data: &[u8]) -> Result<Self, String> {
        Self::from_bytes_index(font_data, 0)
    }

    /// 从字体数据创建，取 TTC 集合里的第 `index` 个字面
    pub fn from_bytes_index(font_data: &[u8], index: u32) -> Result<Self, String> {
        let settings = FontSettings {
            scale: 40.0,
            collection_index: index,
            ..Default::default()
        };
        let font = Font::from_bytes(font_data, settings)
            .map_err(|e| e.to_string())?;
        Ok(Self::with_main_font(font))
    }

    /// 以 `font` 为主字体建渲染器（其余字面、回退都还没挂）
    pub(crate) fn with_main_font(font: Font) -> Self {
        Self { 
            main_font: font,
            bold_font: OnceLock::new(),
            bold_src: None,
            emoji_font: None,
            symbol_font: OnceLock::new(),
            fallback: None,
            cache: Arc::new(Mutex::new(HashMap::new())),
            wrap_memo: Default::default(),
            width_memo: Default::default(),
        }
    }

    /// `text` 在 `max_width` 下排几行：记过就直接给，没记过 `compute` 一次再记（text/wrap_memo.rs）。
    /// `compute` 必须只用这个渲染器量字 —— 记忆表挂在它身上，键里没有字体
    pub(crate) fn memo_wrapped_lines(&self, text: &str, max_width: f32, size: f32, ls: f32, bold: bool, compute: impl FnOnce() -> usize) -> usize {
        self.wrap_memo.lines(text, max_width, size, ls, bold, compute)
    }

    /// 判断主字体是否有该字符的字形（无字形时 fontdue 返回索引 0）
    fn main_has_glyph(&self, ch: char) -> bool {
        self.main_font.lookup_glyph_index(ch) != 0
    }

    /// 字形位图缓存的字节预算。默认 24MB：实测 1200 个不同汉字 ≈ 8.7MB，
    /// 一个中文应用把常用字全走一遍再乘几个字号也就在这个量级。
    /// 可用 `MINI_GLYPH_CACHE_MB` 覆盖。
    fn glyph_cache_budget() -> usize {
        static BUDGET: OnceLock<usize> = OnceLock::new();
        *BUDGET.get_or_init(|| {
            let mb = std::env::var("MINI_GLYPH_CACHE_MB")
                .ok()
                .and_then(|v| v.trim().parse::<usize>().ok())
                .filter(|v| *v > 0)
                .unwrap_or(24);
            mb * 1024 * 1024
        })
    }

    /// 超预算就整表清空。
    ///
    /// 不做 LRU 是有意的：字形位图小而多（几万条），维护访问顺序的开销比重新
    /// 光栅化还贵，而清空之后下一屏只会重新栅格化**当前可见**的那些字。
    fn trim_glyph_cache(
        cache: &mut HashMap<(char, u32, bool), (Metrics, Arc<[u8]>)>,
        incoming: usize,
    ) {
        // 每 256 条才真的去累加一次：这个函数在每次缓存未命中时都会被调用，
        // 每次都扫全表的话（到顶时几千条）比光栅化本身还贵。
        if cache.len() % 256 != 0 {
            return;
        }
        // 粗估：位图字节 + 键值与 Metrics 的固定开销
        const PER_ENTRY_OVERHEAD: usize = 64;
        let used: usize = cache.values().map(|(_, b)| b.len() + PER_ENTRY_OVERHEAD).sum();
        if used + incoming <= Self::glyph_cache_budget() {
            return;
        }
        if std::env::var_os("MINI_FONT_LOG").is_some() {
            eprintln!(
                "🔤 字形缓存到顶（{:.1}MB / {} 条），清空重来",
                used as f32 / 1048576.0,
                cache.len()
            );
        }
        cache.clear();
    }

    /// 真实粗体字面（首次调用时才解析，见字段注释）。
    fn bold_face(&self) -> Option<&Font> {
        self.bold_font
            .get_or_init(|| {
                // 同一个字体文件（TTC）里的别的字面：与常规字面共用那份字节（mmap，见 text/load.rs）
                self.bold_src.as_ref()?;
                Self::detect_bold_face(self.main_font.bytes(), &self.main_font)
            })
            .as_ref()
    }

    /// 是否有真实粗体字面可用（无则调用方回退 faux-bold）。
    pub fn has_bold_face(&self) -> bool {
        self.bold_face().is_some()
    }

    /// 按字重选择字体：粗体优先用真实粗体字面（主字体集合内的 W6 等），
    /// 该字面缺字形时回退到常规选择链。
    fn font_for_weight(&self, ch: char, bold: bool) -> &Font {
        if bold {
            if let Some(font) = self.bold_face() {
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
        let bold = bold && self.has_bold_face();
        let mut cursor_x = x;
        let size_key = (size * 10.0) as u32; // 将 size 转换为整数 key，保留1位小数精度
        // 覆盖率 → alpha（线性，不做 gamma）。覆盖率 0 查出来是 0，即「不画」
        let alpha_lut: [u8; 256] =
            std::array::from_fn(|c| (paint.color.a as f32 * (c as f32 / 255.0)) as u8);
        
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
                let t = crate::renderer::frame_cost::start();
                let (metrics, bitmap) = font.rasterize(ch, size);
                crate::renderer::frame_cost::record("字形", t, String::new);
                let bitmap: Arc<[u8]> = bitmap.into();
                
                // 存入缓存（封顶，见 glyph_cache_budget）
                let mut cache = self.cache.lock().unwrap();
                Self::trim_glyph_cache(&mut cache, bitmap.len());
                cache.insert((ch, size_key, bold), (metrics, bitmap.clone()));
                (metrics, bitmap)
            };
            
            if metrics.width == 0 || metrics.height == 0 {
                cursor_x += metrics.advance_width + letter_spacing;
                continue;
            }

            let glyph_x = cursor_x + metrics.xmin as f32;
            let glyph_y = y - metrics.height as f32 - metrics.ymin as f32;
            // 每列/每行的落点只取决于 gx/gy，提到像素循环外各算一次
            let cols: Vec<i32> = (0..metrics.width).map(|gx| (glyph_x + gx as f32).round() as i32).collect();
            let rows: Vec<i32> = (0..metrics.height).map(|gy| (glyph_y + gy as f32).round() as i32).collect();
            canvas.blend_coverage(&cols, &rows, &bitmap, &alpha_lut, paint.color);

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
        // 记忆表（text/wrap_memo.rs）：setData 之后整棵重建，页面上没变的几百段字又要逐字查字形、
        // 量前进宽度 —— 一加真机「节点与滚动」页上建树时间的两成（fontdue `metrics_raw`、`font_for_weight`）。
        // 结果只取决于（这个渲染器、文字、字号、字间距、粗细），命中时与现算逐位相同。
        // 单个字不记：查表不比直接量便宜
        if text.len() <= 4 {
            return self.measure_text_weighted_uncached(text, size, letter_spacing, bold);
        }
        self.width_memo
            .lines(text, 0.0, size, letter_spacing, bold, || self.measure_text_weighted_uncached(text, size, letter_spacing, bold))
    }

    fn measure_text_weighted_uncached(&self, text: &str, size: f32, letter_spacing: f32, bold: bool) -> f32 {
        let bold = bold && self.has_bold_face();
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
        let bold = bold && self.has_bold_face();
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
    
}
