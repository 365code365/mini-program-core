//! taffy 的量尺寸缓存在长竖排列表上要能中（third_party/taffy/PATCHES.md）。
//!
//! 结构与 `wx-components` 的 list 页一样：定高的 scroll-view → 宽度靠 stretch 的竖排列表 → 200 行
//! （竖排、定高、左右 padding）→ 每行一个叶子。taffy 0.12 原版按槽覆盖，改一行要把 200 行重量 20 多遍；
//! 升级 taffy 或去掉补丁之后这里会先红。
use std::cell::Cell;
use taffy::prelude::*;
fn list_tree() -> (TaffyTree<()>, NodeId, NodeId, Vec<NodeId>) {
    let mut t: TaffyTree<()> = TaffyTree::new();
    let rows: Vec<NodeId> = (0..200)
        .map(|_| {
            let leaf = t.new_leaf(Style::default()).unwrap();
            let rs = Style {
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.0,
                size: Size { width: auto(), height: length(115.5_f32) },
                padding: Rect { left: length(34.5_f32), right: length(34.5_f32), top: zero(), bottom: zero() },
                ..Default::default()
            };
            t.new_with_children(rs, &[leaf]).unwrap()
        })
        .collect();
    let list_style = Style {
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.0,
        padding: Rect { left: zero(), right: zero(), top: length(21.0_f32), bottom: length(21.0_f32) },
        ..Default::default()
    };
    let list = t.new_with_children(list_style, &rows).unwrap();
    let scroll_style = Style { flex_direction: FlexDirection::Column, size: Size { width: auto(), height: length(805.0_f32) }, ..Default::default() };
    let scroll = t.new_with_children(scroll_style, &[list]).unwrap();
    let root_style = Style { flex_direction: FlexDirection::Column, size: Size { width: length(1079.0_f32), height: auto() }, ..Default::default() };
    let root = t.new_with_children(root_style, &[scroll]).unwrap();
    (t, root, list, rows)
}
/// 排一遍，返回度量回调被调了几次
fn layout(t: &mut TaffyTree<()>, root: NodeId) -> u32 {
    let n = Cell::new(0u32);
    t.compute_layout_with_measure(root, Size::MAX_CONTENT, |known, _avail, _id, _ctx, _style| {
        n.set(n.get() + 1);
        Size { width: known.width.unwrap_or(0.0), height: known.height.unwrap_or(0.0) }
    })
    .unwrap();
    n.get()
}
#[test]
fn 改一行不重量整列() {
    let (mut t, root, list, rows) = list_tree();
    let first = layout(&mut t, root);
    // 原版 6600（每个叶子 33 次），补丁后 1400
    assert!(first <= 2000, "首次布局度量了 {first} 次");
    t.mark_dirty(rows[5]).unwrap();
    assert_eq!(layout(&mut t, root), 0, "只标脏一行（叶子没脏），不该再量任何叶子");
    t.mark_dirty(list).unwrap();
    assert_eq!(layout(&mut t, root), 0, "只标脏列表容器，200 行都该命中缓存");
    let y = t.layout(rows[199]).unwrap().location.y;
    assert!((y - (21.0 + 199.0 * 115.5)).abs() < 1.0, "第 200 行的位置 {y}");
}
