//! Swiper 轮播图组件

use super::base::*;
use super::ImageComponent;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Rect as GeoRect};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use taffy::prelude::*;

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
    
    /// 绘制 swiper 组件
    pub fn draw(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        Self::draw_with_text(node, canvas, x, y, w, h, sf, None);
    }
    
    /// 绘制 swiper 组件（带文本渲染器）
    pub fn draw_with_text(node: &RenderNode, canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32, text_renderer: Option<&TextRenderer>) {
        // 裁剪区域
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        
        // 绘制背景
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
            .unwrap_or(Color::new(255, 255, 255, 100));
        let indicator_active_color = node.attrs.get("indicator-active-color")
            .and_then(|s| parse_color_str(s))
            .unwrap_or(Color::WHITE);
        
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
        
        // 只绘制当前 swiper-item
        if current < node.children.len() {
            let child = &node.children[current];
            Self::draw_swiper_item(canvas, child, x, y, w, h, sf, text_renderer);
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
    
    /// 绘制单个 swiper-item
    fn draw_swiper_item(
        canvas: &mut Canvas,
        item: &RenderNode,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        sf: f32,
        text_renderer: Option<&TextRenderer>
    ) {
        // 绘制 swiper-item 的背景
        if let Some(bg) = item.style.background_color {
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
        
        // 全屏图片轮播（banner）：若 item 内含 image，则整屏铺满绘制真实图片
        if let Some(img_node) = Self::find_image(item) {
            ImageComponent::draw(img_node, canvas, text_renderer, x, y, w, h, sf);
            return;
        }
        
        // 纯文本 banner（swiper-item 通常 justify/align center）：整体居中绘制，
        // 与 HTML 的 .wx-swiper-item flex 居中一致，避免文字固定在左上角。
        let lines = Self::collect_text_lines(item);
        if !lines.is_empty() {
            let gap = 6.0 * sf;
            // fs 为逻辑字号，绘制按物理像素 fs*sf
            let mut total_h = 0.0;
            for (i, (_t, fs, _c)) in lines.iter().enumerate() {
                total_h += fs * sf * 1.3;
                if i + 1 < lines.len() { total_h += gap; }
            }
            let mut cy = y + (h - total_h) / 2.0;
            if let Some(tr) = text_renderer {
                for (t, fs, c) in &lines {
                    let fs_px = fs * sf;
                    let tw = tr.measure_text(t, fs_px);
                    let tx = x + (w - tw) / 2.0;
                    let paint = Paint::new().with_color(*c);
                    tr.draw_text(canvas, t, tx.max(x), cy + fs_px, fs_px, &paint);
                    cy += fs_px * 1.3 + gap;
                }
            }
            return;
        }
        
        // 其它复杂内容：退回原有从上到下的简易布局
        Self::draw_children(canvas, &item.children, x, y, w, h, sf, text_renderer, Color::WHITE);
    }
    
    /// 收集 item 内的文本行（文本, 物理字号, 颜色），用于居中绘制纯文本 banner。
    /// 若包含带背景的 view（标签等）复杂结构，返回空表示走通用布局。
    fn collect_text_lines(item: &RenderNode) -> Vec<(String, f32, Color)> {
        let sf_hint = 2.0; // 仅用于判断，真实字号在调用处按 style*sf 计算；这里返回逻辑字号*sf
        let _ = sf_hint;
        let mut lines = Vec::new();
        fn walk(node: &RenderNode, out: &mut Vec<(String, f32, Color)>, inherited: Color) -> bool {
            for child in &node.children {
                match child.tag.as_str() {
                    "text" | "#text" => {
                        let t = child.text.trim();
                        if !t.is_empty() {
                            let color = child.style.text_color.unwrap_or(inherited);
                            // 物理字号：style.font_size 已是逻辑值，绘制处乘 sf；这里存逻辑值，
                            // 由调用方统一乘 sf。用占位 1.0，下面在外部乘 sf。
                            out.push((t.to_string(), child.style.font_size, color));
                        }
                    }
                    "view" => {
                        // 含背景的 view（标签/卡片）视为复杂结构，放弃居中简化
                        if child.style.background_color.is_some() { return false; }
                        if !walk(child, out, child.style.text_color.unwrap_or(inherited)) { return false; }
                    }
                    "image" => return false,
                    _ => {}
                }
            }
            true
        }
        if walk(item, &mut lines, Color::WHITE) { lines } else { Vec::new() }
    }
    
    /// 递归绘制子元素
    fn draw_children(
        canvas: &mut Canvas,
        children: &[RenderNode],
        parent_x: f32,
        parent_y: f32,
        parent_w: f32,
        parent_h: f32,
        sf: f32,
        text_renderer: Option<&TextRenderer>,
        inherited_color: Color
    ) {
        let padding = 20.0 * sf;
        let mut current_y = parent_y + padding;
        let content_x = parent_x + padding;
        let content_w = parent_w - padding * 2.0;
        
        for child in children {
            let text_color = child.style.text_color.unwrap_or(inherited_color);
            
            match child.tag.as_str() {
                "view" => {
                    // 检查是否有背景色（可能是标签）
                    if let Some(bg) = child.style.background_color {
                        // 绘制标签背景
                        let tag_h = 28.0 * sf;
                        let tag_text = Self::get_text_content(child);
                        let font_size = 11.0 * sf;
                        let padding_h = 10.0 * sf;
                        
                        let text_w = if let Some(tr) = text_renderer {
                            tr.measure_text(&tag_text, font_size)
                        } else {
                            tag_text.chars().count() as f32 * font_size * 0.6
                        };
                        let tag_w = text_w + padding_h * 2.0;
                        
                        let radius = child.style.border_radius.max(10.0 * sf);
                        let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
                        let mut path = crate::Path::new();
                        path.add_round_rect(content_x, current_y, tag_w, tag_h, radius);
                        canvas.draw_path(&path, &paint);
                        
                        // 绘制标签文本
                        let tag_text_color = Self::get_child_text_color(child).unwrap_or(Color::from_hex(0xFF6B35));
                        if let Some(tr) = text_renderer {
                            let paint = Paint::new().with_color(tag_text_color);
                            tr.draw_text(canvas, &tag_text, content_x + padding_h, current_y + tag_h * 0.7, font_size, &paint);
                        }
                        
                        current_y += tag_h + 10.0 * sf;
                    } else {
                        // 普通 view，递归绘制子元素
                        Self::draw_children(canvas, &child.children, content_x, current_y, content_w, parent_h - (current_y - parent_y), sf, text_renderer, text_color);
                    }
                }
                "text" | "#text" => {
                    let text = &child.text;
                    if !text.is_empty() {
                        let font_size = child.style.font_size * sf;
                        
                        if let Some(tr) = text_renderer {
                            let paint = Paint::new().with_color(text_color);
                            tr.draw_text(canvas, text, content_x, current_y + font_size, font_size, &paint);
                        }
                        
                        current_y += font_size * 1.5;
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
    
    /// 递归查找 item 内的第一个 image 节点（用于整屏 banner 轮播）
    fn find_image(node: &RenderNode) -> Option<&RenderNode> {
        for child in &node.children {
            if child.tag == "image" {
                return Some(child);
            }
            if let Some(found) = Self::find_image(child) {
                return Some(found);
            }
        }
        None
    }
    
    /// 获取节点的文本内容
    fn get_text_content(node: &RenderNode) -> String {
        let mut text = String::new();
        for child in &node.children {
            if child.tag == "text" || child.tag == "#text" {
                text.push_str(&child.text);
            } else {
                text.push_str(&Self::get_text_content(child));
            }
        }
        text
    }
    
    /// 获取子节点的文本颜色
    fn get_child_text_color(node: &RenderNode) -> Option<Color> {
        for child in &node.children {
            if child.tag == "text" || child.tag == "#text" {
                if let Some(color) = child.style.text_color {
                    return Some(color);
                }
            }
        }
        None
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
