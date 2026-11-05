//! rich-text 的 `nodes`（节点数组，或 HTML 字符串经 html.rs 解析成的同形数组）→ 真正的 WXML 子树。
//!
//! 在模板展开时做（parser/template/emit.rs）：生成的节点挂成 `<rich-text>` 的孩子，之后建树、样式（class 匹配页面 /
//! 组件 WXSS、内联 style 全量）、布局、绘制、增量失效都走与普通节点同一条管线。映射规则：
//! - 块级标签 → `rt-<标签>`（按 view 建，默认样式见 ua.rs）；行内内容按「段落」收拢：
//!   一段里只有文字和 `<br>` 时是一个 `<text>`（一个节点，换行交给文本度量 —— 长文的节点数 ≈ 段落数），
//!   有行内标签 / 图片时是一个 `<span>`，孩子是文字、`rt-b` 这类行内标签（可多层）、`<image>`（components/inline_flow.rs 拆 run）；
//! - `<img>` → `<image>`（src；width / height 属性折成样式；没写高就 `widthFix`，没写宽就 `max-width:100%`）；
//! - `<br>` → 换行；`<li>` 前面挂项目符号（ul：• ◦ ▪ 随层级，ol：数字 / 字母 / 罗马数字，认 `start` / `type`）；
//! - 只认官方白名单里的标签（[`TRUSTED`]），之外的**连子树一起丢**（官方 tip）；属性只留 class、style 与各标签自己的那几个，
//!   不带 id；生成的节点不带任何事件绑定（官方：rich-text 内屏蔽所有节点的事件），点到里面的字冒泡到 rich-text 自己；
//! - 文字节点解实体；空白按 HTML 规则合并（`<pre>` 里与写了 `space` 时保留，`space` 时换成那种空格）。
use super::html;
use crate::parser::wxml::{WxmlNode, WxmlNodeType};
use serde_json::Value;

/// 官方 rich-text 页「受信任的 HTML 节点」
const TRUSTED: &[&str] = &[
    "a", "abbr", "address", "article", "aside", "b", "bdi", "bdo", "big", "blockquote", "br", "caption", "center", "cite", "code", "col",
    "colgroup", "dd", "del", "div", "dl", "dt", "em", "fieldset", "font", "footer", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "i",
    "img", "ins", "label", "legend", "li", "mark", "nav", "ol", "p", "pre", "q", "rt", "ruby", "s", "section", "small", "span", "strong",
    "sub", "sup", "table", "tbody", "td", "tfoot", "th", "thead", "tr", "tt", "u", "ul",
];

/// 行内标签（其余白名单标签按块级）
const INLINE: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "big", "cite", "code", "del", "em", "font", "i", "ins", "label", "mark", "q", "rt", "ruby", "s", "small",
    "span", "strong", "sub", "sup", "tt", "u",
];

/// 递归上限（HTML 那条路在 html.rs 已经限过；节点数组可以任意深）
const MAX_DEPTH: usize = html::MAX_DEPTH;

pub fn is_inline_html(name: &str) -> bool {
    INLINE.contains(&name)
}

/// 转换选项
#[derive(Clone, Copy, Default)]
pub struct Opts {
    /// `space` 属性：连续空格保留并换成这种空格
    pub space: Option<char>,
}

/// 入口：`nodes` 的取值（数组 / 单个节点对象 / HTML 字符串）→ rich-text 的孩子
pub fn expand(nodes: &Value, opts: Opts) -> Vec<WxmlNode> {
    let parsed;
    let list: &[Value] = match nodes {
        Value::Array(a) => a,
        Value::String(s) if s.trim().is_empty() => return Vec::new(),
        Value::String(s) => {
            parsed = html::parse(s);
            parsed.as_array().map(|a| a.as_slice()).unwrap_or(&[])
        }
        Value::Object(_) => std::slice::from_ref(nodes),
        _ => return Vec::new(),
    };
    let mut cx = Cx { opts, pre: 0, lists: 0 };
    cx.blocks(list, 0)
}

struct Cx {
    opts: Opts,
    /// 在几层 `<pre>` 里（空白保留）
    pre: usize,
    /// 在几层列表里（项目符号样式、嵌套列表的外边距）
    lists: usize,
}

/// 节点的种类
enum Kind<'a> {
    Text(&'a str),
    Elem(String),
    /// 不受信任 / 认不出：连子树丢
    Drop,
}

