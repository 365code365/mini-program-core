//! JS 值语义：真值判断、数字/字符串互转、number 的输出格式
//!
//! `expr_new` 的一片。**纯搬迁**：从 759 行按职责切开，一行逻辑没改。
use super::ast::parse;
use super::eval::eval;
use super::*;

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

pub(super) fn num(n: f64) -> Value {
    Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null)
}

pub(super) fn to_number(v: &Value) -> f64 {
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
pub(super) fn to_js_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

pub(super) fn number_to_string(n: &Number) -> String {
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

pub(super) fn format_f64(f: f64) -> String {
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
