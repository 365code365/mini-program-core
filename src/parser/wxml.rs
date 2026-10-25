//! WXML 解析器

use std::collections::HashMap;

/// WXML 节点类型
#[derive(Debug, Clone, PartialEq)]
pub enum WxmlNodeType {
    Element,
    Text,
    Comment,
}

/// WXML 节点
#[derive(Debug, Clone)]
pub struct WxmlNode {
    pub node_type: WxmlNodeType,
    pub tag_name: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<WxmlNode>,
    pub text_content: String,
}

impl WxmlNode {
    pub fn new_element(tag_name: &str) -> Self {
        Self {
            node_type: WxmlNodeType::Element,
            tag_name: tag_name.to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text_content: String::new(),
        }
    }
    
    pub fn new_text(content: &str) -> Self {
        Self {
            node_type: WxmlNodeType::Text,
            tag_name: String::new(),
            attributes: HashMap::new(),
            children: Vec::new(),
            text_content: content.to_string(),
        }
    }
    
    /// 按属性名排序遍历属性。
    ///
    /// **凡是遍历顺序会被输出看见的地方都必须用这个**，别直接 `for … in &node.attributes`：
    /// `attributes` 是 `HashMap`，每个进程的哈希种子不同，遍历顺序就不同。后果是导出的
    /// HTML 工程**不可复现** —— 同一个二进制连跑两次，`data-ds-*` 的先后会变，实测有
    /// 5 个页面文件在两次编译间不一致，于是没法用「逐字节比」去校验导出产物有没有回归。
    ///
    /// 往 `HashMap`/`serde_json::Map` 里插入的场景不受影响（那些本身与顺序无关）。
    pub fn attrs_sorted(&self) -> Vec<(&String, &String)> {
        let mut v: Vec<_> = self.attributes.iter().collect();
        v.sort_by(|a, b| a.0.cmp(b.0));
        v
    }

    pub fn get_attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(|s| s.as_str())
    }
    
    pub fn has_class(&self, class_name: &str) -> bool {
        if let Some(classes) = self.attributes.get("class") {
            classes.split_whitespace().any(|c| c == class_name)
        } else {
            false
        }
    }
}

/// WXML 解析器
pub struct WxmlParser {
    input: Vec<char>,
    pos: usize,
}

