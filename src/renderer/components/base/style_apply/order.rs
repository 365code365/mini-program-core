//! 声明生效顺序：简写要先于长写落地，否则 border 会被 border-color 冲掉
//!
//! `style_apply_new` 的一片。**纯搬迁**：从 792 行按职责切开，一行逻辑没改。
use super::*;

/// 一条声明的落地优先序：**简写在前，细项在后**。
///
/// 级联把所有中选规则合并成一张 map，简写与细项的先后关系在这一步就丢了。
/// 于是 `.dot{border:2rpx solid #ccc}` + `.dot.on{border-color:#FF6B35}` 两条规则
/// 合并后，map 里同时躺着 `border` 和 `border-color` —— 谁后落地谁赢。
/// map 的遍历顺序在 Rust 里是随机的（每进程一个 hash 种子），
/// 所以同一份源码**每次运行渲染结果都可能不同**（那个圆点时橙时灰）。
///
/// 真正的 CSS 模型是解析期就把简写展开成细项，这里用更小的改动达到同样效果：
/// 按「简写 → 方向细项 → 单属性细项」的固定档位排序，档位内按属性名排序，
/// 保证细项永远覆盖简写，且结果与运行次数无关。
pub(super) fn declaration_order(name: &str) -> u8 {
    match name {
        // 全能简写
        "font" | "background" | "border" | "border-radius" | "margin" | "padding"
        | "flex" | "transition" | "animation" | "grid-area" | "inset" => 0,
        // 方向/边简写（仍是简写，但比 `border` 更具体）
        "border-top" | "border-right" | "border-bottom" | "border-left"
        | "border-width" | "border-color" | "border-style"
        | "margin-block" | "margin-inline" | "padding-block" | "padding-inline"
        | "background-position" | "background-size" | "flex-flow" => 1,
        // 其余都是细项
        _ => 2,
    }
}

/// 把级联后的声明按确定性顺序取出（见 [`declaration_order`]）
pub(super) fn sorted_declarations(css: &HashMap<String, StyleValue>) -> Vec<(&str, &StyleValue)> {
    let mut out: Vec<(&str, &StyleValue)> = css.iter().map(|(k, v)| (k.as_str(), v)).collect();
    out.sort_by(|a, b| declaration_order(a.0).cmp(&declaration_order(b.0)).then(a.0.cmp(b.0)));
    out
}
