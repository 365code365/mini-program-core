//! 语法树与递归下降解析（含三元、成员访问、下标、函数调用）
//!
//! `expr_new` 的一片。**纯搬迁**：从 759 行按职责切开，一行逻辑没改。
use super::lexer::{Lexer, Tok};

#[derive(Debug, Clone)]
pub(super) enum Expr {
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
    /// 函数调用：**解析得下来但不求值**（求值恒为 `null`）。微信的 WXML 里也只有
    /// wxs 模块能调用函数，模板表达式里写 `{{ f(x) }}` 同样拿不到结果 —— 保留这个
    /// 变体是为了不让解析在这种写法上整条失败（否则整个属性值都没了）。
    #[allow(dead_code)]
    Call(Box<Expr>, Vec<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub(super) enum BinOp {
    Add, Sub, Mul, Div, Mod,
    EqEq, EqEqEq, NotEq, NotEqEq,
    Lt, Lte, Gt, Gte,
}

// ============================ 解析（优先级爬升） ============================

pub(super) fn parse(input: &str) -> Result<Expr, String> {
    let toks = Lexer::new(input).tokenize()?;
    let mut p = Parser { toks, pos: 0 };
    let e = p.parse_ternary()?;
    if p.cur() != &Tok::Eof {
        return Err(format!("unexpected trailing token: {:?}", p.cur()));
    }
    Ok(e)
}

pub(super) struct Parser {
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
