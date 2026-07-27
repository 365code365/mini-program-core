//! 布局系统单元测试
//! 测试 Flexbox 布局的各种场景

use taffy::prelude::*;

/// 测试基本的 Flexbox 列布局
#[test]
fn test_flex_column_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 创建三个子节点
    let child1 = taffy.new_leaf(Style {
        size: Size { width: length(100.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let child2 = taffy.new_leaf(Style {
        size: Size { width: length(100.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let child3 = taffy.new_leaf(Style {
        size: Size { width: length(100.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    // 创建父容器（列布局）
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: auto() },
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        &[child1, child2, child3],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    // 验证布局结果
    let layout1 = taffy.layout(child1).unwrap();
    let layout2 = taffy.layout(child2).unwrap();
    let layout3 = taffy.layout(child3).unwrap();
    
    assert_eq!(layout1.location.y, 0.0);
    assert_eq!(layout2.location.y, 50.0);
    assert_eq!(layout3.location.y, 100.0);
}

/// 测试 Flexbox 行布局
#[test]
fn test_flex_row_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child1 = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(100.0) },
        ..Default::default()
    }).unwrap();
    
    let child2 = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(100.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(100.0) },
            flex_direction: FlexDirection::Row,
            ..Default::default()
        },
        &[child1, child2],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout1 = taffy.layout(child1).unwrap();
    let layout2 = taffy.layout(child2).unwrap();
    
    assert_eq!(layout1.location.x, 0.0);
    assert_eq!(layout2.location.x, 50.0);
}

/// 测试 justify-content: center
#[test]
fn test_justify_content_center() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(100.0) },
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::CENTER),
            ..Default::default()
        },
        &[child],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout = taffy.layout(child).unwrap();
    assert_eq!(layout.location.x, 75.0); // (200 - 50) / 2
}

/// 测试 justify-content: space-between
#[test]
fn test_justify_content_space_between() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child1 = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let child2 = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(100.0) },
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::SPACE_BETWEEN),
            ..Default::default()
        },
        &[child1, child2],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout1 = taffy.layout(child1).unwrap();
    let layout2 = taffy.layout(child2).unwrap();
    
    assert_eq!(layout1.location.x, 0.0);
    assert_eq!(layout2.location.x, 150.0); // 200 - 50
}

/// 测试 align-items: center
#[test]
fn test_align_items_center() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(30.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(100.0) },
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            ..Default::default()
        },
        &[child],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout = taffy.layout(child).unwrap();
    assert_eq!(layout.location.y, 35.0); // (100 - 30) / 2
}

/// 测试 flex-grow
#[test]
fn test_flex_grow() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child1 = taffy.new_leaf(Style {
        flex_grow: 1.0,
        size: Size { width: auto(), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let child2 = taffy.new_leaf(Style {
        flex_grow: 2.0,
        size: Size { width: auto(), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(300.0), height: length(100.0) },
            flex_direction: FlexDirection::Row,
            ..Default::default()
        },
        &[child1, child2],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout1 = taffy.layout(child1).unwrap();
    let layout2 = taffy.layout(child2).unwrap();
    
    assert_eq!(layout1.size.width, 100.0); // 300 * 1/3
    assert_eq!(layout2.size.width, 200.0); // 300 * 2/3
}

/// 测试 padding
#[test]
fn test_padding() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(100.0) },
            padding: Rect {
                top: length(10.0),
                right: length(10.0),
                bottom: length(10.0),
                left: length(10.0),
            },
            ..Default::default()
        },
        &[child],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout = taffy.layout(child).unwrap();
    assert_eq!(layout.location.x, 10.0);
    assert_eq!(layout.location.y, 10.0);
}

/// 测试 margin
#[test]
fn test_margin() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(50.0) },
        margin: Rect {
            top: length(20.0),
            right: length(0.0),
            bottom: length(0.0),
            left: length(20.0),
        },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(100.0) },
            ..Default::default()
        },
        &[child],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout = taffy.layout(child).unwrap();
    assert_eq!(layout.location.x, 20.0);
    assert_eq!(layout.location.y, 20.0);
}

