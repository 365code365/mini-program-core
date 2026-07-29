//! `<icon>` 内置图标的矢量字形表 —— 对齐微信。
//!
//! ## 数据来源
//! 全部取自腾讯官方开源的 WeUI 图标（MIT License），微信自己的 `<icon>` 组件用的
//! 就是这套图形：
//! - 核心类型（success / info / warn / waiting / cancel / clear / download /
//!   search / circle）来自 [WeUI](https://github.com/Tencent/weui)
//!   `src/style/icon/weui-icon.less` 里的 `mask-image` 矢量数据；
//! - 扩展类型（close / back / arrow / plus / star / heart / chat）来自
//!   [weui-icon v1.0.2](https://github.com/weui/weui-icon) 的 filled / outlined 两套。
//!
//! ## 两个关键结论（决定了画法）
//! 1. **WeUI 图标是单色蒙版**：CSS 是 `background-color: currentColor` + `mask-image`。
//!    所以 `success` 并不是「绿色实心圆 + 白色对勾」，而是**一整块绿色里挖出一个
//!    对勾形状的洞**。区别在把图标放到有色背景上时非常明显：微信是透出背景色，
//!    我们以前画的白色对勾会糊成一块白。引擎的 `fill_path` 本来就是 even-odd
//!    （扫描线交点按对填充），子路径反向绕出来的洞可以原样还原。
//! 2. **字形没有占满整个盒子**：24 视图盒里圆的直径是 20（`weui-icon-circle`
//!    的 1000 视图盒里直径 833.33，同一个 0.8333 比例），也就是字面本身留了边。
//!    所以 `<icon size="23">` 的圆直径约 19.2px，不是 23px。渲染时按
//!    `preserveAspectRatio="xMidYMid meet"`（等比缩放取小 + 居中）把视图盒放进
//!    组件盒，非正方形字形（箭头是 12×24）也就自然居中不变形。
//!
//! 这张表同时供三处使用，保证「原生渲染 / HTML 导出 / HTML 运行时」三边一致：
//! 原生 `IconComponent::draw`、编译期 `icon_svg()`、运行时 `__WXICON` 表。

/// 一个图标字形：视图盒尺寸 + 旋转角 + SVG 路径数据。
///
/// `rot` 只取 0/90/180/270，用来复用同一个箭头字形做上下左右四个朝向
/// （WeUI 只提供了「左」和「右」）。
#[derive(Debug, Clone, Copy)]
pub struct IconGlyph {
    pub vw: f32,
    pub vh: f32,
    pub rot: f32,
    pub d: &'static str,
}

impl IconGlyph {
    /// 旋转 90/270 度后视图盒的宽高要互换。
    pub fn view_size(&self) -> (f32, f32) {
        if self.rot == 90.0 || self.rot == 270.0 {
            (self.vh, self.vw)
        } else {
            (self.vw, self.vh)
        }
    }
}

const fn g(vw: f32, vh: f32, d: &'static str) -> IconGlyph {
    IconGlyph { vw, vh, rot: 0.0, d }
}

const fn gr(vw: f32, vh: f32, rot: f32, d: &'static str) -> IconGlyph {
    IconGlyph { vw, vh, rot, d }
}

// ───────────────────────── WeUI 核心图标（Tencent/weui, MIT）─────────────────────────

