//! WXSS 文本 → 规则表：词法、声明块、`@import` / `@keyframes` / `page` 等。
//!
//! 从 1453 行的 wxss.rs 拆出来（**纯搬迁**）。选择器怎么解析见 `selector.rs`。
use super::*;

/// `@import` / `@keyframes` / `@media` / `@font-face` / 跳过未知 at-rule
mod at_rules;

pub struct WxssParser {
    input: Vec<char>,
    pos: usize,
}

impl WxssParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }
    
    /// 解析单个长度值（用于 border 简写等）
    pub fn parse_length_value(&mut self) -> Option<(f32, LengthUnit)> {
        let value: String = self.input.iter().collect();
        Self::parse_length(&value)
    }
    
    pub fn parse(&mut self) -> Result<StyleSheet, String> {
        let mut stylesheet = StyleSheet::new();
        self.parse_into(&mut stylesheet)?;
        // 解析 var() 自定义属性引用（`@media` 块里的规则已经并进来了，一起解析）
        stylesheet.resolve_variables();
        Ok(stylesheet)
    }

    /// 把规则依书写顺序追加进 `stylesheet`。`@media` 成立时块里的规则就地并进来
    /// （书写顺序决定层叠，不能挪到最后）
    pub(super) fn parse_into(&mut self, stylesheet: &mut StyleSheet) -> Result<(), String> {
        while self.pos < self.input.len() {
            self.skip_whitespace_and_comments();
            if self.pos >= self.input.len() {
                break;
            }
            // at-rule
            if self.current_char() == '@' {
                if let Some(import) = self.try_parse_import() {
                    stylesheet.imports.push(import);
                } else if let Some(rule) = self.try_parse_keyframes() {
                    stylesheet.keyframes.push(rule);
                } else if self.try_parse_media(stylesheet)? {
                    // 已并进 stylesheet（条件不成立时整块丢掉）
                } else if let Some(face) = self.try_parse_font_face() {
                    stylesheet.font_faces.push(face);
                } else {
                    self.skip_at_rule();
                }
                continue;
            }
            if let Some(rule) = self.parse_rule()? {
                stylesheet.rules.push(rule);
            }
        }
        Ok(())
    }
    
    
    fn parse_rule(&mut self) -> Result<Option<StyleRule>, String> {
        self.skip_whitespace_and_comments();
        
        let selector = self.parse_selector();
        if selector.is_empty() {
            return Ok(None);
        }
        
        self.skip_whitespace_and_comments();
        
        if self.current_char() != '{' {
            return Err(format!("Expected '{{' after selector '{}'", selector));
        }
        self.advance();
        
        let properties = self.parse_properties()?;
        
        self.skip_whitespace_and_comments();
        if self.current_char() == '}' {
            self.advance();
        }
        
        Ok(Some(StyleRule::new(selector, properties)))
    }
    
    fn parse_selector(&mut self) -> String {
        let mut selector = String::new();
        
        while self.pos < self.input.len() {
            let c = self.current_char();
            if c == '{' || c == '}' {
                break;
            }
            selector.push(c);
            self.advance();
        }
        
        selector.trim().to_string()
    }
    
    fn parse_properties(&mut self) -> Result<HashMap<String, StyleValue>, String> {
        let mut properties = HashMap::new();
        
        loop {
            self.skip_whitespace_and_comments();
            
            if self.pos >= self.input.len() || self.current_char() == '}' {
                break;
            }
            
            let name = self.parse_property_name();
            if name.is_empty() {
                break;
            }
            
            self.skip_whitespace();
            
            if self.current_char() != ':' {
                continue;
            }
            self.advance();
            
            self.skip_whitespace();
            
            let value = self.parse_property_value();
            
            if self.current_char() == ';' {
                self.advance();
            }
            
            let parsed_value = Self::parse_value(&name, &value);
            properties.insert(name, parsed_value);
        }
        
        Ok(properties)
    }
    
    fn parse_property_name(&mut self) -> String {
        let mut name = String::new();
        
        while self.pos < self.input.len() {
            let c = self.current_char();
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        
        name
    }
    
    fn parse_property_value(&mut self) -> String {
        let mut value = String::new();
        let mut paren_depth = 0;
        
        while self.pos < self.input.len() {
            let c = self.current_char();
            
            if c == '(' { paren_depth += 1; }
            else if c == ')' { paren_depth -= 1; }
            
            if paren_depth == 0 && (c == ';' || c == '}') {
                break;
            }
            
            value.push(c);
            self.advance();
        }
        
        value.trim().to_string()
    }
    
    pub(super) fn parse_value(name: &str, value: &str) -> StyleValue {
        let value = value.trim();
        
        // 自定义属性（--x）原样保留字符串，供 var() 解析
        if name.starts_with("--") {
            return StyleValue::String(value.to_string());
        }
        
        // 含 var() 的值先保留原始字符串，待 resolve_variables 二次处理
        if value.contains("var(") {
            return StyleValue::String(value.to_string());
        }
        
        // calc()：尝试静态求值（仅同单位加减 / 无单位）
        if value.starts_with("calc(") {
            if let Some(v) = Self::eval_calc(value) {
                return v;
            }
            return StyleValue::String(value.to_string());
        }

        // 单独的 `env(safe-area-inset-*)`：宿主给了非零安全区才折成长度（见 calc_env.rs）
        if value.starts_with("env(") || value.starts_with("constant(") {
            if let Some(v) = Self::eval_env(value) {
                return v;
            }
        }
        
        // 颜色值
        if value.starts_with('#') {
            if let Some(color) = Self::parse_color(value) {
                return StyleValue::Color(color);
            }
        }
        
        if value.starts_with("rgb") {
            if let Some(color) = Self::parse_rgb_color(value) {
                return StyleValue::Color(color);
            }
        }
        
        // 线性渐变：保留完整字符串，交由渲染层解析绘制
        if value.starts_with("linear-gradient") {
            return StyleValue::String(value.to_string());
        }
        // 径向渐变暂以首色兜底
        if value.starts_with("radial-gradient") {
            if let Some(color) = Self::parse_gradient_fallback(value) {
                return StyleValue::Color(color);
            }
        }
        
        // 命名颜色
        if let Some(color) = Self::parse_named_color(value) {
            return StyleValue::Color(color);
        }
        
        // 无单位数值属性：这些属性的裸数字是「倍数 / 纯数」而不是长度。
        // `parse_length` 会把裸数字一律当成 px，于是 `line-height:1.9` 变成 1.9 像素
        // （行距被压成一条线），`opacity:.5` / `z-index:10` 因为拿到的是 Length 而不是
        // Number，对应的样式处理分支根本不会命中 —— 等于整条声明被静默丢掉。
        if Self::is_unitless_property(name) {
            if let Ok(num) = value.parse::<f32>() {
                return StyleValue::Number(num);
            }
        }
        
        // 长度值
        if let Some((num, unit)) = Self::parse_length(value) {
            return StyleValue::Length(num, unit);
        }
        
        // 特殊值
        match value {
            "auto" => return StyleValue::Auto,
            "none" => return StyleValue::None,
            "inherit" | "initial" | "unset" => return StyleValue::String(value.to_string()),
            _ => {}
        }
        
        // 数字
        if let Ok(num) = value.parse::<f32>() {
            return StyleValue::Number(num);
        }
        
        StyleValue::String(value.to_string())
    }
    
    /// 裸数字应按「数值」而非「长度」理解的属性
    fn is_unitless_property(name: &str) -> bool {
        matches!(
            name,
            "line-height"
                | "opacity"
                | "z-index"
                | "flex"
                | "flex-grow"
                | "flex-shrink"
                | "order"
                | "font-weight"
                | "animation-iteration-count"
                | "aspect-ratio"
                | "zoom"
        )
    }

    // 颜色解析只有一份实现：`renderer::components::color_parse`。
    // 这里保留三个薄封装是为了不动上面 6 处调用点的分支结构（`#` / `rgb` / 命名色
    // 各走一支，顺序有意义）。以前 wxss 自己带一份，和渲染层那份对 alpha、
    // 百分比通道、4 位十六进制的支持各不相同，同一个色值在两条路径上会画出两种结果。
    fn parse_named_color(name: &str) -> Option<Color> {
        crate::renderer::components::parse_named_color(name)
    }

    fn parse_color(value: &str) -> Option<Color> {
        let v = value.trim();
        // 调用点已保证以 `#` 开头，这里补一次以防将来被复用
        let with_hash = if v.starts_with('#') { v.to_string() } else { format!("#{v}") };
        crate::renderer::components::parse_color_str(&with_hash)
    }

    fn parse_rgb_color(value: &str) -> Option<Color> {
        crate::renderer::components::parse_color_str(value)
    }

    /// 解析渐变值，提取第一个颜色作为 fallback（用于 `radial-gradient`：
    /// 我们还不画径向渐变，先用首个色标当纯色兜底）。
    fn parse_gradient_fallback(value: &str) -> Option<Color> {
        let start = value.find('(')?;
        let end = value.rfind(')')?;
        // 顶层逗号切分：`rgba(0, 0, 0, .5)` 内部的逗号不能算分隔符
        for part in crate::renderer::components::split_top_commas(&value[start + 1..end]) {
            let part = part.trim();
            if part.is_empty() || part.ends_with("deg") || part.starts_with("to ") {
                continue;
            }
            // `<color> <position>`：位置在最后，去掉后再解析颜色
            if let Some(color) = crate::renderer::components::parse_color_str(part) {
                return Some(color);
            }
            if let Some((head, _)) = part.rsplit_once(char::is_whitespace) {
                if let Some(color) = crate::renderer::components::parse_color_str(head) {
                    return Some(color);
                }
            }
        }
        None
    }
    
    // `eval_calc`（calc 静态求值）与安全区 `env()` 在 calc_env.rs
    
    pub(super) fn parse_length(value: &str) -> Option<(f32, LengthUnit)> {
        let value = value.trim();
        
        let units = [
            ("rpx", LengthUnit::Rpx),
            ("px", LengthUnit::Px),
            ("vw", LengthUnit::Vw),
            ("vh", LengthUnit::Vh),
            ("rem", LengthUnit::Rem),
            ("em", LengthUnit::Em),
            ("%", LengthUnit::Percent),
        ];
        
        for (suffix, unit) in units {
            if value.ends_with(suffix) {
                let num = value.trim_end_matches(suffix).parse().ok()?;
                return Some((num, unit));
            }
        }
        
        if let Ok(num) = value.parse::<f32>() {
            return Some((num, LengthUnit::Px));
        }
        
        None
    }
    
    fn current_char(&self) -> char {
        self.input.get(self.pos).copied().unwrap_or('\0')
    }
    
    fn advance(&mut self) {
        self.pos += 1;
    }
    
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current_char().is_whitespace() {
            self.advance();
        }
    }
    
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.skip_whitespace();
            if self.starts_with("/*") {
                self.skip_comment();
            } else {
                break;
            }
        }
    }
    
    fn skip_comment(&mut self) {
        self.advance();
        self.advance();
        while self.pos < self.input.len() && !self.starts_with("*/") {
            self.advance();
        }
        if self.pos < self.input.len() {
            self.advance();
            self.advance();
        }
    }
    
    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if self.pos + i >= self.input.len() || self.input[self.pos + i] != *c {
                return false;
            }
        }
        true
    }
}
