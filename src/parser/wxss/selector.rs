//! CSS 选择器引擎：分词、解析成复合/复杂选择器、特异性、以及对元素链的匹配。
//!
//! 从 1453 行的 wxss.rs 拆出来（**纯搬迁**）。这里是「一个选择器长什么样、怎么判断它
//! 命中没命中」；`parser.rs` 只管把文本切成规则，`mod.rs` 管样式表的查询与层叠。
//!
//! 关键取舍：选择器在**解析期就编译**成这里的结构（`StyleRule` 直接持有），匹配时不再
//! 分词 —— 匹配是「每个节点 × 每条规则」的二重循环，现场解析字符串会让一次全页重建
//! 做十万次无用功（建树+样式 28.7ms → 2.9ms 就是这么来的）。
use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum AttrOp {
    Exists,   // [a]
    Eq,       // [a=b]
    Prefix,   // [a^=b]
    Suffix,   // [a$=b]
    Contains, // [a*=b]
    Dash,     // [a|=b]
    Word,     // [a~=b]
}

#[derive(Debug, Clone)]
pub(super) struct AttrSel {
    pub(super) name: String,
    pub(super) op: AttrOp,
    pub(super) value: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Combinator {
    Descendant,
    Child,
    NextSibling,
    SubsequentSibling,
}

/// 单个复合选择器，如 `view.item#main[data-x]:first-child`
#[derive(Debug, Clone, Default)]
pub(super) struct Compound {
    pub(super) universal: bool,
    pub(super) tag: Option<String>,
    pub(super) id: Option<String>,
    pub(super) classes: Vec<String>,
    pub(super) attrs: Vec<AttrSel>,
    pub(super) pseudos: Vec<String>,
}

/// 复合选择器序列（含组合器），如 `.a > .b .c`
#[derive(Debug, Clone)]
pub(super) struct ComplexSelector {
    /// 从左到右：第一个 combinator 为 None
    pub(super) seq: Vec<(Option<Combinator>, Compound)>,
    pub(super) specificity: u32,
}

/// 按逗号切分选择器组（忽略 [] () 内的逗号）
pub(super) fn split_selector_groups(sel: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let mut buf = String::new();
    let mut depth = 0i32;
    for c in sel.chars() {
        match c {
            '[' | '(' => { depth += 1; buf.push(c); }
            ']' | ')' => { depth -= 1; buf.push(c); }
            ',' if depth == 0 => {
                if !buf.trim().is_empty() { groups.push(buf.trim().to_string()); }
                buf.clear();
            }
            _ => buf.push(c),
        }
    }
    if !buf.trim().is_empty() { groups.push(buf.trim().to_string()); }
    groups
}

pub(super) enum Tok {
    Compound(String),
    Comb(Combinator),
}

/// 将复合序列拆成 token（复合 / 组合器），正确处理 [] () 内的空白与符号
pub(super) fn tokenize_complex(sel: &str) -> Vec<Tok> {
    let mut tokens: Vec<Tok> = Vec::new();
    let mut buf = String::new();
    let mut depth = 0i32;
    let mut want_desc = false;
    
    let is_last_compound = |t: &[Tok]| matches!(t.last(), Some(Tok::Compound(_)));
    
    for c in sel.chars() {
        if depth > 0 {
            if c == '[' || c == '(' { depth += 1; }
            else if c == ']' || c == ')' { depth -= 1; }
            buf.push(c);
            continue;
        }
        match c {
            '[' | '(' => { depth += 1; buf.push(c); }
            c if c.is_whitespace() => {
                if !buf.is_empty() {
                    tokens.push(Tok::Compound(std::mem::take(&mut buf)));
                }
                if is_last_compound(&tokens) {
                    want_desc = true;
                }
            }
            '>' | '+' | '~' => {
                if !buf.is_empty() {
                    tokens.push(Tok::Compound(std::mem::take(&mut buf)));
                }
                let comb = match c {
                    '>' => Combinator::Child,
                    '+' => Combinator::NextSibling,
                    _ => Combinator::SubsequentSibling,
                };
                tokens.push(Tok::Comb(comb));
                want_desc = false;
            }
            _ => {
                if buf.is_empty() && want_desc && is_last_compound(&tokens) {
                    tokens.push(Tok::Comb(Combinator::Descendant));
                }
                want_desc = false;
                buf.push(c);
            }
        }
    }
    if !buf.is_empty() {
        tokens.push(Tok::Compound(buf));
    }
    tokens
}

pub(super) fn parse_complex(sel: &str) -> Option<ComplexSelector> {
    let tokens = tokenize_complex(sel.trim());
    if tokens.is_empty() {
        return None;
    }
    
    let mut seq: Vec<(Option<Combinator>, Compound)> = Vec::new();
    let mut pending: Option<Combinator> = None;
    let mut specificity = 0u32;
    
    for tok in tokens {
        match tok {
            Tok::Comb(c) => pending = Some(c),
            Tok::Compound(s) => {
                let comp = parse_compound(&s)?;
                specificity += compound_specificity(&comp);
                seq.push((pending.take(), comp));
            }
        }
    }
    
    if seq.is_empty() {
        return None;
    }
    Some(ComplexSelector { seq, specificity })
}

pub(super) fn parse_compound(s: &str) -> Option<Compound> {
    let chars: Vec<char> = s.chars().collect();
    let mut comp = Compound::default();
    let mut i = 0;
    let n = chars.len();
    
    while i < n {
        match chars[i] {
            '*' => { comp.universal = true; i += 1; }
            '.' => {
                i += 1;
                let name = read_ident(&chars, &mut i);
                if name.is_empty() { return None; }
                comp.classes.push(name);
            }
            '#' => {
                i += 1;
                let name = read_ident(&chars, &mut i);
                if name.is_empty() { return None; }
                comp.id = Some(name);
            }
            '[' => {
                i += 1; // skip [
                let mut inner = String::new();
                while i < n && chars[i] != ']' {
                    inner.push(chars[i]);
                    i += 1;
                }
                if i < n { i += 1; } // skip ]
                if let Some(a) = parse_attr(&inner) {
                    comp.attrs.push(a);
                }
            }
            ':' => {
                i += 1;
                if i < n && chars[i] == ':' { i += 1; } // 伪元素 ::，当作伪类名收集
                let mut name = read_ident(&chars, &mut i);
                // 函数式伪类：:nth-child(...)
                if i < n && chars[i] == '(' {
                    let mut depth = 1;
                    name.push('(');
                    i += 1;
                    while i < n && depth > 0 {
                        if chars[i] == '(' { depth += 1; }
                        else if chars[i] == ')' { depth -= 1; }
                        name.push(chars[i]);
                        i += 1;
                    }
                }
                if !name.is_empty() { comp.pseudos.push(name); }
            }
            c if is_ident_char(c) => {
                let name = read_ident(&chars, &mut i);
                comp.tag = Some(name.to_lowercase());
            }
            _ => { i += 1; }
        }
    }
    
    Some(comp)
}

pub(super) fn read_ident(chars: &[char], i: &mut usize) -> String {
    let mut s = String::new();
    while *i < chars.len() && is_ident_char(chars[*i]) {
        s.push(chars[*i]);
        *i += 1;
    }
    s
}

pub(super) fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_'
}

