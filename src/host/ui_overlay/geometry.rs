//! 提示框（Toast / Loading）与 Modal 的盒子几何，**逻辑像素**、相对视口左上角。
//!
//! 绘制（`tip.rs` / `modal.rs`）、按钮命中（`modal_hit.rs`）与「宿主原生组件被谁挡住」
//! （`host::native_occlusion`）都从这里取同一份算式。从前绘制按物理像素、命中按逻辑像素
//! 各写一份（行高 1.5 对 1.6、逐字换行对按总宽估行数），多行内容时按钮命中区与画出来的
//! 差几个像素；原生组件（宿主的 video 之类）盖在引擎画面上面，盒子算偏一点，弹窗边上就会
//! 露出一条视频 —— 所以只留一份。
//!
//! ## 数值从哪来
//! 微信的提示框与对话框是客户端原生控件，视觉规范与 WeUI 2.x（微信 8.0 那一版设计语言）一致，
//! 下面的常量逐条取自 WeUI v2.4.4 的 `weui-toast.less` / `weui-dialog.less`（iOS 皮肤）
//! 与浅色主题变量：
//! - 提示框：带图标 136×136、圆角 12、底色 BG-4 `#4C4C4C`、图标 40 + 下间距 16、
//!   标题 17px 行高 1.4；纯文字 14px、上下内边距 12、宽度随文字在 152~216 之间；
//!   盒子中心在视口 **40%** 高处（`top: 40%; transform: translate(-50%, -50%)`）。
//! - 对话框：视口宽 ≥ 352 时宽 320，否则左右各留 16；圆角 12；标题区 `32 24 16`、
//!   17px 粗体；内容 17px 行高 1.4、底边距 32、颜色 FG-1；没有标题时内容当主文字
//!   （粗体、FG-0、`min-height: 40`、上内边距 32）；按钮条高 56。
//!
//! 文案长度的上限按官方文档：带图标时标题「最多显示 7 个汉字长度」，纯文字「最多两行」。
use super::draw::{text_width, wrap_lines};
use super::ModalState;
use crate::text::TextRenderer;

/// 提示框里画什么图标
#[derive(Clone, Debug, PartialEq)]
pub enum TipIcon {
    Success,
    Error,
    Loading,
    /// `wx.showToast({ image })`：自定义图标路径（优先于 icon）
    Image(String),
    /// 纯文字
    None,
}

impl TipIcon {
    /// `icon` 是逻辑层归一过的值；`image` 非空时优先
    pub fn of(icon: &str, image: &str) -> Self {
        if !image.is_empty() {
            return TipIcon::Image(image.to_string());
        }
        match icon {
            "none" => TipIcon::None,
            "error" => TipIcon::Error,
            "loading" => TipIcon::Loading,
            _ => TipIcon::Success,
        }
    }
}

pub const TIP_RADIUS: f32 = 12.0;
pub const TIP_BG: u32 = 0x4C4C4C;
/// rgba(255,255,255,0.9) 压在 #4C4C4C 上的结果；Loading 的圈 WeUI 直接写的就是 #EDEDED
pub const TIP_FG: u32 = 0xEDEDED;
const TIP_BOX: f32 = 136.0;
pub const TIP_ICON: f32 = 40.0;
const TIP_ICON_GAP: f32 = 16.0;
const TIP_FONT: f32 = 17.0;
const TIP_PAD_X: f32 = 12.0;
/// 带图标时标题最多 7 个汉字（官方文档）
const TIP_MAX_CHARS: f32 = 7.0;
const TEXT_TIP_FONT: f32 = 14.0;
const TEXT_TIP_PAD_Y: f32 = 12.0;
const TEXT_TIP_MIN_W: f32 = 152.0;
const TEXT_TIP_MAX_W: f32 = 216.0;
const TEXT_TIP_MAX_LINES: usize = 2;
const LINE: f32 = 1.4;
/// 盒子中心在视口高度的这个比例上
const TIP_CENTER_Y: f32 = 0.4;

/// 提示框的盒子与里面各块的位置（逻辑像素）
#[derive(Clone, Debug)]
pub struct TipBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// 图标的左上角与边长；纯文字时 None
    pub icon: Option<(f32, f32, f32)>,
    /// 已经断好行、截好「…」的标题
    pub lines: Vec<String>,
    pub font: f32,
    pub line_h: f32,
    /// 第一行行盒的上沿
    pub text_top: f32,
    /// 文字水平居中的中线
    pub center_x: f32,
}

