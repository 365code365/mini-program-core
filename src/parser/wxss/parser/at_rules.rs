//! at-rule：`@import` / `@keyframes` / `@media` / `@font-face`，以及不认识的 at-rule 的跳过。
//!
//! `parser.rs` 的子模块：前三个函数从 519 行的 parser.rs 原样搬出来（纯搬迁，只把可见性放宽到
//! `pub(super)`），`@media` 与 `@font-face` 是之后加的。
use super::*;

impl WxssParser {
    /// 尝试解析 @import "path"; / @import url("path");
    pub(super) fn try_parse_import(&mut self) -> Option<String> {
        let start = self.pos;
        // 读取 at-rule 名
        if !self.starts_with("@import") {
            return None;
        }
        self.pos += "@import".len();
        self.skip_whitespace();
        
        // 可选 url(
        let mut used_url = false;
        if self.starts_with("url(") {
            self.pos += 4;
            used_url = true;
            self.skip_whitespace();
        }
        
        let quote = self.current_char();
        if quote != '"' && quote != '\'' {
            // 非预期格式，回退
            self.pos = start;
            return None;
        }
        self.advance();
        let mut path = String::new();
        while self.pos < self.input.len() && self.current_char() != quote {
            path.push(self.current_char());
            self.advance();
        }
        if self.pos < self.input.len() { self.advance(); } // 关闭引号
        
        if used_url {
            self.skip_whitespace();
            if self.current_char() == ')' { self.advance(); }
        }
        // 跳到分号
        while self.pos < self.input.len() && self.current_char() != ';' && self.current_char() != '\n' {
            self.advance();
        }
        if self.current_char() == ';' { self.advance(); }
        
        Some(path)
    }
    
    /// 尝试解析 `@keyframes name { 0% {...} 50%,80% {...} to {...} }`
    /// （同时兼容 `@-webkit-keyframes`）。失败时回退到 `skip_at_rule`。
    pub(super) fn try_parse_keyframes(&mut self) -> Option<KeyframesRule> {
        let start = self.pos;
        let prefixes = ["@keyframes", "@-webkit-keyframes", "@-moz-keyframes"];
        let matched = prefixes.iter().find(|p| self.starts_with(p))?;
        self.pos += matched.len();
        self.skip_whitespace_and_comments();

        // 动画名
        let mut name = String::new();
        while self.pos < self.input.len() {
            let c = self.current_char();
            if c.is_whitespace() || c == '{' {
                break;
            }
            name.push(c);
            self.advance();
        }
        self.skip_whitespace_and_comments();
        if name.is_empty() || self.current_char() != '{' {
            self.pos = start;
            return None;
        }
        self.advance(); // 吃掉 '{'

        let mut steps: Vec<KeyframeStep> = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.pos >= self.input.len() {
                break;
            }
            if self.current_char() == '}' {
                self.advance();
                break;
            }
            // 关键帧选择器（可能是 `0%, 100%` 这种多值）
            let selector = self.parse_selector();
            if self.current_char() != '{' {
                // 结构异常：整条规则放弃，交给通用跳过逻辑
                self.pos = start;
                return None;
            }
            self.advance();
            let properties = self.parse_properties().ok()?;
            self.skip_whitespace_and_comments();
            if self.current_char() == '}' {
                self.advance();
            }

            for part in selector.split(',') {
                if let Some(offset) = parse_keyframe_offset(part.trim()) {
                    steps.push(KeyframeStep { offset, properties: properties.clone() });
                }
            }
        }

        steps.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap_or(std::cmp::Ordering::Equal));
        Some(KeyframesRule { name, steps })
    }

    /// `@media <条件> { 规则… }`：条件在解析期求值（见 `media.rs`），成立就把块里的规则就地并进
    /// `sheet`（块里还能再嵌 `@media` / `@keyframes`），不成立整块丢掉。不是 `@media` 返回 false
    pub(super) fn try_parse_media(&mut self, sheet: &mut StyleSheet) -> Result<bool, String> {
        if !self.starts_with("@media") {
            return Ok(false);
        }
        let start = self.pos;
        self.pos += "@media".len();
        let mut cond = String::new();
        while self.pos < self.input.len() && self.current_char() != '{' && self.current_char() != ';' {
            cond.push(self.current_char());
            self.advance();
        }
        if self.current_char() != '{' {
            self.pos = start;
            return Ok(false);
        }
        self.advance();
        let body_start = self.pos;
        let mut depth = 1;
        while self.pos < self.input.len() {
            match self.current_char() {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            self.advance();
        }
        let body: String = self.input[body_start..self.pos.min(self.input.len())].iter().collect();
        self.advance(); // 吃掉收尾的 '}'
        if super::super::media::media_matches(cond.trim()) {
            WxssParser::new(&body).parse_into(sheet)?;
        }
        Ok(true)
    }

    /// `@font-face { font-family: X; src: url(...) format('truetype'); }`：记下来并交给字体登记表
    /// （`text_faces`：包内文件当场登记，https 后台下载）。不是 `@font-face` 返回 None
    pub(super) fn try_parse_font_face(&mut self) -> Option<FontFace> {
        if !self.starts_with("@font-face") {
            return None;
        }
        let start = self.pos;
        self.pos += "@font-face".len();
        self.skip_whitespace_and_comments();
        if self.current_char() != '{' {
            self.pos = start;
            return None;
        }
        self.advance();
        let mut block = String::new();
        while self.pos < self.input.len() && self.current_char() != '}' {
            block.push(self.current_char());
            self.advance();
        }
        self.advance();
        let mut family = String::new();
        let mut src = String::new();
        for decl in block.split(';') {
            let Some((k, v)) = decl.split_once(':') else { continue };
            match k.trim().to_ascii_lowercase().as_str() {
                "font-family" => family = v.trim().trim_matches('"').trim_matches('\'').trim().to_string(),
                // 多个来源取第一个 url(...)
                "src" => {
                    if let Some(i) = v.find("url(") {
                        let rest = &v[i + 4..];
                        let end = rest.find(')').unwrap_or(rest.len());
                        src = rest[..end].trim().trim_matches('"').trim_matches('\'').to_string();
                    }
                }
                _ => {}
            }
        }
        if family.is_empty() || src.is_empty() {
            return None;
        }
        crate::text_faces::load(0, &family, &src);
        Some(FontFace { family, src })
    }

    pub(super) fn skip_at_rule(&mut self) {
        while self.pos < self.input.len() && self.current_char() != ';' && self.current_char() != '{' {
            self.advance();
        }
        if self.current_char() == '{' {
            let mut depth = 1;
            self.advance();
            while self.pos < self.input.len() && depth > 0 {
                if self.current_char() == '{' { depth += 1; }
                if self.current_char() == '}' { depth -= 1; }
                self.advance();
            }
        } else {
            self.advance();
        }
    }
}
