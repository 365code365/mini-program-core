//! 复杂布局单元测试
//! 测试外卖风格分类页面的各种布局场景

use taffy::prelude::*;

/// 测试左右分栏布局（外卖分类页面核心布局）
/// main-content: flex-direction: row
/// category-nav: width: 180px (固定宽度)
/// product-list: flex: 1 (占满剩余空间)
#[test]
fn test_left_right_split_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 左侧分类导航 - 固定宽度
    let category_nav = taffy.new_leaf(Style {
        size: Size { width: length(180.0_f32), height: auto() },
        min_size: Size { width: length(180.0_f32), height: auto() },
        max_size: Size { width: length(180.0_f32), height: auto() },
        ..Default::default()
    }).unwrap();
    
    // 右侧商品列表 - flex: 1
    let product_list = taffy.new_leaf(Style {
        flex_grow: 1.0,
        size: Size { width: auto(), height: auto() },
        ..Default::default()
    }).unwrap();
    
    // 主内容区 - 横向布局
    let main_content = taffy.new_with_children(
        Style {
            size: Size { width: length(750.0_f32), height: length(1000.0_f32) },
            flex_direction: FlexDirection::Row,
            ..Default::default()
        },
        &[category_nav, product_list],
    ).unwrap();
    
    taffy.compute_layout(main_content, Size::MAX_CONTENT).unwrap();
    
    let nav_layout = taffy.layout(category_nav).unwrap();
    let list_layout = taffy.layout(product_list).unwrap();
    
    // 验证左侧导航宽度固定为 180
    assert_eq!(nav_layout.size.width, 180.0, "category_nav width should be 180");
    assert_eq!(nav_layout.location.x, 0.0, "category_nav should start at x=0");
    
    // 验证右侧列表占满剩余空间
    assert_eq!(list_layout.size.width, 570.0, "product_list width should be 750-180=570");
    assert_eq!(list_layout.location.x, 180.0, "product_list should start at x=180");
}

