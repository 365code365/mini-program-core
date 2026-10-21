//! 求值：成员/下标取值、二元运算、比较与相等语义
//!
//! `expr_new` 的一片。**纯搬迁**：从 759 行按职责切开，一行逻辑没改。
use super::ast::{BinOp, Expr, ObjEntry};
use super::js_value::{is_truthy, num, to_js_string, to_number};
use super::*;

pub(super) fn eval(e: &Expr, data: &Value) -> Value {
    match e {
        Expr::Num(n) => num(*n),
        Expr::Str(s) => Value::String(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Null => Value::Null,
        Expr::Ident(id) => match super::wxs::module_value(id) {
            // 当前文件里的 wxs 模块：`{{m.PI}}` 取导出的常量
            Some(v) => v,
            None => data.get(id.as_str()).cloned().unwrap_or(Value::Null),
        },
        Expr::Array(items) => Value::Array(items.iter().map(|i| eval(i, data)).collect()),
        Expr::Object(props) => {
            let mut map = serde_json::Map::new();
            for p in props {
                match p {
                    ObjEntry::Prop(k, v) => {
                        map.insert(k.clone(), eval(v, data));
                    }
                    ObjEntry::Spread(x) => spread_into(&mut map, eval(x, data)),
                }
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
        // `{{ }}` 里只有 wxs 模块的函数能调用（微信同样如此）：`m.f(x)` / `m.a.f(x)` / `m['f'](x)`。
        // 其余写法（数据里的函数、链式调用的结果再调用）求值为 null，与从前一样不崩
        Expr::Call(callee, args) => match callee_path(callee) {
            Some((root, path)) => {
                let argv: Vec<Value> = args.iter().map(|a| eval(a, data)).collect();
                super::wxs::call(&root, &path, &argv).unwrap_or(Value::Null)
            }
            None => Value::Null,
        },
    }
}

/// 被调用的表达式拆成「根名字 + 成员路径」：`m.a.f` → (`m`, [`a`,`f`])；
/// 不是从一个名字出发的成员链（比如 `(a || b).f`）返回 None
fn callee_path(e: &Expr) -> Option<(String, Vec<String>)> {
    match e {
        Expr::Member(obj, key) => {
            let (root, mut path) = match obj.as_ref() {
                Expr::Ident(id) => (id.clone(), Vec::new()),
                other => callee_path(other)?,
            };
            path.push(key.clone());
            Some((root, path))
        }
        Expr::Index(obj, idx) => {
            let Expr::Str(key) = idx.as_ref() else { return None };
            let (root, mut path) = match obj.as_ref() {
                Expr::Ident(id) => (id.clone(), Vec::new()),
                other => callee_path(other)?,
            };
            path.push(key.clone());
            Some((root, path))
        }
        _ => None,
    }
}

/// `{...x}`：按 JS 语义把 `x` 的自有键并进来（同名后写的赢）。数组/字符串展开成
/// 下标键，`null`/数字/布尔什么都不加。
fn spread_into(map: &mut serde_json::Map<String, Value>, v: Value) {
    match v {
        Value::Object(m) => map.extend(m),
        Value::Array(a) => {
            for (i, item) in a.into_iter().enumerate() {
                map.insert(i.to_string(), item);
            }
        }
        Value::String(s) => {
            for (i, c) in s.chars().enumerate() {
                map.insert(i.to_string(), Value::String(c.to_string()));
            }
        }
        _ => {}
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