/// 实心圆 + 对勾形的洞
const SUCCESS: IconGlyph = g(24.0, 24.0, "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10zm-1.177-7.86l-2.765-2.767L7 12.431l3.119 3.121a1 1 0 001.414 0l5.952-5.95-1.062-1.062-5.6 5.6z");
/// 只有对勾（圆角收笔，微信 `success_no_circle`）
const SUCCESS_NO_CIRCLE: IconGlyph = g(24.0, 24.0, "M8.657 18.435L3 12.778l1.414-1.414 4.95 4.95L20.678 5l1.414 1.414-12.02 12.021a1 1 0 01-1.415 0z");
/// 圆环 + 对勾
const SUCCESS_CIRCLE: IconGlyph = g(24.0, 24.0, "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10zm0-1.2a8.8 8.8 0 100-17.6 8.8 8.8 0 000 17.6zm-1.172-6.242l5.809-5.808.848.849-5.95 5.95a1 1 0 01-1.414 0L7 12.426l.849-.849 2.98 2.98z");
/// 实心圆 + 「i」的洞
const INFO: IconGlyph = g(24.0, 24.0, "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10zm-.75-12v7h1.5v-7h-1.5zM12 9a1 1 0 100-2 1 1 0 000 2z");
/// 圆环 + 「i」
const INFO_CIRCLE: IconGlyph = g(24.0, 24.0, "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10zm0-1.2a8.8 8.8 0 100-17.6 8.8 8.8 0 000 17.6zM11.4 10h1.2v7h-1.2v-7zm.6-1a1 1 0 110-2 1 1 0 010 2z");
/// 实心圆 + 「!」的洞
const WARN: IconGlyph = g(24.0, 24.0, "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10zm-.763-15.864l.11 7.596h1.305l.11-7.596h-1.525zm.759 10.967c.512 0 .902-.383.902-.882 0-.5-.39-.882-.902-.882a.878.878 0 00-.896.882c0 .499.396.882.896.882z");
/// 实心圆 + 时针分针的洞
const WAITING: IconGlyph = g(24.0, 24.0, "M12.75 11.38V6h-1.5v6l4.243 4.243 1.06-1.06-3.803-3.804zM12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10z");
/// 圆环 + 时针分针（也用作独立的 clock）
const WAITING_CIRCLE: IconGlyph = g(24.0, 24.0, "M12.6 11.503l3.891 3.891-.848.849L11.4 12V6h1.2v5.503zM12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10zm0-1.2a8.8 8.8 0 100-17.6 8.8 8.8 0 000 17.6z");
/// 圆环 + 叉（微信 `cancel` 是描边圆，不是实心圆）
const CANCEL: IconGlyph = g(24.0, 24.0, "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10zm0-1.2a8.8 8.8 0 100-17.6 8.8 8.8 0 000 17.6zM12.849 12l3.11 3.111-.848.849L12 12.849l-3.111 3.11-.849-.848L11.151 12l-3.11-3.111.848-.849L12 11.151l3.111-3.11.849.848L12.849 12z");
/// 实心圆 + 叉的洞（微信 `clear`，输入框右侧的清除按钮）
const CLEAR: IconGlyph = g(24.0, 24.0, "M13.06 12l3.006-3.005-1.06-1.06L12 10.938 8.995 7.934l-1.06 1.06L10.938 12l-3.005 3.005 1.06 1.06L12 13.062l3.005 3.005 1.06-1.06L13.062 12zM12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10z");
const DOWNLOAD: IconGlyph = g(24.0, 24.0, "M11.25 12.04l-1.72-1.72-1.06 1.06 2.828 2.83a1 1 0 001.414-.001l2.828-2.828-1.06-1.061-1.73 1.73V7h-1.5v5.04zm0-5.04V2h1.5v5h6.251c.55 0 .999.446.999.996v13.008a.998.998 0 01-.996.996H4.996A.998.998 0 014 21.004V7.996A1 1 0 014.999 7h6.251z");
const SEARCH: IconGlyph = g(24.0, 24.0, "M16.31 15.561l4.114 4.115-.848.848-4.123-4.123a7 7 0 11.857-.84zM16.8 11a5.8 5.8 0 10-11.6 0 5.8 5.8 0 0011.6 0z");
/// 空心圆（原始数据是 1000 视图盒，比例与 24 盒一致）
const CIRCLE: IconGlyph = g(1000.0, 1000.0, "M500 916.667C269.881 916.667 83.333 730.119 83.333 500 83.333 269.881 269.881 83.333 500 83.333c230.119 0 416.667 186.548 416.667 416.667 0 230.119-186.548 416.667-416.667 416.667zm0-50c202.504 0 366.667-164.163 366.667-366.667 0-202.504-164.163-366.667-366.667-366.667-202.504 0-366.667 164.163-366.667 366.667 0 202.504 164.163 366.667 366.667 366.667z");