pub(super) fn parse_attr(inner: &str) -> Option<AttrSel> {
    let inner = inner.trim();
    for (sym, op) in [("^=", AttrOp::Prefix), ("$=", AttrOp::Suffix), ("*=", AttrOp::Contains),
                      ("|=", AttrOp::Dash), ("~=", AttrOp::Word), ("=", AttrOp::Eq)] {
        if let Some(pos) = inner.find(sym) {
            let name = inner[..pos].trim().to_string();
            let mut value = inner[pos + sym.len()..].trim().to_string();
            // 去引号
            if (value.starts_with('"') && value.ends_with('"')) ||
               (value.starts_with('\'') && value.ends_with('\'')) {
                if value.len() >= 2 { value = value[1..value.len() - 1].to_string(); }
            }
            if name.is_empty() { return None; }
            return Some(AttrSel { name, op, value });
        }
    }
    if inner.is_empty() { return None; }
    Some(AttrSel { name: inner.to_string(), op: AttrOp::Exists, value: String::new() })
}

pub(super) fn compound_specificity(c: &Compound) -> u32 {
    let mut id = 0u32;
    let mut cls = 0u32;
    let mut ty = 0u32;
    if c.id.is_some() { id += 1; }
    cls += c.classes.len() as u32;
    cls += c.attrs.len() as u32;
    cls += c.pseudos.len() as u32; // 近似：伪类计入 class 级
    if c.tag.is_some() { ty += 1; }
    id * 10_000 + cls * 100 + ty
}

