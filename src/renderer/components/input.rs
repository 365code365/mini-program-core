//! input 组件 - 输入框
//! 
//! 微信小程序 input 组件的完整实现
//! 
//! ## 属性
//! 
//! | 属性 | 类型 | 默认值 | 说明 |
//! |------|------|--------|------|
//! | value | string | | 输入框的初始内容 |
//! | type | string | text | 输入类型：text/number/idcard/digit/safe-password/nickname |
//! | password | boolean | false | 是否是密码类型 |
//! | placeholder | string | | 输入框为空时占位符 |
//! | placeholder-style | string | | 占位符的样式（仅支持 color） |
//! | placeholder-class | string | input-placeholder | 占位符的样式类 |
//! | disabled | boolean | false | 是否禁用 |
//! | maxlength | number | 140 | 最大输入长度，-1 为不限制 |
//! | cursor-spacing | number | 0 | 光标与键盘的距离（px） |
//! | focus | boolean | false | 获取焦点 |
//! | confirm-type | string | done | 键盘右下角按钮文字：send/search/next/go/done |
//! | confirm-hold | boolean | false | 点击键盘确认按钮时是否保持键盘不收起 |
//! | cursor | number | | 指定 focus 时的光标位置 |
//! | selection-start | number | -1 | 光标起始位置，需与 selection-end 搭配使用 |
//! | selection-end | number | -1 | 光标结束位置，需与 selection-start 搭配使用 |
//! | adjust-position | boolean | true | 键盘弹起时，是否自动上推页面 |
//! 
//! ## 事件
//! 
//! | 事件 | 说明 | 返回值 |
//! |------|------|--------|
//! | bindinput | 输入时触发 | event.detail = {value, cursor, keyCode} |
//! | bindfocus | 聚焦时触发 | event.detail = {value, height} |
//! | bindblur | 失焦时触发 | event.detail = {value} |
//! | bindconfirm | 点击完成按钮时触发 | event.detail = {value} |
//! 
//! ## CSS 支持
//! 
//! 支持完整的 CSS 样式，包括：
//! - width/height
//! - padding
//! - background-color
//! - border/border-radius
//! - font-size/color
//! - box-shadow
//! - opacity

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;
use std::time::Instant;
use std::sync::{Mutex, OnceLock};

/// 光标闪烁周期（毫秒）
const CURSOR_BLINK_INTERVAL_MS: u64 = 530;

/// 默认最大输入长度
const DEFAULT_MAXLENGTH: i32 = 140;

/// 闪烁相位的零点。
///
/// **不能用进程启动时间**：那样点进输入框的瞬间光标可能正好处在「隐藏」的半个周期里，
/// 用户会看到「点了没反应」。微信是一聚焦就立刻显示光标，然后从那一刻开始数拍子，
/// 所以这里在每次聚焦时把零点重置（见 [`reset_cursor_blink`]）。
static BLINK_ORIGIN: OnceLock<Mutex<Instant>> = OnceLock::new();

fn blink_origin() -> &'static Mutex<Instant> {
    BLINK_ORIGIN.get_or_init(|| Mutex::new(Instant::now()))
}

/// 聚焦时重置闪烁相位：光标立刻可见，并从此刻开始数半个周期。
pub fn reset_cursor_blink() {
    if let Ok(mut o) = blink_origin().lock() {
        *o = Instant::now();
    }
}

/// 当前这一拍光标该不该显示。
///
/// 宿主也要用它：光标闪烁得靠宿主在**相位翻转时**安排一次重绘，
/// 否则画面停在最后一次绘制的那一帧 —— 表现就是「光标不会跳」。
/// 尤其是输入框在 `position:fixed` 覆盖层里时，那层画布只在标脏时才重画。
pub fn cursor_blink_visible() -> bool {
    let elapsed = blink_origin()
        .lock()
        .map(|o| o.elapsed().as_millis() as u64)
        .unwrap_or(0);
    blink_visible_at(elapsed)
}

