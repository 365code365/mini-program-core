//! 盒子的绘制：背景、渐变、圆角、逐边边框、阴影
//!
//! 由 `base/mod.rs` 组合。**纯搬迁**：从 1925 行的 base.rs 按职责切开，一行代码没改。
use super::*;

/// 绘制盒子阴影
pub fn draw_box_shadow(canvas: &mut Canvas, shadow: &BoxShadow, x: f32, y: f32, w: f32, h: f32, border_radius: f32) {
    if shadow.inset {
        return; // 暂不支持内阴影
    }
    // 模糊掩膜 + 一次混合（实现与理由见 components::shadow）
    crate::renderer::components::shadow::draw_outer_shadow(
        canvas, shadow.color,
        shadow.offset_x, shadow.offset_y, shadow.blur, shadow.spread,
        x, y, w, h, border_radius,
    );
}

/// 获取有效的边框圆角（未按盒子尺寸裁剪）
pub fn get_border_radii(style: &NodeStyle) -> [f32; 4] {
    [
        style.border_radius_tl.unwrap_or(style.border_radius),
        style.border_radius_tr.unwrap_or(style.border_radius),
        style.border_radius_br.unwrap_or(style.border_radius),
        style.border_radius_bl.unwrap_or(style.border_radius),
    ]
}

/// 获取按盒子尺寸裁剪后的圆角（与 CSS 一致：每个角最多为对应边的一半）。
///
/// 关键修复：`border-radius:50%` 在解析期被换算成 `50% * screen_width`（≈187px），
/// 对小元素（如关闭按钮圆圈）会产生远超盒子的半径，圆角路径的控制点飞到盒外，画出
/// 巨大杂散曲线。这里在绘制期按 min(w,h)/2 夹紧，既修复杂散曲线又让 50% 得到正确圆形。
pub fn get_border_radii_clamped(style: &NodeStyle, w: f32, h: f32) -> [f32; 4] {
    let max_r = (w.min(h) / 2.0).max(0.0);
    let clamp = |r: f32| r.max(0.0).min(max_r);
    let [tl, tr, br, bl] = get_border_radii(style);
    [clamp(tl), clamp(tr), clamp(br), clamp(bl)]
}

/// 绘制背景和边框
pub fn draw_background(canvas: &mut Canvas, style: &NodeStyle, x: f32, y: f32, w: f32, h: f32) {
    use crate::renderer::draw_profile::Timer;
    // 绘制阴影（在背景之前）
    if let Some(shadow) = &style.box_shadow {
        let _t = Timer::start("背景:阴影");
        draw_box_shadow(canvas, shadow, x, y, w, h, style.border_radius);
    }
    
    let radii = get_border_radii_clamped(style, w, h);
    let has_different_radii = radii[0] != radii[1] || radii[1] != radii[2] || radii[2] != radii[3];
    let uniform_radius = radii[0];
    
    // 绘制背景：线性渐变优先，否则纯色
    if let Some(grad) = &style.background_gradient {
        let _t = Timer::start("背景:渐变");
        draw_linear_gradient(canvas, grad, x, y, w, h, radii, style.opacity);
    } else if let Some(bg) = style.background_color {
        let _t = Timer::start(if uniform_radius > 0.0 || has_different_radii { "背景:圆角填充" } else { "背景:纯色" });
        let mut paint = Paint::new().with_color(bg).with_style(PaintStyle::Fill);
        if style.opacity < 1.0 { 
            paint.color.a = (paint.color.a as f32 * style.opacity) as u8; 
        }
        
        if has_different_radii {
            let mut path = Path::new();
            add_round_rect_with_radii(&mut path, x, y, w, h, radii);
            canvas.draw_path(&path, &paint);
        } else if uniform_radius > 0.0 {
            // 快速圆角填充（实心内部 + 抗锯齿角），避免整块 4x 扫描线
            canvas.fill_round_rect(x, y, w, h, uniform_radius, paint.color);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &paint);
        }
    }
    
    // 绘制边框（宽度精确 + 抗锯齿的环形填充）
    if style.border_width > 0.0 {
        let _t = Timer::start("背景:边框环");
        if let Some(bc) = style.border_color {
            let fade = |c: Color| if style.opacity < 1.0 {
                Color::new(c.r, c.g, c.b, (c.a as f32 * style.opacity) as u8)
            } else { c };
            // border-top-color 这类「只改某一边颜色」的写法：CSS 里宽度来自 border 简写，
            // 单边宽度为 0，所以不能走 draw_side_borders；必须把环按对角线分成四段着色
            // （加载动画 `border-top-color` 转圈就依赖这个）。
            let sides = [
                style.border_top_color.unwrap_or(bc),
                style.border_right_color.unwrap_or(bc),
                style.border_bottom_color.unwrap_or(bc),
                style.border_left_color.unwrap_or(bc),
            ];
            if sides.iter().any(|c| *c != bc) {
                stroke_round_rect_ring_sides(
                    canvas, x, y, w, h, radii, style.border_width,
                    [fade(sides[0]), fade(sides[1]), fade(sides[2]), fade(sides[3])],
                );
            } else {
                stroke_round_rect_ring(canvas, x, y, w, h, radii, style.border_width, fade(bc));
            }
        }
    }
    
    // 各边独立边框（常用于列表分割线 border-bottom 等），画为轴对齐细矩形
    {
        let _t = Timer::start("背景:单边框");
        draw_side_borders(canvas, style, x, y, w, h);
    }
}

