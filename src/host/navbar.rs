//! 默认导航栏（`navigationStyle: default` 的页面顶上那一条）：几何、配色、画法、命中。
//!
//! 微信里它是**原生外壳**的一部分，和 tabBar 一样不归页面画。引擎在宿主打开
//! `navigationBar` 选项时自己画它（见 `engine_navbar.rs`），理由是三件只有画在同一张画面里
//! 才做得对的事：
//!
//! - **转场带着导航栏一起走**：微信 navigateTo 是整页（导航栏 + 内容）从右边滑进来。宿主另画
//!   一条导航栏的话，它和引擎画面不在同一条线程上出帧，滑动时两者错开一两帧，交界处撕开一条缝；
//! - **弹窗遮罩压住导航栏**：`wx.showModal` / ActionSheet / picker 的遮罩盖住整个窗口；
//! - **换页不改画面尺寸**：普通页与 custom 页之间切换时画面始终是整屏，宿主不用为
//!   导航栏出现 / 消失把引擎重建一遍（从前 Flutter 那一帧会把旧纹理拉伸着显示出来）。
//!
//! 尺寸按微信：状态栏之下 iOS 44、Android 48（社区实测的通用值；Android 的胶囊上下居中，
//! iOS 的胶囊距状态栏 4、下面留 8，见 `capsule.rs`）。标题 17px 居中、粗体，前景色只有
//! white / black 两种（`navigationBarTextStyle`），底色 `navigationBarBackgroundColor`
//! （缺省 #000000 + white，与 app.json 文档的默认值一致）。
//!
//! 画法全是纯函数：输入一份 [`NavBar`]，画进一块透明底的画布，单测离开引擎就能做。
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect};
use crate::text::TextRenderer;
use serde_json::Value;

/// 状态栏之下那一截的高度（逻辑像素）
pub fn bar_height(platform: &str) -> f32 {
    if platform.eq_ignore_ascii_case("android") {
        48.0
    } else {
        44.0
    }
}

/// 标题字号（逻辑像素）
pub const TITLE_SIZE: f32 = 17.0;
/// 返回键的命中宽度：从左边缘起这么宽、状态栏之下整条高都算点到返回
pub const BACK_HIT_WIDTH: f32 = 56.0;
/// 返回箭头（逻辑像素，相对导航栏左边与竖直中线）：尖在 x=17，两臂末端在 x=27、上下各 10
const ARROW_TIP_X: f32 = 17.0;
const ARROW_END_X: f32 = 27.0;
const ARROW_HALF_H: f32 = 10.0;
const ARROW_STROKE: f32 = 2.0;

/// 一帧导航栏要画的全部内容
#[derive(Clone, Debug, PartialEq)]
pub struct NavBar {
    pub title: String,
    pub background: Color,
    /// 前景是白色（`navigationBarTextStyle: white`）
    pub white: bool,
    /// `wx.showNavigationBarLoading` 在转圈
    pub loading: bool,
    /// 页面栈多于一页：左边画返回键
    pub back: bool,
}

impl NavBar {
    /// 从页面的窗口配置读（app.json `window` ← 页面 json ← 运行时 `setNavigationBar*`，
    /// 即 `MiniEngine::page_window`）
    pub fn from_window(w: &Value, depth: usize) -> Self {
        let s = |k: &str| w.get(k).and_then(|v| v.as_str());
        Self {
            title: s("navigationBarTitleText").unwrap_or("").to_string(),
            background: s("navigationBarBackgroundColor").and_then(parse_hex).unwrap_or(Color::new(0, 0, 0, 255)),
            // 缺省 white（与 app.json 文档一致）；只有明确写 black 才是黑字
            white: s("navigationBarTextStyle") != Some("black"),
            loading: w.get("navigationBarLoading").and_then(|v| v.as_bool()).unwrap_or(false),
            back: depth > 1,
        }
    }

    pub fn foreground(&self) -> Color {
        if self.white {
            Color::new(255, 255, 255, 255)
        } else {
            Color::new(0, 0, 0, 255)
        }
    }
}

/// `#rgb` / `#rrggbb` / `#rrggbbaa`（导航栏底色只认十六进制，与微信一致）
pub fn parse_hex(s: &str) -> Option<Color> {
    let h = s.trim().strip_prefix('#')?;
    let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
    match h.len() {
        3 => Some(Color::new(v(0, 1)? * 17, v(1, 1)? * 17, v(2, 1)? * 17, 255)),
        6 => Some(Color::new(v(0, 2)?, v(2, 2)?, v(4, 2)?, 255)),
        8 => Some(Color::new(v(0, 2)?, v(2, 2)?, v(4, 2)?, v(6, 2)?)),
        _ => None,
    }
}

