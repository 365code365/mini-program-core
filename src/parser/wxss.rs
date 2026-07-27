//! WXSS 解析器 - 完整支持微信小程序样式
//!
//! 选择器引擎支持：标签、类、id、通配 `*`、属性选择器 `[a]`/`[a=b]`/`[a^=b]`
//! `[a$=b]`/`[a*=b]`/`[a|=b]`/`[a~=b]`、结构伪类、以及组合器（后代、子 `>`、
//! 相邻兄弟 `+`、通用兄弟 `~`）的解析与正确的特异性计算。
//!
//! 说明：`get_styles`/`get_styles_el` 只提供「当前元素」上下文（无祖先/兄弟信息），
//! 因此带组合器的选择器（如 `.a .b`）在这两个入口下不会匹配（需要祖先链，
//! 由渲染层后续接线提供）。单复合选择器（标签/类/id/属性/通配及其组合）完整匹配。

use std::collections::HashMap;
use crate::Color;

/// 样式值
#[derive(Debug, Clone)]
pub enum StyleValue {
    Length(f32, LengthUnit),
    Color(Color),
    String(String),
    Number(f32),
    Auto,
    None,
}

/// 长度单位
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthUnit {
    Px,
    Rpx,
    Percent,
    Em,
    Rem,
    Vw,
    Vh,
}

/// 样式规则
#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: String,
    pub properties: HashMap<String, StyleValue>,
    /// 选择器在解析期就编译好的形态（逗号分组后每组一个复合选择器）。
    ///
    /// 匹配是「每个节点 × 每条规则」的二重循环，从前在里面现场
    /// `parse_complex(选择器字符串)` —— 一次全页重建就是十万次字符串分词，
    /// 占掉整次 setData 重建的绝大部分时间。
    compiled: Vec<ComplexSelector>,
}

impl StyleRule {
    /// 由选择器字符串构造，顺手编译选择器
    pub fn new(selector: String, properties: HashMap<String, StyleValue>) -> Self {
        let compiled = split_selector_groups(&selector)
            .into_iter()
            .filter_map(|part| parse_complex(&part))
            .collect();
        Self { selector, properties, compiled }
    }
}

/// `@keyframes` 里的一个关键帧（`0%` / `from` / `to` / `50%`）
#[derive(Debug, Clone)]
pub struct KeyframeStep {
    /// 时间轴位置，0.0 ~ 1.0
    pub offset: f32,
    pub properties: HashMap<String, StyleValue>,
}

/// 一条 `@keyframes name { ... }` 规则
#[derive(Debug, Clone)]
pub struct KeyframesRule {
    pub name: String,
    /// 按 offset 升序
    pub steps: Vec<KeyframeStep>,
}

/// 样式表
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    pub rules: Vec<StyleRule>,
    /// @import 记录的外部 wxss 路径（由加载器解析合并）
    pub imports: Vec<String>,
    /// `@keyframes` 动画时间轴（原生端由渲染器按帧求值）
    pub keyframes: Vec<KeyframesRule>,
}

// ============================ 选择器模型 ============================

#[derive(Debug, Clone, Copy, PartialEq)]
enum AttrOp {
    Exists,   // [a]
    Eq,       // [a=b]
    Prefix,   // [a^=b]
    Suffix,   // [a$=b]
    Contains, // [a*=b]
    Dash,     // [a|=b]
    Word,     // [a~=b]
}

#[derive(Debug, Clone)]
struct AttrSel {
    name: String,
    op: AttrOp,
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Combinator {
    Descendant,
    Child,
    NextSibling,
    SubsequentSibling,
}

/// 单个复合选择器，如 `view.item#main[data-x]:first-child`
#[derive(Debug, Clone, Default)]
struct Compound {
    universal: bool,
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attrs: Vec<AttrSel>,
    pseudos: Vec<String>,
}

/// 复合选择器序列（含组合器），如 `.a > .b .c`
#[derive(Debug, Clone)]
struct ComplexSelector {
    /// 从左到右：第一个 combinator 为 None
    seq: Vec<(Option<Combinator>, Compound)>,
    specificity: u32,
}

/// 匹配目标（当前元素上下文）
pub struct MatchTarget<'a> {
    pub tag: &'a str,
    pub id: Option<&'a str>,
    pub classes: &'a [&'a str],
    pub attrs: Option<&'a HashMap<String, String>>,
    /// 在兄弟中的位置（0 起）与兄弟总数，用于 :first-child/:last-child/:nth-child
    pub sibling_index: usize,
    pub sibling_count: usize,
    /// 是否处于按压态（供 `:active` 求值）
    pub pressed: bool,
}

