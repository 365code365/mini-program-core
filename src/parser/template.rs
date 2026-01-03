//! 模板引擎 - 处理数据绑定和条件渲染

use super::wxml::WxmlNode;
use serde_json::Value as JsonValue;

/// 模板引擎
pub struct TemplateEngine;

impl TemplateEngine {
    /// 渲染模板，替换 {{}} 表达式
    pub fn render(nodes: &[WxmlNode], data: &JsonValue) -> Vec<WxmlNode> {
        Self::render_with_virtual_list(nodes, data, None)
    }
    
    /// 渲染模板，支持虚拟列表
    /// viewport: (scroll_offset, viewport_height) 用于虚拟列表优化
    pub fn render_with_virtual_list(nodes: &[WxmlNode], data: &JsonValue, viewport: Option<(f32, f32)>) -> Vec<WxmlNode> {
        let mut result = Vec::new();
        
        for node in nodes {
            if let Some(rendered) = Self::render_node_with_viewport(node, data, viewport) {
                result.extend(rendered);
            }
        }
        
        result
    }
    
    fn render_node_with_viewport(node: &WxmlNode, data: &JsonValue, viewport: Option<(f32, f32)>) -> Option<Vec<WxmlNode>> {
        match node.node_type {
            super::wxml::WxmlNodeType::Text => {
                let text = Self::interpolate(&node.text_content, data);
                Some(vec![WxmlNode::new_text(&text)])
            }
            super::wxml::WxmlNodeType::Element => {
                // 处理 wx:if
                if let Some(condition) = node.attributes.get("wx:if") {
                    let expr = Self::extract_expression(condition);
                    if !Self::evaluate_condition(&expr, data) {
                        return None;
                    }
                }
                
                // 处理 wx:for - 使用虚拟列表优化
                if let Some(for_expr) = node.attributes.get("wx:for") {
                    return Some(Self::render_for_loop_virtual(node, for_expr, data, viewport));
                }
                
                // 普通元素
                let mut new_node = WxmlNode::new_element(&node.tag_name);
                
                // 处理属性
                for (key, value) in &node.attributes {
                    if key.starts_with("wx:") {
                        continue;
                    }
                    let new_value = Self::interpolate(value, data);
                    new_node.attributes.insert(key.clone(), new_value);
                }
                
                // 处理子节点
                new_node.children = Self::render_with_virtual_list(&node.children, data, viewport);
                
                Some(vec![new_node])
            }
            _ => Some(vec![node.clone()]),
        }
    }
    
    /// 虚拟列表渲染 - 只渲染可见区域的元素
    /// 注意：为了保持布局正确，我们仍然渲染所有元素，视口裁剪在绘制阶段处理
    fn render_for_loop_virtual(node: &WxmlNode, for_expr: &str, data: &JsonValue, _viewport: Option<(f32, f32)>) -> Vec<WxmlNode> {
        let array_name = Self::extract_expression(for_expr);
        let item_name = node.attributes.get("wx:for-item")
            .map(|s| s.as_str())
            .unwrap_or("item");
        let index_name = node.attributes.get("wx:for-index")
            .map(|s| s.as_str())
            .unwrap_or("index");
        
        let array = match Self::get_value(&array_name, data) {
            Some(v) => v,
            None => return Vec::new(),
        };
        
        let arr = match array.as_array() {
            Some(a) => a,
            None => return Vec::new(),
        };
        
        // 直接使用完整渲染，视口裁剪在绘制阶段处理
        Self::render_for_loop_full(node, arr, item_name, index_name, data)
    }
    
    /// 完整渲染 for 循环（不使用虚拟列表）
    fn render_for_loop_full(node: &WxmlNode, arr: &[JsonValue], item_name: &str, index_name: &str, data: &JsonValue) -> Vec<WxmlNode> {
        let mut result = Vec::new();
        
        for (index, item) in arr.iter().enumerate() {
            // 创建循环上下文
            let mut loop_data = data.clone();
            if let Some(obj) = loop_data.as_object_mut() {
                obj.insert(item_name.to_string(), item.clone());
                obj.insert(index_name.to_string(), JsonValue::Number(index.into()));
            }
            
            // 渲染节点（不包含 wx:for 属性）
            let mut new_node = WxmlNode::new_element(&node.tag_name);
            
            for (key, value) in &node.attributes {
                if key.starts_with("wx:") {
                    continue;
                }
                let new_value = Self::interpolate(value, &loop_data);
                new_node.attributes.insert(key.clone(), new_value);
            }
            
            new_node.children = Self::render(&node.children, &loop_data);
            result.push(new_node);
        }
        
        result
    }
    
    /// 插值替换 {{expression}}
    fn interpolate(template: &str, data: &JsonValue) -> String {
        let mut result = template.to_string();
        let mut start = 0;
        
        while let Some(open) = result[start..].find("{{") {
            let open = start + open;
            if let Some(close) = result[open..].find("}}") {
                let close = open + close;
                let expr = &result[open + 2..close].trim();
                let value = Self::evaluate_expression(expr, data);
                result = format!("{}{}{}", &result[..open], value, &result[close + 2..]);
                start = open + value.len();
            } else {
                break;
            }
        }
        
        result
    }
    
