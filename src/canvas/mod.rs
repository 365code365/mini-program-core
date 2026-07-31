//! Canvas 画布模块 - 核心渲染接口

use crate::{Color, Paint, PaintStyle, Path, Point, Rect};

/// 画布状态
#[derive(Clone)]
struct CanvasState {
    clip_rect: Option<Rect>,
    translation: (f32, f32),
}

/// 画布 - 主要渲染接口
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<Color>,
    clip_rect: Option<Rect>,
    translation: (f32, f32),
    state_stack: Vec<CanvasState>,
}

/// 图片缩放结果缓存（key 见 `Canvas::draw_image_cached`）
static SCALED_IMAGE_CACHE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Vec<Color>>>>,
> = std::sync::OnceLock::new();

/// 缩小用的中间图（mip）缓存：key 见 `Canvas::draw_image_cached`，值是 RGBA 字节。
///
/// 面积滤波（盒式）只在这里做一次，逐帧绘制退化为在这张小图上做双线性 ——
/// 和浏览器的多级过滤是同一个思路。少了这一层的话，轮播平移时亚像素偏移每帧都变，
/// 缩放结果缓存全是未命中，面积滤波按帧重算（实测首页 135FPS → 58FPS）。
static IMAGE_MIP_CACHE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<(u32, u32, Vec<u8>)>>>,
> = std::sync::OnceLock::new();

#[allow(clippy::type_complexity)]
fn image_mip_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<(u32, u32, Vec<u8>)>>>
{
    IMAGE_MIP_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// 面积平均降采样：目标每个像素取源图对应矩形footprint 的 alpha 加权均值。
fn downscale_area(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw as usize) * (dh as usize) * 4];
    let step_x = sw as f32 / dw as f32;
    let step_y = sh as f32 / dh as f32;
    for dy in 0..dh {
        let y0 = (dy as f32 * step_y).floor() as u32;
        let y1 = (((dy + 1) as f32 * step_y).ceil() as u32).min(sh).max(y0 + 1);
        for dx in 0..dw {
            let x0 = (dx as f32 * step_x).floor() as u32;
            let x1 = (((dx + 1) as f32 * step_x).ceil() as u32).min(sw).max(x0 + 1);
            let (mut wr, mut wg, mut wb, mut wa, mut n) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for sy in y0..y1 {
                let row = (sy * sw) as usize * 4;
                for sx in x0..x1 {
                    let i = row + sx as usize * 4;
                    let a = src[i + 3] as f32;
                    // RGB 按 alpha 加权：透明像素不该把它的颜色混进边缘
                    wr += src[i] as f32 * a;
                    wg += src[i + 1] as f32 * a;
                    wb += src[i + 2] as f32 * a;
                    wa += a;
                    n += 1.0;
                }
            }
            let o = ((dy * dw + dx) as usize) * 4;
            if wa > 0.0 {
                out[o] = (wr / wa).round().clamp(0.0, 255.0) as u8;
                out[o + 1] = (wg / wa).round().clamp(0.0, 255.0) as u8;
                out[o + 2] = (wb / wa).round().clamp(0.0, 255.0) as u8;
                out[o + 3] = (wa / n.max(1.0)).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

fn scaled_image_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, std::sync::Arc<Vec<Color>>>> {
    SCALED_IMAGE_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

// ── 按职责切开的几片（都是 `impl Canvas`）──
/// 图片绘制与缩放缓存
mod images;
/// 路径光栅化
mod paths;
/// 像素级读写与混合
mod pixels;
/// 基本图形
mod shapes;

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![Color::TRANSPARENT; (width * height) as usize],
            clip_rect: None,
            translation: (0.0, 0.0),
            state_stack: Vec::new(),
        }
    }

    /// 保存当前状态（裁剪区域和变换）
    pub fn save(&mut self) {
        self.state_stack.push(CanvasState {
            clip_rect: self.clip_rect,
            translation: self.translation,
        });
    }

    /// 恢复上一次保存的状态
    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.clip_rect = state.clip_rect;
            self.translation = state.translation;
        }
    }

    /// 平移坐标系
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.translation.0 += dx;
        self.translation.1 += dy;
    }

    pub fn width(&self) -> u32 { self.width }

    pub fn height(&self) -> u32 { self.height }

    /// 获取像素数据引用
    pub fn pixels(&self) -> &[Color] {
        &self.pixels
    }

    /// 获取像素数据可变引用
    pub fn pixels_mut(&mut self) -> &mut [Color] {
        &mut self.pixels
    }

    /// 从另一个 Canvas 复制像素数据
    pub fn copy_from(&mut self, src: &Canvas) {
        let copy_len = self.pixels.len().min(src.pixels.len());
        self.pixels[..copy_len].copy_from_slice(&src.pixels[..copy_len]);
    }

    /// 清空画布
    pub fn clear(&mut self, color: Color) {
        self.pixels.fill(color);
    }

    /// 设置裁剪区域
    pub fn clip_rect(&mut self, rect: Rect) {
        // Intersect with existing clip rect if any
        if let Some(current) = self.clip_rect {
            let x = current.x.max(rect.x);
            let y = current.y.max(rect.y);
            let right = current.right().min(rect.right());
            let bottom = current.bottom().min(rect.bottom());
            
            if right > x && bottom > y {
                self.clip_rect = Some(Rect::new(x, y, right - x, bottom - y));
            } else {
                // No intersection, empty rect
                self.clip_rect = Some(Rect::new(0.0, 0.0, 0.0, 0.0));
            }
        } else {
            self.clip_rect = Some(rect);
        }
    }

    /// 重置裁剪区域
    pub fn reset_clip(&mut self) {
        self.clip_rect = None;
    }

    /// 导出为 RGBA 字节数组
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity((self.width * self.height * 4) as usize);
        for pixel in &self.pixels {
            data.push(pixel.r);
            data.push(pixel.g);
            data.push(pixel.b);
            data.push(pixel.a);
        }
        data
    }

    /// 保存为 PNG
    pub fn save_png(&self, path: &str) -> Result<(), String> {
        use image::{ImageBuffer, Rgba};

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(
            self.width,
            self.height,
            self.to_rgba()
        ).ok_or("Failed to create image buffer")?;

        img.save(path).map_err(|e| e.to_string())
    }
}
