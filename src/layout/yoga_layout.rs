//! Yoga 布局引擎 (使用 Taffy - Rust 实现的 Flexbox 布局)

use taffy::prelude::*;
use std::collections::HashMap;

/// 布局节点
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for LayoutNode {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }
}

/// 样式属性（从 WXSS 解析）
#[derive(Debug, Clone, Default)]
pub struct LayoutStyle {
    // 尺寸
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    
    // Flex 属性
    pub flex_direction: FlexDir,
    pub flex_wrap: FlexWrapMode,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,
    pub align_self: AlignSelf,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Option<f32>,
    
    // 间距
    pub margin: [f32; 4],      // top, right, bottom, left
    pub padding: [f32; 4],     // top, right, bottom, left
    pub gap: f32,
    
    // 定位
    pub position: Position,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum FlexDir {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum FlexWrapMode {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    #[default]
    Stretch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AlignContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    #[default]
    Stretch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AlignSelf {
    #[default]
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

/// Yoga 布局树
pub struct YogaLayout {
    taffy: TaffyTree,
    root: NodeId,
    node_map: HashMap<usize, NodeId>,
    results: HashMap<usize, LayoutNode>,
}

impl YogaLayout {
    pub fn new(width: f32, height: f32) -> Self {
        let mut taffy = TaffyTree::new();
        
        // 创建根节点
        let root = taffy.new_leaf(Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            ..Default::default()
        }).unwrap();
        
        Self {
            taffy,
            root,
            node_map: HashMap::new(),
            results: HashMap::new(),
        }
    }
    
    /// 添加节点
    pub fn add_node(&mut self, id: usize, parent_id: Option<usize>, style: &LayoutStyle) -> usize {
        let taffy_style = self.convert_style(style);
        let node = self.taffy.new_leaf(taffy_style).unwrap();
        
        // 添加到父节点
        let parent = parent_id
            .and_then(|pid| self.node_map.get(&pid).copied())
            .unwrap_or(self.root);
        
        self.taffy.add_child(parent, node).unwrap();
        self.node_map.insert(id, node);
        
        id
    }
    
    /// 计算布局
    pub fn compute(&mut self) {
        self.taffy.compute_layout(self.root, Size::MAX_CONTENT).unwrap();
        
        // 收集布局结果
        self.results.clear();
        self.collect_layout(self.root, 0.0, 0.0, 0);
    }
    
    fn collect_layout(&mut self, node: NodeId, offset_x: f32, offset_y: f32, id: usize) {
        let layout = self.taffy.layout(node).unwrap();
        
        let x = offset_x + layout.location.x;
        let y = offset_y + layout.location.y;
        
        self.results.insert(id, LayoutNode {
            x,
            y,
            width: layout.size.width,
            height: layout.size.height,
        });
        
        // 递归处理子节点
        let children: Vec<_> = self.taffy.children(node).unwrap().into_iter().collect();
        for (i, child) in children.iter().enumerate() {
            // 查找子节点 ID
            let child_id = self.node_map.iter()
                .find(|(_, &n)| n == *child)
                .map(|(&id, _)| id)
                .unwrap_or(id * 1000 + i + 1);
            
            self.collect_layout(*child, x, y, child_id);
        }
    }
    
    /// 获取节点布局
    pub fn get_layout(&self, id: usize) -> Option<&LayoutNode> {
        self.results.get(&id)
    }
    
    /// 获取所有布局结果
    pub fn get_all_layouts(&self) -> &HashMap<usize, LayoutNode> {
        &self.results
    }
    
    /// 转换样式
    fn convert_style(&self, style: &LayoutStyle) -> Style {
        Style {
            display: Display::Flex,
            
            size: Size {
                width: style.width.map(length).unwrap_or(auto()),
                height: style.height.map(length).unwrap_or(auto()),
            },
            
            min_size: Size {
                width: style.min_width.map(length).unwrap_or(auto()),
                height: style.min_height.map(length).unwrap_or(auto()),
            },
            
            max_size: Size {
                width: style.max_width.map(length).unwrap_or(auto()),
                height: style.max_height.map(length).unwrap_or(auto()),
            },
            
            flex_direction: match style.flex_direction {
                FlexDir::Row => taffy::FlexDirection::Row,
                FlexDir::RowReverse => taffy::FlexDirection::RowReverse,
                FlexDir::Column => taffy::FlexDirection::Column,
                FlexDir::ColumnReverse => taffy::FlexDirection::ColumnReverse,
            },
            
            flex_wrap: match style.flex_wrap {
                FlexWrapMode::NoWrap => taffy::FlexWrap::NoWrap,
                FlexWrapMode::Wrap => taffy::FlexWrap::Wrap,
                FlexWrapMode::WrapReverse => taffy::FlexWrap::WrapReverse,
            },
            
            justify_content: Some(match style.justify_content {
                JustifyContent::FlexStart => taffy::JustifyContent::FLEX_START,
                JustifyContent::FlexEnd => taffy::JustifyContent::FLEX_END,
                JustifyContent::Center => taffy::JustifyContent::CENTER,
                JustifyContent::SpaceBetween => taffy::JustifyContent::SPACE_BETWEEN,
                JustifyContent::SpaceAround => taffy::JustifyContent::SPACE_AROUND,
                JustifyContent::SpaceEvenly => taffy::JustifyContent::SPACE_EVENLY,
            }),
            
            align_items: Some(match style.align_items {
                AlignItems::FlexStart => taffy::AlignItems::FLEX_START,
                AlignItems::FlexEnd => taffy::AlignItems::FLEX_END,
                AlignItems::Center => taffy::AlignItems::CENTER,
                AlignItems::Baseline => taffy::AlignItems::BASELINE,
                AlignItems::Stretch => taffy::AlignItems::STRETCH,
            }),
            
            align_content: Some(match style.align_content {
                AlignContent::FlexStart => taffy::AlignContent::FLEX_START,
                AlignContent::FlexEnd => taffy::AlignContent::FLEX_END,
                AlignContent::Center => taffy::AlignContent::CENTER,
                AlignContent::SpaceBetween => taffy::AlignContent::SPACE_BETWEEN,
                AlignContent::SpaceAround => taffy::AlignContent::SPACE_AROUND,
                AlignContent::Stretch => taffy::AlignContent::STRETCH,
            }),
            
            align_self: match style.align_self {
                AlignSelf::Auto => None,
                AlignSelf::FlexStart => Some(taffy::AlignSelf::FLEX_START),
                AlignSelf::FlexEnd => Some(taffy::AlignSelf::FLEX_END),
                AlignSelf::Center => Some(taffy::AlignSelf::CENTER),
                AlignSelf::Baseline => Some(taffy::AlignSelf::BASELINE),
                AlignSelf::Stretch => Some(taffy::AlignSelf::STRETCH),
            },
            
            flex_grow: style.flex_grow,
            flex_shrink: style.flex_shrink,
            flex_basis: style.flex_basis.map(length).unwrap_or(auto()),
            
            margin: Rect {
                top: length(style.margin[0]),
                right: length(style.margin[1]),
                bottom: length(style.margin[2]),
                left: length(style.margin[3]),
            },
            
            padding: Rect {
                top: length(style.padding[0]),
                right: length(style.padding[1]),
                bottom: length(style.padding[2]),
                left: length(style.padding[3]),
            },
            
            gap: Size {
                width: length(style.gap),
                height: length(style.gap),
            },
            
            position: match style.position {
                Position::Relative => taffy::Position::Relative,
                Position::Absolute => taffy::Position::Absolute,
            },
            
            inset: Rect {
                top: style.top.map(length).unwrap_or(auto()),
                right: style.right.map(length).unwrap_or(auto()),
                bottom: style.bottom.map(length).unwrap_or(auto()),
                left: style.left.map(length).unwrap_or(auto()),
            },
            
            ..Default::default()
        }
    }
}

impl LayoutStyle {
    /// 从 CSS 属性解析
    pub fn from_css(props: &HashMap<String, String>, screen_width: f32) -> Self {
        let mut style = Self::default();
        style.flex_shrink = 1.0; // 默认值
        
        for (key, value) in props {
            match key.as_str() {
                "width" => style.width = parse_dimension(value, screen_width),
                "height" => style.height = parse_dimension(value, screen_width),
                "min-width" => style.min_width = parse_dimension(value, screen_width),
                "min-height" => style.min_height = parse_dimension(value, screen_width),
                "max-width" => style.max_width = parse_dimension(value, screen_width),
                "max-height" => style.max_height = parse_dimension(value, screen_width),
                
                "flex-direction" => {
                    style.flex_direction = match value.as_str() {
                        "row" => FlexDir::Row,
                        "row-reverse" => FlexDir::RowReverse,
                        "column" => FlexDir::Column,
                        "column-reverse" => FlexDir::ColumnReverse,
                        _ => FlexDir::Row,
                    };
                }
                
                "flex-wrap" => {
                    style.flex_wrap = match value.as_str() {
                        "nowrap" => FlexWrapMode::NoWrap,
                        "wrap" => FlexWrapMode::Wrap,
                        "wrap-reverse" => FlexWrapMode::WrapReverse,
                        _ => FlexWrapMode::NoWrap,
                    };
                }
                
                "justify-content" => {
                    style.justify_content = match value.as_str() {
                        "flex-start" | "start" => JustifyContent::FlexStart,
                        "flex-end" | "end" => JustifyContent::FlexEnd,
                        "center" => JustifyContent::Center,
                        "space-between" => JustifyContent::SpaceBetween,
                        "space-around" => JustifyContent::SpaceAround,
                        "space-evenly" => JustifyContent::SpaceEvenly,
                        _ => JustifyContent::FlexStart,
                    };
                }
                
                "align-items" => {
                    style.align_items = match value.as_str() {
                        "flex-start" | "start" => AlignItems::FlexStart,
                        "flex-end" | "end" => AlignItems::FlexEnd,
                        "center" => AlignItems::Center,
                        "baseline" => AlignItems::Baseline,
                        "stretch" => AlignItems::Stretch,
                        _ => AlignItems::Stretch,
                    };
                }
                
                "align-self" => {
                    style.align_self = match value.as_str() {
                        "auto" => AlignSelf::Auto,
                        "flex-start" | "start" => AlignSelf::FlexStart,
                        "flex-end" | "end" => AlignSelf::FlexEnd,
                        "center" => AlignSelf::Center,
                        "baseline" => AlignSelf::Baseline,
                        "stretch" => AlignSelf::Stretch,
                        _ => AlignSelf::Auto,
                    };
                }
                
                "flex-grow" => {
                    style.flex_grow = value.parse().unwrap_or(0.0);
                }
                
                "flex-shrink" => {
                    style.flex_shrink = value.parse().unwrap_or(1.0);
                }
                
                "flex-basis" => {
                    style.flex_basis = parse_dimension(value, screen_width);
                }
                
                "flex" => {
                    // 简写: flex: grow shrink basis
                    let parts: Vec<&str> = value.split_whitespace().collect();
                    if let Some(grow) = parts.get(0) {
                        style.flex_grow = grow.parse().unwrap_or(0.0);
                    }
                    if let Some(shrink) = parts.get(1) {
                        style.flex_shrink = shrink.parse().unwrap_or(1.0);
                    }
                    if let Some(basis) = parts.get(2) {
                        style.flex_basis = parse_dimension(basis, screen_width);
                    }
                }
                
                "gap" => {
                    style.gap = parse_dimension(value, screen_width).unwrap_or(0.0);
                }
                
                "margin" => {
                    style.margin = parse_spacing(value, screen_width);
                }
                "margin-top" => style.margin[0] = parse_dimension(value, screen_width).unwrap_or(0.0),
                "margin-right" => style.margin[1] = parse_dimension(value, screen_width).unwrap_or(0.0),
                "margin-bottom" => style.margin[2] = parse_dimension(value, screen_width).unwrap_or(0.0),
                "margin-left" => style.margin[3] = parse_dimension(value, screen_width).unwrap_or(0.0),
                
                "padding" => {
                    style.padding = parse_spacing(value, screen_width);
                }
                "padding-top" => style.padding[0] = parse_dimension(value, screen_width).unwrap_or(0.0),
                "padding-right" => style.padding[1] = parse_dimension(value, screen_width).unwrap_or(0.0),
                "padding-bottom" => style.padding[2] = parse_dimension(value, screen_width).unwrap_or(0.0),
                "padding-left" => style.padding[3] = parse_dimension(value, screen_width).unwrap_or(0.0),
                
                "position" => {
                    style.position = match value.as_str() {
                        "absolute" => Position::Absolute,
                        _ => Position::Relative,
                    };
                }
                
                "top" => style.top = parse_dimension(value, screen_width),
                "right" => style.right = parse_dimension(value, screen_width),
                "bottom" => style.bottom = parse_dimension(value, screen_width),
                "left" => style.left = parse_dimension(value, screen_width),
                
                "display" => {
                    if value == "flex" {
                        // 默认就是 flex
                    }
                }
                
                _ => {}
            }
        }
        
        style
    }
}

/// 解析尺寸值
fn parse_dimension(value: &str, screen_width: f32) -> Option<f32> {
    let value = value.trim();
    
    if value == "auto" || value.is_empty() {
        return None;
    }
    
    if value.ends_with("rpx") {
        let num: f32 = value.trim_end_matches("rpx").parse().ok()?;
        Some(num * screen_width / 750.0)
    } else if value.ends_with("px") {
        value.trim_end_matches("px").parse().ok()
    } else if value.ends_with('%') {
        let percent: f32 = value.trim_end_matches('%').parse().ok()?;
        Some(percent / 100.0 * screen_width)
    } else {
        value.parse().ok()
    }
}

/// 解析间距值 (margin/padding)
fn parse_spacing(value: &str, screen_width: f32) -> [f32; 4] {
    let parts: Vec<&str> = value.split_whitespace().collect();
    
    match parts.len() {
        1 => {
            let v = parse_dimension(parts[0], screen_width).unwrap_or(0.0);
            [v, v, v, v]
        }
        2 => {
            let v = parse_dimension(parts[0], screen_width).unwrap_or(0.0);
            let h = parse_dimension(parts[1], screen_width).unwrap_or(0.0);
            [v, h, v, h]
        }
        3 => {
            let top = parse_dimension(parts[0], screen_width).unwrap_or(0.0);
            let h = parse_dimension(parts[1], screen_width).unwrap_or(0.0);
            let bottom = parse_dimension(parts[2], screen_width).unwrap_or(0.0);
            [top, h, bottom, h]
        }
        4 => {
            let top = parse_dimension(parts[0], screen_width).unwrap_or(0.0);
            let right = parse_dimension(parts[1], screen_width).unwrap_or(0.0);
            let bottom = parse_dimension(parts[2], screen_width).unwrap_or(0.0);
            let left = parse_dimension(parts[3], screen_width).unwrap_or(0.0);
            [top, right, bottom, left]
        }
        _ => [0.0, 0.0, 0.0, 0.0],
    }
}