fn kind(v: &Value) -> Kind<'_> {
    let Some(o) = v.as_object() else {
        return match v.as_str() {
            Some(s) => Kind::Text(s),
            None => Kind::Drop,
        };
    };
    let ty = o.get("type").and_then(Value::as_str).unwrap_or("node");
    if ty == "text" {
        return Kind::Text(o.get("text").and_then(Value::as_str).unwrap_or(""));
    }
    let Some(name) = o.get("name").and_then(Value::as_str) else {
        // 没写 name 只写了 text 的（老写法）当文本
        return o.get("text").and_then(Value::as_str).map(Kind::Text).unwrap_or(Kind::Drop);
    };
    // 官方：name 大小写不敏感
    let name = name.trim().to_ascii_lowercase();
    if TRUSTED.contains(&name.as_str()) { Kind::Elem(name) } else { Kind::Drop }
}

fn children_of(v: &Value) -> &[Value] {
    v.get("children").and_then(Value::as_array).map(|a| a.as_slice()).unwrap_or(&[])
}

fn attr<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    let a = v.get("attrs")?.as_object()?;
    // 官方说 attrs「遵循 Pascal 命名法」，实际写法大小写都有：不分大小写找
    a.iter().find(|(key, _)| key.eq_ignore_ascii_case(k)).and_then(|(_, v)| match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    })
}

/// 行内（能进段落）：文字、br、img、行内标签且子树里没有块
fn is_inline(v: &Value, depth: usize) -> bool {
    match kind(v) {
        Kind::Text(_) | Kind::Drop => true,
        Kind::Elem(n) if n == "br" || n == "img" => true,
        Kind::Elem(n) => is_inline_html(&n) && depth < MAX_DEPTH && children_of(v).iter().all(|c| is_inline(c, depth + 1)),
    }
}

/// class / style 照抄（官方：全局支持 class 和 style，不支持 id）
fn element(tag: &str, src: &Value) -> WxmlNode {
    let mut n = WxmlNode::new_element(tag);
    for k in ["class", "style"] {
        if let Some(v) = attr(src, k).map(str::trim).filter(|v| !v.is_empty()) {
            n.attributes.insert(k.to_string(), v.to_string());
        }
    }
    n
}

impl Cx {
    /// 一串兄弟（块级上下文）：行内的连续几个收成一段，块级的各自成块
    fn blocks(&mut self, list: &[Value], depth: usize) -> Vec<WxmlNode> {
        let mut out = Vec::new();
        let mut run: Vec<&Value> = Vec::new();
        for v in list {
            if is_inline(v, depth) {
                run.push(v);
            } else {
                self.paragraph(&run, depth, &mut out);
                run.clear();
                if let Some(b) = self.block(v, depth) {
                    out.push(b);
                }
            }
        }
        self.paragraph(&run, depth, &mut out);
        out
    }

    fn block(&mut self, v: &Value, depth: usize) -> Option<WxmlNode> {
        let Kind::Elem(name) = kind(v) else { return None };
        if depth >= MAX_DEPTH {
            return None;
        }
        let kids = children_of(v);
        match name.as_str() {
            "hr" => Some(element("rt-hr", v)),
            // 列宽 / 列组：只是表格布局的提示，不产出内容
            "col" | "colgroup" => None,
            "ul" | "ol" => {
                let tag = match (name.as_str(), self.lists > 0) {
                    ("ul", false) => "rt-ul",
                    ("ul", true) => "rt-ul-nested",
                    (_, false) => "rt-ol",
                    _ => "rt-ol-nested",
                };
                let mut n = element(tag, v);
                let start = attr(v, "start").and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(1);
                let ty = attr(v, "type").map(str::trim).unwrap_or("1").to_string();
                let level = self.lists;
                self.lists += 1;
                let mut k = 0i64;
                for c in kids {
                    match kind(c) {
                        Kind::Elem(cn) if cn == "li" => {
                            let marker = if name == "ul" { ["•", "◦", "▪"][level.min(2)].to_string() } else { ol_marker(start + k, &ty) };
                            k += 1;
                            if let Some(li) = self.list_item(c, &marker, depth + 1) {
                                n.children.push(li);
                            }
                        }
                        // 列表里直接写的文字 / 别的块：照常排（浏览器也是这样显示的）
                        _ => n.children.extend(self.blocks(std::slice::from_ref(c), depth + 1)),
                    }
                }
                self.lists -= 1;
                Some(n)
            }
            "li" => self.list_item(v, "•", depth),
            "pre" => {
                let mut n = element("rt-pre", v);
                self.pre += 1;
                n.children = self.blocks(kids, depth + 1);
                self.pre -= 1;
                Some(n)
            }
            _ => {
                let mut n = element(&format!("rt-{name}"), v);
                n.children = self.blocks(kids, depth + 1);
                Some(n)
            }
        }
    }