    /// 提取 {{}} 中的表达式
    fn extract_expression(s: &str) -> String {
        let s = s.trim();
        if s.starts_with("{{") && s.ends_with("}}") {
            s[2..s.len() - 2].trim().to_string()
        } else {
            s.to_string()
        }
    }
    
    /// 计算表达式
    fn evaluate_expression(expr: &str, data: &JsonValue) -> String {
        let expr = expr.trim();
        
        // 三元表达式: condition ? true_val : false_val
        if let Some(q_pos) = expr.find('?') {
            if let Some(c_pos) = expr[q_pos..].find(':') {
                let condition = &expr[..q_pos].trim();
                let true_val = &expr[q_pos + 1..q_pos + c_pos].trim();
                let false_val = &expr[q_pos + c_pos + 1..].trim();
                
                if Self::evaluate_condition(condition, data) {
                    return Self::evaluate_expression(true_val, data);
                } else {
                    return Self::evaluate_expression(false_val, data);
                }
            }
        }
        
        // 字符串字面量
        if (expr.starts_with('\'') && expr.ends_with('\'')) ||
           (expr.starts_with('"') && expr.ends_with('"')) {
            return expr[1..expr.len() - 1].to_string();
        }
        
        // 数字字面量
        if let Ok(_) = expr.parse::<f64>() {
            return expr.to_string();
        }
        
        // 处理 .length 属性（数组或字符串）
        if expr.ends_with(".length") {
            let base_path = &expr[..expr.len() - 7]; // 去掉 ".length"
            if let Some(value) = Self::get_value(base_path, data) {
                if let Some(arr) = value.as_array() {
                    return arr.len().to_string();
                } else if let Some(s) = value.as_str() {
                    return s.chars().count().to_string();
                }
            }
            return "0".to_string();
        }
        
        // 变量访问
        if let Some(value) = Self::get_value(expr, data) {
            return Self::json_to_string(value);
        }
        
        expr.to_string()
    }
    
    /// 计算条件表达式
    fn evaluate_condition(expr: &str, data: &JsonValue) -> bool {
        let expr = expr.trim();
        
        // 否定 - 处理 !variable 形式
        if expr.starts_with('!') {
            let inner = expr[1..].trim();
            // 如果是变量，先获取值再取反
            if let Some(value) = Self::get_value(inner, data) {
                return !Self::is_truthy(value);
            }
            // 否则递归处理
            return !Self::evaluate_condition(inner, data);
        }
        
        // 比较运算
        for op in &["===", "!==", "==", "!=", ">=", "<=", ">", "<"] {
            if let Some(pos) = expr.find(op) {
                let left = Self::evaluate_expression(&expr[..pos], data);
                let right = Self::evaluate_expression(&expr[pos + op.len()..], data);
                
                return match *op {
                    "===" | "==" => left == right,
                    "!==" | "!=" => left != right,
                    ">" => left.parse::<f64>().unwrap_or(0.0) > right.parse::<f64>().unwrap_or(0.0),
                    "<" => left.parse::<f64>().unwrap_or(0.0) < right.parse::<f64>().unwrap_or(0.0),
                    ">=" => left.parse::<f64>().unwrap_or(0.0) >= right.parse::<f64>().unwrap_or(0.0),
                    "<=" => left.parse::<f64>().unwrap_or(0.0) <= right.parse::<f64>().unwrap_or(0.0),
                    _ => false,
                };
            }
        }
        
        // 布尔值
        if let Some(value) = Self::get_value(expr, data) {
            return Self::is_truthy(value);
        }
        
        // 如果变量不存在，返回 false
        // 只有字面量 "true" 才返回 true
        expr == "true"
    }
    
    /// 获取数据值
    fn get_value<'a>(path: &str, data: &'a JsonValue) -> Option<&'a JsonValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data;
        
        for part in parts {
            // 处理数组索引 item[0]
            if let Some(bracket_pos) = part.find('[') {
                let name = &part[..bracket_pos];
                let index_str = &part[bracket_pos + 1..part.len() - 1];
                
                if !name.is_empty() {
                    current = current.get(name)?;
                }
                
                if let Ok(index) = index_str.parse::<usize>() {
                    current = current.get(index)?;
                }
            } else {
                current = current.get(part)?;
            }
        }
        
        Some(current)
    }
    
    fn json_to_string(value: &JsonValue) -> String {
        match value {
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "".to_string(),
            // 对于对象和数组，生成 JSON 字符串
            // 将双引号替换为单引号，避免与 HTML 属性引号冲突
            JsonValue::Object(_) | JsonValue::Array(_) => {
                value.to_string().replace('"', "'")
            }
        }
    }
    
    fn is_truthy(value: &JsonValue) -> bool {
        match value {
            JsonValue::Null => false,
            JsonValue::Bool(b) => *b,
            JsonValue::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            JsonValue::String(s) => !s.is_empty(),
            JsonValue::Array(a) => !a.is_empty(),
            JsonValue::Object(_) => true,
        }
    }
}
