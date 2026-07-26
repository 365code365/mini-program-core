//! 渲染相关逻辑

use mini_render::Canvas;

/// 将内容渲染到窗口缓冲区
pub fn present_to_buffer(
    buffer: &mut [u32],
    buffer_width: u32,
    buffer_height: u32,
    canvas: &Canvas,
    fixed_canvas: Option<&Canvas>,
    tabbar_canvas: Option<&Canvas>,
    scroll_offset: i32,
    has_tabbar: bool,
    tabbar_physical_height: u32,
) {
    let pixels = canvas.pixels();
    let canvas_width = canvas.width();
    let canvas_height = canvas.height();
    
    let content_area_height = buffer_height - if has_tabbar { tabbar_physical_height } else { 0 };
    
    // 背景色 (0xF5F5F5)
    let bg_color: u32 = 0xF5F5F5;
    
    let copy_width = buffer_width.min(canvas_width) as usize;
    
    // 渲染主内容
    for dst_y in 0..content_area_height {
        let src_y = dst_y as i32 + scroll_offset;
        let dst_row_start = (dst_y * buffer_width) as usize;
        
        if src_y >= 0 && src_y < canvas_height as i32 {
            let src_row_start = (src_y as u32 * canvas_width) as usize;
            
            // 逐行 zip：切片迭代器让编译器省掉每个像素的边界检查并做向量化。
            // 上屏要转换整屏（750x1334 ≈ 100 万）像素，索引访问的边界检查在这里很显眼。
            let src_row = &pixels[src_row_start..src_row_start + copy_width];
            let dst_row = &mut buffer[dst_row_start..dst_row_start + copy_width];
            for (dst, color) in dst_row.iter_mut().zip(src_row) {
                *dst = ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
            }
            
            // 填充剩余宽度
            if buffer_width as usize > copy_width {
                buffer[dst_row_start + copy_width..dst_row_start + buffer_width as usize].fill(bg_color);
            }
        } else {
            buffer[dst_row_start..dst_row_start + buffer_width as usize].fill(bg_color);
        }
    }
    
    // 渲染 TabBar（宿主外壳）：在页面内容之上、页面 fixed 覆盖层之下。
    // 与浏览器里 #app 内的全屏遮罩压暗底部导航的层叠结果一致；此前 tabBar 画在最后，
    // 弹窗遮罩压不住它，和 H5 观感相反。
    if has_tabbar {
        if let Some(tabbar_canvas) = tabbar_canvas {
            let tabbar_pixels = tabbar_canvas.pixels();
            let tabbar_width = tabbar_canvas.width() as usize;
            let tabbar_height = tabbar_canvas.height();
            
            let draw_h = tabbar_physical_height.min(tabbar_height);
            let draw_w = (buffer_width as usize).min(tabbar_width);
            
            for y in 0..draw_h {
                let dst_y = content_area_height + y;
                if dst_y >= buffer_height { break; }
                let src_row = (y as usize) * tabbar_width;
                let dst_row = (dst_y * buffer_width) as usize;
                
                for x in 0..draw_w {
                    let color = &tabbar_pixels[src_row + x];
                    buffer[dst_row + x] = ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
                }
            }
        }
    }
    
    // 渲染 fixed 元素（覆盖到整个视口，含 tabBar 区域）
    if let Some(fixed_canvas) = fixed_canvas {
        let fixed_pixels = fixed_canvas.pixels();
        let fixed_width = fixed_canvas.width() as usize;
        let fixed_height = fixed_canvas.height();
        
        let draw_h = buffer_height.min(fixed_height);
        let draw_w = (buffer_width as usize).min(fixed_width);
        
        for y in 0..draw_h {
            let src_row = (y as usize) * fixed_width;
            let dst_row = (y * buffer_width) as usize;
            
            for x in 0..draw_w {
                let color = &fixed_pixels[src_row + x];
                if color.a > 0 {
                    let dst_idx = dst_row + x;
                    if color.a == 255 {
                        buffer[dst_idx] = ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
                    } else {
                        let dst = buffer[dst_idx];
                        let alpha = color.a as u32;
                        let inv_alpha = 255 - alpha;
                        let r = (color.r as u32 * alpha + ((dst >> 16) & 0xFF) * inv_alpha) / 255;
                        let g = (color.g as u32 * alpha + ((dst >> 8) & 0xFF) * inv_alpha) / 255;
                        let b = (color.b as u32 * alpha + (dst & 0xFF) * inv_alpha) / 255;
                        buffer[dst_idx] = (r << 16) | (g << 8) | b;
                    }
                }
            }
        }
    }
}