    fn list_item(&mut self, v: &Value, marker: &str, depth: usize) -> Option<WxmlNode> {
        let mut li = element("rt-li", v);
        let mut m = WxmlNode::new_element("rt-marker");
        m.children.push(WxmlNode::new_text(marker));
        li.children.push(m);
        li.children.extend(self.blocks(children_of(v), depth + 1));
        Some(li)
    }

    /// 一段行内内容 → 一个 `<text>`（只有文字 / 换行）或一个 `<span>`（有行内标签 / 图片）
    fn paragraph(&mut self, run: &[&Value], depth: usize, out: &mut Vec<WxmlNode>) {
        if run.is_empty() {
            return;
        }
        let plain = run.iter().all(|v| match kind(v) {
            Kind::Text(_) | Kind::Drop => true,
            Kind::Elem(n) => n == "br",
        });
        let keep_ws = self.pre > 0 || self.opts.space.is_some();
        if plain {
            let mut s = String::new();
            for v in run {
                match kind(v) {
                    Kind::Text(t) => s.push_str(&self.text(t)),
                    Kind::Elem(_) => s.push('\n'),
                    Kind::Drop => {}
                }
            }
            let s = if keep_ws { s } else { trim_lines(&s) };
            if s.trim().is_empty() && !s.contains('\n') && !(keep_ws && !s.is_empty()) {
                return;
            }
            let mut t = WxmlNode::new_element("text");
            if self.pre > 0 {
                t.attributes.insert("style".into(), "white-space:pre".into());
            }
            t.children.push(WxmlNode::new_text(&s));
            out.push(t);
            return;
        }
        let mut span = WxmlNode::new_element("span");
        for v in run {
            if let Some(n) = self.inline(v, depth + 1) {
                span.children.push(n);
            }
        }
        if !keep_ws {
            trim_edges(&mut span);
        }
        if !span.children.is_empty() {
            out.push(span);
        }
    }

    fn inline(&mut self, v: &Value, depth: usize) -> Option<WxmlNode> {
        match kind(v) {
            Kind::Text(t) => {
                let s = self.text(t);
                (!s.is_empty()).then(|| WxmlNode::new_text(&s))
            }
            Kind::Drop => None,
            Kind::Elem(n) if n == "br" => Some(WxmlNode::new_text("\n")),
            Kind::Elem(n) if n == "img" => Some(image(v)),
            Kind::Elem(n) => {
                if depth >= MAX_DEPTH {
                    return None;
                }
                let mut e = element(&format!("rt-{n}"), v);
                for c in children_of(v) {
                    if let Some(k) = self.inline(c, depth + 1) {
                        e.children.push(k);
                    }
                }
                Some(e)
            }
        }
    }

    /// 文字：解实体；按 `space` / pre / HTML 规则处理空白
    fn text(&self, raw: &str) -> String {
        let s = html::decode(raw);
        if let Some(c) = self.opts.space {
            let mut b = [0u8; 4];
            return s.replace(' ', c.encode_utf8(&mut b));
        }
        if self.pre > 0 {
            return s;
        }
        collapse(&s)
    }
}

/// 连续空白（含换行）合成一个空格（HTML `white-space: normal`）。实体解出来的 nbsp 不算空白
fn collapse(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut ws = false;
    for ch in s.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0c') {
            if !ws {
                out.push(' ');
            }
            ws = true;
        } else {
            out.push(ch);
            ws = false;
        }
    }
    out
}

/// 纯文字段落：每行（`<br>` 切开的）去掉首尾空格（行首行尾的空白在浏览器里不占位）
fn trim_lines(s: &str) -> String {
    s.split('\n').map(str::trim).collect::<Vec<_>>().join("\n")
}