/// 点在返回键上（视口逻辑坐标）。`top` 是状态栏高，`bar_h` 是状态栏之下那一截
pub fn back_hit(x: f32, y: f32, top: f32, bar_h: f32) -> bool {
    x < BACK_HIT_WIDTH && y >= top && y < top + bar_h
}

/// 标题能占的最大宽度：左右各让出返回键与胶囊的位置，标题始终居中（微信是这样截断的）
pub fn title_max_width(width: f32, capsule_left: Option<f32>) -> f32 {
    let right = capsule_left.map(|l| width - l + 8.0).unwrap_or(0.0);
    let side = right.max(BACK_HIT_WIDTH);
    (width - side * 2.0).max(0.0)
}

/// 画进 `canvas`（物理像素，`dpr` 倍；画布至少 `(top + bar_h) × dpr` 高）。
///
/// `top` 是状态栏高：状态栏那一截也填导航栏底色（微信的导航栏一直延伸到屏幕顶）。
/// `back_pressed`：手指按在返回键上（图标半透明）；`spin_step` 是转圈的第几步（0..12）
#[allow(clippy::too_many_arguments)]
pub fn draw(
    canvas: &mut Canvas,
    bar: &NavBar,
    width: f32,
    top: f32,
    bar_h: f32,
    dpr: f32,
    text: Option<&TextRenderer>,
    title_max: f32,
    back_pressed: bool,
    spin_step: u32,
) {
    let s = dpr.max(0.1);
    canvas.reset_clip();
    canvas.clear(Color::TRANSPARENT);
    canvas.draw_rect(
        &Rect::new(0.0, 0.0, width * s, (top + bar_h) * s),
        &Paint::new().with_color(bar.background).with_style(PaintStyle::Fill),
    );
    let fg = bar.foreground();
    let cy = (top + bar_h / 2.0) * s;
    if bar.back {
        let a = if back_pressed { 128 } else { 255 };
        draw_arrow(canvas, cy, s, Color::new(fg.r, fg.g, fg.b, a));
    }
    let Some(tr) = text else { return };
    let size = TITLE_SIZE * s;
    let bold = tr.has_bold_face();
    let title = ellipsize(tr, &bar.title, size, bold, title_max * s);
    let tw = tr.measure_text_weighted(&title, size, 0.0, bold);
    let tx = (width * s - tw) / 2.0;
    // draw_text 的 y 是基线：中线往下 0.36 个字号（与 tabBar 角标同一条经验式）
    let paint = Paint::new().with_color(fg).with_style(PaintStyle::Fill);
    tr.draw_text_weighted(canvas, &title, tx, cy + size * 0.36, size, 0.0, bold, &paint);
    if bar.loading {
        // 微信的导航栏 loading：标题左边一个小圈
        let r = 8.0 * s;
        let cx = if title.is_empty() { width * s / 2.0 } else { tx - 8.0 * s - r };
        draw_spinner(canvas, cx, cy, r, s, fg, spin_step);
    }
}

/// 太长的标题从尾部截断，补「…」
fn ellipsize(tr: &TextRenderer, title: &str, size: f32, bold: bool, max_w: f32) -> String {
    if max_w <= 0.0 || tr.measure_text_weighted(title, size, 0.0, bold) <= max_w {
        return title.to_string();
    }
    let chars: Vec<char> = title.chars().collect();
    for n in (0..chars.len()).rev() {
        let t: String = chars[..n].iter().collect::<String>() + "…";
        if tr.measure_text_weighted(&t, size, 0.0, bold) <= max_w {
            return t;
        }
    }
    "…".to_string()
}

/// 一条有厚度的线段（四边形）。`draw_path` 只填充，所以线段自己展开成多边形
fn thick_line(path: &mut Path, (x0, y0): (f32, f32), (x1, y1): (f32, f32), w: f32) {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let (nx, ny) = (-dy / len * w / 2.0, dx / len * w / 2.0);
    path.move_to(x0 + nx, y0 + ny);
    path.line_to(x1 + nx, y1 + ny);
    path.line_to(x1 - nx, y1 - ny);
    path.line_to(x0 - nx, y0 - ny);
    path.close();
}

/// 返回箭头「‹」：两条臂 + 尖上补一个圆（两段四边形在尖上接不严）
fn draw_arrow(canvas: &mut Canvas, cy: f32, s: f32, color: Color) {
    let paint = Paint::new().with_color(color).with_style(PaintStyle::Fill).with_anti_alias(true);
    let tip = (ARROW_TIP_X * s, cy);
    let mut p = Path::new();
    thick_line(&mut p, (ARROW_END_X * s, cy - ARROW_HALF_H * s), tip, ARROW_STROKE * s);
    canvas.draw_path(&p, &paint);
    let mut p = Path::new();
    thick_line(&mut p, tip, (ARROW_END_X * s, cy + ARROW_HALF_H * s), ARROW_STROKE * s);
    canvas.draw_path(&p, &paint);
    canvas.draw_circle(tip.0, tip.1, ARROW_STROKE * s / 2.0, &paint);
}