// ─────────────── weui-icon v1.0.2 扩展图标（weui/weui-icon, MIT）───────────────

const CLOSE: IconGlyph = g(24.0, 24.0, "M12 10.586l5.657-5.657 1.414 1.414L13.414 12l5.657 5.657-1.414 1.414L12 13.414l-5.657 5.657-1.414-1.414L10.586 12 4.929 6.343 6.343 4.93z");
/// 左尖角（12×24，微信返回按钮的形状）
const BACK: IconGlyph = g(12.0, 24.0, "M3.343 12l7.071 7.071L9 20.485l-7.778-7.778a1 1 0 010-1.414L9 3.515l1.414 1.414L3.344 12z");
/// 右尖角（12×24，列表右侧箭头）
const ARROW: IconGlyph = g(12.0, 24.0, "M10.157 12.711L4.5 18.368l-1.414-1.414 4.95-4.95-4.95-4.95L4.5 5.64l5.657 5.657a1 1 0 010 1.414z");
/// 上/下尖角：复用右尖角旋转（WeUI 只给了左右两个方向）
const ARROW_UP: IconGlyph = gr(12.0, 24.0, 270.0, ARROW.d);
const ARROW_DOWN: IconGlyph = gr(12.0, 24.0, 90.0, ARROW.d);
const PLUS: IconGlyph = g(24.0, 24.0, "M11 11V4h2v7h7v2h-7v7h-2v-7H4v-2h7z");
/// 减号：按 `add` 的横杠几何（4..20，粗 2）配一根，保证加减成对时视觉一致
const MINUS: IconGlyph = g(24.0, 24.0, "M4 11h16v2H4z");
const STAR: IconGlyph = g(24.0, 24.0, "M12 18.5l-4.672 2.456a1 1 0 01-1.451-1.054l.892-5.202-3.78-3.685a1 1 0 01.555-1.706l5.223-.759 2.336-4.733a1 1 0 011.794 0l2.336 4.733 5.223.76a1 1 0 01.555 1.705L17.23 14.7l.892 5.202a1 1 0 01-1.45 1.054L12 18.5z");
const STAR_O: IconGlyph = g(24.0, 24.0, "M15.941 14.28l3.942-3.841-5.447-.792L12 4.711 9.564 9.647l-5.447.792L8.06 14.28l-.93 5.425L12 17.144l4.872 2.562-.93-5.425zM12 18.5l-4.672 2.456a1 1 0 01-1.451-1.054l.892-5.202-3.78-3.685a1 1 0 01.555-1.706l5.223-.759 2.336-4.733a1 1 0 011.794 0l2.336 4.733 5.223.76a1 1 0 01.555 1.705L17.23 14.7l.892 5.202a1 1 0 01-1.45 1.054L12 18.5z");
const HEART: IconGlyph = g(24.0, 24.0, "M4.536 5.778a5 5 0 017.07 0c.183.183.42.41.708.682.288-.272.524-.499.707-.682a5 5 0 017.125 7.016L13.02 19.92a1 1 0 01-1.414 0L4.48 12.795a5 5 0 01.055-7.017z");
const HEART_O: IconGlyph = g(24.0, 24.0, "M19.285 12.645a3.8 3.8 0 00-5.416-5.332c-.192.192-.436.427-.732.707l-.823.775-.823-.775c-.297-.28-.54-.515-.733-.707a3.8 3.8 0 00-5.374 0c-1.468 1.469-1.485 3.844-.054 5.32l6.984 6.984 6.97-6.972zm-14.75-6.18a5 5 0 017.072 0c.182.182.418.41.707.682.288-.272.524-.5.707-.683a5 5 0 017.125 7.017l-7.125 7.126a1 1 0 01-1.414 0L4.48 13.48a5 5 0 01.055-7.017z");
const CHAT: IconGlyph = g(24.0, 24.0, "M11 19l-2.293 2.293A1 1 0 017 20.586V19H3.5A1.5 1.5 0 012 17.5v-12A1.5 1.5 0 013.5 4h17A1.5 1.5 0 0122 5.5v12a1.5 1.5 0 01-1.5 1.5H11z");
const CHAT_O: IconGlyph = g(24.0, 24.0, "M10.503 17.8H20.5a.3.3 0 00.3-.3v-12a.3.3 0 00-.3-.3h-17a.3.3 0 00-.3.3v12a.3.3 0 00.3.3h4.7v2.303l2.303-2.303zM11 19l-2.293 2.293A1 1 0 017 20.586V19H3.5A1.5 1.5 0 012 17.5v-12A1.5 1.5 0 013.5 4h17A1.5 1.5 0 0122 5.5v12a1.5 1.5 0 01-1.5 1.5H11z");

