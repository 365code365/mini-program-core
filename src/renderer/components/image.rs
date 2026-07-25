//! image 组件 - 图片
//! 
//! 支持完整的 CSS 样式，同时保留微信默认样式作为 fallback
//! 属性：
//! - src: 图片资源地址（支持网络URL和本地路径）
//! - mode: 图片裁剪、缩放模式
//!   - scaleToFill: 缩放模式，不保持纵横比缩放图片
//!   - aspectFit: 缩放模式，保持纵横比缩放图片，完整显示
//!   - aspectFill: 缩放模式，保持纵横比缩放图片，只保证短边完全显示
//!   - widthFix: 缩放模式，宽度不变，高度自动变化
//!   - heightFix: 缩放模式，高度不变，宽度自动变化
//! - lazy-load: 懒加载
//! - show-menu-by-longpress: 长按显示菜单
//! 
//! CSS 支持：
//! - width/height: 自定义尺寸
//! - background-color: 自定义占位符背景色
//! - border: 边框样式
//! - border-radius: 圆角（支持四角独立设置）
//! - box-shadow: 阴影
//! - opacity: 透明度
//! - object-fit: 图片填充模式（可覆盖 mode 属性）

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::io::Read;

/// 图片缓存数据
struct ImageData {
    data: Vec<u8>,  // RGBA 数据
    width: u32,
    height: u32,
}

/// 动图（GIF）解码结果：逐帧 RGBA + 每帧展示时长
struct AnimatedImage {
    /// 每帧 (RGBA 数据, 该帧持续毫秒)
    frames: Vec<(Vec<u8>, u32)>,
    width: u32,
    height: u32,
    /// 一轮播放的总时长（毫秒，至少 1）
    total_ms: u32,
    /// 首次解码时刻，用作播放起点（保证首帧渲染确定为第 0 帧）
    started: std::time::Instant,
}

impl AnimatedImage {
    /// 按「距首次加载的经过时间」取当前帧下标（循环播放）
    fn current_index(&self) -> usize {
        if self.frames.len() <= 1 {
            return 0;
        }
        let elapsed = self.started.elapsed().as_millis() as u64;
        let mut offset = (elapsed % self.total_ms.max(1) as u64) as u32;
        for (index, (_, delay)) in self.frames.iter().enumerate() {
            let delay = (*delay).max(1);
            if offset < delay {
                return index;
            }
            offset -= delay;
        }
        self.frames.len() - 1
    }
}

/// 全局图片缓存
static IMAGE_CACHE: OnceLock<Arc<Mutex<HashMap<String, Option<ImageData>>>>> = OnceLock::new();
/// 全局动图缓存（GIF 多帧）
static ANIM_CACHE: OnceLock<Arc<Mutex<HashMap<String, Arc<AnimatedImage>>>>> = OnceLock::new();

fn get_image_cache() -> &'static Arc<Mutex<HashMap<String, Option<ImageData>>>> {
    IMAGE_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

fn get_anim_cache() -> &'static Arc<Mutex<HashMap<String, Arc<AnimatedImage>>>> {
    ANIM_CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// 该资源是否为多帧动图（GIF）。
pub fn is_animated(src: &str) -> bool {
    get_anim_cache()
        .lock()
        .map(|cache| cache.get(src).map(|a| a.frames.len() > 1).unwrap_or(false))
        .unwrap_or(false)
}

/// 已缓存动图的一轮播放时长（毫秒）；非动图返回 None。
pub fn animation_total_ms(src: &str) -> Option<u32> {
    let cache = get_anim_cache().lock().ok()?;
    cache.get(src).filter(|a| a.frames.len() > 1).map(|a| a.total_ms)
}

/// 尝试把字节解码为多帧动图；单帧或非 GIF 返回 None。
fn decode_animated_gif(bytes: &[u8]) -> Option<AnimatedImage> {
    // 仅对 GIF magic 尝试多帧解码，避免无谓开销
    if bytes.len() < 6 || &bytes[..4] != b"GIF8" {
        return None;
    }
    use image::codecs::gif::GifDecoder;
    use image::AnimationDecoder;

    let decoder = GifDecoder::new(std::io::Cursor::new(bytes.to_vec())).ok()?;
    let frames = decoder.into_frames().collect_frames().ok()?;
    if frames.len() <= 1 {
        return None;
    }

    let mut out: Vec<(Vec<u8>, u32)> = Vec::with_capacity(frames.len());
    let mut width = 0;
    let mut height = 0;
    let mut total_ms: u32 = 0;
    for frame in frames {
        let (numer, denom) = frame.delay().numer_denom_ms();
        // GIF 常见 0/1 延时表示「尽快」，浏览器按 100ms 处理，这里保持一致
        let delay = if denom == 0 { 100 } else { (numer / denom.max(1)).max(10) };
        let buffer = frame.into_buffer();
        width = buffer.width();
        height = buffer.height();
        total_ms = total_ms.saturating_add(delay);
        out.push((buffer.into_raw(), delay));
    }
    if out.is_empty() || width == 0 || height == 0 {
        return None;
    }
    Some(AnimatedImage {
        frames: out,
        width,
        height,
        total_ms: total_ms.max(1),
        started: std::time::Instant::now(),
    })
}

