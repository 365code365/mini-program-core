//! rich-text 的 `nodes` 写成 HTML 字符串时：解析成与节点数组同一形状的 JSON
//! （`[{name, attrs:{class, style, src…}, children:[…]}, {type:"text", text}]`），之后与数组走同一条转换（tree.rs）。
//!
//! 自己写的小分词器，不引依赖：rich-text 收到的多半是 CMS / 公众号编辑器吐出来的片段，要的是**容错**而不是规范全集：
//! - 标签名、属性名不分大小写；属性值可以带双引号、单引号或不带引号，也可以只有名字（值为空串）；
//! - `<!-- 注释 -->`、`<!DOCTYPE …>`、`<?xml …?>` 丢掉；`<script>` / `<style>` 连内容一起丢（找不到结束标签就丢到末尾）；
//! - 空元素（br / img / hr …）不需要闭合；`<x/>` 自闭合；
//! - 没闭合的标签在末尾自动闭合；对不上的结束标签：栈里有同名的就一路关到它，没有就忽略；
//! - `<p>` 碰到块级开始标签自动结束、`<li>` 碰到下一个 `<li>` 自动结束（同列表内），`<td>/<th>/<tr>` 同理 —— HTML 的隐式结束规则里最常见的几条；
//! - `<` 后面不是字母、`/`、`!`、`?` 时按普通文字（`a < b`）；
//! - 实体解码：常见命名实体 + `&#123;` / `&#x1F600;`，认不出的原样留着；
//! - 嵌套深度上限 [`MAX_DEPTH`]，再深的开始标签当作不存在（只留文字）：防病态输入把后面的递归压爆栈。
use serde_json::{json, Map, Value};

/// 嵌套深度上限（正常文章 < 20 层）
pub const MAX_DEPTH: usize = 128;

const VOID: &[&str] = &["br", "img", "hr", "input", "meta", "link", "col", "area", "base", "wbr", "source", "embed", "param", "track"];

/// 开始时会隐式结束一个打开着的 `<p>` 的标签
const CLOSES_P: &[&str] = &[
    "address", "article", "aside", "blockquote", "div", "dl", "fieldset", "footer", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr",
    "nav", "ol", "p", "pre", "section", "table", "ul",
];

struct Open {
    name: String,
    attrs: Map<String, Value>,
    children: Vec<Value>,
}

/// HTML 片段 → 节点数组
pub fn parse(html: &str) -> Value {
    let mut stack: Vec<Open> = vec![Open { name: String::new(), attrs: Map::new(), children: Vec::new() }];
    let b = html.as_bytes();
    let mut i = 0;
    let mut text_start = 0;
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let next = b.get(i + 1).copied().unwrap_or(0);
        if !(next.is_ascii_alphabetic() || next == b'/' || next == b'!' || next == b'?') {
            i += 1;
            continue;
        }
        flush_text(&mut stack, &html[text_start..i]);
        if html[i..].starts_with("<!--") {
            i = html[i + 4..].find("-->").map(|p| i + 4 + p + 3).unwrap_or(b.len());
        } else if next == b'!' || next == b'?' {
            i = html[i..].find('>').map(|p| i + p + 1).unwrap_or(b.len());
        } else if next == b'/' {
            let (name, end) = read_name(html, i + 2);
            i = html[end..].find('>').map(|p| end + p + 1).unwrap_or(b.len());
            close(&mut stack, &name);
        } else {
            let (name, end) = read_name(html, i + 1);
            let (attrs, self_closed, end) = read_attrs(html, end);
            i = end;
            if name == "script" || name == "style" {
                // 原始文本元素：内容不是 HTML，整段丢掉
                let lower = html[i..].to_ascii_lowercase();
                i = lower.find(&format!("</{name}")).map(|p| i + p).unwrap_or(b.len());
                i = html[i..].find('>').map(|p| i + p + 1).unwrap_or(b.len());
            } else {
                open(&mut stack, name, attrs, self_closed);
            }
        }
        text_start = i;
    }
    flush_text(&mut stack, &html[text_start.min(html.len())..]);
    while stack.len() > 1 {
        pop(&mut stack);
    }
    Value::Array(stack.pop().map(|r| r.children).unwrap_or_default())
}

fn flush_text(stack: &mut [Open], raw: &str) {
    if raw.is_empty() {
        return;
    }
    if let Some(top) = stack.last_mut() {
        // 文字原样（实体在 tree.rs 统一解：节点数组里的文本节点也「支持 entities」，这里先解就解两遍了）
        top.children.push(json!({ "type": "text", "text": raw }));
    }
}

fn read_name(s: &str, from: usize) -> (String, usize) {
    let b = s.as_bytes();
    let mut j = from;
    while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-' || b[j] == b':' || b[j] == b'_') {
        j += 1;
    }
    (s[from..j].to_ascii_lowercase(), j)
}