/// 绘制各边独立边框（border-top/right/bottom/left）。
///
/// 自绘型组件（input/textarea 这些自己画盒子的）也要能调它 —— 否则
/// `border-bottom: 2rpx solid …`（表单下划线的标准写法）画不出来。
pub fn draw_side_borders(canvas: &mut Canvas, style: &NodeStyle, x: f32, y: f32, w: f32, h: f32) {
    let fallback = style.border_color.unwrap_or(Color::from_hex(0xE5E5E5));
    let alpha = |c: Color| if style.opacity < 1.0 {
        Color::new(c.r, c.g, c.b, (c.a as f32 * style.opacity) as u8)
    } else { c };
    let mut fill = |rx: f32, ry: f32, rw: f32, rh: f32, c: Color| {
        if rw <= 0.0 || rh <= 0.0 { return; }
        let paint = Paint::new().with_color(alpha(c)).with_style(PaintStyle::Fill);
        canvas.draw_rect(&GeoRect::new(rx, ry, rw, rh), &paint);
    };
    if style.border_top_width > 0.0 {
        fill(x, y, w, style.border_top_width, style.border_top_color.unwrap_or(fallback));
    }
    if style.border_bottom_width > 0.0 {
        fill(x, y + h - style.border_bottom_width, w, style.border_bottom_width, style.border_bottom_color.unwrap_or(fallback));
    }
    if style.border_left_width > 0.0 {
        fill(x, y, style.border_left_width, h, style.border_left_color.unwrap_or(fallback));
    }
    if style.border_right_width > 0.0 {
        fill(x + w - style.border_right_width, y, style.border_right_width, h, style.border_right_color.unwrap_or(fallback));
    }
}

