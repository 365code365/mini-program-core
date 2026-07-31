//! 逐条 CSS 属性落到样式对象上（最长的一段 match）
//!
//! `style_apply_new` 的一片。**纯搬迁**：从 792 行按职责切开，一行逻辑没改。
use super::*;

/// 应用单个样式属性
pub(super) fn apply_style_property(
    name: &str,
    value: &StyleValue,
    ts: &mut Style,
    ns: &mut NodeStyle,
    ctx: &mut ComponentContext
) {
    let sf = ctx.scale_factor;
    match name {
            "width" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.size.width = v; }
            "height" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.size.height = v; }
            "min-width" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.min_size.width = v; }
            "min-height" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.min_size.height = v; }
            "max-width" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.max_size.width = v; }
            "max-height" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.max_size.height = v; }
            "padding" => if let Some([t, r, b, l]) = box_sides_px(value, ctx) {
                ts.padding = Rect {
                    top: length(t * sf), right: length(r * sf),
                    bottom: length(b * sf), left: length(l * sf),
                };
                ns.padding_top = t; ns.padding_right = r; ns.padding_bottom = b; ns.padding_left = l;
            }
            "padding-top" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.top = length(v * sf); ns.padding_top = v; }
            "padding-right" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.right = length(v * sf); ns.padding_right = v; }
            "padding-bottom" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.bottom = length(v * sf); ns.padding_bottom = v; }
            "padding-left" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.padding.left = length(v * sf); ns.padding_left = v; }
            "margin" => {
                if matches!(value, StyleValue::Auto) {
                    // margin: auto -> 居中
                    ts.margin = Rect { top: auto(), right: auto(), bottom: auto(), left: auto() };
                } else if let Some([t, r, b, l]) = box_sides_px(value, ctx) {
                    ts.margin = Rect {
                        top: length(t * sf), right: length(r * sf),
                        bottom: length(b * sf), left: length(l * sf),
                    };
                }
            }
            "margin-top" => if matches!(value, StyleValue::Auto) { ts.margin.top = auto(); } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.top = length(v * sf); }
            "margin-right" => if matches!(value, StyleValue::Auto) { ts.margin.right = auto(); } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.right = length(v * sf); }
            "margin-bottom" => if matches!(value, StyleValue::Auto) { ts.margin.bottom = auto(); } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.bottom = length(v * sf); }
            "margin-left" => if matches!(value, StyleValue::Auto) { ts.margin.left = auto(); } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.margin.left = length(v * sf); }
            "display" => if let StyleValue::String(s) = value {
                match s.as_str() {
                    "none" => ts.display = Display::None,
                    "block" => {
                        ts.display = Display::Block;
                        ns.is_block = true;
                    }
                    "flex" => ts.display = Display::Flex,
                    "grid" => ts.display = Display::Grid,
                    _ => ts.display = Display::Flex,
                };
            }
            // `box-sizing`：决定 width/height 算的是内容盒还是边框盒。
            //
            // 从前这条属性被整条忽略（布局引擎里没有这个概念），于是写
            // `box-sizing: content-box` 的元素会比浏览器窄一圈内边距 + 边框。
            // 两端的缺省都是 `border-box`（编译出的 H5 里有 `*{box-sizing:border-box}`，
            // 布局引擎的缺省也是它），所以只有显式写 content-box 的地方会变。
            "box-sizing" => if let StyleValue::String(s) = value {
                ts.box_sizing = match s.as_str() {
                    "content-box" => BoxSizing::ContentBox,
                    _ => BoxSizing::BorderBox,
                };
            }
            "flex-direction" => if let StyleValue::String(s) = value {
                ts.flex_direction = match s.as_str() {
                    "row" => FlexDirection::Row,
                    "row-reverse" => FlexDirection::RowReverse,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    "column" => FlexDirection::Column,
                    _ => FlexDirection::Column,
                };
            }
            "flex-wrap" => if let StyleValue::String(s) = value {
                ts.flex_wrap = match s.as_str() {
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    _ => FlexWrap::NoWrap,
                };
            }
            "flex-grow" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.flex_grow = v; }
            "flex" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                ts.flex_grow = v;
                // flex: <number> implies flex-grow: <number>, flex-shrink: 1, flex-basis: 0
                ts.flex_shrink = 1.0;
                ts.flex_basis = Dimension::length(0.0);
            }
            "flex-shrink" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.flex_shrink = v; }
            "flex-basis" => if let Some(v) = to_dimension(value, ctx.screen_width, ctx.screen_height, sf) { ts.flex_basis = v; }
            "justify-content" => if let StyleValue::String(s) = value {
                ts.justify_content = Some(match s.as_str() {
                    "center" => JustifyContent::CENTER,
                    "space-between" => JustifyContent::SPACE_BETWEEN,
                    "space-around" => JustifyContent::SPACE_AROUND,
                    "space-evenly" => JustifyContent::SPACE_EVENLY,
                    "flex-end" | "end" => JustifyContent::FLEX_END,
                    "flex-start" | "start" => JustifyContent::FLEX_START,
                    _ => JustifyContent::FLEX_START,
                });
            }
            "align-items" => if let StyleValue::String(s) = value {
                let align = match s.as_str() {
                    "center" => AlignItems::CENTER,
                    "flex-end" | "end" => AlignItems::FLEX_END,
                    "flex-start" | "start" => AlignItems::FLEX_START,
                    "stretch" => AlignItems::STRETCH,
                    "baseline" => AlignItems::BASELINE,
                    _ => AlignItems::FLEX_START,
                };
                ts.align_items = Some(align);
            }
            "align-self" => if let StyleValue::String(s) = value {
                ts.align_self = Some(match s.as_str() {
                    "center" => AlignSelf::CENTER,
                    "flex-end" | "end" => AlignSelf::FLEX_END,
                    "flex-start" | "start" => AlignSelf::FLEX_START,
                    "stretch" => AlignSelf::STRETCH,
                    "baseline" => AlignSelf::BASELINE,
                    _ => AlignSelf::START,
                });
            }
            "align-content" => if let StyleValue::String(s) = value {
                ts.align_content = Some(match s.as_str() {
                    "center" => AlignContent::CENTER,
                    "flex-end" | "end" => AlignContent::FLEX_END,
                    "flex-start" | "start" => AlignContent::FLEX_START,
                    "stretch" => AlignContent::STRETCH,
                    "space-between" => AlignContent::SPACE_BETWEEN,
                    "space-around" => AlignContent::SPACE_AROUND,
                    "space-evenly" => AlignContent::SPACE_EVENLY,
                    _ => AlignContent::FLEX_START,
                });
            }
            "gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { 
                let sv = v * sf;
                ts.gap = Size { width: length(sv), height: length(sv) }; 
            }
            "row-gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.gap.height = length(v * sf); }
            "column-gap" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ts.gap.width = length(v * sf); }
            "background-color" | "background" | "background-image" => {
                match value {
                    StyleValue::Color(c) => ns.background_color = Some(*c),
                    StyleValue::String(s) if s.contains("linear-gradient(") => {
                        if let Some(g) = parse_linear_gradient(s) {
                            // 兜底纯色（渐变未绘制处使用）取首个停靠点
                            ns.background_color = g.stops.first().map(|(_, c)| *c);
                            ns.background_gradient = Some(g);
                        }
                    }
                    StyleValue::String(s) => {
                        if let Some(c) = parse_color_str(s) { ns.background_color = Some(c); }
                    }
                    _ => {}
                }
            }
            "color" => {
                if let StyleValue::Color(c) = value { 
                    ns.text_color = Some(*c); 
                } else if let StyleValue::String(s) = value {
                    if let Some(c) = parse_color_str(s) {
                        ns.text_color = Some(c);
                    }
                }
            }
            "border-color" => {
                if let StyleValue::Color(c) = value { 
                    ns.border_color = Some(*c); 
                } else if let StyleValue::String(s) = value {
                    if let Some(c) = parse_color_str(s) {
                        ns.border_color = Some(c);
                    }
                }
            }
            "border-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_width = v * sf; }
            "border-radius" => {
                // 支持 border-radius 简写：1-4 个值
                if let StyleValue::String(s) = value {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    let values: Vec<f32> = parts.iter()
                        .filter_map(|p| {
                            let sv = parse_inline_value(p);
                            to_px(&sv, ctx.screen_width, ctx.screen_height)
                        })
                        .map(|v| v * sf)
                        .collect();
                    
                    match values.len() {
                        1 => {
                            ns.border_radius = values[0];
                        }
                        2 => {
                            // top-left/bottom-right, top-right/bottom-left
                            ns.border_radius_tl = Some(values[0]);
                            ns.border_radius_br = Some(values[0]);
                            ns.border_radius_tr = Some(values[1]);
                            ns.border_radius_bl = Some(values[1]);
                        }
                        3 => {
                            // top-left, top-right/bottom-left, bottom-right
                            ns.border_radius_tl = Some(values[0]);
                            ns.border_radius_tr = Some(values[1]);
                            ns.border_radius_bl = Some(values[1]);
                            ns.border_radius_br = Some(values[2]);
                        }
                        4 => {
                            // top-left, top-right, bottom-right, bottom-left
                            ns.border_radius_tl = Some(values[0]);
                            ns.border_radius_tr = Some(values[1]);
                            ns.border_radius_br = Some(values[2]);
                            ns.border_radius_bl = Some(values[3]);
                        }
                        _ => {}
                    }
                } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) {
                    ns.border_radius = v * sf;
                }
            }
            "border-top-left-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius_tl = Some(v * sf); }
            "border-top-right-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius_tr = Some(v * sf); }
            "border-bottom-right-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius_br = Some(v * sf); }
            "border-bottom-left-radius" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_radius_bl = Some(v * sf); }
            "border" => {
                // border: 1px solid #000
                if let StyleValue::String(s) = value {
                    parse_border_shorthand(s, ns, ctx.screen_width, sf);
                }
            }
            "border-top" | "border-right" | "border-bottom" | "border-left" => {
                if let StyleValue::String(s) = value {
                    let (w, c) = parse_border_side(s, ctx.screen_width, sf);
                    match name {
                        "border-top" => { if let Some(w) = w { ns.border_top_width = w; } ns.border_top_color = c.or(ns.border_top_color); }
                        "border-right" => { if let Some(w) = w { ns.border_right_width = w; } ns.border_right_color = c.or(ns.border_right_color); }
                        "border-bottom" => { if let Some(w) = w { ns.border_bottom_width = w; } ns.border_bottom_color = c.or(ns.border_bottom_color); }
                        _ => { if let Some(w) = w { ns.border_left_width = w; } ns.border_left_color = c.or(ns.border_left_color); }
                    }
                }
            }
            "border-top-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_top_width = v * sf; }
            "border-right-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_right_width = v * sf; }
            "border-bottom-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_bottom_width = v * sf; }
            "border-left-width" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.border_left_width = v * sf; }
            "border-top-color" => { if let Some(c) = color_value(value) { ns.border_top_color = Some(c); } }
            "border-right-color" => { if let Some(c) = color_value(value) { ns.border_right_color = Some(c); } }
            "border-bottom-color" => { if let Some(c) = color_value(value) { ns.border_bottom_color = Some(c); } }
            "border-left-color" => { if let Some(c) = color_value(value) { ns.border_left_color = Some(c); } }
            "font-size" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.font_size = v; }
            "font-weight" => {
                // 数字字重（400/700）与关键字（normal/bold）都要支持
                let text = match value {
                    StyleValue::String(s) => s.clone(),
                    StyleValue::Number(n) => format!("{}", *n as i32),
                    StyleValue::Length(v, _) => format!("{}", *v as i32),
                    _ => String::new(),
                };
                ns.font_weight = match text.as_str() {
                    "100" => FontWeight::W100,
                    "200" => FontWeight::W200,
                    "300" | "light" => FontWeight::W300,
                    "400" | "normal" => FontWeight::Normal,
                    "500" | "medium" => FontWeight::W500,
                    "600" | "semibold" => FontWeight::W600,
                    "700" | "bold" => FontWeight::Bold,
                    "800" => FontWeight::W800,
                    "900" | "black" => FontWeight::W900,
                    _ => FontWeight::Normal,
                };
            }
            "text-align" => if let StyleValue::String(s) = value {
                ns.text_align = match s.as_str() {
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    "justify" => TextAlign::Justify,
                    _ => TextAlign::Left,
                };
            }
            "text-decoration" | "text-decoration-line" => if let StyleValue::String(s) = value {
                ns.text_decoration = match s.as_str() {
                    "underline" => TextDecoration::Underline,
                    "line-through" => TextDecoration::LineThrough,
                    "overline" => TextDecoration::Overline,
                    _ => TextDecoration::None,
                };
            }
            "line-height" => {
                // 顺序很关键：无单位值是「倍数」，必须先判数字。
                // 之前先走 to_px，`line-height:1.9` 被当成 1.9 像素，行距直接被压成一条线。
                if let StyleValue::Number(n) = value {
                    ns.line_height_scale = Some(*n);
                    ns.line_height = None;
                } else if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) {
                    ns.line_height = Some(v);
                    ns.line_height_scale = None;
                }
            }
            "letter-spacing" => if let Some(v) = to_px(value, ctx.screen_width, ctx.screen_height) { ns.letter_spacing = v; }
            // 字体栈原样留到绘制/度量期解析（见 `text_family::renderer_for_family`）：
            // 哪个字族可用取决于本机装了什么字体，样式解析期不该下这个判断。
            "font-family" => {
                if let StyleValue::String(s) = value {
                    let v = s.trim();
                    if !v.is_empty() && v != "inherit" {
                        ns.font_family = Some(std::sync::Arc::from(v));
                    }
                }
            }
            "white-space" => if let StyleValue::String(s) = value {
                ns.white_space = match s.as_str() {
                    "nowrap" => WhiteSpace::NoWrap,
                    "pre" => WhiteSpace::Pre,
                    "pre-wrap" => WhiteSpace::PreWrap,
                    "pre-line" => WhiteSpace::PreLine,
                    _ => WhiteSpace::Normal,
                };
            }
            "text-overflow" => if let StyleValue::String(s) = value {
                ns.text_overflow = match s.as_str() {
                    "ellipsis" => TextOverflow::Ellipsis,
                    _ => TextOverflow::Clip,
                };
            }
            "overflow" | "overflow-x" | "overflow-y" => if let StyleValue::String(s) = value {
                ns.overflow = match s.as_str() {
                    "hidden" => Overflow::Hidden,
                    "scroll" => Overflow::Scroll,
                    "auto" => Overflow::Auto,
                    _ => Overflow::Visible,
                };
                let taffy_overflow = match s.as_str() {
                    "hidden" | "scroll" | "auto" => taffy::style::Overflow::Hidden,
                    _ => taffy::style::Overflow::Visible,
                };
                ts.overflow.x = taffy_overflow;
                ts.overflow.y = taffy_overflow;
            }
            "vertical-align" => if let StyleValue::String(s) = value {
                ns.vertical_align = match s.as_str() {
                    "top" => VerticalAlign::Top,
                    "middle" => VerticalAlign::Middle,
                    "bottom" => VerticalAlign::Bottom,
                    _ => VerticalAlign::Baseline,
                };
            }
            "word-break" => if let StyleValue::String(s) = value {
                ns.word_break = match s.as_str() {
                    "break-all" => WordBreak::BreakAll,
                    "keep-all" => WordBreak::KeepAll,
                    "break-word" => WordBreak::BreakWord,
                    _ => WordBreak::Normal,
                };
            }
            "z-index" => if let Some(n) = unitless_number(value) { ns.z_index = n as i32; }
            "opacity" => if let Some(n) = unitless_number(value) { ns.opacity = n.clamp(0.0, 1.0); }
            "box-shadow" => if let StyleValue::String(s) = value {
                if let Some(shadow) = parse_box_shadow(s, ctx.screen_width) {
                    ns.box_shadow = Some(shadow);
                }
            }
            "transform" => if let StyleValue::String(s) = value {
                // 用带单位换算的解析：rpx 位移必须按 750 设计宽折算，否则位移量翻倍
                if let Some(transform) = crate::renderer::anim::parse_transform_px(s, ctx.screen_width, sf) {
                    ns.transform = Some(transform);
                } else if let Some(transform) = parse_transform(s) {
                    ns.transform = Some(transform);
                }
            }
            // ── CSS 动画：animation 简写 + 各分项 ──
            // transition 简写：只取时长/延迟/缓动，属性列表忽略（在常态↔按压态之间整体插值）
            "transition" => if let StyleValue::String(s) = value {
                ns.transition = parse_transition_shorthand(s);
            }
            "transition-duration" => if let StyleValue::String(s) = value {
                let mut t = ns.transition.unwrap_or_default();
                t.duration = parse_css_time(s.split(',').next().unwrap_or("0"));
                ns.transition = Some(t);
            }
            "transition-delay" => if let StyleValue::String(s) = value {
                let mut t = ns.transition.unwrap_or_default();
                t.delay = parse_css_time(s.split(',').next().unwrap_or("0"));
                ns.transition = Some(t);
            }
            "transition-timing-function" => if let StyleValue::String(s) = value {
                let mut t = ns.transition.unwrap_or_default();
                t.curve = parse_timing_function(s.split(',').next().unwrap_or("ease"));
                ns.transition = Some(t);
            }
            "animation" => if let StyleValue::String(s) = value {
                ns.animation = crate::renderer::anim::AnimationSpec::parse_shorthand(s);
            }
            "animation-name" => if let StyleValue::String(s) = value {
                let name = s.trim().split(',').next().unwrap_or("").trim().to_string();
                if name.is_empty() || name == "none" {
                    ns.animation = None;
                } else {
                    let mut spec = ns.animation.clone().unwrap_or_default();
                    spec.name = name;
                    ns.animation = Some(spec);
                }
            }
            "animation-duration" | "animation-delay" => {
                let text = match value {
                    StyleValue::String(s) => s.clone(),
                    StyleValue::Length(v, _) => format!("{}s", v),
                    StyleValue::Number(n) => format!("{}s", n),
                    _ => String::new(),
                };
                if let Some(secs) = crate::renderer::anim::parse_time(text.split(',').next().unwrap_or("")) {
                    let mut spec = ns.animation.clone().unwrap_or_default();
                    if name == "animation-duration" { spec.duration = secs; } else { spec.delay = secs; }
                    ns.animation = Some(spec);
                }
            }
            "animation-timing-function" => if let StyleValue::String(s) = value {
                if let Some(timing) = crate::renderer::anim::Timing::parse(s.split(',').next().unwrap_or("").trim()) {
                    let mut spec = ns.animation.clone().unwrap_or_default();
                    spec.timing = timing;
                    ns.animation = Some(spec);
                }
            }
            "animation-iteration-count" => {
                let mut spec = ns.animation.clone().unwrap_or_default();
                spec.iterations = match value {
                    StyleValue::Number(n) => *n,
                    StyleValue::String(s) if s.trim() == "infinite" => f32::INFINITY,
                    StyleValue::String(s) => s.trim().parse().unwrap_or(1.0),
                    _ => 1.0,
                };
                ns.animation = Some(spec);
            }
            "animation-direction" => if let StyleValue::String(s) = value {
                use crate::renderer::anim::Direction;
                let mut spec = ns.animation.clone().unwrap_or_default();
                spec.direction = match s.trim() {
                    "reverse" => Direction::Reverse,
                    "alternate" => Direction::Alternate,
                    "alternate-reverse" => Direction::AlternateReverse,
                    _ => Direction::Normal,
                };
                ns.animation = Some(spec);
            }
            "animation-fill-mode" => if let StyleValue::String(s) = value {
                use crate::renderer::anim::FillMode;
                let mut spec = ns.animation.clone().unwrap_or_default();
                spec.fill = match s.trim() {
                    "forwards" => FillMode::Forwards,
                    "backwards" => FillMode::Backwards,
                    "both" => FillMode::Both,
                    _ => FillMode::None,
                };
                ns.animation = Some(spec);
            }
            "position" => if let StyleValue::String(s) = value {
                match s.as_str() {
                    "absolute" => {
                        ts.position = Position::Absolute;
                        ns.is_positioned = true;
                    }
                    "fixed" => {
                        // fixed 定位：使用 absolute 让 Taffy 处理，但标记为 fixed
                        ts.position = Position::Absolute;
                        ns.is_fixed = true;
                        ns.is_positioned = true;
                    }
                    "relative" => {
                        ts.position = Position::Relative;
                        ns.is_positioned = true;
                    }
                    _ => ts.position = Position::Relative,
                };
            }
            // ── 定位偏移 top/right/bottom/left ──
            //
            // 百分比必须交给布局引擎按「包含块」解析，不能在这里换算成像素：
            // `to_px` 对百分比一律乘屏幕宽度，于是居中老写法
            // `left:50%; top:50%; margin:-44rpx` 里的 `top:50%` 变成 187.5px，
            // 元素被推到父容器下方（视频页封面上的播放按钮就这样被裁掉一半）。
            // `position:fixed` 另算：固定层用 ns.fixed_* 的像素值，
            // 相对视口解析（水平轴用视口宽、垂直轴用视口高）。
            "top" => set_inset(value, ctx, sf, Axis::Vertical, &mut ts.inset.top, &mut ns.fixed_top),
            "left" => set_inset(value, ctx, sf, Axis::Horizontal, &mut ts.inset.left, &mut ns.fixed_left),
            "right" => set_inset(value, ctx, sf, Axis::Horizontal, &mut ts.inset.right, &mut ns.fixed_right),
            "bottom" => set_inset(value, ctx, sf, Axis::Vertical, &mut ts.inset.bottom, &mut ns.fixed_bottom),
            _ => {}
    }
}

// 线性渐变的绘制搬到了 `components::gradient`（那里有轴对齐快路径与一致性测试），
// 这里重新导出以保持 `use super::base::*` 的调用点不变。
