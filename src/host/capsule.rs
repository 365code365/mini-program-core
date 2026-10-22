//! 右上角的胶囊按钮（微信的「•••」菜单 + 「⊙」关闭）：位置、画法、命中哪一半。
//!
//! 微信里胶囊是**原生外壳**的一部分，压在页面最上面。引擎只在 `navigationStyle: custom`
//! 的页面上画它（宿主打开 `menuButton` 选项时，见 `engine_capsule.rs`）：这种页面宿主不画导航栏，
//! 引擎画面顶到状态栏下面，胶囊压在页面自己画的导航栏上 —— 开发者正是拿
//! `wx.getMenuButtonBoundingClientRect` 去对齐它，所以**画的位置与那个接口返回的必须是同一个矩形**
//! （[`rect`] 与 `prelude/app-info.js` 是同一条公式，e2e `menu_button_e2e` 守着）。
//! 普通页面的导航栏是宿主的原生控件，胶囊由宿主画在导航栏里（Flutter 插件的 `MiniCapsule`，
//! 几何与配色与这里一致）。桌面窗体（`app_window/capsule_host.rs`）调同一个 [`draw`]；
//! H5 导出用 SVG 按同一组数写死（`compiler/html/capsule.rs`）。
//!
//! 画法：直接画进 fixed 覆盖层的画布（`render_fixed_layer` 末尾）。这样它压在页面与
//! `position:fixed` 元素之上、Toast / Modal / picker 之下，CPU 与 GPU 两条合成路径都不用改
//! （GPU 镜像 fixed 层）。代价是按压态变了要重画 fixed 层 —— 只在按下 / 移出 / 抬起时各一次。
//!
//! 尺寸与配色按微信 iOS：87×32、距右 7、距状态栏底 4；`navigationBarTextStyle: white`
//! （深色导航栏）用半透明黑底白图标，否则半透明白底黑图标。图标几何按真机截图量的（逻辑像素）。
use crate::{Canvas, Color, Rect};
use serde_json::Value;

/// 默认尺寸与边距（与 `prelude/app-info.js` 的 `getMenuButtonBoundingClientRect` 同一组数）
pub const WIDTH: f32 = 87.0;
pub const HEIGHT: f32 = 32.0;
pub const RIGHT_GAP: f32 = 7.0;
/// 距状态栏底：iOS 4（44 高的导航栏里不居中，下面留 8）
pub const TOP_GAP: f32 = 4.0;
/// Android 8：48 高的导航栏里上下居中（`navbar::bar_height`）
pub const TOP_GAP_ANDROID: f32 = 8.0;

/// 按平台取距状态栏底的间距
pub fn top_gap(platform: &str) -> f32 {
    if platform.eq_ignore_ascii_case("android") {
        TOP_GAP_ANDROID
    } else {
        TOP_GAP
    }
}

/// 点在了哪一半
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Half {
    /// 左半「•••」：微信在这里弹分享 / 设置菜单，SDK 交给宿主
    Menu,
    /// 右半「⊙」：退出小程序，交给宿主
    Close,
}

impl Half {
    /// `menuButton` 事件的 `action` 字段
    pub fn action(self) -> &'static str {
        match self {
            Half::Menu => "menu",
            Half::Close => "close",
        }
    }
}

/// 胶囊在视口里的位置（逻辑像素）。`info` 是交给逻辑层的那份系统信息（`engine_options::system_info`）：
/// 宿主给了 `menuButton`（真机上量的）就用它，否则与 JS 同一条公式 ——
/// `top = (statusBarHeight || 20) + 4`、`right = windowWidth - 7`
pub fn rect(info: &Value) -> Rect {
    if let Some(mb) = info.get("menuButton").and_then(|v| v.as_object()) {
        let f = |k: &str| mb.get(k).and_then(|v| v.as_f64()).map(|v| v as f32);
        if let (Some(l), Some(t), Some(w), Some(h)) = (f("left"), f("top"), f("width"), f("height")) {
            if w > 0.0 && h > 0.0 {
                return Rect::new(l, t, w, h);
            }
        }
    }
    // JS 的 `||`：0 / 缺省都按 20
    let sb = info
        .get("statusBarHeight")
        .and_then(|v| v.as_f64())
        .filter(|v| *v != 0.0)
        .unwrap_or(20.0) as f32;
    let ww = info.get("windowWidth").and_then(|v| v.as_f64()).unwrap_or(375.0) as f32;
    let right = ww - RIGHT_GAP;
    let platform = info.get("platform").and_then(|v| v.as_str()).unwrap_or("");
    Rect::new(right - WIDTH, sb + top_gap(platform), WIDTH, HEIGHT)
}

