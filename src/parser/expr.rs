//! WXML `{{ }}` 表达式引擎
//!
//! 官方小程序的 `{{ }}` 里是完整的 JS 表达式子集。之前 `template.rs` 用手写
//! 字符串匹配来求值，只能处理单个运算符、一层三元、变量路径，和官方差距很大。
//!
//! 本模块实现一个真正的表达式解析器：词法分析 -> 递归下降/优先级爬升生成 AST
//! -> 基于 `serde_json::Value` 的求值，尽量贴合 JS 语义。支持：
//! - 算术：`+ - * / %`（`+` 遇字符串做拼接）
//! - 比较：`== === != !== < <= > >=`
//! - 逻辑：`&& || !`（`&&`/`||` 返回操作数值，符合 JS 短路语义）
//! - 三元：`cond ? a : b`
//! - 成员/索引：`a.b.c`、`a[i]`、`a.b[i].c`、数组/字符串 `.length`
//! - 字面量：数字、字符串、`true/false/null/undefined`、数组 `[..]`、对象 `{..}`
//! - 分组：`( .. )`

use serde_json::{Number, Value};

// ============================ 对外 API ============================

/// 对单个表达式字符串求值，返回 JSON 值。解析失败返回 `Null`。
pub fn eval_str(expr: &str, data: &Value) -> Value {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Value::Null;
    }
    match parse(trimmed) {
        Ok(ast) => eval(&ast, data),
        Err(_) => Value::Null,
    }
}

/// 求值并按 WXML 文本/属性输出规则转成字符串。
/// `null`/`undefined` 输出空串，对象/数组输出单引号 JSON（兼容属性内嵌）。
pub fn eval_to_display(expr: &str, data: &Value) -> String {
    render_value(&eval_str(expr, data))
}

/// JS 真值判断
pub fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0 && !f.is_nan()).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,   // JS 里数组/对象恒为真
        Value::Object(_) => true,
    }
}

/// 把值渲染成 WXML 输出字符串
pub fn render_value(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n),
        Value::String(s) => s.clone(),
        // 对象/数组：JSON 序列化并把双引号换成单引号，避免与 HTML 属性引号冲突
        Value::Array(_) | Value::Object(_) => v.to_string().replace('"', "'"),
    }
}

