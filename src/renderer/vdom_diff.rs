//! WXML 虚拟节点树 Diff
//!
//! 官方 Skyline 逻辑层 `setData` 后，框架对新旧虚拟节点树做 diff，只把差异
//! patch 应用到渲染层，而不是整棵重建。当前引擎每次 `setData` 都全量重渲 +
//! 重排（见 `wxml_renderer::update_layout_if_needed`）。
//!
//! 本模块提供在两棵 `WxmlNode`（模板渲染后的结果）之间计算最小 patch 的能力，
//! 作为增量渲染的基础。集成到 taffy 布局管线（局部 relayout）属于后续工作，
//! 但有了 patch 列表后，渲染层可以判断「结构未变、仅文本/属性变化」从而跳过
//! 昂贵的重排。
//!
//! Diff 采用按索引对齐的策略（后续可扩展为基于 `wx:key` 的带 key diff）。

use crate::parser::wxml::{WxmlNode, WxmlNodeType};

/// 节点路径：从根开始的子节点索引序列
pub type Path = Vec<usize>;

/// 一条 patch 操作
#[derive(Debug, Clone, PartialEq)]
pub enum Patch {
    /// 整体替换该路径的节点（标签或节点类型变化）
    Replace(Path, WxmlNode),
    /// 更新文本节点内容
    UpdateText(Path, String),
    /// 设置/更新属性
    SetAttr(Path, String, String),
    /// 删除属性
    RemoveAttr(Path, String),
    /// 在指定路径节点的 children 末尾插入子节点（index 为插入位置）
    InsertChild(Path, usize, WxmlNode),
    /// 删除指定路径节点的第 index 个子节点
    RemoveChild(Path, usize),
}

/// 计算从 `old` 到 `new` 两个森林（同级节点列表）的 patch 列表。
///
/// 根路径为空 `[]`，其下第 i 个节点路径为 `[i]`。
pub fn diff_forest(old: &[WxmlNode], new: &[WxmlNode]) -> Vec<Patch> {
    let mut patches = Vec::new();
    diff_children(&[], old, new, &mut patches);
    patches
}

/// 是否只包含文本/属性级别的 patch（结构未变），可用于跳过重排的快速判断
pub fn is_structural(patches: &[Patch]) -> bool {
    patches.iter().any(|p| matches!(
        p,
        Patch::Replace(..) | Patch::InsertChild(..) | Patch::RemoveChild(..)
    ))
}

fn diff_node(path: &Path, old: &WxmlNode, new: &WxmlNode, patches: &mut Vec<Patch>) {
    // 节点类型或标签不同 -> 整体替换
    if old.node_type != new.node_type || old.tag_name != new.tag_name {
        patches.push(Patch::Replace(path.clone(), new.clone()));
        return;
    }
    
    match new.node_type {
        WxmlNodeType::Text | WxmlNodeType::Comment => {
            if old.text_content != new.text_content {
                patches.push(Patch::UpdateText(path.clone(), new.text_content.clone()));
            }
        }
        WxmlNodeType::Element => {
            // 属性差异
            for (k, v) in &new.attributes {
                match old.attributes.get(k) {
                    Some(ov) if ov == v => {}
                    _ => patches.push(Patch::SetAttr(path.clone(), k.clone(), v.clone())),
                }
            }
            for k in old.attributes.keys() {
                if !new.attributes.contains_key(k) {
                    patches.push(Patch::RemoveAttr(path.clone(), k.clone()));
                }
            }
            // 子节点差异
            diff_children(path, &old.children, &new.children, patches);
        }
    }
}

fn diff_children(parent: &Path, old: &[WxmlNode], new: &[WxmlNode], patches: &mut Vec<Patch>) {
    let min = old.len().min(new.len());
    for i in 0..min {
        let mut child_path = parent.clone();
        child_path.push(i);
        diff_node(&child_path, &old[i], &new[i], patches);
    }
    if new.len() > old.len() {
        // 新增子节点
        for (i, node) in new.iter().enumerate().skip(old.len()) {
            patches.push(Patch::InsertChild(parent.clone(), i, node.clone()));
        }
    } else if old.len() > new.len() {
        // 删除多余子节点（从后往前删，索引才稳定）
        for i in (new.len()..old.len()).rev() {
            patches.push(Patch::RemoveChild(parent.clone(), i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::wxml::WxmlParser;

    fn parse(wxml: &str) -> Vec<WxmlNode> {
        WxmlParser::new(wxml).parse().unwrap()
    }

    #[test]
    fn test_no_change() {
        let a = parse("<view><text>hi</text></view>");
        let b = parse("<view><text>hi</text></view>");
        let patches = diff_forest(&a, &b);
        assert!(patches.is_empty());
    }

    #[test]
    fn test_text_change_only() {
        let a = parse("<view><text>hi</text></view>");
        let b = parse("<view><text>bye</text></view>");
        let patches = diff_forest(&a, &b);
        assert_eq!(patches.len(), 1);
        assert!(!is_structural(&patches));
        assert!(matches!(patches[0], Patch::UpdateText(_, _)));
    }

    #[test]
    fn test_attr_change() {
        let a = parse(r#"<view class="a"></view>"#);
        let b = parse(r#"<view class="b"></view>"#);
        let patches = diff_forest(&a, &b);
        assert!(matches!(patches[0], Patch::SetAttr(_, _, _)));
        assert!(!is_structural(&patches));
    }

    #[test]
    fn test_structural_change() {
        let a = parse("<view><text>1</text></view>");
        let b = parse("<view><text>1</text><text>2</text></view>");
        let patches = diff_forest(&a, &b);
        assert!(is_structural(&patches));
    }

    #[test]
    fn test_tag_replace() {
        let a = parse("<view></view>");
        let b = parse("<text></text>");
        let patches = diff_forest(&a, &b);
        assert!(matches!(patches[0], Patch::Replace(_, _)));
    }
}