/// 元素描述（用于祖先链匹配，拥有所有权）
#[derive(Debug, Clone, Default)]
pub struct ElementDesc {
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: HashMap<String, String>,
    /// 在兄弟中的位置（0 起）与兄弟总数（用于结构性伪类）
    pub sibling_index: usize,
    pub sibling_count: usize,
    /// 是否处于按压态（供 `:active` 求值）
    pub pressed: bool,
}

impl ElementDesc {
    pub fn new(tag: &str, id: Option<&str>, classes: &[&str], attrs: &HashMap<String, String>) -> Self {
        Self {
            tag: tag.to_string(),
            id: id.map(|s| s.to_string()),
            classes: classes.iter().map(|s| s.to_string()).collect(),
            attrs: attrs.clone(),
            sibling_index: 0,
            sibling_count: 1,
            pressed: false,
        }
    }

    /// 标记按压态（供 `:active` 匹配）
    pub fn with_pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    /// 设置兄弟位置（供结构性伪类 :first-child/:last-child/:nth-child 匹配）
    pub fn with_position(mut self, index: usize, count: usize) -> Self {
        self.sibling_index = index;
        self.sibling_count = count.max(1);
        self
    }
}

impl StyleSheet {
    pub fn new() -> Self {
        Self { rules: Vec::new(), imports: Vec::new(), keyframes: Vec::new() }
    }

    /// 按名字查关键帧时间轴（后定义的同名规则覆盖先定义的，与 CSS 一致）
    pub fn keyframes_named(&self, name: &str) -> Option<&KeyframesRule> {
        self.keyframes.iter().rev().find(|k| k.name == name)
    }

    /// 样式表里是否出现过 `:active` 规则。
    /// 建树期据此决定要不要多算一份「按压态样式」，没有就一分钱不花。
    pub fn has_active_rules(&self) -> bool {
        self.rules.iter().any(|r| r.selector.contains(":active"))
    }

    /// 样式表里是否出现过属性选择器（`[data-x]` / `[type=text]`）。
    ///
    /// 没有的话，祖先链里的 `ElementDesc` 就不必带属性表 —— 那份 HashMap 会随
    /// 「每个节点 × 祖先链克隆」被深拷贝很多遍，是一次 setData 全量重建的大头之一。
    pub fn has_attr_selectors(&self) -> bool {
        self.rules.iter().any(|r| r.selector.contains('['))
    }

    /// 将 `other`（通常是被 @import 的样式表）的规则并入本表前部，
    /// 使本表（局部）规则在同特异性时因书写顺序更靠后而胜出。
    pub fn prepend_rules(&mut self, other: StyleSheet) {
        let mut rules = other.rules;
        rules.append(&mut self.rules);
        self.rules = rules;
        let mut frames = other.keyframes;
        frames.append(&mut self.keyframes);
        self.keyframes = frames;
    }
    
    /// 解析所有 `var(--x, fallback)` 引用。
    ///
    /// 采用全局自定义属性池（所有规则中声明的 `--x` 合并，后者覆盖前者）。
    /// 这不完全等同于 CSS 的作用域级联，但覆盖了「设计令牌集中声明」的常见用法。
    fn resolve_variables(&mut self) {
        let mut vars: HashMap<String, String> = HashMap::new();
        for rule in &self.rules {
            for (k, v) in &rule.properties {
                if k.starts_with("--") {
                    if let StyleValue::String(s) = v {
                        vars.insert(k.clone(), s.clone());
                    }
                }
            }
        }
        if vars.is_empty() {
            return;
        }
        
        for rule in &mut self.rules {
            let names: Vec<String> = rule.properties.keys().cloned().collect();
            for name in names {
                let raw = match rule.properties.get(&name) {
                    Some(StyleValue::String(s)) if s.contains("var(") => s.clone(),
                    _ => continue,
                };
                let resolved = resolve_var_string(&raw, &vars);
                let nv = WxssParser::parse_value(&name, &resolved);
                rule.properties.insert(name, nv);
            }
        }
    }
    
