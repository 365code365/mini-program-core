//! UI 覆盖层模块 - Toast/Loading/Modal 的状态和渲染

use crate::{Canvas, Color, Paint};
use crate::text::TextRenderer;
use std::time::Instant;

/// Toast 状态
#[derive(Clone)]
pub struct ToastState {
    pub title: String,
    pub icon: String,
    pub visible: bool,
    pub start_time: Instant,
    pub duration_ms: u32,
}

/// Loading 状态
#[derive(Clone)]
pub struct LoadingState {
    pub title: String,
    pub visible: bool,
}

/// Modal 状态
#[derive(Clone)]
pub struct ModalState {
    pub title: String,
    pub content: String,
    pub show_cancel: bool,
    pub cancel_text: String,
    pub confirm_text: String,
    pub visible: bool,
    pub pressed_button: Option<String>, // "cancel" or "confirm"
}

/// 渲染 UI 覆盖层（Toast/Loading/Modal）
pub fn render_ui_overlay(
    buffer: &mut [u32],
    width: u32, height: u32, sf: f32, last_frame: Instant,
    toast: &Option<ToastState>, loading: &Option<LoadingState>, modal: &Option<ModalState>,
    text_renderer: Option<&TextRenderer>
) {
    // 渲染 Loading（优先级最高）
    if let Some(loading) = loading {
        if loading.visible {
            render_loading_to_buffer(buffer, width, height, &loading.title, sf, last_frame, text_renderer);
            return;
        }
    }
    
    // 渲染 Modal
    if let Some(modal) = modal {
        if modal.visible {
            render_modal_to_buffer(buffer, width, height, modal, sf, text_renderer);
            return;
        }
    }
    
    // 渲染 Toast
    if let Some(toast) = toast {
        if toast.visible {
            render_toast_to_buffer(buffer, width, height, &toast.title, &toast.icon, sf, text_renderer);
        }
    }
}


/// 渲染 Toast 到 buffer
fn render_toast_to_buffer(
    buffer: &mut [u32],
    width: u32, height: u32, title: &str, icon: &str, sf: f32,
    text_renderer: Option<&TextRenderer>
) {
    let toast_padding = (20.0 * sf) as i32;
    let toast_min_width = (140.0 * sf) as i32;
    let toast_height = if icon == "none" { (50.0 * sf) as i32 } else { (120.0 * sf) as i32 };
    let icon_size = (50.0 * sf) as i32;
    let font_size = 14.0 * sf;
    
    let text_width = if let Some(tr) = text_renderer {
        tr.measure_text(title, font_size) as i32
    } else {
        (title.chars().count() as f32 * font_size * 0.6) as i32
    };
    let toast_width = toast_min_width.max(text_width + toast_padding * 2);
    
    let toast_x = (width as i32 - toast_width) / 2;
    let toast_y = (height as i32 - toast_height) / 2;
    
    let bg_color = 0xFF4C4C4Cu32;
    let radius = (12.0 * sf) as i32;
    
    draw_rounded_rect(buffer, width, height, toast_x, toast_y, toast_width, toast_height, radius, bg_color);
    
    if icon != "none" {
        let icon_x = toast_x + (toast_width - icon_size) / 2;
        let icon_y = toast_y + toast_padding;
        let icon_color = if icon == "success" { 0xFF09BB07u32 } else { 0xFFFFFFFFu32 };
        
        if icon == "success" {
            draw_checkmark(buffer, width, height, icon_x, icon_y, icon_size, icon_color, sf);
        } else {
            draw_circle_outline(buffer, width, height, icon_x + icon_size/2, icon_y + icon_size/2, icon_size/2 - 4, icon_color, (3.0 * sf) as i32);
        }
    }
    
    if let Some(tr) = text_renderer {
        let text_y = if icon == "none" { 
            toast_y + (toast_height - font_size as i32) / 2 
        } else { 
            toast_y + toast_padding + icon_size + (12.0 * sf) as i32 
        };
        let text_x = toast_x + (toast_width - text_width) / 2;
        
        draw_text_direct(buffer, width, height, tr, title, text_x, text_y, font_size, Color::WHITE);
    }
}