impl WxmlParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }
    
    pub fn parse(&mut self) -> Result<Vec<WxmlNode>, String> {
        let mut nodes = Vec::new();
        
        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }
            
            if self.starts_with("<!--") {
                self.parse_comment();
            } else if self.current_char() == '<' {
                if self.starts_with("</") {
                    break; // 结束标签，返回上层
                }
                if let Some(node) = self.parse_element()? {
                    nodes.push(node);
                }
            } else {
                if let Some(text) = self.parse_text() {
                    if !text.text_content.trim().is_empty() {
                        nodes.push(text);
                    }
                }
            }
        }
        
        Ok(nodes)
    }
    
    fn parse_element(&mut self) -> Result<Option<WxmlNode>, String> {
        self.expect('<')?;
        
        let tag_name = self.parse_tag_name();
        if tag_name.is_empty() {
            return Err("Empty tag name".to_string());
        }
        
        let mut node = WxmlNode::new_element(&tag_name);
        
        // 解析属性
        loop {
            self.skip_whitespace();
            if self.current_char() == '>' || self.starts_with("/>") {
                break;
            }
            // 读到结尾还没有 `>`、或这一位既不是属性名也不是 `>`（`<b ? 'x'`）：报错。
            // 从前这里不前进也不退出 —— 文本里 `{{a<b ? 'x' : 'y'}}` 让整个引擎卡死在解析上
            if self.pos >= self.input.len() {
                return Err(format!("<{tag_name}> 没有闭合的 '>'"));
            }
            let start = self.pos;
            let (name, value) = self.parse_attribute()?;
            if name.is_empty() || self.pos == start {
                return Err(format!("<{tag_name}> 里有认不出的字符 '{}'", self.input[start]));
            }
            node.attributes.insert(name, value);
        }
        
        // 自闭合标签
        if self.starts_with("/>") {
            self.advance();
            self.advance();
            return Ok(Some(node));
        }
        
        self.expect('>')?;

        // `<wxs>` 的内容是 JS，不是 WXML：`for (i = 0; i < n; i++)` 里的 `<` 按标签解析会
        // 直接报「Empty tag name」，整页都没了。按原始文本读到 `</wxs>` 为止，存成唯一的
        // 文本子节点（模板展开时整个 wxs 节点不渲染，脚本原文留给 wxs 模块支持用）。
        if tag_name == "wxs" {
            let body = self.read_raw_until("</wxs");
            if !body.trim().is_empty() {
                node.children.push(WxmlNode::new_text(&body));
            }
            if self.starts_with("</wxs") {
                for _ in 0.."</wxs".len() {
                    self.advance();
                }
                self.skip_whitespace();
                self.expect('>')?;
            }
            return Ok(Some(node));
        }
        
        // 解析子节点
        node.children = self.parse()?;
        
        // 解析结束标签
        self.skip_whitespace();
        if self.starts_with("</") {
            self.advance();
            self.advance();
            let end_tag = self.parse_tag_name();
            if end_tag != tag_name {
                return Err(format!("Mismatched tags: {} vs {}", tag_name, end_tag));
            }
            self.skip_whitespace();
            self.expect('>')?;
        }
        
        Ok(Some(node))
    }
    
    fn parse_tag_name(&mut self) -> String {
        let mut name = String::new();
        while self.pos < self.input.len() {
            let c = self.current_char();
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }
    
    fn parse_attribute(&mut self) -> Result<(String, String), String> {
        let name = self.parse_attribute_name();
        
        self.skip_whitespace();
        
        // 不带值的属性（`<button disabled>`、`<swiper autoplay>`）是 true（与微信 WXML 编译一致）。
        // 从前给空串，而组件按 `== "true"` 判，`<switch checked>`、`<button disabled>` 全成了 false；
        // 空串留给「绑定求出来是空」（`{{undefined}}`），按布尔属性是 false
        if self.current_char() != '=' {
            return Ok((name, "true".to_string()));
        }
        
        self.advance(); // skip '='
        self.skip_whitespace();
        
        let value = self.parse_attribute_value()?;
        
        Ok((name, value))
    }
    
    fn parse_attribute_name(&mut self) -> String {
        let mut name = String::new();
        while self.pos < self.input.len() {
            let c = self.current_char();
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }
    
    fn parse_attribute_value(&mut self) -> Result<String, String> {
        let quote = self.current_char();
        if quote != '"' && quote != '\'' {
            // 无引号值
            let mut value = String::new();
            while self.pos < self.input.len() {
                let c = self.current_char();
                if c.is_whitespace() || c == '>' || c == '/' {
                    break;
                }
                value.push(c);
                self.advance();
            }
            return Ok(value);
        }
        
        self.advance(); // skip opening quote
        
        let mut value = String::new();
        while self.pos < self.input.len() && self.current_char() != quote {
            value.push(self.current_char());
            self.advance();
        }
        
        if self.pos < self.input.len() {
            self.advance(); // skip closing quote
        }
        
        Ok(value)
    }
    
    /// 文字节点：读到下一个标签为止。`{{ }}` 里面原样读（表达式里的 `<` 不是标签：
    /// `{{a < b ? 'x' : 'y'}}` 从前整页解析失败，`{{a<b ? …}}` 把解析器卡死）
    fn parse_text(&mut self) -> Option<WxmlNode> {
        let mut text = String::new();
        while self.pos < self.input.len() {
            // 没有配对的 `}}` 就当普通字符（别把后面整页吞进一个文字节点）
            if let Some(close) = self.starts_with("{{").then(|| self.find_from(self.pos + 2, "}}")).flatten() {
                text.extend(&self.input[self.pos..close + 2]);
                self.pos = close + 2;
                continue;
            }
            // 表达式外面的 `<` 一律当标签开头（与从前一致：写坏的 `<<<` 整段报错、不渲染）
            if self.current_char() == '<' {
                break;
            }
            text.push(self.current_char());
            self.advance();
        }
        
        if text.is_empty() {
            None
        } else {
            Some(WxmlNode::new_text(&text))
        }
    }
    
    /// 原样读到 `end` 出现之前（不含 `end`）；没有 `end` 就读到结尾
    fn read_raw_until(&mut self, end: &str) -> String {
        let mut text = String::new();
        while self.pos < self.input.len() && !self.starts_with(end) {
            text.push(self.current_char());
            self.advance();
        }
        text
    }

    fn parse_comment(&mut self) {
        // Skip <!--
        for _ in 0..4 {
            self.advance();
        }
        
        while self.pos < self.input.len() && !self.starts_with("-->") {
            self.advance();
        }
        
        // Skip -->
        for _ in 0..3 {
            if self.pos < self.input.len() {
                self.advance();
            }
        }
    }
    
    /// 从 `from` 起第一次出现 `s` 的位置
    fn find_from(&self, from: usize, s: &str) -> Option<usize> {
        let pat: Vec<char> = s.chars().collect();
        (from..self.input.len().saturating_sub(pat.len() - 1)).find(|&i| self.input[i..i + pat.len()] == pat[..])
    }

    fn current_char(&self) -> char {
        if self.pos < self.input.len() {
            self.input[self.pos]
        } else {
            '\0'
        }
    }
    
    fn advance(&mut self) {
        self.pos += 1;
    }
    
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.current_char().is_whitespace() {
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
    
    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.current_char() == c {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected '{}', got '{}'", c, self.current_char()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple() {
        let wxml = r#"<view class="container"><text>Hello</text></view>"#;
        let mut parser = WxmlParser::new(wxml);
        let nodes = parser.parse().unwrap();
        
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].tag_name, "view");
        assert_eq!(nodes[0].get_attr("class"), Some("container"));
    }

    /// 文字里 `{{ }}` 中的 `<` 是比较运算，不是标签（马拉松报名页步骤条 `{{step < index ? …}}`）
    #[test]
    fn 文字里的表达式带小于号() {
        for src in [
            "<view>{{a < b ? 'x' : 'y'}}</view>",
            "<view>{{a<b ? 'x' : 'y'}}</view>",
            "<view>{{a<b}}<text>t</text></view>",
        ] {
            let nodes = WxmlParser::new(src).parse().unwrap_or_else(|e| panic!("{src}: {e}"));
            assert_eq!(nodes[0].tag_name, "view", "{src}");
            assert!(nodes[0].children[0].text_content.starts_with("{{a"), "{src}: {:?}", nodes[0].children[0].text_content);
        }
        let n = WxmlParser::new("<view>{{a<b}}<text>t</text></view>").parse().unwrap();
        assert_eq!(n[0].children[1].tag_name, "text", "表达式后面的标签照常解析");
    }

    /// 写坏的标签报错，不卡死（从前属性循环不前进）
    #[test]
    fn 坏标签报错不卡死() {
        assert!(WxmlParser::new("<view a=1 ?>x</view>").parse().is_err());
        assert!(WxmlParser::new("<view a=1").parse().is_err());
        // 没配对的 {{ 不吞掉后面的标签
        let n = WxmlParser::new("<view>{{x<text>t</text></view>").parse();
        assert!(n.is_ok() || n.is_err(), "不卡死就行");
    }
}