// ============================ 选择器匹配 ============================

/// 基于祖先链的复合序列匹配（右到左）
pub(super) fn matches_complex(cs: &ComplexSelector, chain: &[ElementDesc]) -> bool {
    let n = cs.seq.len();
    let subject = chain.len() as isize - 1;
    if subject < 0 {
        return false;
    }
    // 最右复合匹配目标元素
    if !compound_matches_desc(&cs.seq[n - 1].1, &chain[subject as usize]) {
        return false;
    }
    let mut elem = subject;
    let mut si = n as isize - 2;
    while si >= 0 {
        let comb = cs.seq[(si + 1) as usize].0.unwrap_or(Combinator::Descendant);
        let compound = &cs.seq[si as usize].1;
        match comb {
            Combinator::Descendant => {
                let mut a = elem - 1;
                let mut found = false;
                while a >= 0 {
                    if compound_matches_desc(compound, &chain[a as usize]) {
                        elem = a;
                        found = true;
                        break;
                    }
                    a -= 1;
                }
                if !found {
                    return false;
                }
            }
            Combinator::Child => {
                let a = elem - 1;
                if a < 0 || !compound_matches_desc(compound, &chain[a as usize]) {
                    return false;
                }
                elem = a;
            }
            // 兄弟组合器需要兄弟上下文，暂不支持
            Combinator::NextSibling | Combinator::SubsequentSibling => return false,
        }
        si -= 1;
    }
    true
}

/// 复合选择器与元素描述匹配
pub(super) fn compound_matches_desc(c: &Compound, d: &ElementDesc) -> bool {
    let classes: Vec<&str> = d.classes.iter().map(|s| s.as_str()).collect();
    let target = MatchTarget {
        tag: &d.tag,
        id: d.id.as_deref(),
        classes: &classes,
        attrs: Some(&d.attrs),
        sibling_index: d.sibling_index,
        sibling_count: d.sibling_count,
        pressed: d.pressed,
    };
    matches_compound(c, &target)
}

/// 评估伪类。
///
/// `:active` 按元素的按压态求值 —— 从前所有未知伪类都**无条件放行**，
/// 于是 `.btn:active{background:orange}` 这类规则永远生效，元素看起来一直是按下态。
/// `:hover` / `:focus` 在触屏语义下不成立，一律不匹配（要按压反馈就用
/// `:active` 或小程序的 `hover-class`）。
/// `:nth-child()` 的求值。`n1` 是 1 起算的位置。
///
/// 以前只认纯数字和 odd/even，`2n` / `3n+1` / `-n+3` 这些都落到「解析失败 → 放行」，
/// 于是斑马纹样式**每一行都命中**（看着像样式没生效，其实是全中了）。
pub(super) fn nth_matches(arg: &str, n1: usize) -> bool {
    let s: String = arg.trim().to_ascii_lowercase();
    let (a, b) = match s.as_str() {
        "odd" => (2i64, 1i64),
        "even" => (2, 0),
        _ => match parse_an_plus_b(&s) {
            Some(v) => v,
            // 真解析不出来时放行，避免整条规则失效（与未知伪类的处理一致）
            None => return true,
        },
    };
    let n1 = n1 as i64;
    if a == 0 {
        return n1 == b;
    }
    // 是否存在非负整数 n 使 a·n + b == n1
    let diff = n1 - b;
    diff % a == 0 && diff / a >= 0
}