/// 渲染 Loading 到 buffer
fn render_loading_to_buffer(
    buffer: &mut [u32],
    width: u32, height: u32, title: &str, sf: f32, last_frame: Instant,
    text_renderer: Option<&TextRenderer>
) {
    let loading_size = (120.0 * sf) as i32;
    let loading_x = (width as i32 - loading_size) / 2;
    let loading_y = (height as i32 - loading_size) / 2;
    let radius = (12.0 * sf) as i32;
    let bg_color = 0xFF4C4C4Cu32;
    
    draw_rounded_rect(buffer, width, height, loading_x, loading_y, loading_size, loading_size, radius, bg_color);
    
    let center_x = loading_x + loading_size / 2;
    let center_y = loading_y + (45.0 * sf) as i32;
    let spinner_radius = (22.0 * sf) as i32;
    let time = last_frame.elapsed().as_secs_f32();
    let angle = time * 5.0;
    let dot_radius = (4.0 * sf) as i32;
    
    for i in 0..12 {
        let seg_angle = angle + (i as f32 * std::f32::consts::PI / 6.0);
        let alpha = ((12 - i) as f32 / 12.0 * 255.0) as u8;
        let color = 0xFF000000 | ((alpha as u32) << 16) | ((alpha as u32) << 8) | (alpha as u32);
        
        let dot_x = center_x + (spinner_radius as f32 * seg_angle.cos()) as i32;
        let dot_y = center_y + (spinner_radius as f32 * seg_angle.sin()) as i32;
        
        draw_filled_circle(buffer, width, height, dot_x, dot_y, dot_radius, color);
    }
    
    if let Some(tr) = text_renderer {
        let font_size = 14.0 * sf;
        let text_width = tr.measure_text(title, font_size) as i32;
        let text_x = loading_x + (loading_size - text_width) / 2;
        let text_y = loading_y + loading_size - (35.0 * sf) as i32;
        
        draw_text_direct(buffer, width, height, tr, title, text_x, text_y, font_size, Color::WHITE);
    }
}