/// 混排段落：第一个文字去掉开头空白、最后一个去掉结尾空白（逐层往里找）
fn trim_edges(span: &mut WxmlNode) {
    fn first_text(n: &mut WxmlNode) -> Option<&mut WxmlNode> {
        for c in n.children.iter_mut() {
            if c.node_type == WxmlNodeType::Text {
                return Some(c);
            }
            if c.tag_name == "image" {
                return None;
            }
            if let Some(t) = first_text(c) {
                return Some(t);
            }
        }
        None
    }
    fn last_text(n: &mut WxmlNode) -> Option<&mut WxmlNode> {
        for c in n.children.iter_mut().rev() {
            if c.node_type == WxmlNodeType::Text {
                return Some(c);
            }
            if c.tag_name == "image" {
                return None;
            }
            if let Some(t) = last_text(c) {
                return Some(t);
            }
        }
        None
    }
    if let Some(t) = first_text(span) {
        t.text_content = t.text_content.trim_start_matches(' ').to_string();
    }
    if let Some(t) = last_text(span) {
        t.text_content = t.text_content.trim_end_matches(' ').to_string();
    }
}

/// `<img>` → `<image>`：只认 src / width / height（官方白名单还有 alt：没有可显示的地方，丢）
fn image(v: &Value) -> WxmlNode {
    let mut n = WxmlNode::new_element("image");
    if let Some(src) = attr(v, "src") {
        n.attributes.insert("src".into(), src.trim().to_string());
    }
    if let Some(c) = attr(v, "class").map(str::trim).filter(|c| !c.is_empty()) {
        n.attributes.insert("class".into(), c.to_string());
    }
    let user = attr(v, "style").unwrap_or("").trim();
    let dim = |k: &str| {
        attr(v, k).map(str::trim).filter(|s| !s.is_empty()).map(|s| {
            if s.chars().all(|c| c.is_ascii_digit() || c == '.') { format!("{s}px") } else { s.to_string() }
        })
    };
    let (w, h) = (dim("width"), dim("height"));
    let has = |k: &str| user.split(';').any(|d| d.split(':').next().map(|p| p.trim().eq_ignore_ascii_case(k)).unwrap_or(false));
    // 属性在前、内联 style 在后：style 优先（与浏览器里「属性是表现提示、style 盖过它」一致）
    let mut style = String::new();
    if let Some(w) = &w {
        style.push_str(&format!("width:{w};"));
    }
    if let Some(h) = &h {
        style.push_str(&format!("height:{h};"));
    }
    if w.is_none() && !has("width") {
        style.push_str("max-width:100%;");
    }
    style.push_str(user);
    if h.is_none() && !has("height") {
        n.attributes.insert("mode".into(), "widthFix".into());
    }
    if !style.is_empty() {
        n.attributes.insert("style".into(), style);
    }
    n
}

/// 有序列表的序号（`type`：1 / a / A / i / I）
fn ol_marker(n: i64, ty: &str) -> String {
    let body = match ty {
        "a" | "A" if n >= 1 => {
            let mut k = n;
            let mut s = Vec::new();
            while k > 0 {
                k -= 1;
                s.push((b'a' + (k % 26) as u8) as char);
                k /= 26;
            }
            let s: String = s.into_iter().rev().collect();
            if ty == "A" { s.to_ascii_uppercase() } else { s }
        }
        "i" | "I" if (1..4000).contains(&n) => {
            let table = [(1000, "m"), (900, "cm"), (500, "d"), (400, "cd"), (100, "c"), (90, "xc"), (50, "l"), (40, "xl"), (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i")];
            let mut k = n;
            let mut s = String::new();
            for (v, r) in table {
                while k >= v {
                    s.push_str(r);
                    k -= v;
                }
            }
            if ty == "I" { s.to_ascii_uppercase() } else { s }
        }
        _ => n.to_string(),
    };
    format!("{body}.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn 白名单外连子树丢_段落收拢() {
        let v = expand(&json!("<div>a<b>b</b><video><p>x</p></video></div><p>纯文字<br>第二行</p>"), Opts::default());
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].tag_name, "rt-div");
        assert_eq!(v[0].children[0].tag_name, "span", "有行内标签的段落是 span");
        assert_eq!(v[0].children.len(), 1, "video 连子树丢掉");
        assert_eq!(v[1].children[0].tag_name, "text", "纯文字段落是一个 text");
        assert_eq!(v[1].children[0].children[0].text_content, "纯文字\n第二行");
    }

    #[test]
    fn 序号() {
        assert_eq!(ol_marker(3, "1"), "3.");
        assert_eq!(ol_marker(28, "a"), "ab.");
        assert_eq!(ol_marker(14, "I"), "XIV.");
    }
}