/// 命中：落在胶囊里返回哪一半（以竖分隔线为界）
pub fn half_at(r: Rect, x: f32, y: f32) -> Option<Half> {
    if x < r.x || x >= r.right() || y < r.y || y >= r.bottom() {
        return None;
    }
    Some(if x < r.x + r.width / 2.0 { Half::Menu } else { Half::Close })
}

/// 一套配色。`dark` = 深色导航栏（白图标）
struct Palette {
    bg: Color,
    border: Color,
    divider: Color,
    icon: Color,
    press: Color,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: Color::new(0, 0, 0, 38),
            border: Color::new(255, 255, 255, 64),
            divider: Color::new(255, 255, 255, 64),
            icon: Color::new(255, 255, 255, 255),
            press: Color::new(255, 255, 255, 46),
        }
    } else {
        Palette {
            bg: Color::new(255, 255, 255, 153),
            border: Color::new(0, 0, 0, 20),
            divider: Color::new(0, 0, 0, 51),
            icon: Color::new(0, 0, 0, 255),
            press: Color::new(0, 0, 0, 26),
        }
    }
}

// 图标几何（逻辑像素，相对各自那一半的中心）
const DOT_GAP: f32 = 6.2;
const DOT_R: f32 = 1.7;
const DOT_R_MID: f32 = 2.4;
const RING_R_OUT: f32 = 7.4;
const RING_STROKE: f32 = 1.9;
const RING_DOT_R: f32 = 2.7;
/// 竖分隔线占胶囊高度的比例（32 高时 18）
const DIVIDER_FRAC: f32 = 18.0 / 32.0;

/// 画进 `canvas`（物理像素，`dpr` 倍）。`pressed`：手指按着且还在那一半里
pub fn draw(canvas: &mut Canvas, r: Rect, dpr: f32, dark: bool, pressed: Option<Half>) {
    let p = palette(dark);
    let s = dpr.max(0.1);
    let (x0, y0, w, h) = (r.x * s, r.y * s, r.width * s, r.height * s);
    let rad = h / 2.0;
    // 发丝线：0.5 逻辑像素，至少一个物理像素（1 倍屏上 0.5px 的线会淡到看不见）
    let hair = (0.5 * s).max(1.0);
    let bounds = (x0, y0, w, h);
    let pill = |fx: f32, fy: f32| (0.5 - pill_sdf(fx, fy, x0, y0, w, h, rad)).clamp(0.0, 1.0);
    // 底色
    paint(canvas, bounds, p.bg, pill);
    // 按压态：只压暗按着的那一半
    if let Some(half) = pressed {
        let (hx0, hx1) = match half {
            Half::Menu => (x0, x0 + w / 2.0),
            Half::Close => (x0 + w / 2.0, x0 + w),
        };
        paint(canvas, bounds, p.press, |fx, fy| pill(fx, fy) * span(fx, hx0, hx1));
    }
    // 描边：轮廓往里 hair 宽的一圈
    paint(canvas, bounds, p.border, |fx, fy| {
        let d = pill_sdf(fx, fy, x0, y0, w, h, rad);
        ((0.5 - d).clamp(0.0, 1.0) - (0.5 - (d + hair)).clamp(0.0, 1.0)).max(0.0)
    });
    // 竖分隔线
    let (dx, dh) = (x0 + w / 2.0 - hair / 2.0, h * DIVIDER_FRAC);
    let dy = y0 + (h - dh) / 2.0;
    paint(canvas, (dx - 1.0, dy - 1.0, hair + 2.0, dh + 2.0), p.divider, |fx, fy| {
        span(fx, dx, dx + hair) * span(fy, dy, dy + dh)
    });
    let cy = y0 + h / 2.0;
    // 左半：三个点，中间那个大一圈
    let mx = x0 + w / 4.0;
    for (off, rr) in [(-DOT_GAP, DOT_R), (0.0, DOT_R_MID), (DOT_GAP, DOT_R)] {
        disc(canvas, mx + off * s, cy, rr * s, p.icon);
    }
    // 右半：圆环 + 圆心
    let cx = x0 + w * 0.75;
    let (ro, ri) = (RING_R_OUT * s, (RING_R_OUT - RING_STROKE) * s);
    paint(canvas, (cx - ro - 1.0, cy - ro - 1.0, ro * 2.0 + 2.0, ro * 2.0 + 2.0), p.icon, |fx, fy| {
        let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
        ((ro - d + 0.5).clamp(0.0, 1.0) - (ri - d + 0.5).clamp(0.0, 1.0)).max(0.0)
    });
    disc(canvas, cx, cy, RING_DOT_R * s, p.icon);
}