/// 渲染 Modal 到 buffer
fn render_modal_to_buffer(
    buffer: &mut [u32],
    width: u32, height: u32, modal: &ModalState, sf: f32,
    text_renderer: Option<&TextRenderer>
) {
    // 绘制半透明遮罩
    for i in 0..buffer.len() {
        let existing = buffer[i];
        let r = ((existing >> 16) & 0xFF) / 2;
        let g = ((existing >> 8) & 0xFF) / 2;
        let b = (existing & 0xFF) / 2;
        buffer[i] = 0xFF000000 | (r << 16) | (g << 8) | b;
    }
    
    let modal_width = (280.0 * sf) as i32;
    let modal_padding = (24.0 * sf) as i32;
    let title_font_size = 17.0 * sf;
    let content_font_size = 14.0 * sf;
    let button_font_size = 17.0 * sf;
    let button_height = (50.0 * sf) as i32;
    let gap = (16.0 * sf) as i32;
    
    let title_line_height = (title_font_size * 1.4) as i32;
    let content_max_width = modal_width - modal_padding * 2;
    
    let (content_lines, wrapped_content) = if let Some(tr) = text_renderer {
        wrap_text(tr, &modal.content, content_font_size, content_max_width as f32)
    } else {
        (vec![modal.content.clone()], 1)
    };
    let content_total_height = (content_font_size * 1.5) as i32 * wrapped_content.max(1) as i32;
    
    let modal_height = modal_padding + title_line_height + gap + content_total_height + gap + button_height;
    let modal_x = (width as i32 - modal_width) / 2;
    let modal_y = (height as i32 - modal_height) / 2;
    let radius = (14.0 * sf) as i32;
    let bg_color = 0xFFFFFFFFu32;
    
    draw_rounded_rect(buffer, width, height, modal_x, modal_y, modal_width, modal_height, radius, bg_color);
    
    if let Some(tr) = text_renderer {
        let title_text_w = tr.measure_text(&modal.title, title_font_size) as i32;
        let title_x = modal_x + (modal_width - title_text_w) / 2;
        let title_y = modal_y + modal_padding;
        
        draw_text_direct(buffer, width, height, tr, &modal.title, title_x, title_y, title_font_size, Color::BLACK);
        
        let content_start_y = title_y + title_line_height + gap;
        let line_height = (content_font_size * 1.5) as i32;
        
        for (i, line) in content_lines.iter().enumerate() {
            let line_w = tr.measure_text(line, content_font_size) as i32;
            let line_x = modal_x + (modal_width - line_w) / 2;
            let line_y = content_start_y + i as i32 * line_height;
            draw_text_direct(buffer, width, height, tr, line, line_x, line_y, content_font_size, Color::from_hex(0x888888));
        }
        
        let line_y = modal_y + modal_height - button_height - 1;
        let line_color = 0xFFE5E5E5u32;
        for px in modal_x..(modal_x + modal_width) {
            if px >= 0 && px < width as i32 && line_y >= 0 && line_y < height as i32 {
                let idx = (line_y as u32 * width + px as u32) as usize;
                if idx < buffer.len() { buffer[idx] = line_color; }
            }
        }
        
        let button_y = modal_y + modal_height - button_height;
        let btn_text_y = button_y + (button_height - button_font_size as i32) / 2;
        let pressed_bg = 0xFFF0F0F0u32;
        
        if modal.show_cancel {
            let button_width = modal_width / 2;
            
            if modal.pressed_button.as_deref() == Some("cancel") {
                draw_rounded_rect_partial(buffer, width, height, modal_x, button_y, button_width, button_height, radius, pressed_bg, true, false);
            }
            
            let cancel_text_w = tr.measure_text(&modal.cancel_text, button_font_size) as i32;
            let cancel_x = modal_x + (button_width - cancel_text_w) / 2;
            draw_text_direct(buffer, width, height, tr, &modal.cancel_text, cancel_x, btn_text_y, button_font_size, Color::BLACK);
            
            let vline_x = modal_x + button_width;
            for py in button_y..(button_y + button_height) {
                if vline_x >= 0 && vline_x < width as i32 && py >= 0 && py < height as i32 {
                    let idx = (py as u32 * width + vline_x as u32) as usize;
                    if idx < buffer.len() { buffer[idx] = line_color; }
                }
            }
            
            if modal.pressed_button.as_deref() == Some("confirm") {
                draw_rounded_rect_partial(buffer, width, height, modal_x + button_width, button_y, button_width, button_height, radius, pressed_bg, false, true);
            }
            
            let confirm_text_w = tr.measure_text(&modal.confirm_text, button_font_size) as i32;
            let confirm_x = modal_x + button_width + (button_width - confirm_text_w) / 2;
            draw_text_direct(buffer, width, height, tr, &modal.confirm_text, confirm_x, btn_text_y, button_font_size, Color::from_hex(0x576B95));
        } else {
            if modal.pressed_button.as_deref() == Some("confirm") {
                draw_rounded_rect_partial(buffer, width, height, modal_x, button_y, modal_width, button_height, radius, pressed_bg, true, true);
            }
            
            let confirm_text_w = tr.measure_text(&modal.confirm_text, button_font_size) as i32;
            let confirm_x = modal_x + (modal_width - confirm_text_w) / 2;
            draw_text_direct(buffer, width, height, tr, &modal.confirm_text, confirm_x, btn_text_y, button_font_size, Color::from_hex(0x576B95));
        }
    }
}