/// 12 根辐条的转圈（WeUI loading 的样子）：当前步那根最实，往后依次变淡
fn draw_spinner(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, s: f32, fg: Color, step: u32) {
    for i in 0..12u32 {
        let ang = (i as f32) * std::f32::consts::TAU / 12.0 - std::f32::consts::FRAC_PI_2;
        let (c, sn) = (ang.cos(), ang.sin());
        let age = (step + 12 - i) % 12;
        let a = (255.0 * (1.0 - age as f32 / 12.0 * 0.85)) as u8;
        let paint = Paint::new().with_color(Color::new(fg.r, fg.g, fg.b, a)).with_style(PaintStyle::Fill).with_anti_alias(true);
        let mut p = Path::new();
        thick_line(&mut p, (cx + c * r * 0.5, cy + sn * r * 0.5), (cx + c * r, cy + sn * r), 1.6 * s);
        canvas.draw_path(&p, &paint);
    }
}

/// 转圈这一刻在第几步（一秒一圈）
pub fn spin_step(secs: f32) -> u32 {
    ((secs.max(0.0).fract() * 12.0) as u32).min(11)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn 高度按平台_android_48_其余_44() {
        assert_eq!(bar_height("android"), 48.0);
        assert_eq!(bar_height("ios"), 44.0);
        assert_eq!(bar_height("devtools"), 44.0);
    }

    #[test]
    fn 窗口配置缺省是黑底白字_写了才换() {
        let b = NavBar::from_window(&json!({}), 1);
        assert_eq!((b.background, b.white, b.back, b.loading), (Color::new(0, 0, 0, 255), true, false, false));
        let b = NavBar::from_window(
            &json!({"navigationBarTitleText":"标题","navigationBarBackgroundColor":"#07c160",
                    "navigationBarTextStyle":"black","navigationBarLoading":true}),
            2,
        );
        assert_eq!(b.title, "标题");
        assert_eq!(b.background, Color::new(0x07, 0xc1, 0x60, 255));
        assert!(!b.white && b.back && b.loading);
        assert_eq!(parse_hex("#fff"), Some(Color::new(255, 255, 255, 255)));
        assert_eq!(parse_hex("red"), None);
    }

    #[test]
    fn 返回键只认左边一截_状态栏里不算() {
        assert!(back_hit(10.0, 30.0, 24.0, 48.0));
        assert!(back_hit(55.0, 71.0, 24.0, 48.0));
        assert!(!back_hit(56.0, 40.0, 24.0, 48.0));
        assert!(!back_hit(10.0, 20.0, 24.0, 48.0), "状态栏里");
        assert!(!back_hit(10.0, 72.0, 24.0, 48.0), "导航栏下面");
    }

    #[test]
    fn 标题让出胶囊与返回键_始终居中() {
        // 375 宽，胶囊左边 281：右边让 102，左右对称各让 102
        assert_eq!(title_max_width(375.0, Some(281.0)), 375.0 - 204.0);
        assert_eq!(title_max_width(375.0, None), 375.0 - 112.0);
    }

    /// 底色铺满状态栏 + 导航栏，返回键只在栈深 > 1 时画
    #[test]
    fn 底色铺到状态栏_返回箭头画在左边() {
        let (w, top, h, dpr) = (375.0, 20.0, 44.0, 2.0);
        let mut c = Canvas::new(750, 128);
        let mut bar = NavBar::from_window(&json!({"navigationBarBackgroundColor":"#07c160"}), 2);
        draw(&mut c, &bar, w, top, h, dpr, None, 200.0, false, 0);
        let px = |c: &Canvas, x: f32, y: f32| c.get_pixel((x * dpr) as u32, (y * dpr) as u32);
        assert_eq!(px(&c, 200.0, 5.0), Color::new(0x07, 0xc1, 0x60, 255), "状态栏那一截也是导航栏底色");
        assert_eq!(px(&c, 200.0, 60.0), Color::new(0x07, 0xc1, 0x60, 255));
        // 箭头的尖（x=17、竖直中线 42）：白色
        let tip = px(&c, 17.5, 42.0);
        assert!(tip.r > 200 && tip.g > 200, "箭头 {tip:?}");
        bar.back = false;
        draw(&mut c, &bar, w, top, h, dpr, None, 200.0, false, 0);
        assert_eq!(px(&c, 17.5, 42.0), Color::new(0x07, 0xc1, 0x60, 255), "栈底不画返回键");
    }
}