/// 读到 `>` 为止：(属性表, 是否 `/>` 自闭合, `>` 之后的位置)
fn read_attrs(s: &str, from: usize) -> (Map<String, Value>, bool, usize) {
    let b = s.as_bytes();
    let mut attrs = Map::new();
    let mut j = from;
    let mut self_closed = false;
    loop {
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= b.len() {
            return (attrs, self_closed, j);
        }
        match b[j] {
            b'>' => return (attrs, self_closed, j + 1),
            b'/' => {
                self_closed = true;
                j += 1;
                continue;
            }
            _ => {}
        }
        self_closed = false;
        let start = j;
        while j < b.len() && !b[j].is_ascii_whitespace() && !matches!(b[j], b'=' | b'>' | b'/') {
            j += 1;
        }
        let name = s[start..j].to_ascii_lowercase();
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        let mut value = String::new();
        if j < b.len() && b[j] == b'=' {
            j += 1;
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < b.len() && (b[j] == b'"' || b[j] == b'\'') {
                let q = b[j];
                let vs = j + 1;
                j = vs;
                while j < b.len() && b[j] != q {
                    j += 1;
                }
                value = decode(&s[vs..j]);
                j = (j + 1).min(b.len());
            } else {
                let vs = j;
                while j < b.len() && !b[j].is_ascii_whitespace() && b[j] != b'>' {
                    j += 1;
                }
                value = decode(&s[vs..j]);
            }
        }
        if !name.is_empty() {
            attrs.insert(name, Value::String(value));
        }
        if start == j {
            // 读不动（不该发生）：跳过一个字节，保证前进
            j += 1;
        }
    }
}

fn open(stack: &mut Vec<Open>, name: String, attrs: Map<String, Value>, self_closed: bool) {
    // 隐式结束
    if CLOSES_P.contains(&name.as_str()) && stack.last().is_some_and(|t| t.name == "p") {
        pop(stack);
    }
    let sibling_closer = |n: &str| -> Option<(&'static [&'static str], &'static [&'static str])> {
        match n {
            "li" => Some((&["li"], &["ul", "ol"])),
            "dt" | "dd" => Some((&["dt", "dd"], &["dl"])),
            "tr" => Some((&["tr"], &["table", "thead", "tbody", "tfoot"])),
            "td" | "th" => Some((&["td", "th"], &["tr", "table"])),
            _ => None,
        }
    };
    if let Some((same, scope)) = sibling_closer(&name) {
        if let Some(k) = stack.iter().rposition(|o| same.contains(&o.name.as_str()) || scope.contains(&o.name.as_str())) {
            if same.contains(&stack[k].name.as_str()) {
                while stack.len() > k {
                    pop(stack);
                }
            }
        }
    }
    if VOID.contains(&name.as_str()) || self_closed {
        if let Some(top) = stack.last_mut() {
            top.children.push(json!({ "name": name, "attrs": Value::Object(attrs) }));
        }
        return;
    }
    if stack.len() > MAX_DEPTH {
        return;
    }
    stack.push(Open { name, attrs, children: Vec::new() });
}

fn close(stack: &mut Vec<Open>, name: &str) {
    if let Some(k) = stack.iter().skip(1).rposition(|o| o.name == name) {
        let k = k + 1;
        while stack.len() > k {
            pop(stack);
        }
    }
}

fn pop(stack: &mut Vec<Open>) {
    if stack.len() <= 1 {
        return;
    }
    let o = stack.pop().expect("len > 1");
    let v = json!({ "name": o.name, "attrs": Value::Object(o.attrs), "children": o.children });
    if let Some(top) = stack.last_mut() {
        top.children.push(v);
    }
}

/// HTML 实体解码（常见命名实体 + 数字实体）。官方：文本节点「支持 entities」
pub fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        let tail = &rest[p + 1..];
        let semi = tail.find(';').filter(|&k| k > 0 && k <= 10);
        let ch = semi.and_then(|k| entity(&tail[..k]));
        match (semi, ch) {
            (Some(k), Some(c)) => {
                out.push(c);
                rest = &tail[k + 1..];
            }
            _ => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    if let Some(num) = name.strip_prefix('#') {
        let code = match num.strip_prefix(['x', 'X']) {
            Some(h) => u32::from_str_radix(h, 16).ok()?,
            None => num.parse::<u32>().ok()?,
        };
        return char::from_u32(code).filter(|c| *c != '\0');
    }
    Some(match name {
        "nbsp" => '\u{a0}',
        "lt" => '<',
        "gt" => '>',
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        "ensp" => '\u{2002}',
        "emsp" => '\u{2003}',
        "thinsp" => '\u{2009}',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "hellip" => '…',
        "mdash" => '—',
        "ndash" => '–',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "middot" => '·',
        "bull" => '•',
        "times" => '×',
        "divide" => '÷',
        "yen" => '¥',
        "cent" => '¢',
        "pound" => '£',
        "euro" => '€',
        "deg" => '°',
        "plusmn" => '±',
        "laquo" => '«',
        "raquo" => '»',
        "sect" => '§',
        "para" => '¶',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 容错_未闭合_无引号_注释_脚本() {
        let v = parse("<!-- x --><P class=a STYLE='color:red'>hi<b>yo<script>alert(1)</script><br>x &amp; y &#x41;");
        let s = v.to_string();
        assert!(!s.contains("alert"), "script 连内容丢掉：{s}");
        assert!(!s.contains("<!--"), "{s}");
        let p = &v[0];
        assert_eq!(p["name"], "p");
        assert_eq!(p["attrs"]["class"], "a");
        assert_eq!(p["attrs"]["style"], "color:red");
        assert_eq!(p["children"][1]["name"], "b", "{s}");
        assert!(s.contains("x &amp; y &#x41;"), "文字原样留给 tree.rs 解码：{s}");
        assert_eq!(decode("x &amp; y &#x41;&#128512;&bogus; &"), "x & y A😀&bogus; &");
    }

    #[test]
    fn 隐式结束_p_与_li() {
        let v = parse("<p>a<p>b<ul><li>1<li>2</ul>");
        assert_eq!(v.as_array().unwrap().len(), 3, "{v}");
        assert_eq!(v[2]["children"].as_array().unwrap().len(), 2, "{v}");
        assert_eq!(parse("a < b")[0]["text"], "a < b");
    }
}
