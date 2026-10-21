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
use super::image_cache::ByteLru;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};

/// 图片缓存数据
pub(super) struct ImageData {
    /// RGBA 数据。用 Arc 共享：`load_image` 每帧都会被调用，
    /// 以前每次都把整张解码后的位图深拷贝一份（一张 1500x1000 的图就是 6MB memcpy/帧/张）。
    data: Arc<Vec<u8>>,
    pub(super) width: u32,
    pub(super) height: u32,
    /// 动图当前帧序号（静态图恒为 0）。缩放缓存 key 要带上它，
    /// 否则 GIF 会一直复用第一帧的重采样结果。
    frame_index: u32,
}

/// 动图（GIF）解码结果：逐帧 RGBA + 每帧展示时长
pub(super) struct AnimatedImage {
    /// 每帧 (RGBA 数据, 该帧持续毫秒)
    frames: Vec<(Arc<Vec<u8>>, u32)>,
    width: u32,
    height: u32,
    /// 一轮播放的总时长（毫秒，至少 1）
    total_ms: u32,
    /// 首次解码时刻，用作播放起点（保证首帧渲染确定为第 0 帧）
    started: std::time::Instant,
}

impl AnimatedImage {
    pub(super) fn width(&self) -> u32 { self.width }
    pub(super) fn height(&self) -> u32 { self.height }

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

/// 全局图片缓存（**按字节封顶 + LRU**，见 `image_cache.rs`）。
///
/// 以前是无上限的 `HashMap`：解码后的 RGBA 一张 1200×1200 就 5.5MB，长列表滚一遍
/// 几十张全留着，换页也不释放（进程级缓存）。手机上就是被系统杀掉。
static IMAGE_CACHE: OnceLock<Arc<Mutex<ByteLru<Option<Arc<ImageData>>>>>> = OnceLock::new();
/// 全局动图缓存（GIF 多帧）。一段动图是「帧数 × 单帧 RGBA」，比静态图更吃内存，
/// 所以同样封顶，预算取解码图预算的一半。
static ANIM_CACHE: OnceLock<Arc<Mutex<ByteLru<Arc<AnimatedImage>>>>> = OnceLock::new();

pub(super) fn get_image_cache() -> &'static Arc<Mutex<ByteLru<Option<Arc<ImageData>>>>> {
    IMAGE_CACHE.get_or_init(|| Arc::new(Mutex::new(ByteLru::new(super::image_cache::budget_bytes()))))
}

pub(super) fn get_anim_cache() -> &'static Arc<Mutex<ByteLru<Arc<AnimatedImage>>>> {
    ANIM_CACHE
        .get_or_init(|| Arc::new(Mutex::new(ByteLru::new(super::image_cache::budget_bytes() / 2))))
}

/// 解码后一张图占多少字节（RGBA）
fn image_bytes(d: &Option<Arc<ImageData>>) -> usize {
    match d {
        // 失败记录只占一个 key，但要给个非零值，免得几万条失败记录白占内存
        None => 64,
        Some(img) => img.data.len() + 64,
    }
}

fn anim_bytes(a: &AnimatedImage) -> usize {
    a.frames.iter().map(|(f, _)| f.len()).sum::<usize>() + 128
}

/// 图片缓存现状（诊断/内存报告用）
pub fn image_cache_report() -> String {
    let budget = super::image_cache::budget_bytes();
    let (n1, b1, e1) = get_image_cache()
        .lock()
        .map(|c| (c.len(), c.bytes(), c.evicted()))
        .unwrap_or((0, 0, 0));
    let (n2, b2, e2) = get_anim_cache()
        .lock()
        .map(|c| (c.len(), c.bytes(), c.evicted()))
        .unwrap_or((0, 0, 0));
    format!(
        "静态图 {n1} 张 / {:.1}MB（逐出 {e1}），动图 {n2} 段 / {:.1}MB（逐出 {e2}），预算 {:.0}MB + {:.0}MB",
        b1 as f32 / 1048576.0,
        b2 as f32 / 1048576.0,
        budget as f32 / 1048576.0,
        budget as f32 / 2097152.0,
    )
}

/// 一张图的 RGBA 像素（与 `<image>` 同一份缓存；远程图没到时给 None 并在后台下载）。
/// 给宿主浮层（`wx.previewImage`）用
pub fn image_rgba(src: &str) -> Option<(Arc<Vec<u8>>, u32, u32)> {
    load_image(src).map(|d| (d.data.clone(), d.width, d.height))
}

/// 图片缓存的字节预算（MB，诊断/测试用）
pub fn image_cache_budget_mb() -> usize {
    super::image_cache::budget_bytes() / (1024 * 1024)
}

/// 清空图片缓存（切换小程序、收到系统内存告警时调用）
pub fn clear_image_caches() {
    if let Ok(mut c) = get_image_cache().lock() {
        c.clear();
    }
    if let Ok(mut c) = get_anim_cache().lock() {
        c.clear();
    }
}

/// 该资源是否为多帧动图（GIF）。
pub fn is_animated(src: &str) -> bool {
    get_anim_cache()
        .lock()
        .map(|cache| cache.peek(src).map(|a| a.frames.len() > 1).unwrap_or(false))
        .unwrap_or(false)
}

/// 已解码的多帧动图此刻该显示第几帧（与 `load_image` 同一个时钟）；静态图、还没解码过的返回 None。
/// 渲染器拿它判断「画布上那张动图该不该换帧了」（`wxml_renderer/animated_image.rs`）。
pub fn animated_frame(src: &str) -> Option<u32> {
    let cache = get_anim_cache().lock().ok()?;
    cache.peek(src).filter(|a| a.frames.len() > 1).map(|a| a.current_index() as u32)
}

/// 已缓存动图的一轮播放时长（毫秒）；非动图返回 None。
pub fn animation_total_ms(src: &str) -> Option<u32> {
    let cache = get_anim_cache().lock().ok()?;
    cache.peek(src).filter(|a| a.frames.len() > 1).map(|a| a.total_ms)
}

/// 加载与解码（本地 / 远程 / GIF）
mod load;
use load::*;

pub struct ImageComponent;

impl ImageComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        // 替换元素：作者写死的尺寸就是它的最小尺寸，别被兄弟压没。
        // 必须在合成默认尺寸**之前**调用 —— CSS 语义里「指定尺寸」只算作者写的那个，
        // 引擎给 textarea 之类补的默认高度不算，那种情况仍应允许被父级压缩。
        super::pin_replaced_min_size(&mut ts);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let src = node.get_attr("src").unwrap_or("");
        let mode = node.get_attr("mode").unwrap_or("scaleToFill");
        
        // 检查 CSS 是否定义了尺寸和样式
        let has_custom_width = !dim_is_auto(ts.size.width);
        let has_custom_height = !dim_is_auto(ts.size.height);
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
                if super::image_net::log_enabled() {
                    eprintln!("🖼 ✓ 画出 {:.0}x{:.0} src={:?}", w, h, src);
                }
                // 绘制图片（透明度通过背景色已经处理）
                // 走带缩放缓存的绘制：动画页面每帧重绘时不再重复重采样
                canvas.draw_image_cached(
                    &format!("{}#{}", src, img_data.frame_index),
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
        //
        // 诊断（`MINI_IMG_LOG=1`）：画成占位是「还在下载」「下载失败」还是「src 为空」，
        // 光看图片分不出来 —— 而这三种的修法完全不同。
        if super::image_net::log_enabled() {
            eprintln!("🖼 ▢ 占位 {:.0}x{:.0} src={:?}", w, h, src);
        }
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
        _radius: f32,
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
