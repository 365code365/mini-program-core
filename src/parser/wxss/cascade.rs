//! 级联：给一条元素链挑出命中的规则，按（特异性，书写顺序）排好，交出声明的**借用**。
//!
//! 从前只有 [`StyleSheet::get_styles_chain`]：把命中规则的每条声明 `clone` 进一张新的
//! `HashMap<String, StyleValue>`。建树时每个节点要问两三次（常态 / 按压态，容器还要再问一遍），
//! 一次 setData 整棵重建就是几千次字符串与样式值的深拷贝 —— 一加真机上整棵重建 3~4ms 里，
//! 分配器（jemalloc）一项就占了一成多。建树那条热路径改走这里，只借不拷。
//!
//! 结果与 `get_styles_chain` 逐项相同：后者就是把这里的序列按顺序 insert 进 map（同名后者覆盖前者）。
use super::*;

/// 见 [`StyleSheet::positional_subjects`]
#[derive(Debug, Default)]
pub struct PositionalSubjects {
    /// 有不带类名也不带标签的按位置选的规则：谁都可能受影响
    pub any: bool,
    pub classes: std::collections::HashSet<String>,
    /// 小写
    pub tags: std::collections::HashSet<String>,
}

impl PositionalSubjects {
    /// 这个元素（标签 + class 属性）会不会被按位置选的规则选中
    pub fn may_match(&self, tag: &str, class_attr: Option<&str>) -> bool {
        self.any
            || self.tags.contains(&tag.to_ascii_lowercase())
            || class_attr.is_some_and(|c| c.split_whitespace().any(|c| self.classes.contains(c)))
    }
}

impl StyleSheet {
    /// 命中这条链的全部声明，按级联顺序排好（后面的覆盖前面的）。同名属性可能出现多次，以最后一次为准。
    ///
    /// 同一条规则里的属性名互不相同，所以规则内部的遍历顺序不影响「最后一次」是谁。
    pub fn cascade_chain(&self, chain: &[ElementDesc]) -> Vec<(&str, &StyleValue)> {
        if chain.is_empty() {
            return Vec::new();
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
        let mut out = Vec::with_capacity(matched.iter().map(|m| m.2.len()).sum());
        for (_, _, props) in matched {
            out.extend(props.iter().map(|(k, v)| (k.as_str(), v)));
        }
        out
    }

    /// 哪些元素的样式可能随「在兄弟里排第几 / 兄弟有几个」变：带 `:first-child` / `:last-child` /
    /// `:only-child` / `:nth-*` 的复合选择器里写的类名与标签名。复合选择器里既没类名也没标签
    /// （`*:first-child`、`[data-x]:nth-child(2)`）就是「谁都可能」（`any`）。
    /// 失效分析据此判断：一串兄弟里多了 / 少了一个 fixed 元素，会不会让别的兄弟换样式
    pub fn positional_subjects(&self) -> PositionalSubjects {
        let mut out = PositionalSubjects::default();
        for rule in &self.rules {
            for cs in &rule.compiled {
                for (_, c) in &cs.seq {
                    let positional = c.pseudos.iter().any(|p| {
                        matches!(p.as_str(), "first-child" | "last-child" | "only-child") || p.starts_with("nth-")
                    });
                    if !positional {
                        continue;
                    }
                    if c.classes.is_empty() && c.tag.is_none() {
                        out.any = true;
                    }
                    out.classes.extend(c.classes.iter().cloned());
                    out.tags.extend(c.tag.iter().map(|t| t.to_ascii_lowercase()));
                }
            }
        }
        out
    }

    /// 基于祖先链匹配（chain 从根到目标，最后一个为目标元素）。
    /// 支持后代 / 子 `>` 组合器；相邻/通用兄弟因无兄弟上下文暂不匹配。
    pub fn get_styles_chain(&self, chain: &[ElementDesc]) -> HashMap<String, StyleValue> {
        let mut styles = HashMap::new();
        for (k, v) in self.cascade_chain(chain) {
            styles.insert(k.to_string(), v.clone());
        }
        styles
    }
}