pub fn tip_box(title: &str, icon: &TipIcon, (vw, vh): (f32, f32), tr: Option<&TextRenderer>) -> TipBox {
    let cx = vw / 2.0;
    let cy = vh * TIP_CENTER_Y;
    if *icon == TipIcon::None {
        let (font, line_h) = (TEXT_TIP_FONT, TEXT_TIP_FONT * LINE);
        let max_w = (TEXT_TIP_MAX_W.min(vw - 32.0) - TIP_PAD_X * 2.0).max(font);
        let lines = wrap_lines(tr, title, font, max_w, TEXT_TIP_MAX_LINES, false);
        let widest = lines.iter().map(|l| text_width(tr, l, font, false)).fold(0.0, f32::max);
        let w = (widest + TIP_PAD_X * 2.0).clamp(TEXT_TIP_MIN_W.min(vw - 32.0), TEXT_TIP_MAX_W.min(vw - 32.0));
        let h = TEXT_TIP_PAD_Y * 2.0 + lines.len() as f32 * line_h;
        let (x, y) = (cx - w / 2.0, cy - h / 2.0);
        return TipBox { x, y, w, h, icon: None, lines, font, line_h, text_top: y + TEXT_TIP_PAD_Y, center_x: cx };
    }
    let (font, line_h) = (TIP_FONT, TIP_FONT * LINE);
    let (w, h) = (TIP_BOX, TIP_BOX);
    let (x, y) = (cx - w / 2.0, cy - h / 2.0);
    // 一行，放不下就截「…」。136 减去两边 12 只有 112，装不下 7 个 17px 的汉字（119）——
    // 文档说的是「最多显示 7 个汉字」，按 7 个字放宽
    let max_w = (w - TIP_PAD_X * 2.0).max(font * TIP_MAX_CHARS);
    let lines = if title.is_empty() { Vec::new() } else { wrap_lines(tr, title, font, max_w, 1, false) };
    let content_h = TIP_ICON + if lines.is_empty() { 0.0 } else { TIP_ICON_GAP + line_h };
    let top = y + (h - content_h) / 2.0;
    TipBox {
        x,
        y,
        w,
        h,
        icon: Some((cx - TIP_ICON / 2.0, top, TIP_ICON)),
        lines,
        font,
        line_h,
        text_top: top + TIP_ICON + TIP_ICON_GAP,
        center_x: cx,
    }
}

pub const MODAL_RADIUS: f32 = 12.0;
const MODAL_W: f32 = 320.0;
/// 视口比它窄时改成左右各留 16（WeUI 的 `@media (min-width: 352px)`）
const MODAL_W_BREAK: f32 = 352.0;
const MODAL_PAD_X: f32 = 24.0;
const MODAL_FONT: f32 = 17.0;
pub const MODAL_FOOTER_H: f32 = 56.0;
/// 盒子最高占视口的比例（`max-height: 90%`）；超出的内容行截掉（WeUI 是内部滚动，这里不做）
const MODAL_MAX_H: f32 = 0.9;

/// Modal 的盒子与各块位置（逻辑像素）
#[derive(Clone, Debug)]
pub struct ModalGeom {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub title_lines: Vec<String>,
    pub title_top: f32,
    pub body_lines: Vec<String>,
    pub body_top: f32,
    /// 没有标题：内容当主文字画（粗体、主色）
    pub body_is_main: bool,
    pub font: f32,
    pub line_h: f32,
    pub footer_y: f32,
}