    /// 获取元素样式（仅类名 + 标签上下文，向后兼容）
    pub fn get_styles(&self, class_names: &[&str], tag_name: &str) -> HashMap<String, StyleValue> {
        let desc = ElementDesc::new(tag_name, None, class_names, &HashMap::new());
        self.get_styles_chain(&[desc])
    }
    
    /// 获取元素样式（含 id 与属性上下文），支持 `#id`/`[attr]`/`*`（无祖先）。
    pub fn get_styles_el(
        &self,
        tag: &str,
        id: Option<&str>,
        classes: &[&str],
        attrs: &HashMap<String, String>,
    ) -> HashMap<String, StyleValue> {
        let desc = ElementDesc::new(tag, id, classes, attrs);
        self.get_styles_chain(&[desc])
    }
    
    /// 基于祖先链匹配（chain 从根到目标，最后一个为目标元素）。
    /// 支持后代 / 子 `>` 组合器；相邻/通用兄弟因无兄弟上下文暂不匹配。
    pub fn get_styles_chain(&self, chain: &[ElementDesc]) -> HashMap<String, StyleValue> {
        if chain.is_empty() {
            return HashMap::new();
        }
        // (specificity, order, &properties)
        let mut matched: Vec<(u32, usize, &HashMap<String, StyleValue>)> = Vec::new();
        
        for (order, rule) in self.rules.iter().enumerate() {
            let mut best: Option<u32> = None;
            for cs in &rule.compiled {
                if matches_complex(cs, chain) {
                    best = Some(best.map_or(cs.specificity, |b| b.max(cs.specificity)));
                }
            }
            if let Some(spec) = best {
                matched.push((spec, order, &rule.properties));
            }
        }
        
        // 按 (特异性, 书写顺序) 升序，后者覆盖前者
        matched.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        
        let mut styles = HashMap::new();
        for (_, _, props) in matched {
            for (k, v) in props {
                styles.insert(k.clone(), v.clone());
            }
        }
        styles
    }
}

// ============================ 选择器解析 ============================

/// 解析字符串中的 var(--name, fallback) 引用
fn resolve_var_string(s: &str, vars: &HashMap<String, String>) -> String {
    let mut result = s.to_string();
    // 迭代替换（支持多个/嵌套 var()），设上限防止死循环
    for _ in 0..16 {
        let start = match result.find("var(") {
            Some(p) => p,
            None => break,
        };
        // 找到匹配的右括号
        let mut depth = 0i32;
        let mut end = None;
        for (i, c) in result[start..].char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    end = Some(start + i);
                    break;
                }
            }
        }
        let end = match end {
            Some(e) => e,
            None => break,
        };
        let inner = &result[start + 4..end];
        let mut parts = inner.splitn(2, ',');
        let name = parts.next().unwrap_or("").trim().to_string();
        let fallback = parts.next().map(|f| f.trim().to_string()).unwrap_or_default();
        let value = vars.get(&name).cloned().unwrap_or(fallback);
        result.replace_range(start..=end, &value);
    }
    result
}