// ─────────── 引擎扩展：无圆底的 i / ! 标记（微信没有这两个类型）───────────
//
// 直接拿 WeUI 圆里那个「洞」当独立图标会细到看不见（24 盒里竖条只有 1.5 宽），
// 所以按同样的方形收笔风格重画到填满字面，比例参照 WeUI 的 i/!（竖条 ≈ 盒宽的
// 17%、圆点直径 ≈ 盒宽的 20%）。

const INFO_NO_CIRCLE: IconGlyph = g(24.0, 24.0, "M9.9 9.2h4.2V21H9.9zM12 7.3a2.4 2.4 0 100-4.8 2.4 2.4 0 000 4.8z");
const WARN_NO_CIRCLE: IconGlyph = g(24.0, 24.0, "M9.9 3h4.2v11.8H9.9zM12 21.6a2.4 2.4 0 100-4.8 2.4 2.4 0 000 4.8z");

/// 类型名 → 字形。别名在这里一次拍平，三处调用方都不用再各自认别名。
pub fn icon_glyph(icon_type: &str) -> Option<IconGlyph> {
    let g = match icon_type {
        "success" => SUCCESS,
        "success_no_circle" | "success-no-circle" => SUCCESS_NO_CIRCLE,
        "success_circle" | "success-circle" => SUCCESS_CIRCLE,
        "info" => INFO,
        "info_circle" | "info-circle" => INFO_CIRCLE,
        "warn" => WARN,
        "waiting" => WAITING,
        "waiting_circle" | "waiting-circle" | "waiting_no_circle" | "clock" | "time" => WAITING_CIRCLE,
        "cancel" => CANCEL,
        "clear" => CLEAR,
        "download" => DOWNLOAD,
        "search" => SEARCH,
        "circle" => CIRCLE,
        "close" | "cancel_no_circle" | "cancel-no-circle" => CLOSE,
        "back" | "arrow_left" | "arrow-left" => BACK,
        "arrow" | "arrow_right" | "arrow-right" => ARROW,
        "arrow_up" | "arrow-up" => ARROW_UP,
        "arrow_down" | "arrow-down" => ARROW_DOWN,
        "plus" | "add" => PLUS,
        "minus" => MINUS,
        "star" => STAR,
        "star-o" | "star_o" => STAR_O,
        "heart" | "like" => HEART,
        "heart-o" | "heart_o" => HEART_O,
        "chat" | "comment" => CHAT,
        "chat-o" | "chat_o" => CHAT_O,
        "info_no_circle" | "info-no-circle" => INFO_NO_CIRCLE,
        "warn_no_circle" | "warn-no-circle" => WARN_NO_CIRCLE,
        // 微信对未知 type 什么都不画，这里保持一致（以前会默默画成绿色对勾，
        // 结果是数据里 icon 字段拼错时看不出来）
        _ => return None,
    };
    Some(g)
}

/// 所有已支持的类型名（含别名），用于生成 HTML 运行时那张表。
pub const ICON_TYPES: &[&str] = &[
    "success", "success_no_circle", "success-no-circle", "success_circle", "success-circle",
    "info", "info_circle", "info-circle", "warn", "waiting", "waiting_circle", "waiting-circle",
    "waiting_no_circle", "clock", "time", "cancel", "clear", "download", "search", "circle",
    "close", "cancel_no_circle", "cancel-no-circle", "back", "arrow_left", "arrow-left",
    "arrow", "arrow_right", "arrow-right", "arrow_up", "arrow-up", "arrow_down", "arrow-down",
    "plus", "add", "minus", "star", "star-o", "star_o", "heart", "like", "heart-o", "heart_o",
    "chat", "comment", "chat-o", "chat_o", "info_no_circle", "info-no-circle",
    "warn_no_circle", "warn-no-circle",
];