pub fn modal_geom(m: &ModalState, (vw, vh): (f32, f32), tr: Option<&TextRenderer>) -> ModalGeom {
    let w = if vw >= MODAL_W_BREAK { MODAL_W } else { (vw - 32.0).max(MODAL_PAD_X * 4.0) };
    let inner = w - MODAL_PAD_X * 2.0;
    let (font, line_h) = (MODAL_FONT, MODAL_FONT * LINE);
    let max_body_h = vh * MODAL_MAX_H - MODAL_FOOTER_H;
    let has_title = !m.title.is_empty();
    let title_lines = if has_title { wrap_lines(tr, &m.title, font, inner, 3, true) } else { Vec::new() };
    let mut body_lines =
        if m.content.is_empty() { Vec::new() } else { wrap_lines(tr, &m.content, font, inner, usize::MAX, !has_title) };
    // (标题区高, 内容区高, 内容行盒相对内容区顶的偏移)
    let layout = |n_body: usize| -> (f32, f32, f32) {
        let text_h = n_body as f32 * line_h;
        if has_title {
            // 只有标题没有内容：标题区下边距与上边距对称（否则下面空着 16 + 32）
            let hd_bottom = if n_body == 0 { 32.0 } else { 16.0 };
            let hd = 32.0 + title_lines.len() as f32 * line_h + hd_bottom;
            let bd = if n_body == 0 { 0.0 } else { text_h + 32.0 };
            (hd, bd, 0.0)
        } else {
            // 内容是第一个子元素：上内边距 32、min-height 40 里垂直居中、下边距 32
            let area = text_h.max(40.0);
            (0.0, 32.0 + area + 32.0, 32.0 + (area - text_h) / 2.0)
        }
    };
    let (mut hd, mut bd, mut body_off) = layout(body_lines.len());
    while hd + bd > max_body_h && body_lines.len() > 1 {
        body_lines.pop();
        if let Some(last) = body_lines.last_mut() {
            *last = super::draw::ellipsize(tr, &format!("{last}…"), font, inner, !has_title);
        }
        (hd, bd, body_off) = layout(body_lines.len());
    }
    let h = hd + bd + MODAL_FOOTER_H;
    let (x, y) = ((vw - w) / 2.0, (vh - h) / 2.0);
    ModalGeom {
        x,
        y,
        w,
        h,
        title_lines,
        title_top: y + 32.0,
        body_lines,
        body_top: y + hd + body_off,
        body_is_main: !has_title,
        font,
        line_h,
        footer_y: y + h - MODAL_FOOTER_H,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modal(title: &str, content: &str) -> ModalState {
        ModalState::new(title, content)
    }

    #[test]
    fn 带图标的提示框是_136_方块_中心在_40_高处() {
        let b = tip_box("操作成功", &TipIcon::Success, (375.0, 667.0), None);
        assert_eq!((b.w, b.h), (136.0, 136.0));
        assert!((b.x + b.w / 2.0 - 187.5).abs() < 0.01);
        assert!((b.y + b.h / 2.0 - 667.0 * 0.4).abs() < 0.01);
        let (ix, iy, s) = b.icon.unwrap();
        assert_eq!(s, 40.0);
        assert!((ix - (187.5 - 20.0)).abs() < 0.01);
        // 图标 40 + 16 + 一行 23.8 在 136 里垂直居中
        assert!((iy - (b.y + (136.0 - 79.8) / 2.0)).abs() < 0.01);
        assert!((b.text_top - (iy + 56.0)).abs() < 0.01);
    }

    #[test]
    fn 带图标时标题最多一行_超出补省略号() {
        let b = tip_box("这是一个特别特别长的标题", &TipIcon::Success, (375.0, 667.0), None);
        assert_eq!(b.lines.len(), 1);
        assert!(b.lines[0].ends_with('…'), "{:?}", b.lines);
        // 7 个汉字正好放得下（文档：最多显示 7 个汉字长度）
        let b = tip_box("处理中请稍候啊", &TipIcon::Loading, (375.0, 667.0), None);
        assert_eq!(b.lines, vec!["处理中请稍候啊".to_string()]);
    }

    #[test]
    fn 纯文字提示框随文字变宽_最多两行() {
        let short = tip_box("已复制", &TipIcon::None, (375.0, 667.0), None);
        assert_eq!(short.w, 152.0, "短文字取最小宽");
        assert!((short.h - (24.0 + 19.6)).abs() < 0.01);
        let long = tip_box("这是一条只有文字的提示，文字长了会换行显示，再长就截断了", &TipIcon::None, (375.0, 667.0), None);
        // 216 - 24 = 192 放得下 13 个 14px 的字：宽 13×14 + 24
        assert_eq!(long.w, 13.0 * 14.0 + 24.0);
        assert_eq!(long.lines.len(), 2);
        assert!(long.lines[1].ends_with('…'));
        assert!(long.icon.is_none());
    }

    #[test]
    fn 对话框_320_宽_居中_按钮条_56() {
        let g = modal_geom(&modal("提示", "确认退出登录？"), (375.0, 667.0), None);
        assert_eq!(g.w, 320.0);
        assert!((g.x - 27.5).abs() < 0.01);
        // 标题区 32+23.8+16，内容 23.8+32，按钮 56
        assert!((g.h - (71.8 + 55.8 + 56.0)).abs() < 0.01, "h = {}", g.h);
        assert!((g.y - (667.0 - g.h) / 2.0).abs() < 0.01);
        assert!((g.footer_y - (g.y + g.h - 56.0)).abs() < 0.01);
        assert!(!g.body_is_main);
    }

    #[test]
    fn 英文单词不从中间断() {
        // 272 宽、17px：中文 6 个 102 + 「（」17 + 「showCancel:」11×9.35≈103 = 222，再加「 false）」放不下
        let g = modal_geom(&modal("", "只有一个按钮（showCancel: false）"), (375.0, 667.0), None);
        assert_eq!(g.body_lines.len(), 2, "{:?}", g.body_lines);
        assert!(g.body_lines[1].starts_with("false"), "{:?}", g.body_lines);
        assert!(!g.body_lines[0].ends_with(' '));
    }

    #[test]
    fn 窄屏对话框左右留_16_没标题时内容当主文字() {
        let g = modal_geom(&modal("", "只有内容"), (320.0, 568.0), None);
        assert_eq!(g.w, 288.0);
        assert!(g.body_is_main);
        assert!(g.title_lines.is_empty());
        // 32 + max(23.8, 40) + 32 + 56
        assert!((g.h - (32.0 + 40.0 + 32.0 + 56.0)).abs() < 0.01);
    }

    #[test]
    fn 对话框不超过视口九成_多出的内容截掉() {
        let long = "很长的内容".repeat(200);
        let g = modal_geom(&modal("标题", &long), (375.0, 667.0), None);
        assert!(g.h <= 667.0 * 0.9 + 0.01, "h = {}", g.h);
        assert!(g.body_lines.last().unwrap().ends_with('…'));
    }
}