/// 距相位零点 `elapsed_ms` 毫秒时光标是否可见（纯函数，便于测试）。
/// 每个周期的前半段显示。
pub fn blink_visible_at(elapsed_ms: u64) -> bool {
    (elapsed_ms / CURSOR_BLINK_INTERVAL_MS) % 2 == 0
}

/// 闪烁半周期（毫秒）
pub fn cursor_blink_interval_ms() -> u64 {
    CURSOR_BLINK_INTERVAL_MS
}

fn should_show_cursor() -> bool {
    cursor_blink_visible()
}

/// 最近一次画光标的矩形（页面画布的设备像素坐标：x, y, w, h）。
///
/// 给宿主定位输入法候选框用（`set_ime_cursor_area`）：不告诉系统光标在哪，
/// macOS/Windows 会把候选词面板摆到一个默认位置 —— 看起来就是「离输入框很远」。
static LAST_CARET: OnceLock<Mutex<Option<(f32, f32, f32, f32)>>> = OnceLock::new();

fn last_caret_slot() -> &'static Mutex<Option<(f32, f32, f32, f32)>> {
    LAST_CARET.get_or_init(|| Mutex::new(None))
}

pub fn last_caret_rect() -> Option<(f32, f32, f32, f32)> {
    last_caret_slot().lock().ok().and_then(|c| *c)
}

fn set_last_caret_rect(rect: Option<(f32, f32, f32, f32)>) {
    if let Ok(mut c) = last_caret_slot().lock() {
        *c = rect;
    }
}

/// 输入类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputType {
    Text,
    Number,
    IdCard,
    Digit,
    SafePassword,
    Nickname,
}

impl InputType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "number" => Self::Number,
            "idcard" => Self::IdCard,
            "digit" => Self::Digit,
            "safe-password" => Self::SafePassword,
            "nickname" => Self::Nickname,
            _ => Self::Text,
        }
    }
    
    /// 验证输入字符是否符合类型要求
    pub fn validate_char(&self, c: char) -> bool {
        match self {
            Self::Number => c.is_ascii_digit() || c == '-',
            Self::Digit => c.is_ascii_digit() || c == '.',
            Self::IdCard => c.is_ascii_digit() || c == 'X' || c == 'x',
            _ => true,
        }
    }
    
    /// 验证完整输入是否符合类型要求
    pub fn validate_input(&self, input: &str) -> bool {
        match self {
            Self::Number => input.chars().all(|c| c.is_ascii_digit() || c == '-'),
            Self::Digit => {
                let dot_count = input.chars().filter(|&c| c == '.').count();
                dot_count <= 1 && input.chars().all(|c| c.is_ascii_digit() || c == '.')
            }
            Self::IdCard => {
                input.len() <= 18 && input.chars().all(|c| c.is_ascii_digit() || c == 'X' || c == 'x')
            }
            _ => true,
        }
    }
}

/// 确认按钮类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfirmType {
    Send,
    Search,
    Next,
    Go,
    Done,
}

impl ConfirmType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "send" => Self::Send,
            "search" => Self::Search,
            "next" => Self::Next,
            "go" => Self::Go,
            _ => Self::Done,
        }
    }
}

/// 解析 placeholder-style 属性
fn parse_placeholder_style(style_str: &str) -> Option<Color> {
    // 支持格式: "color: #999999" 或 "color: rgb(153, 153, 153)"
    for part in style_str.split(';') {
        let part = part.trim();
        if part.starts_with("color:") {
            let color_str = part.trim_start_matches("color:").trim();
            return parse_color_value(color_str);
        }
    }
    None
}

