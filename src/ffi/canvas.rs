//! FFI 接口 - C/C++ 绑定

use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect};
use std::ffi::CStr;
use std::os::raw::c_char;

/// 创建画布
#[no_mangle]
pub extern "C" fn mr_canvas_new(width: u32, height: u32) -> *mut Canvas {
    Box::into_raw(Box::new(Canvas::new(width, height)))
}

/// 销毁画布
#[no_mangle]
pub extern "C" fn mr_canvas_free(canvas: *mut Canvas) {
    if !canvas.is_null() {
        unsafe { drop(Box::from_raw(canvas)); }
    }
}

/// 清空画布
#[no_mangle]
pub extern "C" fn mr_canvas_clear(canvas: *mut Canvas, r: u8, g: u8, b: u8, a: u8) {
    if let Some(canvas) = unsafe { canvas.as_mut() } {
        canvas.clear(Color::new(r, g, b, a));
    }
}

/// 绘制矩形
#[no_mangle]
pub extern "C" fn mr_canvas_draw_rect(
    canvas: *mut Canvas,
    x: f32, y: f32, width: f32, height: f32,
    r: u8, g: u8, b: u8, a: u8,
    style: u8, stroke_width: f32
) {
    if let Some(canvas) = unsafe { canvas.as_mut() } {
        let paint = Paint {
            color: Color::new(r, g, b, a),
            style: match style {
                0 => PaintStyle::Fill,
                1 => PaintStyle::Stroke,
                _ => PaintStyle::FillAndStroke,
            },
            stroke_width,
            ..Default::default()
        };
        canvas.draw_rect(&Rect::new(x, y, width, height), &paint);
    }
}

/// 绘制圆形
#[no_mangle]
pub extern "C" fn mr_canvas_draw_circle(
    canvas: *mut Canvas,
    cx: f32, cy: f32, radius: f32,
    r: u8, g: u8, b: u8, a: u8,
    style: u8, stroke_width: f32
) {
    if let Some(canvas) = unsafe { canvas.as_mut() } {
        let paint = Paint {
            color: Color::new(r, g, b, a),
            style: match style {
                0 => PaintStyle::Fill,
                1 => PaintStyle::Stroke,
                _ => PaintStyle::FillAndStroke,
            },
            stroke_width,
            ..Default::default()
        };
        canvas.draw_circle(cx, cy, radius, &paint);
    }
}

/// 绘制线段
#[no_mangle]
pub extern "C" fn mr_canvas_draw_line(
    canvas: *mut Canvas,
    x0: f32, y0: f32, x1: f32, y1: f32,
    r: u8, g: u8, b: u8, a: u8,
    stroke_width: f32
) {
    if let Some(canvas) = unsafe { canvas.as_mut() } {
        let paint = Paint {
            color: Color::new(r, g, b, a),
            stroke_width,
            ..Default::default()
        };
        canvas.draw_line(x0, y0, x1, y1, &paint);
    }
}

/// 创建路径
#[no_mangle]
pub extern "C" fn mr_path_new() -> *mut Path {
    Box::into_raw(Box::new(Path::new()))
}

/// 销毁路径
#[no_mangle]
pub extern "C" fn mr_path_free(path: *mut Path) {
    if !path.is_null() {
        unsafe { drop(Box::from_raw(path)); }
    }
}

/// 路径移动到
#[no_mangle]
pub extern "C" fn mr_path_move_to(path: *mut Path, x: f32, y: f32) {
    if let Some(path) = unsafe { path.as_mut() } {
        path.move_to(x, y);
    }
}

/// 路径画线到
#[no_mangle]
pub extern "C" fn mr_path_line_to(path: *mut Path, x: f32, y: f32) {
    if let Some(path) = unsafe { path.as_mut() } {
        path.line_to(x, y);
    }
}

/// 路径二次贝塞尔曲线
#[no_mangle]
pub extern "C" fn mr_path_quad_to(path: *mut Path, cx: f32, cy: f32, x: f32, y: f32) {
    if let Some(path) = unsafe { path.as_mut() } {
        path.quad_to(cx, cy, x, y);
    }
}

/// 路径三次贝塞尔曲线
#[no_mangle]
pub extern "C" fn mr_path_cubic_to(path: *mut Path, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
    if let Some(path) = unsafe { path.as_mut() } {
        path.cubic_to(c1x, c1y, c2x, c2y, x, y);
    }
}

/// 路径闭合
#[no_mangle]
pub extern "C" fn mr_path_close(path: *mut Path) {
    if let Some(path) = unsafe { path.as_mut() } {
        path.close();
    }
}

/// 路径添加圆角矩形
#[no_mangle]
pub extern "C" fn mr_path_add_round_rect(path: *mut Path, x: f32, y: f32, w: f32, h: f32, radius: f32) {
    if let Some(path) = unsafe { path.as_mut() } {
        path.add_round_rect(x, y, w, h, radius);
    }
}

/// 路径添加椭圆
#[no_mangle]
pub extern "C" fn mr_path_add_oval(path: *mut Path, cx: f32, cy: f32, rx: f32, ry: f32) {
    if let Some(path) = unsafe { path.as_mut() } {
        path.add_oval(cx, cy, rx, ry);
    }
}

/// 绘制路径
#[no_mangle]
pub extern "C" fn mr_canvas_draw_path(
    canvas: *mut Canvas,
    path: *const Path,
    r: u8, g: u8, b: u8, a: u8,
    style: u8, stroke_width: f32
) {
    if let (Some(canvas), Some(path)) = (unsafe { canvas.as_mut() }, unsafe { path.as_ref() }) {
        let paint = Paint {
            color: Color::new(r, g, b, a),
            style: match style {
                0 => PaintStyle::Fill,
                1 => PaintStyle::Stroke,
                _ => PaintStyle::FillAndStroke,
            },
            stroke_width,
            ..Default::default()
        };
        canvas.draw_path(path, &paint);
    }
}

/// 获取画布宽度
#[no_mangle]
pub extern "C" fn mr_canvas_width(canvas: *const Canvas) -> u32 {
    unsafe { canvas.as_ref().map(|c| c.width()).unwrap_or(0) }
}

/// 获取画布高度
#[no_mangle]
pub extern "C" fn mr_canvas_height(canvas: *const Canvas) -> u32 {
    unsafe { canvas.as_ref().map(|c| c.height()).unwrap_or(0) }
}

/// 获取像素数据（RGBA）
#[no_mangle]
pub extern "C" fn mr_canvas_get_pixels(canvas: *const Canvas, out: *mut u8, len: usize) -> usize {
    if let Some(canvas) = unsafe { canvas.as_ref() } {
        let data = canvas.to_rgba();
        let copy_len = data.len().min(len);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), out, copy_len);
        }
        copy_len
    } else {
        0
    }
}

/// 保存为 PNG
#[no_mangle]
pub extern "C" fn mr_canvas_save_png(canvas: *const Canvas, path: *const c_char) -> bool {
    if let (Some(canvas), Some(path_cstr)) = (
        unsafe { canvas.as_ref() },
        unsafe { path.as_ref() }
    ) {
        let path_str = unsafe { CStr::from_ptr(path_cstr) };
        if let Ok(path) = path_str.to_str() {
            return canvas.save_png(path).is_ok();
        }
    }
    false
}