/// 在页面顶部露出的空白里画下拉刷新指示器（微信那套三点跑马灯）。
///
/// - 下拉过程：三个点随进度依次淡入并变深，到阈值时全部点亮 —— 用户据此知道「再拉就刷新」。
/// - 刷新期间：三点循环呼吸，直到逻辑层调用 `wx.stopPullDownRefresh()`。
///
/// 画在窗口缓冲上而不是页面画布上：这块空白本来就不属于页面内容
/// （页面画布只有内容那么高），指示器是宿主外壳的一部分，和微信一致。
pub fn render_pull_indicator(
    buffer: &mut [u32],
    buffer_width: u32,
    buffer_height: u32,
    scale: f32,
    gap_logical: f32,
    refreshing: bool,
    progress: f32,
    phase: f32,
) {
    use mini_render::{Color, Paint};

    let gap_px = (gap_logical * scale).round() as u32;
    if gap_px < 8 || buffer_width == 0 {
        return;
    }
    let band = gap_px.min(buffer_height);
    let mut layer = Canvas::new(buffer_width, band);
    layer.clear(Color::TRANSPARENT);

    let cx = buffer_width as f32 / 2.0;
    // 指示器贴着空白区域底部一点，下拉越多越往下走（跟着内容走，不是钉在顶上）
    let cy = (band as f32 * 0.5).min(gap_px as f32 - 10.0 * scale).max(6.0 * scale);
    let dot_r = 3.0 * scale;
    let spacing = 11.0 * scale;

    for i in 0..3 {
        let alpha = if refreshing {
            // 相位错开的呼吸：t 在 0..1 之间往复，映射成 0.3~1.0 的不透明度
            let t = (phase - i as f32 * 0.18).rem_euclid(1.0);
            0.3 + 0.7 * (1.0 - (t * 2.0 - 1.0).abs())
        } else {
            // 下拉过程：第 i 个点从 progress = i/3 开始淡入
            ((progress - i as f32 / 3.0) * 3.0).clamp(0.0, 1.0) * 0.85
        };
        if alpha <= 0.02 {
            continue;
        }
        let paint = Paint::new()
            .with_color(Color::new(
                0x88,
                0x88,
                0x88,
                (alpha * 255.0).clamp(0.0, 255.0) as u8,
            ))
            .with_anti_alias(true);
        layer.draw_circle(cx + (i as f32 - 1.0) * spacing, cy, dot_r, &paint);
    }

    // 合成到窗口缓冲（缓冲是不透明 RGB，按 alpha 手动混合）
    let pixels = layer.pixels();
    for y in 0..band {
        let row = (y * buffer_width) as usize;
        for x in 0..buffer_width as usize {
            let c = pixels[row + x];
            if c.a == 0 {
                continue;
            }
            let dst = buffer[row + x];
            let (dr, dg, db) = ((dst >> 16) & 0xFF, (dst >> 8) & 0xFF, dst & 0xFF);
            let a = c.a as u32;
            let inv = 255 - a;
            let r = (c.r as u32 * a + dr * inv) / 255;
            let g = (c.g as u32 * a + dg * inv) / 255;
            let b = (c.b as u32 * a + db * inv) / 255;
            buffer[row + x] = (r << 16) | (g << 8) | b;
        }
    }
}