/// 解析颜色值
fn parse_color_value(s: &str) -> Option<Color> {
    let s = s.trim();
    if s.starts_with('#') {
        // 十六进制颜色
        let hex = s.trim_start_matches('#');
        if hex.len() == 6 {
            if let Ok(val) = u32::from_str_radix(hex, 16) {
                return Some(Color::from_hex(val));
            }
        } else if hex.len() == 3 {
            // 短格式 #RGB -> #RRGGBB
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            return Some(Color::new(r * 17, g * 17, b * 17, 255));
        }
    } else if s.starts_with("rgb(") && s.ends_with(')') {
        // rgb(r, g, b) 格式
        let inner = &s[4..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(Color::new(r, g, b, 255));
        }
    } else if s.starts_with("rgba(") && s.ends_with(')') {
        // rgba(r, g, b, a) 格式
        let inner = &s[5..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 4 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            let a = parts[3].trim().parse::<f32>().ok()?;
            return Some(Color::new(r, g, b, (a * 255.0) as u8));
        }
    }
    None
}

pub struct InputComponent;

impl InputComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        // 使用 base 的样式解析，获取 CSS 定义的样式
        let (mut ts, mut ns) = build_base_style(node, ctx);
        // 替换元素：作者写死的尺寸就是它的最小尺寸，别被兄弟压没。
        // 必须在合成默认尺寸**之前**调用 —— CSS 语义里「指定尺寸」只算作者写的那个，
        // 引擎给 textarea 之类补的默认高度不算，那种情况仍应允许被父级压缩。
        super::pin_replaced_min_size(&mut ts);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        // 解析属性
        // `model:value` 是小程序的双向绑定写法，取值语义与 `value` 相同
        let value = node
            .get_attr("value")
            .or_else(|| node.get_attr("model:value"))
            .unwrap_or("");
        let placeholder = node.get_attr("placeholder").unwrap_or("");
        let input_type = InputType::from_str(node.get_attr("type").unwrap_or("text"));
        let password = node.get_attr("password").map(|s| s == "true" || s == "{{true}}").unwrap_or(false)
            || input_type == InputType::SafePassword;
        let disabled = node.get_attr("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let _maxlength = node.get_attr("maxlength")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(DEFAULT_MAXLENGTH);
        let _focus = node.get_attr("focus").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let _cursor = node.get_attr("cursor").and_then(|s| s.parse::<usize>().ok());
        let _selection_start = node.get_attr("selection-start").and_then(|s| s.parse::<i32>().ok()).unwrap_or(-1);
        let _selection_end = node.get_attr("selection-end").and_then(|s| s.parse::<i32>().ok()).unwrap_or(-1);
        let _confirm_type = ConfirmType::from_str(node.get_attr("confirm-type").unwrap_or("done"));
        let _confirm_hold = node.get_attr("confirm-hold").map(|s| s == "true" || s == "{{true}}").unwrap_or(false);
        let _adjust_position = node.get_attr("adjust-position").map(|s| s != "false" && s != "{{false}}").unwrap_or(true);
        let _cursor_spacing = node.get_attr("cursor-spacing").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
        
        // 解析 placeholder 样式
        let placeholder_color = node.get_attr("placeholder-style")
            .and_then(|s| parse_placeholder_style(s))
            .unwrap_or(Color::from_hex(0xBFBFBF));
        
        let is_textarea = node.tag_name == "textarea";
        
        // 检查 CSS 是否定义了样式
        let has_custom_width = !dim_is_auto(ts.size.width);
        let has_custom_height = !dim_is_auto(ts.size.height);
        let has_custom_padding = !(length_px(ts.padding.top) == 0.0) ||
                                  !(length_px(ts.padding.left) == 0.0);
        let has_custom_bg = ns.background_color.is_some();
        let has_custom_border = ns.border_color.is_some() || ns.border_width > 0.0;
        let has_custom_radius = ns.border_radius > 0.0;
        let has_custom_font_size = ns.font_size != 14.0; // 14.0 是 NodeStyle 默认值
        
        // 尺寸处理 - 支持 flex 布局
        if !has_custom_width {
            // 如果设置了 flex-grow，不设置固定宽度
            if ts.flex_grow == 0.0 {
                ts.size.width = percent(1.0);
            }
        }
        
        // 允许收缩
        if ts.flex_shrink == 0.0 {
            ts.flex_shrink = 1.0;
        }
        
        // 默认 padding（要先定 padding，高度按「行高 + 上下内边距」推）
        if !has_custom_padding {
            ts.padding = Rect { 
                top: length(8.0 * sf), 
                right: length(12.0 * sf), 
                bottom: length(8.0 * sf), 
                left: length(12.0 * sf) 
            };
            ns.padding_top = 8.0;
            ns.padding_bottom = 8.0;
            ns.padding_left = 12.0;
            ns.padding_right = 12.0;
        }
        
        // 默认高度：单行输入框 = 字体行盒 + 上下内边距，不再写死 42px。
        //
        // 浏览器里 `<input>` 不写 height 时高度就是「line-height:normal 的行盒 + padding」，
        // 例如 font-size:14px + padding:8px → 32.5px。固定 42px 会让输入框凭空高出近 10px，
        // 其后所有内容一路下移，两端从首屏就开始整体错位（首页搜索栏即是此因）。
        // 行高取西文因子：输入框行盒只由字体决定，不随占位文字是否中文而变（与浏览器一致）。
        if !has_custom_height {
            let pad_v = (ns.padding_top + ns.padding_bottom) * sf;
            let line = ns.font_size * sf * crate::text::LATIN_LINE_HEIGHT_FACTOR;
            ts.size.height = length(if is_textarea {
                (line * 4.0 + pad_v).max(80.0 * sf)
            } else {
                line + pad_v
            });
        }
        
        // 微信小程序 <input> 默认无边框、无背景（透明），交由外层容器决定外观。
        // 之前强制加了白底 + 灰色边框，导致放进带样式的容器里出现「双层框」。
        // 仅当 CSS 未定义时保持透明，不再注入默认边框/底色。
        let _ = (has_custom_bg, has_custom_border, has_custom_radius);
        
        // 默认字体大小
        if !has_custom_font_size {
            ns.font_size = 16.0;
        }
        
        // 显示文本
        let display_text = if value.is_empty() {
            placeholder.to_string()
        } else if password {
            "•".repeat(value.chars().count())
        } else {
            value.to_string()
        };
        
        // 文本颜色 - 只在没有自定义颜色时使用默认值
        if ns.text_color.is_none() {
            ns.text_color = Some(if value.is_empty() {
                placeholder_color
            } else if disabled {
                Color::from_hex(0xBFBFBF)
            } else {
                Color::BLACK
            });
        }
        
        // 禁用状态背景（覆盖自定义样式）
        if disabled && !has_custom_bg {
            ns.background_color = Some(Color::from_hex(0xF5F5F5));
        }
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        Some(RenderNode {
            tag: node.tag_name.clone(),
            text: display_text,
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
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32
    ) {
        Self::draw_with_cursor(node, canvas, text_renderer, x, y, w, h, sf, false, 0);
    }
    
    pub fn draw_with_cursor(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        focused: bool,
        cursor_pos: usize,
    ) {
        Self::draw_with_selection(node, canvas, text_renderer, x, y, w, h, sf, focused, cursor_pos, None);
    }
    
    pub fn draw_with_selection(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32,
        focused: bool,
        cursor_pos: usize,
        selection: Option<(usize, usize)>,
    ) {
        let style = &node.style;
        
        // 获取圆角值（支持四角独立设置）
        let radius_tl = style.border_radius_tl.unwrap_or(style.border_radius);
        let radius_tr = style.border_radius_tr.unwrap_or(style.border_radius);
        let radius_br = style.border_radius_br.unwrap_or(style.border_radius);
        let radius_bl = style.border_radius_bl.unwrap_or(style.border_radius);
        let has_radius = radius_tl > 0.0 || radius_tr > 0.0 || radius_br > 0.0 || radius_bl > 0.0;
        let uniform_radius = radius_tl == radius_tr && radius_tr == radius_br && radius_br == radius_bl;
        
        // 绘制盒子阴影
        if let Some(shadow) = &style.box_shadow {
            draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
        }
        
        // 绘制背景
        if let Some(bg) = style.background_color {
            // 应用透明度
            let bg = if style.opacity < 1.0 {
                Color::new(bg.r, bg.g, bg.b, (bg.a as f32 * style.opacity) as u8)
            } else {
                bg
            };
            
            let paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
            if has_radius {
                let mut path = Path::new();
                if uniform_radius {
                    path.add_round_rect(x, y, w, h, radius_tl);
                } else {
                    path.add_round_rect_varying(x, y, w, h, radius_tl, radius_tr, radius_br, radius_bl);
                }
                canvas.draw_path(&path, &paint);
            } else {
                canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
            }
        }
        
        // 绘制边框 - 聚焦时高亮（除非有自定义边框颜色）
        let border_color = if focused {
            Color::from_hex(0x07C160) // 微信绿色
        } else {
            style.border_color.unwrap_or(Color::from_hex(0xD9D9D9))
        };
        
        if style.border_width > 0.0 {
            stroke_round_rect_ring(
                canvas, x, y, w, h,
                [radius_tl, radius_tr, radius_br, radius_bl],
                style.border_width, border_color,
            );
        }
        // 逐边边框（`border-bottom` 那条下划线是表单最常见的写法）。
        // input 自己画盒子，不走通用的背景/边框绘制，所以这里要显式补一次 ——
        // 漏了它的话「下划线式输入框」在引擎里完全看不见（登录页的手机号/验证码就是）。
        draw_side_borders(canvas, style, x, y, w, h);
        
        // 计算文本位置
        let font_size = style.font_size * sf;
        // 用元素**实际**的内边距，不是写死的 12px。
        //
        // 布局（taffy）用的是 `style.padding`（页面没写时 build 期给 8/12 的默认值），
        // 而这里以前写死 12 —— 页面自己设了 `padding-left` 时两边就对不上：
        // 盒子按 CSS 排版，文字/占位符/光标却固定缩进 12px，看起来「输入提示离左边太远（或太近）」。
        // HTML 端的 `.wx-input` 是 `padding:0` + 外层盒子的 padding，只有按实际值取才对得齐。
        let padding_left = style.padding_left * sf;
        let padding_right = style.padding_right * sf;
        let text_x = x + padding_left;
        
        // 根据 text-align 和 vertical-align 计算位置
        let text_y = match style.vertical_align {
            VerticalAlign::Top => y + font_size + 4.0 * sf,
            VerticalAlign::Bottom => y + h - 4.0 * sf,
            _ => y + (h + font_size) / 2.0 - 2.0 * sf, // Middle/Baseline - 垂直居中
        };
        
        if let Some(tr) = text_renderer {
            // 设置裁剪区域，防止文本溢出输入框
            canvas.save();
            let clip_x = x + padding_left;
            let clip_y = y;
            let clip_w = w - padding_left - padding_right;
            let clip_h = h;
            canvas.clip_rect(GeoRect::new(clip_x, clip_y, clip_w, clip_h));
            
            // 先计算文本偏移（用于所有绘制）
            let text_width = if !node.text.is_empty() {
                tr.measure_text(&node.text, font_size)
            } else {
                0.0
            };
            let available_width = w - padding_left - padding_right;
            
            let mut text_offset = 0.0;
            if focused && text_width > available_width {
                let cursor_text: String = node.text.chars().take(cursor_pos).collect();
                let cursor_x_in_text = tr.measure_text(&cursor_text, font_size);
                
                if cursor_x_in_text > available_width {
                    text_offset = available_width - cursor_x_in_text - font_size;
                }
            }
            
            // 绘制选中背景（需要考虑 text_offset）
            if let Some((sel_start, sel_end)) = selection {
                if sel_start != sel_end && focused {
                    let start_text: String = node.text.chars().take(sel_start).collect();
                    let sel_text: String = node.text.chars().skip(sel_start).take(sel_end - sel_start).collect();
                    
                    let sel_x = text_x + tr.measure_text(&start_text, font_size) + text_offset;
                    let sel_w = tr.measure_text(&sel_text, font_size);
                    let sel_y = y + (h - font_size) / 2.0 - 2.0 * sf;
                    let sel_h = font_size + 4.0 * sf;
                    
                    let sel_paint = Paint::new()
                        .with_color(Color::new(7, 193, 96, 80)) // 半透明绿色
                        .with_style(PaintStyle::Fill);
                    canvas.draw_rect(&GeoRect::new(sel_x, sel_y, sel_w, sel_h), &sel_paint);
                }
            }
            
            // 绘制文本
            let color = style.text_color.unwrap_or(Color::BLACK);
            let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill);
            
            if !node.text.is_empty() {
                // 根据 text-align 调整 x 位置
                let final_x = match style.text_align {
                    TextAlign::Center if text_width <= available_width => {
                        text_x + (available_width - text_width) / 2.0
                    }
                    TextAlign::Right if text_width <= available_width => {
                        text_x + available_width - text_width
                    }
                    _ => text_x + text_offset, // Left 或文本超出时
                };
                
                tr.draw_text(canvas, &node.text, final_x, text_y, font_size, &paint);
            }
            
            // ───────────────── 光标 ─────────────────
            //
            // 只在没有选中（或选中范围为空）时显示，按 CURSOR_BLINK_INTERVAL_MS 闪烁。
            //
            // 几何按微信/iOS 对齐，以前是「1 设备像素的描边线段，高度正好等于字号」：
            // - 1 设备像素在 2x 屏上只有半个逻辑像素，细到几乎看不见（也就更看不出闪烁）；
            //   微信的光标是 **2 逻辑像素**宽，两端带圆角；
            // - 高度取字号会显得比文字矮一截，微信是略高于字面（约 1.15em）并垂直居中。
            if focused && selection.map(|(s, e)| s == e).unwrap_or(true) {
                let cursor_text: String = node.text.chars().take(cursor_pos).collect();
                let cursor_x = text_x + tr.measure_text(&cursor_text, font_size) + text_offset;
                let caret_w = (2.0 * sf).max(2.0);
                let caret_h = font_size * 1.15;
                let caret_x = cursor_x - caret_w / 2.0;
                let caret_y = y + (h - caret_h) / 2.0;
                // 位置每帧都记（不管这一拍显不显示）：输入法候选框要靠它定位
                set_last_caret_rect(Some((caret_x, caret_y, caret_w, caret_h)));
                if should_show_cursor() {
                    let paint = Paint::new()
                        .with_color(Color::from_hex(0x07C160))
                        .with_style(PaintStyle::Fill)
                        .with_anti_alias(true);
                    let mut path = Path::new();
                    path.add_round_rect(caret_x, caret_y, caret_w, caret_h, caret_w / 2.0);
                    canvas.draw_path(&path, &paint);
                }
            }
            
            // 恢复裁剪区域
            canvas.restore();
        }
    }
}

/// 获取输入框的 maxlength 属性
pub fn get_maxlength(attrs: &std::collections::HashMap<String, String>) -> i32 {
    attrs.get("maxlength")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(DEFAULT_MAXLENGTH)
}

/// 获取输入框的 type 属性
pub fn get_input_type(attrs: &std::collections::HashMap<String, String>) -> InputType {
    attrs.get("type")
        .map(|s| InputType::from_str(s))
        .unwrap_or(InputType::Text)
}

/// 检查输入框是否为密码类型
pub fn is_password(attrs: &std::collections::HashMap<String, String>) -> bool {
    attrs.get("password").map(|s| s == "true" || s == "{{true}}").unwrap_or(false)
        || get_input_type(attrs) == InputType::SafePassword
}

/// 检查输入框是否禁用
pub fn is_disabled(attrs: &std::collections::HashMap<String, String>) -> bool {
    attrs.get("disabled").map(|s| s == "true" || s == "{{true}}").unwrap_or(false)
}
