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

// ── 按职责切开的三片 ──
/// WXSS 文本 → 规则表（词法与声明块）
mod parser;
/// CSS 选择器引擎（分词 / 解析 / 特异性 / 匹配）
mod selector;

pub use parser::WxssParser;
// 选择器的内部类型/函数都是 `pub(super)`：同一个 `wxss` 模块下的几片互相可见，
// 对外不暴露（外面只需要 `StyleSheet` / `StyleRule` / `WxssParser`）。
use selector::*;

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

    /// 微信 `<button>` 的默认点击态。文档写明 `button-hover` 是
    /// `{ background-color: rgba(0,0,0,.1); opacity: 0.7 }`。
    ///
    /// 插在样式表**最前面**：页面自己的 `.button-hover` 或 `.wx-btn` 同特异性时
    /// 因书写顺序更靠后而胜出（自定义背景的按钮按下仍是原来的颜色，只变透明）。
    pub fn ensure_button_hover(&mut self) {
        let builtin = WxssParser::new(
            ".button-hover{background-color:rgba(0,0,0,0.1);opacity:0.7;}",
        )
        .parse()
        .unwrap_or_else(|_| StyleSheet::new());
        self.prepend_rules(builtin);
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
        // 注意：**不能**在 `vars` 为空时提前返回。`var(--x, 兜底值)` 的兜底值也要生效，
        // 而这种写法完全可能出现在一个 `--x` 都没声明的样式表里（变量在别的 wxss 里、
        // 或者本来就只想用兜底值）。以前提前返回时，这些属性会以字面量
        // `"var(--x, #ff0000)"` 留在样式里 —— 等于这条声明整个丢掉。
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