/// 微信各类型的默认颜色（`color` 属性和 CSS `color` 都没给时用）。
pub fn icon_default_color(icon_type: &str) -> u32 {
    match icon_type {
        "success" | "success_no_circle" | "success-no-circle" | "success_circle"
        | "success-circle" | "download" => 0x09BB07,
        "info" | "info_circle" | "info-circle" | "info_no_circle" | "info-no-circle"
        | "waiting" | "waiting_circle" | "waiting-circle" | "waiting_no_circle" | "clock"
        | "time" => 0x10AEFF,
        "warn" | "warn_no_circle" | "warn-no-circle" => 0xF76260,
        "cancel" | "clear" => 0xF43530,
        "search" => 0xB2B2B2,
        _ => 0xC8C8CD,
    }
}

/// 生成这个类型对应的内联 SVG（HTML 导出用）。未知类型返回空串。
///
/// 旋转用「先转再平移回第一象限」表达，和原生侧 `IconGlyph::view_size()`
/// 的宽高互换是同一套换算：转 90° 时 `(x,y) -> (vh - y, x)`。
pub fn icon_svg_markup(icon_type: &str) -> String {
    let Some(g) = icon_glyph(icon_type) else { return String::new() };
    let (vw, vh) = g.view_size();
    let inner = match g.rot as i32 {
        90 => format!(
            "<g transform=\"translate({} 0) rotate(90)\"><path d=\"{}\"/></g>",
            g.vh, g.d
        ),
        180 => format!(
            "<g transform=\"translate({} {}) rotate(180)\"><path d=\"{}\"/></g>",
            g.vw, g.vh, g.d
        ),
        270 => format!(
            "<g transform=\"translate(0 {}) rotate(-90)\"><path d=\"{}\"/></g>",
            g.vw, g.d
        ),
        _ => format!("<path d=\"{}\"/>", g.d),
    };
    format!(
        "<svg viewBox=\"0 0 {vw} {vh}\" fill=\"currentColor\" fill-rule=\"evenodd\" \
         preserveAspectRatio=\"xMidYMid meet\" aria-hidden=\"true\">{inner}</svg>"
    )
}