// ============================ 词法 ============================

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    // 运算符与分隔符
    Plus, Minus, Star, Slash, Percent,
    EqEq, EqEqEq, NotEq, NotEqEq,
    Lt, Lte, Gt, Gte,
    AndAnd, OrOr, Not,
    Question, Colon,
    Dot, Comma,
    LParen, RParen,
    LBracket, RBracket,
    LBrace, RBrace,
    Eof,
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self { chars: input.chars().collect(), pos: 0 }
    }
    
    fn cur(&self) -> char {
        self.chars.get(self.pos).copied().unwrap_or('\0')
    }
    
    fn peek(&self, o: usize) -> char {
        self.chars.get(self.pos + o).copied().unwrap_or('\0')
    }
    
    fn tokenize(&mut self) -> Result<Vec<Tok>, String> {
        let mut toks = Vec::new();
        loop {
            while self.cur().is_whitespace() {
                self.pos += 1;
            }
            let c = self.cur();
            if c == '\0' {
                toks.push(Tok::Eof);
                break;
            }
            match c {
                '0'..='9' => toks.push(self.read_number()),
                '\'' | '"' => toks.push(self.read_string(c)?),
                c if is_ident_start(c) => toks.push(self.read_ident()),
                '+' => { toks.push(Tok::Plus); self.pos += 1; }
                '-' => { toks.push(Tok::Minus); self.pos += 1; }
                '*' => { toks.push(Tok::Star); self.pos += 1; }
                '/' => { toks.push(Tok::Slash); self.pos += 1; }
                '%' => { toks.push(Tok::Percent); self.pos += 1; }
                '?' => { toks.push(Tok::Question); self.pos += 1; }
                ':' => { toks.push(Tok::Colon); self.pos += 1; }
                '.' => { toks.push(Tok::Dot); self.pos += 1; }
                ',' => { toks.push(Tok::Comma); self.pos += 1; }
                '(' => { toks.push(Tok::LParen); self.pos += 1; }
                ')' => { toks.push(Tok::RParen); self.pos += 1; }
                '[' => { toks.push(Tok::LBracket); self.pos += 1; }
                ']' => { toks.push(Tok::RBracket); self.pos += 1; }
                '{' => { toks.push(Tok::LBrace); self.pos += 1; }
                '}' => { toks.push(Tok::RBrace); self.pos += 1; }
                '=' => {
                    if self.peek(1) == '=' && self.peek(2) == '=' {
                        toks.push(Tok::EqEqEq); self.pos += 3;
                    } else if self.peek(1) == '=' {
                        toks.push(Tok::EqEq); self.pos += 2;
                    } else {
                        return Err("unexpected '='".into());
                    }
                }
                '!' => {
                    if self.peek(1) == '=' && self.peek(2) == '=' {
                        toks.push(Tok::NotEqEq); self.pos += 3;
                    } else if self.peek(1) == '=' {
                        toks.push(Tok::NotEq); self.pos += 2;
                    } else {
                        toks.push(Tok::Not); self.pos += 1;
                    }
                }
                '<' => {
                    if self.peek(1) == '=' { toks.push(Tok::Lte); self.pos += 2; }
                    else { toks.push(Tok::Lt); self.pos += 1; }
                }
                '>' => {
                    if self.peek(1) == '=' { toks.push(Tok::Gte); self.pos += 2; }
                    else { toks.push(Tok::Gt); self.pos += 1; }
                }
                '&' => {
                    if self.peek(1) == '&' { toks.push(Tok::AndAnd); self.pos += 2; }
                    else { return Err("unexpected '&'".into()); }
                }
                '|' => {
                    if self.peek(1) == '|' { toks.push(Tok::OrOr); self.pos += 2; }
                    else { return Err("unexpected '|'".into()); }
                }
                other => return Err(format!("unexpected char '{}'", other)),
            }
        }
        Ok(toks)
    }
    
    fn read_number(&mut self) -> Tok {
        let start = self.pos;
        while self.cur().is_ascii_digit() {
            self.pos += 1;
        }
        if self.cur() == '.' && self.peek(1).is_ascii_digit() {
            self.pos += 1;
            while self.cur().is_ascii_digit() {
                self.pos += 1;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        Tok::Num(s.parse().unwrap_or(0.0))
    }
    
    fn read_string(&mut self, quote: char) -> Result<Tok, String> {
        self.pos += 1; // skip opening quote
        let mut s = String::new();
        while self.cur() != quote {
            if self.cur() == '\0' {
                return Err("unterminated string".into());
            }
            if self.cur() == '\\' {
                self.pos += 1;
                let esc = self.cur();
                s.push(match esc {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    other => other,
                });
                self.pos += 1;
            } else {
                s.push(self.cur());
                self.pos += 1;
            }
        }
        self.pos += 1; // skip closing quote
        Ok(Tok::Str(s))
    }
    
    fn read_ident(&mut self) -> Tok {
        let start = self.pos;
        while is_ident_part(self.cur()) {
            self.pos += 1;
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        Tok::Ident(s)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_ident_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

// ============================ AST ============================

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Ident(String),
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Pos(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
}

#[derive(Debug, Clone, Copy)]
enum BinOp {
    Add, Sub, Mul, Div, Mod,
    EqEq, EqEqEq, NotEq, NotEqEq,
    Lt, Lte, Gt, Gte,
}

// ============================ 解析（优先级爬升） ============================

fn parse(input: &str) -> Result<Expr, String> {
    let toks = Lexer::new(input).tokenize()?;
    let mut p = Parser { toks, pos: 0 };
    let e = p.parse_ternary()?;
    if p.cur() != &Tok::Eof {
        return Err(format!("unexpected trailing token: {:?}", p.cur()));
    }
    Ok(e)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn cur(&self) -> &Tok {
        self.toks.get(self.pos).unwrap_or(&Tok::Eof)
    }
    
    fn advance(&mut self) -> Tok {
        let t = self.toks.get(self.pos).cloned().unwrap_or(Tok::Eof);
        self.pos += 1;
        t
    }
    
    fn expect(&mut self, t: &Tok) -> Result<(), String> {
        if self.cur() == t {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected {:?}, got {:?}", t, self.cur()))
        }
    }
    
    // ternary := or ('?' ternary ':' ternary)?
    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let cond = self.parse_or()?;
        if self.cur() == &Tok::Question {
            self.advance();
            let then = self.parse_ternary()?;
            self.expect(&Tok::Colon)?;
            let els = self.parse_ternary()?;
            Ok(Expr::Ternary(Box::new(cond), Box::new(then), Box::new(els)))
        } else {
            Ok(cond)
        }
    }
    
    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.cur() == &Tok::OrOr {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    
    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while self.cur() == &Tok::AndAnd {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    
    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match self.cur() {
                Tok::EqEq => BinOp::EqEq,
                Tok::EqEqEq => BinOp::EqEqEq,
                Tok::NotEq => BinOp::NotEq,
                Tok::NotEqEq => BinOp::NotEqEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_relational()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    
    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.cur() {
                Tok::Lt => BinOp::Lt,
                Tok::Lte => BinOp::Lte,
                Tok::Gt => BinOp::Gt,
                Tok::Gte => BinOp::Gte,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    
    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.cur() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    
    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.cur() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }
    
    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.cur() {
            Tok::Not => { self.advance(); Ok(Expr::Not(Box::new(self.parse_unary()?))) }
            Tok::Minus => { self.advance(); Ok(Expr::Neg(Box::new(self.parse_unary()?))) }
            Tok::Plus => { self.advance(); Ok(Expr::Pos(Box::new(self.parse_unary()?))) }
            _ => self.parse_postfix(),
        }
    }
    
    // postfix := primary ( '.' ident | '[' expr ']' | '(' args ')' )*
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.parse_primary()?;
        loop {
            match self.cur() {
                Tok::Dot => {
                    self.advance();
                    if let Tok::Ident(name) = self.advance() {
                        e = Expr::Member(Box::new(e), name);
                    } else {
                        return Err("expected property name after '.'".into());
                    }
                }
                Tok::LBracket => {
                    self.advance();
                    let idx = self.parse_ternary()?;
                    self.expect(&Tok::RBracket)?;
                    e = Expr::Index(Box::new(e), Box::new(idx));
                }
                Tok::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if self.cur() != &Tok::RParen {
                        loop {
                            args.push(self.parse_ternary()?);
                            if self.cur() == &Tok::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    e = Expr::Call(Box::new(e), args);
                }
                _ => break,
            }
        }
        Ok(e)
    }
    
    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Ident(id) => match id.as_str() {
                "true" => Ok(Expr::Bool(true)),
                "false" => Ok(Expr::Bool(false)),
                "null" => Ok(Expr::Null),
                "undefined" => Ok(Expr::Null),
                _ => Ok(Expr::Ident(id)),
            },
            Tok::LParen => {
                let e = self.parse_ternary()?;
                self.expect(&Tok::RParen)?;
                Ok(e)
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                if self.cur() != &Tok::RBracket {
                    loop {
                        items.push(self.parse_ternary()?);
                        if self.cur() == &Tok::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(&Tok::RBracket)?;
                Ok(Expr::Array(items))
            }
            Tok::LBrace => {
                let mut props = Vec::new();
                if self.cur() != &Tok::RBrace {
                    loop {
                        // key 可以是 ident 或字符串
                        let key = match self.advance() {
                            Tok::Ident(k) => k,
                            Tok::Str(k) => k,
                            other => return Err(format!("expected object key, got {:?}", other)),
                        };
                        self.expect(&Tok::Colon)?;
                        let val = self.parse_ternary()?;
                        props.push((key, val));
                        if self.cur() == &Tok::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(&Tok::RBrace)?;
                Ok(Expr::Object(props))
            }
            other => Err(format!("unexpected token in primary: {:?}", other)),
        }
    }
}

// ============================ 求值 ============================

fn eval(e: &Expr, data: &Value) -> Value {
    match e {
        Expr::Num(n) => num(*n),
        Expr::Str(s) => Value::String(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Null => Value::Null,
        Expr::Ident(id) => data.get(id.as_str()).cloned().unwrap_or(Value::Null),
        Expr::Array(items) => Value::Array(items.iter().map(|i| eval(i, data)).collect()),
        Expr::Object(props) => {
            let mut map = serde_json::Map::new();
            for (k, v) in props {
                map.insert(k.clone(), eval(v, data));
            }
            Value::Object(map)
        }
        Expr::Not(x) => Value::Bool(!is_truthy(&eval(x, data))),
        Expr::Neg(x) => num(-to_number(&eval(x, data))),
        Expr::Pos(x) => num(to_number(&eval(x, data))),
        Expr::And(a, b) => {
            let va = eval(a, data);
            if is_truthy(&va) { eval(b, data) } else { va }
        }
        Expr::Or(a, b) => {
            let va = eval(a, data);
            if is_truthy(&va) { va } else { eval(b, data) }
        }
        Expr::Ternary(c, t, f) => {
            if is_truthy(&eval(c, data)) { eval(t, data) } else { eval(f, data) }
        }
        Expr::Bin(op, a, b) => eval_bin(*op, &eval(a, data), &eval(b, data)),
        Expr::Member(obj, key) => {
            let base = eval(obj, data);
            member(&base, key)
        }
        Expr::Index(obj, idx) => {
            let base = eval(obj, data);
            let i = eval(idx, data);
            index(&base, &i)
        }
        // {{ }} 里的函数调用主要用于 WXS 模块，尚未接入，先返回 Null 保证不崩
        Expr::Call(_, _) => Value::Null,
    }
}

fn member(base: &Value, key: &str) -> Value {
    match base {
        Value::Object(map) => map.get(key).cloned().unwrap_or(Value::Null),
        Value::Array(arr) if key == "length" => num(arr.len() as f64),
        Value::String(s) if key == "length" => num(s.chars().count() as f64),
        _ => Value::Null,
    }
}

fn index(base: &Value, idx: &Value) -> Value {
    match base {
        Value::Array(arr) => {
            let i = to_number(idx);
            if i >= 0.0 && i.fract() == 0.0 {
                arr.get(i as usize).cloned().unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        Value::Object(map) => {
            let key = to_js_string(idx);
            map.get(&key).cloned().unwrap_or(Value::Null)
        }
        _ => Value::Null,
    }
}

fn eval_bin(op: BinOp, a: &Value, b: &Value) -> Value {
    match op {
        BinOp::Add => {
            // 任一为字符串则做拼接，否则数值相加（JS 语义）
            if matches!(a, Value::String(_)) || matches!(b, Value::String(_)) {
                Value::String(format!("{}{}", to_js_string(a), to_js_string(b)))
            } else {
                num(to_number(a) + to_number(b))
            }
        }
        BinOp::Sub => num(to_number(a) - to_number(b)),
        BinOp::Mul => num(to_number(a) * to_number(b)),
        BinOp::Div => num(to_number(a) / to_number(b)),
        BinOp::Mod => num(to_number(a) % to_number(b)),
        BinOp::EqEq => Value::Bool(loose_eq(a, b)),
        BinOp::NotEq => Value::Bool(!loose_eq(a, b)),
        BinOp::EqEqEq => Value::Bool(strict_eq(a, b)),
        BinOp::NotEqEq => Value::Bool(!strict_eq(a, b)),
        BinOp::Lt => cmp(a, b, |o| o == std::cmp::Ordering::Less),
        BinOp::Lte => cmp(a, b, |o| o != std::cmp::Ordering::Greater),
        BinOp::Gt => cmp(a, b, |o| o == std::cmp::Ordering::Greater),
        BinOp::Gte => cmp(a, b, |o| o != std::cmp::Ordering::Less),
    }
}

fn cmp<F: Fn(std::cmp::Ordering) -> bool>(a: &Value, b: &Value, f: F) -> Value {
    // 两边都是字符串时按字典序比较，否则按数值比较
    if let (Value::String(sa), Value::String(sb)) = (a, b) {
        return Value::Bool(f(sa.cmp(sb)));
    }
    let na = to_number(a);
    let nb = to_number(b);
    match na.partial_cmp(&nb) {
        Some(o) => Value::Bool(f(o)),
        None => Value::Bool(false), // NaN 比较恒 false
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Number(_), Value::Number(_)) => to_number(a) == to_number(b),
        (Value::Array(_), Value::Array(_)) | (Value::Object(_), Value::Object(_)) => a == b,
        _ => false,
    }
}

fn loose_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Null, _) | (_, Value::Null) => false,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(_), _) | (_, Value::Bool(_)) => to_number(a) == to_number(b),
        (Value::Number(_), Value::Number(_)) => to_number(a) == to_number(b),
        // 数字与字符串：转数字比较
        (Value::Number(_), Value::String(_)) | (Value::String(_), Value::Number(_)) => {
            to_number(a) == to_number(b)
        }
        _ => a == b,
    }
}

// ============================ 类型转换 ============================

fn num(n: f64) -> Value {
    Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null)
}

fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(true) => 1.0,
        Value::Bool(false) => 0.0,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() { 0.0 } else { t.parse::<f64>().unwrap_or(f64::NAN) }
        }
        _ => f64::NAN,
    }
}

/// JS `String()` 语义（用于 `+` 拼接）。为贴合 WXML 输出，null/undefined 取空串。
fn to_js_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

fn number_to_string(n: &Number) -> String {
    if let Some(i) = n.as_i64() {
        i.to_string()
    } else if let Some(u) = n.as_u64() {
        u.to_string()
    } else if let Some(f) = n.as_f64() {
        format_f64(f)
    } else {
        n.to_string()
    }
}

fn format_f64(f: f64) -> String {
    if f.is_nan() {
        "NaN".to_string()
    } else if f.is_infinite() {
        if f > 0.0 { "Infinity".to_string() } else { "-Infinity".to_string() }
    } else if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        f.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ev(expr: &str, data: &Value) -> Value {
        eval_str(expr, data)
    }

    #[test]
    fn test_arithmetic() {
        let d = json!({});
        assert_eq!(ev("1 + 2 * 3", &d), json!(7.0));
        assert_eq!(ev("(1 + 2) * 3", &d), json!(9.0));
        assert_eq!(ev("10 % 3", &d), json!(1.0));
    }

    #[test]
    fn test_variables_and_members() {
        let d = json!({"user": {"name": "Tom", "age": 3}, "list": [10, 20, 30]});
        assert_eq!(ev("user.name", &d), json!("Tom"));
        assert_eq!(ev("user.age + 1", &d), json!(4.0));
        assert_eq!(ev("list[1]", &d), json!(20));
        assert_eq!(ev("list.length", &d), json!(3.0));
    }

    #[test]
    fn test_logical_and_ternary() {
        let d = json!({"active": true, "n": 0});
        assert_eq!(ev("active ? 'on' : 'off'", &d), json!("on"));
        assert_eq!(ev("n > 0 && active", &d), json!(false));
        assert_eq!(ev("n > 0 || active", &d), json!(true));
        assert_eq!(ev("!active", &d), json!(false));
    }

    #[test]
    fn test_string_concat() {
        let d = json!({"name": "World"});
        assert_eq!(ev("'Hello ' + name", &d), json!("Hello World"));
    }

    #[test]
    fn test_complex_condition() {
        let d = json!({"a": 5, "b": 10, "c": 3, "e": 2});
        // 验证多运算符组合不会被旧实现的“找到第一个运算符就截断”破坏
        assert_eq!(ev("a > b && c > e", &d), json!(false));
        assert_eq!(ev("a < b && c > e", &d), json!(true));
    }

    #[test]
    fn test_display() {
        let d = json!({"obj": {"x": 1}});
        assert_eq!(eval_to_display("missing", &d), "");
        assert_eq!(eval_to_display("obj", &d), "{'x':1}");
    }
}
