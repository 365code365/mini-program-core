//! Swiper 轮播图组件

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Rect as GeoRect};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Swiper 状态管理器
pub struct SwiperStateManager {
    states: HashMap<String, SwiperState>,
}

#[derive(Clone)]
pub struct SwiperState {
    pub current: usize,
    pub total: usize,
    pub last_update: std::time::Instant,
    pub autoplay_interval: u64,
    pub autoplay: bool,
}

impl SwiperStateManager {
    pub fn new() -> Self {
        Self { states: HashMap::new() }
    }
    
    pub fn get_or_create(&mut self, id: &str, total: usize, autoplay: bool, interval: u64) -> &mut SwiperState {
        self.states.entry(id.to_string()).or_insert_with(|| SwiperState {
            current: 0,
            total,
            last_update: std::time::Instant::now(),
            autoplay_interval: interval,
            autoplay,
        })
    }
    
    pub fn get(&self, id: &str) -> Option<&SwiperState> {
        self.states.get(id)
    }
    
    pub fn set_current(&mut self, id: &str, current: usize) {
        if let Some(state) = self.states.get_mut(id) {
            state.current = current;
            state.last_update = std::time::Instant::now();
        }
    }
    
    pub fn next(&mut self, id: &str) {
        if let Some(state) = self.states.get_mut(id) {
            state.current = (state.current + 1) % state.total;
            state.last_update = std::time::Instant::now();
        }
    }
    
    pub fn prev(&mut self, id: &str) {
        if let Some(state) = self.states.get_mut(id) {
            if state.current == 0 {
                state.current = state.total.saturating_sub(1);
            } else {
                state.current -= 1;
            }
            state.last_update = std::time::Instant::now();
        }
    }
    
    /// 检查并执行自动播放
    pub fn check_autoplay(&mut self, id: &str) -> bool {
        if let Some(state) = self.states.get_mut(id) {
            if state.autoplay && state.total > 1 {
                let elapsed = state.last_update.elapsed().as_millis() as u64;
                if elapsed >= state.autoplay_interval {
                    state.current = (state.current + 1) % state.total;
                    state.last_update = std::time::Instant::now();
                    return true;
                }
            }
        }
        false
    }
}

/// 全局 Swiper 状态管理器
pub static SWIPER_MANAGER: Lazy<Mutex<SwiperStateManager>> = Lazy::new(|| {
    Mutex::new(SwiperStateManager::new())
});

/// Swiper 组件
pub struct SwiperComponent;

impl SwiperComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        // 默认背景色
        if ns.background_color.is_none() {
            ns.background_color = Some(Color::WHITE);
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "swiper".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    /// 绘制 swiper 组件（只绘制背景和指示点，子元素由渲染器处理）
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        Self::draw_with_text(node, canvas, x, y, w, h, sf, None);
    }
    
    /// 绘制 swiper 组件（带文本渲染器）
    pub fn draw_with_text(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32, text_renderer: Option<&TextRenderer>) {
        let radius = node.style.border_radius;
        
        // 如果有圆角，先裁剪（使用矩形裁剪，圆角通过背景绘制实现）
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        
        // 绘制背景（带圆角）
        draw_background(canvas, &node.style, x, y, w, h);
        
        // 获取属性
        let swiper_id = node.attrs.get("id").cloned()
            .unwrap_or_else(|| format!("swiper_{:.0}_{:.0}", x, y));
        let autoplay = node.attrs.get("autoplay")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(false);
        let interval = node.attrs.get("interval")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000u64);
        let indicator_dots = node.attrs.get("indicator-dots")
            .map(|s| s == "true" || s == "{{true}}")
            .unwrap_or(true);
        let indicator_color = node.attrs.get("indicator-color")
            .and_then(|s| parse_color_str(s))
            .unwrap_or(Color::new(255, 255, 255, 100)); // 半透明白色
        let indicator_active_color = node.attrs.get("indicator-active-color")
            .and_then(|s| parse_color_str(s))
            .unwrap_or(Color::WHITE); // 纯白色
        
        let total = node.children.len();
        if total == 0 {
            canvas.restore();
            return;
        }
        
        // 获取或创建状态，并检查自动播放
        let current = {
            if let Ok(mut manager) = SWIPER_MANAGER.lock() {
                let _state = manager.get_or_create(&swiper_id, total, autoplay, interval);
                manager.check_autoplay(&swiper_id);
                manager.get(&swiper_id).map(|s| s.current).unwrap_or(0)
            } else {
                0
            }
        };
        
        // 绘制当前 swiper-item 的内容
        if current < node.children.len() {
            let child = &node.children[current];
            draw_swiper_item_content(canvas, child, x, y, w, h, sf, text_renderer);
        }
        
        // 绘制指示点
        if indicator_dots && total > 1 {
            let dot_size = 8.0 * sf;
            let dot_gap = 8.0 * sf;
            let total_width = total as f32 * dot_size + (total - 1) as f32 * dot_gap;
            let start_x = x + (w - total_width) / 2.0;
            let dot_y = y + h - 20.0 * sf;
            
            for i in 0..total {
                let dot_x = start_x + i as f32 * (dot_size + dot_gap);
                let color = if i == current { indicator_active_color } else { indicator_color };
                let paint = Paint::new()
                    .with_color(color)
                    .with_style(PaintStyle::Fill)
                    .with_anti_alias(true);
                canvas.draw_circle(dot_x + dot_size / 2.0, dot_y + dot_size / 2.0, dot_size / 2.0, &paint);
            }
        }
        
        canvas.restore();
    }
    
    /// 获取当前显示的 swiper-item 索引
    pub fn get_current(swiper_id: &str) -> usize {
        if let Ok(manager) = SWIPER_MANAGER.lock() {
            manager.get(swiper_id).map(|s| s.current).unwrap_or(0)
        } else {
            0
        }
    }
    
    /// 切换到下一页
    pub fn next(swiper_id: &str) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.next(swiper_id);
        }
    }
    
    /// 切换到上一页
    pub fn prev(swiper_id: &str) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.prev(swiper_id);
        }
    }
    
    /// 切换到指定页
    pub fn set_current(swiper_id: &str, index: usize) {
        if let Ok(mut manager) = SWIPER_MANAGER.lock() {
            manager.set_current(swiper_id, index);
        }
    }
}