/// 测试 flex-wrap
#[test]
fn test_flex_wrap() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child1 = taffy.new_leaf(Style {
        size: Size { width: length(80.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let child2 = taffy.new_leaf(Style {
        size: Size { width: length(80.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let child3 = taffy.new_leaf(Style {
        size: Size { width: length(80.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: auto() },
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            ..Default::default()
        },
        &[child1, child2, child3],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout1 = taffy.layout(child1).unwrap();
    let layout2 = taffy.layout(child2).unwrap();
    let layout3 = taffy.layout(child3).unwrap();
    
    // 前两个在第一行
    assert_eq!(layout1.location.y, 0.0);
    assert_eq!(layout2.location.y, 0.0);
    // 第三个换行
    assert_eq!(layout3.location.y, 50.0);
}

/// 测试 gap
#[test]
fn test_gap() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child1 = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let child2 = taffy.new_leaf(Style {
        size: Size { width: length(50.0), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(100.0) },
            flex_direction: FlexDirection::Row,
            gap: Size { width: length(20.0), height: length(0.0) },
            ..Default::default()
        },
        &[child1, child2],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout1 = taffy.layout(child1).unwrap();
    let layout2 = taffy.layout(child2).unwrap();
    
    assert_eq!(layout1.location.x, 0.0);
    assert_eq!(layout2.location.x, 70.0); // 50 + 20 gap
}

/// 测试 absolute 定位
#[test]
fn test_absolute_position() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child = taffy.new_leaf(Style {
        position: Position::Absolute,
        size: Size { width: length(50.0), height: length(50.0) },
        inset: Rect {
            top: LengthPercentageAuto::length(10.0),
            right: LengthPercentageAuto::auto(),
            bottom: LengthPercentageAuto::auto(),
            left: LengthPercentageAuto::length(10.0),
        },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(200.0) },
            ..Default::default()
        },
        &[child],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout = taffy.layout(child).unwrap();
    assert_eq!(layout.location.x, 10.0);
    assert_eq!(layout.location.y, 10.0);
}

/// 测试百分比宽度
#[test]
fn test_percent_width() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    let child = taffy.new_leaf(Style {
        size: Size { width: percent(0.5), height: length(50.0) },
        ..Default::default()
    }).unwrap();
    
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(100.0) },
            ..Default::default()
        },
        &[child],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let layout = taffy.layout(child).unwrap();
    assert_eq!(layout.size.width, 100.0); // 200 * 50%
}

/// 测试嵌套布局
#[test]
fn test_nested_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 内层子节点
    let inner_child1 = taffy.new_leaf(Style {
        size: Size { width: length(30.0), height: length(30.0) },
        ..Default::default()
    }).unwrap();
    
    let inner_child2 = taffy.new_leaf(Style {
        size: Size { width: length(30.0), height: length(30.0) },
        ..Default::default()
    }).unwrap();
    
    // 内层容器（行布局）
    let inner_container = taffy.new_with_children(
        Style {
            size: Size { width: length(100.0), height: length(50.0) },
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::SPACE_BETWEEN),
            ..Default::default()
        },
        &[inner_child1, inner_child2],
    ).unwrap();
    
    // 外层容器
    let root = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0), height: length(200.0) },
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::CENTER),
            ..Default::default()
        },
        &[inner_container],
    ).unwrap();
    
    taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
    
    let inner_layout = taffy.layout(inner_container).unwrap();
    let child1_layout = taffy.layout(inner_child1).unwrap();
    let child2_layout = taffy.layout(inner_child2).unwrap();
    
    // 内层容器居中
    assert_eq!(inner_layout.location.x, 50.0); // (200 - 100) / 2
    
    // 内层子节点 space-between
    assert_eq!(child1_layout.location.x, 0.0);
    assert_eq!(child2_layout.location.x, 70.0); // 100 - 30
}


