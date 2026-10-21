//! 文本/属性插值（`{{}}` 片段替换、`class`/`style` 数组与对象拼接）与条件求值
//!
//! 模板引擎的一片，从 `template.rs` 原样切出（仅把可见性放宽到 `pub(super)`）。

use super::*;

impl TemplateEngine {
    /// 属性插值。`class` / `style` 绑定到数组或对象时按 CSS 语义拼接，
    /// 而不是把 JSON 直接塞进属性值。
    ///
    /// `class="{{['tabbar', d]}}"` 是 uni-app 每个组件/页面根节点的固定写法
    /// （`d` 是虚拟宿主类名）。原来走通用插值会得到字面量 `['tabbar','']`，
    /// 于是 `.tabbar` / `.root` / `.page` 这些**根节点样式全部失效** ——
    /// 底部导航因此既没有 `position:fixed` 也没有背景，整页配色也对不上。
    pub(super) fn interpolate_attr(key: &str, value: &str, data: &JsonValue) -> String {
        if key != "class" && key != "style" {
            return Self::interpolate(value, data);
        }
        let trimmed = value.trim();
        // 只有「整个属性值就是一个表达式」时才需要特殊拼接
        let inner = match trimmed.strip_prefix("{{").and_then(|s| s.strip_suffix("}}")) {
            Some(i) if !i.contains("{{") => i.trim(),
            _ => return Self::interpolate(value, data),
        };
        let sep = if key == "class" { " " } else { ";" };
        match expr::eval_str(inner, data) {
            JsonValue::Array(items) => items
                .iter()
                .map(expr::render_value)
                .filter(|s| !s.trim().is_empty())
                .collect::<Vec<_>>()
                .join(sep),
            // `class="{{ {active: on} }}"`：取值为真的键；style 取 `k:v`
            JsonValue::Object(map) => map
                .iter()
                .filter(|(_, v)| expr::is_truthy(v))
                .map(|(k, v)| {
                    if key == "class" {
                        k.clone()
                    } else {
                        format!("{k}:{}", expr::render_value(v))
                    }
                })
                .collect::<Vec<_>>()
                .join(sep),
            other => expr::render_value(&other),
        }
    }

    /// 文本/属性插值：替换所有 {{ ... }} 片段
    pub(super) fn interpolate(template: &str, data: &JsonValue) -> String {
        if !template.contains("{{") {
            return template.to_string();
        }
        
        let mut result = String::new();
        let mut rest = template;
        
        while let Some(open) = rest.find("{{") {
            result.push_str(&rest[..open]);
            rest = &rest[open + 2..];
            if let Some(close) = rest.find("}}") {
                let expr_src = &rest[..close];
                result.push_str(&expr::eval_to_display(expr_src, data));
                rest = &rest[close + 2..];
            } else {
                // 没有闭合，原样输出
                result.push_str("{{");
                result.push_str(rest);
                rest = "";
                break;
            }
        }
        result.push_str(rest);
        result
    }
    
    /// 求值一个可能被 {{ }} 包裹的表达式，返回 JSON 值（用于 wx:for）
    pub(super) fn eval_value(raw: &str, data: &JsonValue) -> JsonValue {
        expr::eval_str(&Self::extract_expression(raw), data)
    }
    
    /// 求值条件表达式（用于 wx:if / wx:elif）
    pub(super) fn eval_condition(raw: &str, data: &JsonValue) -> bool {
        expr::is_truthy(&Self::eval_value(raw, data))
    }
    
    /// 提取 {{ }} 中的表达式；若没有大括号则原样返回
    fn extract_expression(s: &str) -> String {
        let s = s.trim();
        if s.starts_with("{{") && s.ends_with("}}") && s.len() >= 4 {
            s[2..s.len() - 2].trim().to_string()
        } else {
            s.to_string()
        }
    }
    
    /// 是否为 wx: 指令属性（渲染时不输出到最终节点）
    pub(super) fn is_directive(key: &str) -> bool {
        // `slot:item`：动态 slot 的接收声明（见 slot.rs），同样不进节点
        key.starts_with("wx:") || key.starts_with("slot:")
    }
}
