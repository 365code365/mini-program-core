//! 求值：成员/下标取值、二元运算、比较与相等语义
//!
//! `expr_new` 的一片。**纯搬迁**：从 759 行按职责切开，一行逻辑没改。
use super::ast::{BinOp, Expr};
use super::js_value::{is_truthy, num, to_js_string, to_number};
use super::*;

pub(super) fn eval(e: &Expr, data: &Value) -> Value {
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

pub(super) fn member(base: &Value, key: &str) -> Value {
    match base {
        Value::Object(map) => map.get(key).cloned().unwrap_or(Value::Null),
        Value::Array(arr) if key == "length" => num(arr.len() as f64),
        Value::String(s) if key == "length" => num(s.chars().count() as f64),
        _ => Value::Null,
    }
}

pub(super) fn index(base: &Value, idx: &Value) -> Value {
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

pub(super) fn eval_bin(op: BinOp, a: &Value, b: &Value) -> Value {
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

pub(super) fn cmp<F: Fn(std::cmp::Ordering) -> bool>(a: &Value, b: &Value, f: F) -> Value {
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

pub(super) fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Number(_), Value::Number(_)) => to_number(a) == to_number(b),
        (Value::Array(_), Value::Array(_)) | (Value::Object(_), Value::Object(_)) => a == b,
        _ => false,
    }
}

pub(super) fn loose_eq(a: &Value, b: &Value) -> bool {
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
