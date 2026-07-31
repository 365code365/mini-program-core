//! 词法：把表达式串切成 token（数字/字符串/标识符/运算符）
//!
//! `expr_new` 的一片。**纯搬迁**：从 759 行按职责切开，一行逻辑没改。

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Tok {
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

pub(super) struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub(super) fn new(input: &str) -> Self {
        Self { chars: input.chars().collect(), pos: 0 }
    }
    
    fn cur(&self) -> char {
        self.chars.get(self.pos).copied().unwrap_or('\0')
    }
    
    fn peek(&self, o: usize) -> char {
        self.chars.get(self.pos + o).copied().unwrap_or('\0')
    }
    
    pub(super) fn tokenize(&mut self) -> Result<Vec<Tok>, String> {
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

pub(super) fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

pub(super) fn is_ident_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

// ============================ AST ============================