/// 加载图片（支持网络URL和本地文件；GIF 动图按时间取当前帧）
fn load_image(src: &str) -> Option<ImageData> {
    // 动图：命中后按经过时间取帧，实现循环播放
    {
        let cache = get_anim_cache();
        if let Ok(cache_guard) = cache.lock() {
            if let Some(anim) = cache_guard.get(src) {
                let index = anim.current_index();
                let (frame, _) = &anim.frames[index];
                return Some(ImageData {
                    data: frame.clone(),
                    width: anim.width,
                    height: anim.height,
                });
            }
        }
    }

    // 检查缓存
    {
        let cache = get_image_cache();
        let cache_guard = cache.lock().ok()?;
        if let Some(cached) = cache_guard.get(src) {
            return cached.as_ref().map(|d| ImageData {
                data: d.data.clone(),
                width: d.width,
                height: d.height,
            });
        }
    }

    let result = if src.starts_with("http://") || src.starts_with("https://") {
        load_image_from_url(src)
    } else {
        load_image_from_file(src)
    };

    // 存入缓存
    {
        let cache = get_image_cache();
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.insert(src.to_string(), result.as_ref().map(|d| ImageData {
                data: d.data.clone(),
                width: d.width,
                height: d.height,
            }));
        }
    }

    result
}

/// 从网络URL加载图片
fn load_image_from_url(url: &str) -> Option<ImageData> {
    // 使用 ureq 下载图片
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .ok()?;
    
    let mut bytes = Vec::new();
    response.into_reader().take(10 * 1024 * 1024).read_to_end(&mut bytes).ok()?;
    
    decode_with_animation(url, &bytes)
}

/// 从本地文件加载图片（按宿主登记的小程序根目录解析包内绝对路径）
fn load_image_from_file(path: &str) -> Option<ImageData> {
    let resolved = crate::assets::resolve(path)?;
    let bytes = std::fs::read(&resolved).ok()?;
    decode_with_animation(path, &bytes)
}

/// 解码字节：若为多帧 GIF 则登记到动图缓存并返回首帧，否则按静态图解码。
fn decode_with_animation(src: &str, bytes: &[u8]) -> Option<ImageData> {
    if let Some(anim) = decode_animated_gif(bytes) {
        let first = ImageData {
            data: anim.frames[0].0.clone(),
            width: anim.width,
            height: anim.height,
        };
        if let Ok(mut cache) = get_anim_cache().lock() {
            cache.insert(src.to_string(), Arc::new(anim));
        }
        return Some(first);
    }
    decode_image_bytes(bytes)
}

/// 解码图片字节数据
fn decode_image_bytes(bytes: &[u8]) -> Option<ImageData> {
    use image::GenericImageView;
    
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    
    Some(ImageData {
        data: rgba.into_raw(),
        width,
        height,
    })
}

pub struct ImageComponent;