/// 生成 HTML 运行时用的类型 → SVG 映射表。
///
/// 直接把编译期算好的字符串塞给运行时，运行时就不用再抄一遍分支逻辑
/// （以前编译期和运行时两份 `iconSvg` 是手工同步的，改一处漏一处）。
pub fn icon_svg_js_table() -> String {
    let mut s = String::from("/* 内置图标：数据源 WeUI (Tencent, MIT) */\nvar __WXICON = {");
    for (i, t) in ICON_TYPES.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("\n  \"{}\": '{}'", t, icon_svg_markup(t)));
    }
    s.push_str("\n};\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::components::svg_path::{append_svg_path, Xf};
    use crate::Path;

    fn glyph_bbox(t: &str) -> (f32, f32, f32, f32) {
        let g = icon_glyph(t).expect(t);
        let mut p = Path::new();
        append_svg_path(g.d, &Xf::IDENTITY, &mut p);
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for c in p.flatten(0.1) {
            for pt in c {
                x0 = x0.min(pt.x);
                y0 = y0.min(pt.y);
                x1 = x1.max(pt.x);
                y1 = y1.max(pt.y);
            }
        }
        (x0, y0, x1, y1)
    }

    #[test]
    fn every_type_has_parseable_glyph() {
        for t in ICON_TYPES {
            let g = icon_glyph(t).unwrap_or_else(|| panic!("{t} 缺字形"));
            let mut p = Path::new();
            append_svg_path(g.d, &Xf::IDENTITY, &mut p);
            let contours = p.flatten(0.2);
            assert!(!contours.is_empty(), "{t} 解析不出轮廓");
            // 字形必须落在视图盒内（留 0.2 容差给曲线展平）
            for c in &contours {
                for pt in c {
                    assert!(
                        pt.x >= -0.2 && pt.x <= g.vw + 0.2 && pt.y >= -0.2 && pt.y <= g.vh + 0.2,
                        "{t} 的点 ({}, {}) 跑出了 {}x{} 视图盒",
                        pt.x, pt.y, g.vw, g.vh
                    );
                }
            }
        }
    }

    #[test]
    fn unknown_type_draws_nothing() {
        assert!(icon_glyph("").is_none());
        assert!(icon_glyph("nope").is_none());
        assert!(icon_glyph("loading").is_none());
    }

    #[test]
    fn circle_glyphs_share_the_weui_diameter_ratio() {
        // WeUI 的圆直径 = 视图盒的 5/6（24 盒里 20，1000 盒里 833.33）
        for (t, view) in [("success", 24.0f32), ("info", 24.0), ("warn", 24.0), ("circle", 1000.0)] {
            let (x0, y0, x1, y1) = glyph_bbox(t);
            let d = ((x1 - x0) + (y1 - y0)) / 2.0;
            let ratio = d / view;
            assert!(
                (ratio - 5.0 / 6.0).abs() < 0.01,
                "{t} 直径占比 {ratio:.4}，应为 0.8333"
            );
            // 圆必须在视图盒里居中
            assert!(((x0 + x1) / 2.0 - view / 2.0).abs() < 0.05, "{t} 水平不居中");
            assert!(((y0 + y1) / 2.0 - view / 2.0).abs() < 0.05, "{t} 垂直不居中");
        }
    }

    #[test]
    fn success_has_a_check_shaped_hole() {
        // 两条子路径：外圆 + 对勾。对勾是洞，不是白色描边。
        let g = icon_glyph("success").unwrap();
        let mut p = Path::new();
        append_svg_path(g.d, &Xf::IDENTITY, &mut p);
        assert_eq!(p.flatten(0.2).len(), 2, "success 应为「圆 + 对勾洞」两条子路径");
    }

    #[test]
    fn cancel_is_a_ring_not_a_disc() {
        // 微信的 cancel 是描边圆 + 叉：外圆、内圆、叉 = 三条子路径
        let g = icon_glyph("cancel").unwrap();
        let mut p = Path::new();
        append_svg_path(g.d, &Xf::IDENTITY, &mut p);
        assert_eq!(p.flatten(0.2).len(), 3, "cancel 应为「外圆 + 内圆 + 叉」");
    }

    #[test]
    fn arrow_directions_rotate_the_same_glyph() {
        let right = icon_glyph("arrow_right").unwrap();
        let up = icon_glyph("arrow_up").unwrap();
        let down = icon_glyph("arrow_down").unwrap();
        assert_eq!(right.d, up.d);
        assert_eq!(right.d, down.d);
        assert_eq!(right.view_size(), (12.0, 24.0));
        // 上下箭头是横着的，视图盒宽高互换
        assert_eq!(up.view_size(), (24.0, 12.0));
        assert_eq!(down.view_size(), (24.0, 12.0));
    }

    #[test]
    fn svg_markup_covers_every_type_and_quotes_safely() {
        for t in ICON_TYPES {
            let m = icon_svg_markup(t);
            assert!(m.starts_with("<svg "), "{t}: {m}");
            assert!(m.contains("evenodd"), "{t} 缺 even-odd 填充规则");
            // 表是用单引号包起来塞进 JS 的，字形里不能出现单引号
            assert!(!m.contains('\''), "{t} 的标记里有单引号，会破坏 JS 表");
        }
        assert_eq!(icon_svg_markup("nope"), "");
    }

    #[test]
    fn js_table_lists_all_types() {
        let js = icon_svg_js_table();
        for t in ICON_TYPES {
            assert!(js.contains(&format!("\"{t}\":")), "JS 表缺 {t}");
        }
    }
}