/// 取「无单位数值」：解析器对 `opacity:.5` 这类值历史上会给出 `Length(_, Px)`，
/// 两种形态都接受，避免声明被静默忽略。
pub fn unitless_number(value: &StyleValue) -> Option<f32> {
    match value {
        StyleValue::Number(n) => Some(*n),
        StyleValue::Length(v, _) => Some(*v),
        StyleValue::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// 圆角矩形的有符号距离（负数在内部），按象限取对应圆角半径。
fn round_rect_sdf(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, radii: [f32; 4]) -> f32 {
    let (hw, hh) = (w / 2.0, h / 2.0);
    let (cx, cy) = (x + hw, y + hh);
    let (dx, dy) = (px - cx, py - cy);
    // radii 顺序：左上、右上、右下、左下
    let r = match (dx >= 0.0, dy >= 0.0) {
        (false, false) => radii[0],
        (true, false) => radii[1],
        (true, true) => radii[2],
        (false, true) => radii[3],
    };
    let r = r.min(hw).min(hh).max(0.0);
    let qx = dx.abs() - (hw - r);
    let qy = dy.abs() - (hh - r);
    let outside = (qx.max(0.0) * qx.max(0.0) + qy.max(0.0) * qy.max(0.0)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

/// 逐边着色的边框环：按盒子对角线把环分成上/右/下/左四段，各段用各自颜色。
///
/// 与浏览器一致的分界方式（对角线斜接）。用有符号距离场做 1px 抗锯齿带，
/// 因此对 `border-radius:50%` 的圆环同样正确。
pub fn stroke_round_rect_ring_sides(
    canvas: &mut Canvas,
    x: f32, y: f32, w: f32, h: f32,
    radii: [f32; 4],
    width: f32,
    colors: [Color; 4],
) {
    if width <= 0.0 || w <= 0.0 || h <= 0.0 { return; }
    let bw = width.min(w / 2.0).min(h / 2.0);
    let inner_radii = [
        (radii[0] - bw).max(0.0),
        (radii[1] - bw).max(0.0),
        (radii[2] - bw).max(0.0),
        (radii[3] - bw).max(0.0),
    ];
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + w).ceil() as i32;
    let y1 = (y + h).ceil() as i32;

    for py in y0..y1 {
        for px in x0..x1 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let d_out = round_rect_sdf(fx, fy, x, y, w, h, radii);
            let d_in = round_rect_sdf(fx, fy, x + bw, y + bw, w - bw * 2.0, h - bw * 2.0, inner_radii);
            // 在外轮廓内 且 在内轮廓外
            let cov_out = (0.5 - d_out).clamp(0.0, 1.0);
            let cov_in = (0.5 - d_in).clamp(0.0, 1.0);
            let coverage = cov_out * (1.0 - cov_in);
            if coverage <= 0.002 { continue; }

            // 归一化方向决定归属哪一边（对角线分界）
            let ndx = (fx - cx) / (w / 2.0).max(0.001);
            let ndy = (fy - cy) / (h / 2.0).max(0.001);
            let color = if ndy.abs() >= ndx.abs() {
                if ndy < 0.0 { colors[0] } else { colors[2] }
            } else if ndx > 0.0 {
                colors[1]
            } else {
                colors[3]
            };
            let alpha = (color.a as f32 * coverage).round().clamp(0.0, 255.0) as u8;
            if alpha > 0 {
                canvas.set_pixel(px, py, Color::new(color.r, color.g, color.b, alpha));
            }
        }
    }
}

/// 以指定宽度绘制（可带圆角的）边框环，抗锯齿。
///
/// 之前边框用 `PaintStyle::Stroke` 走 `draw_line`（Wu 1px 线），既忽略 `border-width`
/// 又在圆角处产生锯齿。这里改为「外圆角矩形 - 内圆角矩形」组成的环形，用扫描线
/// even-odd 填充（自带 4x 超采样抗锯齿），边框宽度精确、边缘平滑。
pub fn stroke_round_rect_ring(
    canvas: &mut Canvas,
    x: f32, y: f32, w: f32, h: f32,
    radii: [f32; 4],
    width: f32,
    color: Color,
) {
    if width <= 0.0 || w <= 0.0 || h <= 0.0 { return; }
    let bw = width.min(w / 2.0).min(h / 2.0);
    let [tl, tr, br, bl] = radii;

    let mut path = Path::new();
    // 外圈
    add_round_rect_with_radii(&mut path, x, y, w, h, radii);
    // 内圈（内缩 border-width，圆角相应减小），形成挖空的环
    let iw = (w - 2.0 * bw).max(0.0);
    let ih = (h - 2.0 * bw).max(0.0);
    let ir = |r: f32| (r - bw).max(0.0);
    add_round_rect_with_radii(&mut path, x + bw, y + bw, iw, ih, [ir(tl), ir(tr), ir(br), ir(bl)]);

    let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
    canvas.draw_path(&path, &paint);
}

/// 添加带有不同圆角的圆角矩形路径。
///
/// 圆角用三次贝塞尔逼近四分之一圆（控制点系数 0.5523），而不是「控制点落在角点」的
/// 单段二次贝塞尔——后者明显比真正的圆弧更方，`border-radius:50%` 会画成方角化的
/// 「squircle」而非圆形。
fn add_round_rect_with_radii(path: &mut Path, x: f32, y: f32, w: f32, h: f32, radii: [f32; 4]) {
    // 四分之一圆的三次贝塞尔控制点比例
    const K: f32 = 0.552_284_75;
    let max_r = (w.min(h) / 2.0).max(0.0);
    let clamp = |r: f32| r.max(0.0).min(max_r);
    let [tl, tr, br, bl] = [clamp(radii[0]), clamp(radii[1]), clamp(radii[2]), clamp(radii[3])];

    // 从左上角圆弧终点开始，顺时针
    path.move_to(x + tl, y);

    // 上边 → 右上角
    path.line_to(x + w - tr, y);
    if tr > 0.0 {
        path.cubic_to(
            x + w - tr + tr * K, y,
            x + w, y + tr - tr * K,
            x + w, y + tr,
        );
    }

    // 右边 → 右下角
    path.line_to(x + w, y + h - br);
    if br > 0.0 {
        path.cubic_to(
            x + w, y + h - br + br * K,
            x + w - br + br * K, y + h,
            x + w - br, y + h,
        );
    }

    // 下边 → 左下角
    path.line_to(x + bl, y + h);
    if bl > 0.0 {
        path.cubic_to(
            x + bl - bl * K, y + h,
            x, y + h - bl + bl * K,
            x, y + h - bl,
        );
    }

    // 左边 → 左上角
    path.line_to(x, y + tl);
    if tl > 0.0 {
        path.cubic_to(
            x, y + tl - tl * K,
            x + tl - tl * K, y,
            x + tl, y,
        );
    }

    path.close();
}

/// 把**替换元素**（image / input / textarea / canvas / video）的最小尺寸钉在它的确定尺寸上。
///
/// CSS 里替换元素有**固有尺寸**，所以 `min-width:auto` / `min-height:auto` 解析成
/// 「指定尺寸与固有尺寸中较小的那个」，而不是 0。少了这一步，它们在一行里会被兄弟
/// 一路压没：AI 选茶推荐页的商品卡一行放 `[图 99px | flex:1 中间栏 | 按钮 88px]`，
/// 中间栏内含长标题时把商品图压成 **0**（整张图消失，实测 `🖼 ▢ 占位 0x236`），
/// 按钮也被挤出卡片；表单页里的 textarea 同样会被压矮一截。
///
/// **不能对非替换元素这么做。** 空的 `<view>` 内容尺寸就是 0，本来就该可压 ——
/// 音乐播放器那个 4px 高的进度条里塞了个 13px 的圆把手，靠的正是「可压」把它压回
/// 轨道高度。一刀切给所有定尺寸叶子加最小尺寸，把手就会撑成整圆顶出轨道
/// （实测 gallery 从 1 张变化扩散到 14 张）。所以这条规则按**元素类型**给。
pub fn pin_replaced_min_size(ts: &mut Style) {
    if dim_is_auto(ts.min_size.width) && dim_length(ts.size.width).is_some() {
        ts.min_size.width = ts.size.width;
    }
    if dim_is_auto(ts.min_size.height) && dim_length(ts.size.height).is_some() {
        ts.min_size.height = ts.size.height;
    }
}
