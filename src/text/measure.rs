//! `TextRenderer` 的度量：文字宽度（字间距、字重、记忆表）与 `line-height: normal` 的行高。
//! 从 text/mod.rs 按职责拆出（**纯搬迁**，一行逻辑没改）：那边只留字体选择与绘制。
use super::*;

impl TextRenderer {
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
            if crate::text::emoji::is_zero_width(ch) {
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
        if crate::text::emoji::is_zero_width(ch) {
            return 0.0;
        }
        if let Some(advance) = Self::emoji_advance(ch, size) {
            return advance;
        }
        let bold = bold && self.has_bold_face();
        self.advance_of(ch, size, bold, self.font_for_weight(ch, bold).metrics(ch, size).advance_width)
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
        if let Some(h) = ios_line_box(size) {
            return h.max(size);
        }
        // 至少一个字号高，避免异常字体度量导致行高塌陷
        (size * factor).max(size)
    }
}