fn disc(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, c: Color) {
    paint(canvas, (cx - r - 1.0, cy - r - 1.0, r * 2.0 + 2.0, r * 2.0 + 2.0), c, |fx, fy| {
        let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
        (r - d + 0.5).clamp(0.0, 1.0)
    });
}

/// 全圆角矩形（胶囊）的有向距离：里面为负（物理像素）
fn pill_sdf(fx: f32, fy: f32, x0: f32, y0: f32, w: f32, h: f32, rad: f32) -> f32 {
    let (cx, cy) = (x0 + w / 2.0, y0 + h / 2.0);
    let qx = (fx - cx).abs() - (w / 2.0 - rad);
    let qy = (fy - cy).abs() - (h / 2.0 - rad);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - rad
}

/// 像素 [p-0.5, p+0.5)（p 是像素中心）落在 [a, b) 里的长度
fn span(p: f32, a: f32, b: f32) -> f32 {
    ((p + 0.5).min(b) - (p - 0.5).max(a)).clamp(0.0, 1.0)
}

/// 在 `(x, y, w, h)` 包围盒里逐像素按覆盖率混一个颜色（source-over，fixed 层是透明底，
/// `Color::blend` 按「半透明叠半透明」算）
fn paint(canvas: &mut Canvas, (x, y, w, h): (f32, f32, f32, f32), c: Color, cov: impl Fn(f32, f32) -> f32) {
    let (cw, ch) = (canvas.width() as i32, canvas.height() as i32);
    let (px0, py0) = ((x.floor() as i32).max(0), (y.floor() as i32).max(0));
    let (px1, py1) = (((x + w).ceil() as i32).min(cw), ((y + h).ceil() as i32).min(ch));
    for py in py0..py1 {
        for px in px0..px1 {
            let k = cov(px as f32 + 0.5, py as f32 + 0.5);
            if k <= 0.0 {
                continue;
            }
            let a = (c.a as f32 * k.min(1.0)).round() as u8;
            if a > 0 {
                canvas.set_pixel(px, py, Color::new(c.r, c.g, c.b, a));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn 位置与_js_的公式一致_宿主给了真值就用它() {
        let r = rect(&json!({"statusBarHeight": 20.0, "windowWidth": 375.0}));
        assert_eq!((r.x, r.y, r.width, r.height), (281.0, 24.0, 87.0, 32.0));
        // 刘海屏 47、宽 393
        let r = rect(&json!({"statusBarHeight": 47.0, "windowWidth": 393.0}));
        assert_eq!((r.x, r.y, r.right()), (299.0, 51.0, 386.0));
        // statusBarHeight 为 0 时 JS 的 `||` 落到 20
        assert_eq!(rect(&json!({"statusBarHeight": 0, "windowWidth": 375})).y, 24.0);
        // Android：48 高的导航栏里上下居中（距状态栏 8）
        assert_eq!(rect(&json!({"statusBarHeight": 24.0, "windowWidth": 411.0, "platform": "android"})).y, 32.0);
        let r = rect(&json!({"windowWidth": 375, "menuButton": {"left": 270, "top": 30, "width": 96, "height": 34}}));
        assert_eq!((r.x, r.y, r.width, r.height), (270.0, 30.0, 96.0, 34.0));
    }

    #[test]
    fn 以分隔线分左右两半_外面不算() {
        let r = Rect::new(281.0, 24.0, 87.0, 32.0);
        assert_eq!(half_at(r, 290.0, 40.0), Some(Half::Menu));
        assert_eq!(half_at(r, 324.0, 40.0), Some(Half::Menu));
        assert_eq!(half_at(r, 325.0, 40.0), Some(Half::Close));
        assert_eq!(half_at(r, 367.0, 55.0), Some(Half::Close));
        assert_eq!(half_at(r, 368.0, 40.0), None);
        assert_eq!(half_at(r, 300.0, 23.9), None);
        assert_eq!(half_at(r, 300.0, 56.0), None);
    }

    /// 透明画布上画一个，返回指定逻辑坐标的像素
    fn shot(dark: bool, pressed: Option<Half>) -> impl Fn(f32, f32) -> Color {
        let mut c = Canvas::new(375 * 2, 80 * 2);
        c.clear(Color::TRANSPARENT);
        draw(&mut c, Rect::new(281.0, 24.0, 87.0, 32.0), 2.0, dark, pressed);
        move |x, y| c.get_pixel((x * 2.0) as u32, (y * 2.0) as u32)
    }

    #[test]
    fn 浅色_半透明白底黑图标_外面一个像素不碰() {
        let px = shot(false, None);
        let bg = px(300.0, 30.0);
        assert_eq!((bg.r, bg.g, bg.b, bg.a), (255, 255, 255, 153), "底色 rgba(255,255,255,.6)");
        // 中间那个点、右半的圆心：纯黑
        let mid = px(281.0 + 87.0 / 4.0, 40.0);
        assert_eq!((mid.r, mid.a), (0, 255));
        let ring_dot = px(281.0 + 87.0 * 0.75, 40.0);
        assert_eq!((ring_dot.r, ring_dot.a), (0, 255));
        // 圆环与圆心之间是空的（只有底色）
        let gap = px(281.0 + 87.0 * 0.75 + 4.2, 40.0);
        assert_eq!(gap.a, 153);
        // 胶囊外：透明
        assert_eq!(px(279.0, 40.0).a, 0);
        assert_eq!(px(300.0, 22.0).a, 0);
        assert_eq!(px(370.0, 40.0).a, 0);
    }

    #[test]
    fn 深色导航栏_半透明黑底白图标() {
        let px = shot(true, None);
        let bg = px(300.0, 30.0);
        assert_eq!((bg.r, bg.g, bg.b, bg.a), (0, 0, 0, 38), "底色 rgba(0,0,0,.15)");
        let mid = px(281.0 + 87.0 / 4.0, 40.0);
        assert_eq!((mid.r, mid.g, mid.b, mid.a), (255, 255, 255, 255));
    }

    #[test]
    fn 按压只压暗按着的那一半() {
        let (idle, menu) = (shot(false, None), shot(false, Some(Half::Menu)));
        let (l, r) = ((300.0, 30.0), (350.0, 30.0));
        assert_ne!(idle(l.0, l.1), menu(l.0, l.1), "左半变了");
        assert_eq!(idle(r.0, r.1), menu(r.0, r.1), "右半不动");
        let close = shot(false, Some(Half::Close));
        assert_eq!(idle(l.0, l.1), close(l.0, l.1));
        assert_ne!(idle(r.0, r.1), close(r.0, r.1));
    }
}