impl ImageComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let src = node.get_attr("src").unwrap_or("");
        let mode = node.get_attr("mode").unwrap_or("scaleToFill");
        
        // 检查 CSS 是否定义了尺寸和样式
        let has_custom_width = !matches!(ts.size.width, Dimension::Auto);
        let has_custom_height = !matches!(ts.size.height, Dimension::Auto);
        let has_custom_bg = ns.background_color.is_some();
        let has_custom_radius = ns.border_radius > 0.0 || 
                                ns.border_radius_tl.is_some() ||
                                ns.border_radius_tr.is_some() ||
                                ns.border_radius_br.is_some() ||
                                ns.border_radius_bl.is_some();
        
        // 默认图片大小 150x100 - 只在 CSS 没有定义时使用
        let default_width = 150.0;
        let default_height = 100.0;
        
        if !has_custom_width {
            ts.size.width = length(default_width * sf);
        }
        if !has_custom_height {
            ts.size.height = length(default_height * sf);
        }
        
        // 注意：不再强制设置默认背景色。
        // 透明 PNG（如图标）不应有不透明底色；仅当 CSS 显式设置 background-color
        // 时才绘制底色，图片加载失败时才回退到占位符浅灰底。
        let _ = has_custom_bg;
        
        // 只在 CSS 没有定义时使用默认圆角
        if !has_custom_radius {
            ns.border_radius = 4.0 * sf;
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        // 存储 src 和 mode 到 text 字段（用 | 分隔）
        let text_data = format!("{}|{}", src, mode);
        
        Some(RenderNode {
            tag: "image".into(),
            text: text_data,
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        _text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        _sf: f32
    ) {
        let style = &node.style;
        
        // 获取圆角值（支持四个角独立设置），并按盒子尺寸夹紧（含 border-radius:50% 圆形图）
        let [radius_tl, radius_tr, radius_br, radius_bl] = get_border_radii_clamped(style, w, h);
        let has_radius = radius_tl > 0.0 || radius_tr > 0.0 || radius_br > 0.0 || radius_bl > 0.0;
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        let radius = radius_tl; // 用于统一圆角的情况
        
        // 解析 src 和 mode
        let parts: Vec<&str> = node.text.split('|').collect();
        let src = parts.get(0).unwrap_or(&"");
        let mode = parts.get(1).unwrap_or(&"scaleToFill");
        
        // 应用透明度
        let apply_opacity = |color: Color| -> Color {
            if style.opacity < 1.0 {
                Color::new(color.r, color.g, color.b, (color.a as f32 * style.opacity) as u8)
            } else {
                color
            }
        };
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, radius);
        }
        
        // 仅当 CSS 显式设置了 background-color 时才绘制底色。
        // 这样透明 PNG（图标）不会被套上不透明底框。
        if let Some(bg) = style.background_color {
            let bg_color = apply_opacity(bg);
            let bg_paint = Paint::new()
                .with_color(bg_color)
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            
            if has_radius {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, y, w, h, radius);
                } else {
                    path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                }
                canvas.draw_path(&path, &bg_paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &bg_paint);
            }
        }
        
        // 尝试加载并绘制图片
        if !src.is_empty() {
            if let Some(img_data) = load_image(src) {
                // 绘制图片（透明度通过背景色已经处理）
                canvas.draw_image(
                    &img_data.data,
                    img_data.width,
                    img_data.height,
                    x, y, w, h,
                    mode,
                    radius,
                );
                
                // 绘制边框
                Self::draw_border(canvas, style, x, y, w, h, has_radius, uniform_radius, 
                                  radius, radius_tl, radius_tr, radius_br, radius_bl);
                return;
            }
        }
        
        // 如果图片加载失败，绘制占位符
        Self::draw_placeholder(canvas, x, y, w, h, has_radius, uniform_radius, 
                               radius, radius_tl, radius_tr, radius_br, radius_bl, style);
    }
    
    /// 绘制边框
    fn draw_border(
        canvas: &mut Canvas,
        style: &NodeStyle,
        x: f32, y: f32, w: f32, h: f32,
        has_radius: bool,
        uniform_radius: bool,
        radius: f32,
        radius_tl: f32, radius_tr: f32, radius_br: f32, radius_bl: f32,
    ) {
        if style.border_width > 0.0 {
            if let Some(bc) = style.border_color {
                let border_color = if style.opacity < 1.0 {
                    Color::new(bc.r, bc.g, bc.b, (bc.a as f32 * style.opacity) as u8)
                } else {
                    bc
                };
                let _ = (has_radius, uniform_radius);
                stroke_round_rect_ring(
                    canvas, x, y, w, h,
                    [radius_tl, radius_tr, radius_br, radius_bl],
                    style.border_width, border_color,
                );
            }
        }
    }
    
    /// 绘制图片占位符（山形+太阳图标）
    fn draw_placeholder(
        canvas: &mut Canvas,
        x: f32, y: f32, w: f32, h: f32,
        has_radius: bool,
        uniform_radius: bool,
        radius: f32,
        radius_tl: f32, radius_tr: f32, radius_br: f32, radius_bl: f32,
        style: &NodeStyle,
    ) {
        // 若 CSS 未显式设置底色，绘制默认浅灰底，保证占位图标可见
        if style.background_color.is_none() {
            let ph_paint = Paint::new()
                .with_color(Color::from_hex(0xF5F5F5))
                .with_style(PaintStyle::Fill)
                .with_anti_alias(true);
            if has_radius {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, y, w, h, radius);
                } else {
                    path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                }
                canvas.draw_path(&path, &ph_paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &ph_paint);
            }
        }
        
        let icon_size = w.min(h) * 0.35;
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        
        let icon_color = Color::from_hex(0xCCCCCC);
        let icon_paint = Paint::new()
            .with_color(icon_color)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        // 太阳（圆形）
        let sun_x = cx - icon_size * 0.25;
        let sun_y = cy - icon_size * 0.3;
        let sun_r = icon_size * 0.15;
        canvas.draw_circle(sun_x, sun_y, sun_r, &icon_paint);
        
        // 山形（三角形）
        let mut mountain = Path::new();
        mountain.move_to(cx - icon_size * 0.5, cy + icon_size * 0.35);
        mountain.line_to(cx - icon_size * 0.15, cy - icon_size * 0.05);
        mountain.line_to(cx + icon_size * 0.1, cy + icon_size * 0.35);
        mountain.close();
        canvas.draw_path(&mountain, &icon_paint);
        
        // 右边大山
        let mut mountain2 = Path::new();
        mountain2.move_to(cx - icon_size * 0.1, cy + icon_size * 0.35);
        mountain2.line_to(cx + icon_size * 0.25, cy - icon_size * 0.25);
        mountain2.line_to(cx + icon_size * 0.55, cy + icon_size * 0.35);
        mountain2.close();
        canvas.draw_path(&mountain2, &icon_paint);
        
        // 绘制边框
        Self::draw_border(canvas, style, x, y, w, h, has_radius, uniform_radius,
                          radius, radius_tl, radius_tr, radius_br, radius_bl);
    }
}