/// 测试 list-item 和 list-info 的布局（模拟实际问题）
/// list-item: flex-direction: row, align-items: center
/// list-info: flex: 1, flex-direction: column, 包含三个 text 子元素
#[test]
fn test_list_item_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // list-img: 固定大小
    let list_img = taffy.new_leaf(Style {
        size: Size { width: length(140.0), height: length(140.0) },
        ..Default::default()
    }).unwrap();
    
    // list-info 的子元素（三个 text）
    let list_name = taffy.new_leaf(Style {
        size: Size { width: Dimension::percent(1.0), height: auto() },
        min_size: Size { width: auto(), height: length(42.0) },
        ..Default::default()
    }).unwrap();
    
    let list_desc = taffy.new_leaf(Style {
        size: Size { width: Dimension::percent(1.0), height: auto() },
        min_size: Size { width: auto(), height: length(36.0) },
        margin: Rect { top: length(8.0), ..Rect::zero() },
        ..Default::default()
    }).unwrap();
    
    let list_price = taffy.new_leaf(Style {
        size: Size { width: Dimension::percent(1.0), height: auto() },
        min_size: Size { width: auto(), height: length(45.0) },
        margin: Rect { top: length(8.0), ..Rect::zero() },
        ..Default::default()
    }).unwrap();
    
    // list-info: flex: 1, flex-direction: column
    let list_info = taffy.new_with_children(
        Style {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            margin: Rect { left: length(16.0), ..Rect::zero() },
            ..Default::default()
        },
        &[list_name, list_desc, list_price],
    ).unwrap();
    
    // list-btn: 固定大小
    let list_btn = taffy.new_leaf(Style {
        size: Size { width: length(79.0), height: length(42.0) },
        ..Default::default()
    }).unwrap();
    
    // list-item: flex-direction: row, align-items: center
    let list_item = taffy.new_with_children(
        Style {
            size: Size { width: length(702.0), height: auto() },
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            padding: Rect { top: length(16.0), bottom: length(16.0), ..Rect::zero() },
            ..Default::default()
        },
        &[list_img, list_info, list_btn],
    ).unwrap();
    
    taffy.compute_layout(list_item, Size::MAX_CONTENT).unwrap();
    
    // 验证布局结果
    let list_item_layout = taffy.layout(list_item).unwrap();
    let list_img_layout = taffy.layout(list_img).unwrap();
    let list_info_layout = taffy.layout(list_info).unwrap();
    let list_btn_layout = taffy.layout(list_btn).unwrap();
    
    eprintln!("list_item: w={} h={}", list_item_layout.size.width, list_item_layout.size.height);
    eprintln!("list_img: x={} y={} w={} h={}", 
        list_img_layout.location.x, list_img_layout.location.y,
        list_img_layout.size.width, list_img_layout.size.height);
    eprintln!("list_info: x={} y={} w={} h={}", 
        list_info_layout.location.x, list_info_layout.location.y,
        list_info_layout.size.width, list_info_layout.size.height);
    eprintln!("list_btn: x={} y={} w={} h={}", 
        list_btn_layout.location.x, list_btn_layout.location.y,
        list_btn_layout.size.width, list_btn_layout.size.height);
    
    // list-info 的高度应该是其子元素高度的总和：42 + 8 + 36 + 8 + 45 = 139
    let expected_list_info_height = 42.0 + 8.0 + 36.0 + 8.0 + 45.0;
    assert!(list_info_layout.size.height >= expected_list_info_height - 1.0, 
        "list_info height should be at least {}, but got {}", 
        expected_list_info_height, list_info_layout.size.height);
}


/// 测试 absolute 定位与 padding 的交互
#[test]
fn test_absolute_with_padding() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 创建 absolute 定位的子元素（badge）
    let badge = taffy.new_leaf(Style {
        position: Position::Absolute,
        size: Size { width: length(50.0), height: length(20.0) },
        inset: Rect {
            top: LengthPercentageAuto::length(0.0),
            right: LengthPercentageAuto::auto(),
            bottom: LengthPercentageAuto::auto(),
            left: LengthPercentageAuto::length(0.0),
        },
        ..Default::default()
    }).unwrap();
    
    // 创建普通子元素
    let content = taffy.new_leaf(Style {
        size: Size { width: length(100.0), height: length(100.0) },
        ..Default::default()
    }).unwrap();
    
    // 创建父容器（position: relative，有 padding）
    let card = taffy.new_with_children(
        Style {
            position: Position::Relative,
            size: Size { width: length(200.0), height: length(200.0) },
            padding: Rect {
                top: length(10.0),
                right: length(10.0),
                bottom: length(10.0),
                left: length(10.0),
            },
            ..Default::default()
        },
        &[badge, content],
    ).unwrap();
    
    taffy.compute_layout(card, Size::MAX_CONTENT).unwrap();
    
    let badge_layout = taffy.layout(badge).unwrap();
    let content_layout = taffy.layout(content).unwrap();
    
    eprintln!("Badge: x={}, y={}, w={}, h={}", 
        badge_layout.location.x, badge_layout.location.y,
        badge_layout.size.width, badge_layout.size.height);
    eprintln!("Content: x={}, y={}, w={}, h={}", 
        content_layout.location.x, content_layout.location.y,
        content_layout.size.width, content_layout.size.height);
    
    // absolute 定位的元素应该在 (0, 0)，忽略 padding
    assert_eq!(badge_layout.location.x, 0.0);
    assert_eq!(badge_layout.location.y, 0.0);
    
    // 普通元素应该在 padding 内
    assert_eq!(content_layout.location.x, 10.0);
    assert_eq!(content_layout.location.y, 10.0);
}
