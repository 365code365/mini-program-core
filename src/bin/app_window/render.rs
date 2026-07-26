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