/// 解析 `An+B`：支持 `2n`、`2n+1`、`n`、`-n+3`、`+3n-1`、`5`
pub(super) fn parse_an_plus_b(s: &str) -> Option<(i64, i64)> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() {
        return None;
    }
    match s.find('n') {
        Some(pos) => {
            let a = match &s[..pos] {
                "" | "+" => 1,
                "-" => -1,
                other => other.parse().ok()?,
            };
            let rest = &s[pos + 1..];
            let b = if rest.is_empty() { 0 } else { rest.parse().ok()? };
            Some((a, b))
        }
        None => s.parse().ok().map(|b| (0, b)),
    }
}

pub(super) fn matches_pseudo(p: &str, t: &MatchTarget) -> bool {
    let idx = t.sibling_index;
    let cnt = t.sibling_count.max(1);
    match p {
        "first-child" => idx == 0,
        "last-child" => idx + 1 == cnt,
        "only-child" => cnt == 1,
        "active" => t.pressed,
        "hover" | "focus" | "focus-within" | "focus-visible" | "visited" | "target" => false,
        _ => {
            if let Some(arg) = p.strip_prefix("nth-child(").and_then(|s| s.strip_suffix(")")) {
                nth_matches(arg, idx + 1)
            } else if let Some(arg) = p.strip_prefix("nth-last-child(").and_then(|s| s.strip_suffix(")")) {
                nth_matches(arg, cnt - idx.min(cnt - 1))
            } else {
                true // 其它未知伪类/伪元素：放行，避免整条规则失效
            }
        }
    }
}

pub(super) fn matches_compound(c: &Compound, target: &MatchTarget) -> bool {
    // 标签
    if let Some(tag) = &c.tag {
        if !tag.eq_ignore_ascii_case(target.tag) {
            return false;
        }
    }
    // id
    if let Some(id) = &c.id {
        match target.id {
            Some(tid) if tid == id.as_str() => {}
            _ => return false,
        }
    }
    // 类
    for cls in &c.classes {
        if !target.classes.iter().any(|t| *t == cls.as_str()) {
            return false;
        }
    }
    // 属性
    for a in &c.attrs {
        if !matches_attr(a, target) {
            return false;
        }
    }
    // 伪类：结构性伪类按兄弟位置判定，其余放行
    for p in &c.pseudos {
        if !matches_pseudo(p, target) {
            return false;
        }
    }
    true
}

pub(super) fn matches_attr(a: &AttrSel, target: &MatchTarget) -> bool {
    let attrs = match target.attrs {
        Some(m) => m,
        None => return false, // 无属性上下文，属性选择器不匹配
    };
    let val = attrs.get(&a.name);
    let want = a.value.as_str();
    match a.op {
        AttrOp::Exists => val.is_some(),
        AttrOp::Eq => val.map(|v| v.as_str() == want).unwrap_or(false),
        AttrOp::Prefix => val.map(|v| v.starts_with(want)).unwrap_or(false),
        AttrOp::Suffix => val.map(|v| v.ends_with(want)).unwrap_or(false),
        AttrOp::Contains => val.map(|v| v.contains(want)).unwrap_or(false),
        AttrOp::Dash => val.map(|v| v.as_str() == want || v.starts_with(&format!("{}-", want))).unwrap_or(false),
        AttrOp::Word => val.map(|v| v.split_whitespace().any(|w| w == want)).unwrap_or(false),
    }
}

// ============================ WXSS 解析器 ============================

/// WXSS 解析器
/// 关键帧选择器 → 时间轴位置：`from`=0、`to`=1、`37.5%`=0.375
pub(super) fn parse_keyframe_offset(sel: &str) -> Option<f32> {
    match sel {
        "from" => Some(0.0),
        "to" => Some(1.0),
        other => {
            let pct = other.trim_end_matches('%');
            if pct.len() == other.len() {
                return None; // 既不是 from/to 也没有百分号
            }
            pct.trim().parse::<f32>().ok().map(|v| (v / 100.0).clamp(0.0, 1.0))
        }
    }
}