/// 按逗号切分选择器组（忽略 [] () 内的逗号）
fn split_selector_groups(sel: &str) -> Vec<String> {
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

enum Tok {
    Compound(String),
    Comb(Combinator),
}

/// 将复合序列拆成 token（复合 / 组合器），正确处理 [] () 内的空白与符号
fn tokenize_complex(sel: &str) -> Vec<Tok> {
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

fn parse_complex(sel: &str) -> Option<ComplexSelector> {
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

fn parse_compound(s: &str) -> Option<Compound> {
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

fn read_ident(chars: &[char], i: &mut usize) -> String {
    let mut s = String::new();
    while *i < chars.len() && is_ident_char(chars[*i]) {
        s.push(chars[*i]);
        *i += 1;
    }
    s
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_'
}

fn parse_attr(inner: &str) -> Option<AttrSel> {
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

fn compound_specificity(c: &Compound) -> u32 {
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
fn matches_complex(cs: &ComplexSelector, chain: &[ElementDesc]) -> bool {
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
fn compound_matches_desc(c: &Compound, d: &ElementDesc) -> bool {
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
fn matches_pseudo(p: &str, t: &MatchTarget) -> bool {
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
                let arg = arg.trim();
                let n1 = idx + 1;
                match arg {
                    "odd" => n1 % 2 == 1,
                    "even" => n1 % 2 == 0,
                    _ => arg.parse::<usize>().map(|k| n1 == k).unwrap_or(true),
                }
            } else {
                true // 其它未知伪类/伪元素：放行，避免整条规则失效
            }
        }
    }
}

fn matches_compound(c: &Compound, target: &MatchTarget) -> bool {
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

fn matches_attr(a: &AttrSel, target: &MatchTarget) -> bool {
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
fn parse_keyframe_offset(sel: &str) -> Option<f32> {
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
                } else {
                    self.skip_at_rule();
                }
                continue;
            }
            
            if let Some(rule) = self.parse_rule()? {
                stylesheet.rules.push(rule);
            }
        }
        
        // 解析 var() 自定义属性引用
        stylesheet.resolve_variables();
        
        Ok(stylesheet)
    }
    
    /// 尝试解析 @import "path"; / @import url("path");
    fn try_parse_import(&mut self) -> Option<String> {
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
    fn try_parse_keyframes(&mut self) -> Option<KeyframesRule> {
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

    fn skip_at_rule(&mut self) {
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
    
    fn parse_value(name: &str, value: &str) -> StyleValue {
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
    
    /// 静态求值 calc()：仅支持同单位（或无单位）的加减法。
    /// 混合单位（如 100% - 20px）无法静态解析，返回 None（由调用方回退为字符串）。
    fn eval_calc(value: &str) -> Option<StyleValue> {
        let inner = value.trim().strip_prefix("calc(")?.strip_suffix(')')?.trim();
        let tokens: Vec<&str> = inner.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }
        
        let mut acc: Option<f32> = None;
        let mut unit: Option<LengthUnit> = None;
        let mut op = '+';
        
        for (i, tok) in tokens.iter().enumerate() {
            if i % 2 == 1 {
                // 运算符位置：仅支持 + / -
                if *tok == "+" || *tok == "-" {
                    op = tok.chars().next().unwrap();
                } else {
                    return None; // 乘除或混合暂不支持
                }
                continue;
            }
            // 操作数
            let (n, u) = if let Some((n, u)) = Self::parse_length(tok) {
                (n, Some(u))
            } else if let Ok(n) = (*tok).parse::<f32>() {
                (n, None)
            } else if tok.starts_with("env(") {
                // `env(safe-area-inset-*)` 记 0：渲染视口是 375×667 的无刘海机型，
                // 四边都没有安全区内缩（对比用的 Chrome 视口同样返回 0）。
                //
                // 从前整条 calc 因为这个 token 解析失败而被**丢弃**，属性等于没写 ——
                // `height: calc(164rpx + env(safe-area-inset-bottom))` 这种写法
                // （uni-app 产物里自定义 tabBar 的标准高度）在浏览器里算出 82px，
                // 在我们这里退成 auto，高度全靠内容撑。
                (0.0, None)
            } else {
                return None;
            };
            if let Some(u) = u {
                match unit {
                    None => unit = Some(u),
                    Some(existing) if existing != u => return None, // 单位不一致
                    _ => {}
                }
            }
            acc = Some(match acc {
                None => n,
                Some(a) => if op == '+' { a + n } else { a - n },
            });
        }
        
        let a = acc?;
        Some(match unit {
            Some(u) => StyleValue::Length(a, u),
            None => StyleValue::Number(a),
        })
    }
    
    fn parse_length(value: &str) -> Option<(f32, LengthUnit)> {
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

/// rpx 转 px (基于 750 设计稿)
pub fn rpx_to_px(rpx: f32, screen_width: f32) -> f32 {
    rpx * screen_width / 750.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(css: &str) -> StyleSheet {
        WxssParser::new(css).parse().unwrap()
    }

    #[test]
    fn test_id_selector() {
        let ss = parse("#main { color: #fff; }");
        let attrs = HashMap::new();
        let s = ss.get_styles_el("view", Some("main"), &[], &attrs);
        assert!(s.contains_key("color"));
        // 不带 id 时不应匹配
        let s2 = ss.get_styles_el("view", None, &[], &attrs);
        assert!(!s2.contains_key("color"));
    }

    #[test]
    fn test_attribute_selector() {
        let ss = parse(r#"[type="primary"] { color: #f00; }"#);
        let mut attrs = HashMap::new();
        attrs.insert("type".to_string(), "primary".to_string());
        let s = ss.get_styles_el("button", None, &[], &attrs);
        assert!(s.contains_key("color"));
    }

    #[test]
    fn test_universal() {
        let ss = parse("* { color: #000; }");
        let attrs = HashMap::new();
        assert!(ss.get_styles_el("view", None, &[], &attrs).contains_key("color"));
        assert!(ss.get_styles_el("text", None, &[], &attrs).contains_key("color"));
    }

    #[test]
    fn test_specificity_id_over_class() {
        let ss = parse(".a { color: #000; } #x { color: #fff; }");
        let attrs = HashMap::new();
        let s = ss.get_styles_el("view", Some("x"), &["a"], &attrs);
        // id 特异性更高
        if let Some(StyleValue::Color(c)) = s.get("color") {
            assert_eq!(c.r, 255);
        } else { panic!("expected color"); }
    }

    #[test]
    fn test_descendant_not_matched_without_ancestors() {
        // 组合器选择器在无祖先上下文下不应匹配（避免误命中）
        let ss = parse(".list .item { color: #f00; }");
        let s = ss.get_styles(&["item"], "view");
        assert!(!s.contains_key("color"));
    }

    #[test]
    fn test_import_recorded() {
        let ss = parse(r#"@import "common.wxss"; .a { color: #000; }"#);
        assert_eq!(ss.imports.len(), 1);
        assert_eq!(ss.imports[0], "common.wxss");
        assert_eq!(ss.rules.len(), 1);
    }

    #[test]
    fn test_css_variables() {
        let ss = parse(r#"
            .root { --main: #ff0000; --gap: 20rpx; }
            .box { color: var(--main); padding: var(--gap); background-color: var(--missing, #00ff00); }
        "#);
        let attrs = HashMap::new();
        let s = ss.get_styles_el("view", None, &["box"], &attrs);
        if let Some(StyleValue::Color(c)) = s.get("color") {
            assert_eq!(c.r, 255);
            assert_eq!(c.g, 0);
        } else { panic!("color var not resolved"); }
        if let Some(StyleValue::Length(v, LengthUnit::Rpx)) = s.get("padding") {
            assert_eq!(*v, 20.0);
        } else { panic!("padding var not resolved"); }
        // fallback 生效
        if let Some(StyleValue::Color(c)) = s.get("background-color") {
            assert_eq!(c.g, 255);
        } else { panic!("var fallback not resolved"); }
    }

    #[test]
    fn test_calc_same_unit() {
        let ss = parse(".a { width: calc(100rpx + 20rpx); height: calc(50px - 10px); }");
        let attrs = HashMap::new();
        let s = ss.get_styles_el("view", None, &["a"], &attrs);
        if let Some(StyleValue::Length(v, LengthUnit::Rpx)) = s.get("width") {
            assert_eq!(*v, 120.0);
        } else { panic!("calc rpx failed"); }
        if let Some(StyleValue::Length(v, LengthUnit::Px)) = s.get("height") {
            assert_eq!(*v, 40.0);
        } else { panic!("calc px failed"); }
    }

    #[test]
    fn test_calc_mixed_unit_falls_back() {
        // 混合单位无法静态计算，保留为字符串（不 panic）
        let ss = parse(".a { width: calc(100% - 20px); }");
        let attrs = HashMap::new();
        let s = ss.get_styles_el("view", None, &["a"], &attrs);
        assert!(matches!(s.get("width"), Some(StyleValue::String(_))));
    }

    #[test]
    fn test_compound_and_specificity_order() {
        let ss = parse(".item { color: #000000; } .item.active { color: #ffffff; }");
        let attrs = HashMap::new();
        let s = ss.get_styles_el("view", None, &["item", "active"], &attrs);
        if let Some(StyleValue::Color(c)) = s.get("color") {
            assert_eq!(c.r, 255);
        } else { panic!("expected color"); }
    }
}