/// 递归绘制 swiper-item 的内容
fn draw_swiper_item_content(
    canvas: &mut Canvas, 
    node: &RenderNode, 
    x: f32, 
    y: f32, 
    w: f32, 
    h: f32, 
    sf: f32,
    text_renderer: Option<&TextRenderer>
) {
    // 绘制背景
    if let Some(bg) = node.style.background_color {
        let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
        let radius = node.style.border_radius;
        if radius > 0.0 {
            let mut path = crate::Path::new();
            path.add_round_rect(x, y, w, h, radius);
            canvas.draw_path(&path, &paint);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
    }
    
    // 计算子元素布局
    let padding = 20.0 * sf;
    let content_x = x + padding;
    let content_y = y + padding;
    let content_w = w - padding * 2.0;
    
    let mut current_y = content_y;
    
    for child in &node.children {
        match child.tag.as_str() {
            "view" => {
                // 检查是否是 banner-tag 样式的 view
                let child_h = if child.style.background_color.is_some() {
                    // 有背景色的 view，可能是标签
                    let tag_h = 28.0 * sf;
                    draw_banner_tag(canvas, child, content_x, current_y, content_w, tag_h, sf, text_renderer);
                    tag_h + 10.0 * sf
                } else {
                    // 普通 view，递归绘制
                    draw_swiper_item_content(canvas, child, content_x, current_y, content_w, h - (current_y - y), sf, text_renderer);
                    0.0
                };
                current_y += child_h;
            }
            "text" | "#text" => {
                let text = &child.text;
                if !text.is_empty() {
                    let text_color = child.style.text_color.unwrap_or(Color::WHITE);
                    let font_size = child.style.font_size * sf;
                    let font_weight = child.style.font_weight;
                    
                    if let Some(tr) = text_renderer {
                        let paint = Paint::new().with_color(text_color);
                        tr.draw_text(canvas, text, content_x, current_y + font_size, font_size, &paint);
                    } else {
                        // 没有文本渲染器时，绘制占位
                        let paint = Paint::new().with_color(text_color).with_style(PaintStyle::Fill);
                        let text_w = text.chars().count() as f32 * font_size * 0.6;
                        canvas.draw_rect(&GeoRect::new(content_x, current_y, text_w.min(content_w), font_size), &paint);
                    }
                    
                    current_y += font_size + 10.0 * sf;
                }
            }
            "image" => {
                // 图片占位
                let img_h = 100.0 * sf;
                let paint = Paint::new().with_color(Color::from_hex(0xCCCCCC)).with_style(PaintStyle::Fill);
                canvas.draw_rect(&GeoRect::new(content_x, current_y, content_w, img_h), &paint);
                current_y += img_h + 10.0 * sf;
            }
            _ => {}
        }
    }
}

/// 绘制 banner 标签（如 "限时特惠"、"会员专享" 等）
fn draw_banner_tag(
    canvas: &mut Canvas,
    node: &RenderNode,
    x: f32,
    y: f32,
    _max_w: f32,
    h: f32,
    sf: f32,
    text_renderer: Option<&TextRenderer>
) {
    let bg_color = node.style.background_color.unwrap_or(Color::WHITE);
    let radius = node.style.border_radius.max(10.0 * sf);
    
    // 获取文本内容
    let text = node.children.iter()
        .find(|c| c.tag == "text" || c.tag == "#text")
        .map(|c| c.text.clone())
        .unwrap_or_default();
    
    // 计算标签宽度
    let font_size = 11.0 * sf;
    let padding_h = 10.0 * sf;
    let text_w = if let Some(tr) = text_renderer {
        tr.measure_text(&text, font_size)
    } else {
        text.chars().count() as f32 * font_size * 0.6
    };
    let tag_w = text_w + padding_h * 2.0;
    
    // 绘制背景
    let paint = Paint::new().with_color(bg_color).with_style(PaintStyle::Fill);
    let mut path = crate::Path::new();
    path.add_round_rect(x, y, tag_w, h, radius);
    canvas.draw_path(&path, &paint);
    
    // 绘制文本
    let text_color = node.children.iter()
        .find(|c| c.tag == "text" || c.tag == "#text")
        .and_then(|c| c.style.text_color)
        .unwrap_or(Color::from_hex(0xFF6B35));
    
    if let Some(tr) = text_renderer {
        let text_paint = Paint::new().with_color(text_color);
        tr.draw_text(canvas, &text, x + padding_h, y + h * 0.7, font_size, &text_paint);
    }
}

/// SwiperItem 组件
pub struct SwiperItemComponent;

impl SwiperItemComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (ts, ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: "swiper-item".into(),
            text: String::new(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
}