/// 文字换行
fn wrap_text(tr: &TextRenderer, text: &str, font_size: f32, max_width: f32) -> (Vec<String>, i32) {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0.0;
    
    for ch in text.chars() {
        let char_width = tr.measure_char(ch, font_size);
        
        if current_width + char_width > max_width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0.0;
        }
        
        current_line.push(ch);
        current_width += char_width;
    }
    
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    let count = lines.len() as i32;
    (lines, count)
}


// ============ 辅助绘图函数 ============

/// 绘制圆角矩形
fn draw_rounded_rect(buffer: &mut [u32], width: u32, height: u32, x: i32, y: i32, w: i32, h: i32, radius: i32, color: u32) {
    for py in y.max(0)..(y + h).min(height as i32) {
        for px in x.max(0)..(x + w).min(width as i32) {
            let in_corner = (px < x + radius || px >= x + w - radius) &&
                           (py < y + radius || py >= y + h - radius);
            if in_corner {
                let cx = if px < x + radius { x + radius } else { x + w - radius };
                let cy = if py < y + radius { y + radius } else { y + h - radius };
                let dist = (((px - cx) * (px - cx) + (py - cy) * (py - cy)) as f32).sqrt();
                if dist > radius as f32 { continue; }
            }
            let idx = (py as u32 * width + px as u32) as usize;
            if idx < buffer.len() { buffer[idx] = color; }
        }
    }
}

/// 绘制部分圆角矩形（用于按钮背景）
fn draw_rounded_rect_partial(buffer: &mut [u32], width: u32, height: u32, x: i32, y: i32, w: i32, h: i32, radius: i32, color: u32, left_rounded: bool, right_rounded: bool) {
    for py in y.max(0)..(y + h).min(height as i32) {
        for px in x.max(0)..(x + w).min(width as i32) {
            let in_bottom_left = left_rounded && px < x + radius && py >= y + h - radius;
            let in_bottom_right = right_rounded && px >= x + w - radius && py >= y + h - radius;
            
            if in_bottom_left {
                let cx = x + radius;
                let cy = y + h - radius;
                let dist = (((px - cx) * (px - cx) + (py - cy) * (py - cy)) as f32).sqrt();
                if dist > radius as f32 { continue; }
            } else if in_bottom_right {
                let cx = x + w - radius;
                let cy = y + h - radius;
                let dist = (((px - cx) * (px - cx) + (py - cy) * (py - cy)) as f32).sqrt();
                if dist > radius as f32 { continue; }
            }
            
            let idx = (py as u32 * width + px as u32) as usize;
            if idx < buffer.len() { buffer[idx] = color; }
        }
    }
}

/// 绘制实心圆
fn draw_filled_circle(buffer: &mut [u32], width: u32, height: u32, cx: i32, cy: i32, radius: i32, color: u32) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            if dist <= radius as f32 {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let idx = (py as u32 * width + px as u32) as usize;
                    if idx < buffer.len() { buffer[idx] = color; }
                }
            }
        }
    }
}

/// 绘制圆环
fn draw_circle_outline(buffer: &mut [u32], width: u32, height: u32, cx: i32, cy: i32, radius: i32, color: u32, thickness: i32) {
    let outer = radius as f32;
    let inner = (radius - thickness) as f32;
    for dy in -(radius + 1)..=(radius + 1) {
        for dx in -(radius + 1)..=(radius + 1) {
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            if dist <= outer && dist >= inner {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    let idx = (py as u32 * width + px as u32) as usize;
                    if idx < buffer.len() { buffer[idx] = color; }
                }
            }
        }
    }
}