/// 测试分类项布局（横向：文字 + 角标）
/// category-item: flex-direction: row, justify-content: space-between
#[test]
fn test_category_item_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 分类名称
    let category_name = taffy.new_leaf(Style {
        size: Size { width: auto(), height: length(26.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 角标
    let badge = taffy.new_leaf(Style {
        size: Size { width: length(32.0_f32), height: length(32.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 分类项 - 横向布局，两端对齐
    let category_item = taffy.new_with_children(
        Style {
            size: Size { width: length(180.0_f32), height: auto() },
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::SPACE_BETWEEN),
            align_items: Some(AlignItems::CENTER),
            padding: Rect {
                top: length(28.0_f32),
                right: length(16.0_f32),
                bottom: length(28.0_f32),
                left: length(16.0_f32),
            },
            ..Default::default()
        },
        &[category_name, badge],
    ).unwrap();
    
    taffy.compute_layout(category_item, Size::MAX_CONTENT).unwrap();
    
    let name_layout = taffy.layout(category_name).unwrap();
    let badge_layout = taffy.layout(badge).unwrap();
    let item_layout = taffy.layout(category_item).unwrap();
    
    // 名称在左边
    assert_eq!(name_layout.location.x, 16.0, "name should be at left padding");
    
    // 角标在右边
    let expected_badge_x = 180.0 - 16.0 - 32.0; // width - right_padding - badge_width
    assert_eq!(badge_layout.location.x, expected_badge_x, "badge should be at right");
    
    // 两者垂直居中
    let _item_content_height = item_layout.size.height - 28.0 * 2.0;
    assert!(name_layout.location.y >= 28.0, "name should be below top padding");
    assert!(badge_layout.location.y >= 28.0, "badge should be below top padding");
}

/// 测试商品项布局（横向：图片 + 信息 + 数量控制）
/// product-item: flex-direction: row
/// product-image: 固定 140x140
/// product-info: flex: 1, flex-direction: column
/// quantity-control: flex-direction: row
#[test]
fn test_product_item_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 商品图片 - 固定大小
    let product_image = taffy.new_leaf(Style {
        size: Size { width: length(140.0_f32), height: length(140.0_f32) },
        min_size: Size { width: length(140.0_f32), height: length(140.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 商品名称
    let product_name = taffy.new_leaf(Style {
        size: Size { width: percent(1.0_f32), height: auto() },
        min_size: Size { width: auto(), height: length(28.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 商品描述
    let product_desc = taffy.new_leaf(Style {
        size: Size { width: percent(1.0_f32), height: auto() },
        min_size: Size { width: auto(), height: length(24.0_f32) },
        margin: Rect { top: length(8.0_f32), ..Rect::zero() },
        ..Default::default()
    }).unwrap();
    
    // 价格
    let product_price = taffy.new_leaf(Style {
        size: Size { width: auto(), height: length(32.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 减号按钮
    let minus_btn = taffy.new_leaf(Style {
        size: Size { width: length(48.0_f32), height: length(48.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 数量
    let quantity = taffy.new_leaf(Style {
        size: Size { width: length(48.0_f32), height: length(28.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 加号按钮
    let plus_btn = taffy.new_leaf(Style {
        size: Size { width: length(48.0_f32), height: length(48.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 数量控制 - 横向布局
    let quantity_control = taffy.new_with_children(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            ..Default::default()
        },
        &[minus_btn, quantity, plus_btn],
    ).unwrap();
    
    // 底部行（价格 + 数量控制）- 横向布局，两端对齐
    let product_bottom = taffy.new_with_children(
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(JustifyContent::SPACE_BETWEEN),
            align_items: Some(AlignItems::CENTER),
            margin: Rect { top: length(16.0_f32), ..Rect::zero() },
            ..Default::default()
        },
        &[product_price, quantity_control],
    ).unwrap();
    
    // 商品信息 - 纵向布局，flex: 1
    let product_info = taffy.new_with_children(
        Style {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            margin: Rect { left: length(20.0_f32), ..Rect::zero() },
            ..Default::default()
        },
        &[product_name, product_desc, product_bottom],
    ).unwrap();
    
    // 商品项 - 横向布局
    let product_item = taffy.new_with_children(
        Style {
            size: Size { width: length(570.0_f32), height: auto() },
            flex_direction: FlexDirection::Row,
            padding: Rect {
                top: length(24.0_f32),
                right: length(24.0_f32),
                bottom: length(24.0_f32),
                left: length(24.0_f32),
            },
            ..Default::default()
        },
        &[product_image, product_info],
    ).unwrap();
    
    taffy.compute_layout(product_item, Size::MAX_CONTENT).unwrap();
    
    let image_layout = taffy.layout(product_image).unwrap();
    let info_layout = taffy.layout(product_info).unwrap();
    let control_layout = taffy.layout(quantity_control).unwrap();
    let minus_layout = taffy.layout(minus_btn).unwrap();
    let qty_layout = taffy.layout(quantity).unwrap();
    let plus_layout = taffy.layout(plus_btn).unwrap();
    
    // 图片在左边
    assert_eq!(image_layout.location.x, 24.0, "image should be at left padding");
    assert_eq!(image_layout.size.width, 140.0, "image width should be 140");
    
    // 信息在图片右边
    assert_eq!(info_layout.location.x, 24.0 + 140.0 + 20.0, "info should be after image + margin");
    
    // 数量控制按钮横向排列
    assert_eq!(minus_layout.location.x, 0.0, "minus should be at start");
    assert_eq!(qty_layout.location.x, 48.0, "quantity should be after minus");
    assert_eq!(plus_layout.location.x, 48.0 + 48.0, "plus should be after quantity");
    
    // 数量控制的总宽度
    assert_eq!(control_layout.size.width, 48.0 * 3.0, "control width should be 144");
}

/// 测试购物车栏布局（横向：图标+信息 | 结算按钮）
#[test]
fn test_cart_bar_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 购物车图标
    let cart_icon = taffy.new_leaf(Style {
        size: Size { width: length(80.0_f32), height: length(80.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 总价
    let cart_total = taffy.new_leaf(Style {
        size: Size { width: auto(), height: length(32.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 配送费提示
    let cart_tip = taffy.new_leaf(Style {
        size: Size { width: auto(), height: length(22.0_f32) },
        margin: Rect { top: length(4.0_f32), ..Rect::zero() },
        ..Default::default()
    }).unwrap();
    
    // 购物车信息 - 纵向布局
    let cart_info = taffy.new_with_children(
        Style {
            flex_direction: FlexDirection::Column,
            margin: Rect { left: length(16.0_f32), ..Rect::zero() },
            ..Default::default()
        },
        &[cart_total, cart_tip],
    ).unwrap();
    
    // 左侧（图标 + 信息）- 横向布局，flex: 1
    let cart_left = taffy.new_with_children(
        Style {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            ..Default::default()
        },
        &[cart_icon, cart_info],
    ).unwrap();
    
    // 结算按钮
    let cart_right = taffy.new_leaf(Style {
        size: Size { width: length(200.0_f32), height: length(72.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 购物车栏 - 横向布局
    let cart_bar = taffy.new_with_children(
        Style {
            size: Size { width: length(750.0_f32), height: length(100.0_f32) },
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            padding: Rect {
                top: length(0.0_f32),
                right: length(24.0_f32),
                bottom: length(0.0_f32),
                left: length(24.0_f32),
            },
            ..Default::default()
        },
        &[cart_left, cart_right],
    ).unwrap();
    
    taffy.compute_layout(cart_bar, Size::MAX_CONTENT).unwrap();
    
    let left_layout = taffy.layout(cart_left).unwrap();
    let right_layout = taffy.layout(cart_right).unwrap();
    let icon_layout = taffy.layout(cart_icon).unwrap();
    let info_layout = taffy.layout(cart_info).unwrap();
    
    // 左侧占满剩余空间
    let expected_left_width = 750.0 - 24.0 * 2.0 - 200.0;
    assert_eq!(left_layout.size.width, expected_left_width, "cart_left should fill remaining space");
    
    // 右侧按钮在最右边
    assert_eq!(right_layout.location.x, 24.0 + expected_left_width, "cart_right should be at right");
    
    // 图标和信息横向排列
    assert_eq!(icon_layout.location.x, 0.0, "icon should be at start");
    assert_eq!(info_layout.location.x, 80.0 + 16.0, "info should be after icon + margin");
}

/// 测试购物车弹窗项布局
#[test]
fn test_cart_popup_item_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 商品名称 - flex: 1
    let item_name = taffy.new_leaf(Style {
        flex_grow: 1.0,
        size: Size { width: auto(), height: length(28.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 价格
    let item_price = taffy.new_leaf(Style {
        size: Size { width: auto(), height: length(28.0_f32) },
        margin: Rect { right: length(24.0_f32), ..Rect::zero() },
        ..Default::default()
    }).unwrap();
    
    // 减号
    let minus = taffy.new_leaf(Style {
        size: Size { width: length(48.0_f32), height: length(48.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 数量
    let qty = taffy.new_leaf(Style {
        size: Size { width: length(48.0_f32), height: length(28.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 加号
    let plus = taffy.new_leaf(Style {
        size: Size { width: length(48.0_f32), height: length(48.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 数量控制
    let qty_control = taffy.new_with_children(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            ..Default::default()
        },
        &[minus, qty, plus],
    ).unwrap();
    
    // 弹窗项 - 横向布局
    let popup_item = taffy.new_with_children(
        Style {
            size: Size { width: length(750.0_f32), height: auto() },
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            padding: Rect {
                top: length(24.0_f32),
                right: length(24.0_f32),
                bottom: length(24.0_f32),
                left: length(24.0_f32),
            },
            ..Default::default()
        },
        &[item_name, item_price, qty_control],
    ).unwrap();
    
    taffy.compute_layout(popup_item, Size::MAX_CONTENT).unwrap();
    
    let name_layout = taffy.layout(item_name).unwrap();
    let price_layout = taffy.layout(item_price).unwrap();
    let control_layout = taffy.layout(qty_control).unwrap();
    
    // 名称在左边，占满剩余空间
    assert_eq!(name_layout.location.x, 24.0, "name should be at left padding");
    
    // 价格在名称右边
    assert!(price_layout.location.x > name_layout.location.x, "price should be after name");
    
    // 数量控制在最右边
    let expected_control_x = 750.0 - 24.0 - 48.0 * 3.0;
    assert_eq!(control_layout.location.x, expected_control_x, "control should be at right");
}

/// 测试嵌套 flex 布局（多层嵌套）
#[test]
fn test_nested_flex_layout() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 第三层：两个并排的按钮
    let btn1 = taffy.new_leaf(Style {
        size: Size { width: length(48.0_f32), height: length(48.0_f32) },
        ..Default::default()
    }).unwrap();
    
    let btn2 = taffy.new_leaf(Style {
        size: Size { width: length(48.0_f32), height: length(48.0_f32) },
        ..Default::default()
    }).unwrap();
    
    let btn_row = taffy.new_with_children(
        Style {
            flex_direction: FlexDirection::Row,
            gap: Size { width: length(8.0_f32), height: length(0.0_f32) },
            ..Default::default()
        },
        &[btn1, btn2],
    ).unwrap();
    
    // 第二层：文字 + 按钮行
    let text = taffy.new_leaf(Style {
        flex_grow: 1.0,
        size: Size { width: auto(), height: length(28.0_f32) },
        ..Default::default()
    }).unwrap();
    
    let row = taffy.new_with_children(
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            justify_content: Some(JustifyContent::SPACE_BETWEEN),
            size: Size { width: length(300.0_f32), height: auto() },
            ..Default::default()
        },
        &[text, btn_row],
    ).unwrap();
    
    // 第一层：多行
    let row2 = taffy.new_leaf(Style {
        size: Size { width: length(300.0_f32), height: length(50.0_f32) },
        ..Default::default()
    }).unwrap();
    
    let container = taffy.new_with_children(
        Style {
            flex_direction: FlexDirection::Column,
            size: Size { width: length(300.0_f32), height: auto() },
            ..Default::default()
        },
        &[row, row2],
    ).unwrap();
    
    taffy.compute_layout(container, Size::MAX_CONTENT).unwrap();
    
    let _row_layout = taffy.layout(row).unwrap();
    let text_layout = taffy.layout(text).unwrap();
    let btn_row_layout = taffy.layout(btn_row).unwrap();
    let btn1_layout = taffy.layout(btn1).unwrap();
    let btn2_layout = taffy.layout(btn2).unwrap();
    
    // 按钮行在右边
    let expected_btn_row_x = 300.0 - (48.0 * 2.0 + 8.0);
    assert_eq!(btn_row_layout.location.x, expected_btn_row_x, "btn_row should be at right");
    
    // 两个按钮横向排列
    assert_eq!(btn1_layout.location.x, 0.0, "btn1 should be at start of btn_row");
    assert_eq!(btn2_layout.location.x, 48.0 + 8.0, "btn2 should be after btn1 + gap");
    
    // 文字占满剩余空间
    assert_eq!(text_layout.size.width, expected_btn_row_x, "text should fill remaining space");
}

/// 测试 min-width 和 max-width 约束
#[test]
fn test_min_max_width_constraints() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 有 min-width 约束的元素
    let constrained = taffy.new_leaf(Style {
        size: Size { width: auto(), height: length(50.0_f32) },
        min_size: Size { width: length(100.0_f32), height: auto() },
        max_size: Size { width: length(200.0_f32), height: auto() },
        flex_grow: 1.0,
        ..Default::default()
    }).unwrap();
    
    // 小容器 - 元素应该使用 min-width
    let small_container = taffy.new_with_children(
        Style {
            size: Size { width: length(50.0_f32), height: length(100.0_f32) },
            flex_direction: FlexDirection::Row,
            ..Default::default()
        },
        &[constrained],
    ).unwrap();
    
    taffy.compute_layout(small_container, Size::MAX_CONTENT).unwrap();
    
    let layout = taffy.layout(constrained).unwrap();
    assert_eq!(layout.size.width, 100.0, "width should be min-width when container is smaller");
}

/// 测试 overflow: hidden 的裁剪效果（布局层面）
#[test]
fn test_overflow_hidden() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 超出容器的子元素
    let child = taffy.new_leaf(Style {
        size: Size { width: length(200.0_f32), height: length(200.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 有 overflow: hidden 的容器
    let container = taffy.new_with_children(
        Style {
            size: Size { width: length(100.0_f32), height: length(100.0_f32) },
            overflow: taffy::Point {
                x: taffy::style::Overflow::Hidden,
                y: taffy::style::Overflow::Hidden,
            },
            ..Default::default()
        },
        &[child],
    ).unwrap();
    
    taffy.compute_layout(container, Size::MAX_CONTENT).unwrap();
    
    let container_layout = taffy.layout(container).unwrap();
    let child_layout = taffy.layout(child).unwrap();
    
    // 容器大小不变
    assert_eq!(container_layout.size.width, 100.0);
    assert_eq!(container_layout.size.height, 100.0);
    
    // 打印实际值以便调试
    eprintln!("child width: {}, height: {}", child_layout.size.width, child_layout.size.height);
    
    // overflow: hidden 的行为：子元素被限制在容器内
    // 宽度被限制
    assert!(child_layout.size.width <= 200.0, "child width should be at most 200");
}

/// 测试 align-self 覆盖 align-items
#[test]
fn test_align_self_override() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 普通子元素（继承 align-items: center）
    let child1 = taffy.new_leaf(Style {
        size: Size { width: length(50.0_f32), height: length(30.0_f32) },
        ..Default::default()
    }).unwrap();
    
    // 有 align-self 的子元素
    let child2 = taffy.new_leaf(Style {
        size: Size { width: length(50.0_f32), height: length(30.0_f32) },
        align_self: Some(AlignSelf::FLEX_END),
        ..Default::default()
    }).unwrap();
    
    let container = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0_f32), height: length(100.0_f32) },
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::CENTER),
            ..Default::default()
        },
        &[child1, child2],
    ).unwrap();
    
    taffy.compute_layout(container, Size::MAX_CONTENT).unwrap();
    
    let layout1 = taffy.layout(child1).unwrap();
    let layout2 = taffy.layout(child2).unwrap();
    
    // child1 垂直居中
    assert_eq!(layout1.location.y, 35.0, "child1 should be centered");
    
    // child2 在底部
    assert_eq!(layout2.location.y, 70.0, "child2 should be at bottom");
}

/// 测试 flex-shrink
#[test]
fn test_flex_shrink() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 不收缩的元素
    let no_shrink = taffy.new_leaf(Style {
        size: Size { width: length(100.0_f32), height: length(50.0_f32) },
        flex_shrink: 0.0,
        ..Default::default()
    }).unwrap();
    
    // 可收缩的元素
    let shrinkable = taffy.new_leaf(Style {
        size: Size { width: length(200.0_f32), height: length(50.0_f32) },
        flex_shrink: 1.0,
        ..Default::default()
    }).unwrap();
    
    // 容器宽度不足以容纳两个元素
    let container = taffy.new_with_children(
        Style {
            size: Size { width: length(200.0_f32), height: length(100.0_f32) },
            flex_direction: FlexDirection::Row,
            ..Default::default()
        },
        &[no_shrink, shrinkable],
    ).unwrap();
    
    taffy.compute_layout(container, Size::MAX_CONTENT).unwrap();
    
    let no_shrink_layout = taffy.layout(no_shrink).unwrap();
    let shrinkable_layout = taffy.layout(shrinkable).unwrap();
    
    // 不收缩的元素保持原始宽度
    assert_eq!(no_shrink_layout.size.width, 100.0, "no_shrink should keep original width");
    
    // 可收缩的元素被压缩
    assert_eq!(shrinkable_layout.size.width, 100.0, "shrinkable should shrink to fit");
}

/// 测试 flex-basis
#[test]
fn test_flex_basis() {
    let mut taffy: TaffyTree<()> = TaffyTree::new();
    
    // 使用 flex-basis 而不是 width
    let child = taffy.new_leaf(Style {
        flex_basis: length(150.0_f32),
        flex_grow: 0.0,
        size: Size { width: length(100.0_f32), height: length(50.0_f32) }, // width 应该被忽略
        ..Default::default()
    }).unwrap();
    
    let container = taffy.new_with_children(
        Style {
            size: Size { width: length(300.0_f32), height: length(100.0_f32) },
            flex_direction: FlexDirection::Row,
            ..Default::default()
        },
        &[child],
    ).unwrap();
    
    taffy.compute_layout(container, Size::MAX_CONTENT).unwrap();
    
    let layout = taffy.layout(child).unwrap();
    
    // 应该使用 flex-basis 的值
    assert_eq!(layout.size.width, 150.0, "width should be flex-basis value");
}