/// 绘制勾号图标
fn draw_checkmark(buffer: &mut [u32], width: u32, height: u32, x: i32, y: i32, size: i32, color: u32, sf: f32) {
    let cx = x + size / 2;
    let cy = y + size / 2;
    let r = size / 2 - (4.0 * sf) as i32;
    let stroke = (4.0 * sf) as i32;
    
    draw_circle_outline(buffer, width, height, cx, cy, r + stroke/2, color, stroke);
    
    let p1_x = cx - r * 2 / 5;
    let p1_y = cy;
    let p2_x = cx - r / 10;
    let p2_y = cy + r / 3;
    let p3_x = cx + r * 2 / 5;
    let p3_y = cy - r / 3;
    
    draw_thick_line(buffer, width, height, p1_x, p1_y, p2_x, p2_y, stroke, color);
    draw_thick_line(buffer, width, height, p2_x, p2_y, p3_x, p3_y, stroke, color);
}

/// 绘制粗线
fn draw_thick_line(buffer: &mut [u32], width: u32, height: u32, x1: i32, y1: i32, x2: i32, y2: i32, thickness: i32, color: u32) {
    let dx = (x2 - x1) as f32;
    let dy = (y2 - y1) as f32;
    let len = (dx * dx + dy * dy).sqrt();
    let steps = (len * 2.0) as i32;
    
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let px = (x1 as f32 + dx * t) as i32;
        let py = (y1 as f32 + dy * t) as i32;
        
        for ddy in -thickness..=thickness {
            for ddx in -thickness..=thickness {
                if ddx * ddx + ddy * ddy <= thickness * thickness {
                    let fx = px + ddx;
                    let fy = py + ddy;
                    if fx >= 0 && fx < width as i32 && fy >= 0 && fy < height as i32 {
                        let idx = (fy as u32 * width + fx as u32) as usize;
                        if idx < buffer.len() { buffer[idx] = color; }
                    }
                }
            }
        }
    }
}

/// 直接绘制文字到 buffer
fn draw_text_direct(
    buffer: &mut [u32],
    buf_width: u32, buf_height: u32,
    tr: &TextRenderer,
    text: &str,
    x: i32, y: i32,
    font_size: f32,
    color: Color
) {
    let text_width = (tr.measure_text(text, font_size) + 10.0) as u32;
    let text_height = (font_size * 1.5) as u32;
    
    if text_width == 0 || text_height == 0 { return; }
    
    let mut temp_canvas = Canvas::new(text_width.max(1), text_height.max(1));
    temp_canvas.clear(Color::TRANSPARENT);
    let paint = Paint::new().with_color(color);
    
    let baseline_y = font_size * 0.85;
    tr.draw_text(&mut temp_canvas, text, 0.0, baseline_y, font_size, &paint);
    
    let temp_pixels = temp_canvas.pixels();
    for py in 0..text_height as i32 {
        for px in 0..text_width as i32 {
            let src_idx = (py as u32 * text_width + px as u32) as usize;
            let dst_x = x + px;
            let dst_y = y + py;
            if dst_x >= 0 && dst_x < buf_width as i32 && dst_y >= 0 && dst_y < buf_height as i32 {
                let dst_idx = (dst_y as u32 * buf_width + dst_x as u32) as usize;
                if dst_idx < buffer.len() && src_idx < temp_pixels.len() {
                    let pixel = temp_pixels[src_idx];
                    if pixel.a > 0 {
                        let existing = buffer[dst_idx];
                        let bg_r = ((existing >> 16) & 0xFF) as u8;
                        let bg_g = ((existing >> 8) & 0xFF) as u8;
                        let bg_b = (existing & 0xFF) as u8;
                        
                        let alpha = pixel.a as f32 / 255.0;
                        let inv_alpha = 1.0 - alpha;
                        
                        let r = (pixel.r as f32 * alpha + bg_r as f32 * inv_alpha) as u8;
                        let g = (pixel.g as f32 * alpha + bg_g as f32 * inv_alpha) as u8;
                        let b = (pixel.b as f32 * alpha + bg_b as f32 * inv_alpha) as u8;
                        
                        buffer[dst_idx] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    }
                }
            }
        }
    }
}
