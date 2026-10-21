//! UI 覆盖层：提示框（Toast / Loading）、Modal、操作菜单、图片预览
//!
//! - `mod.rs`：状态、逻辑层指令的落地（[`apply_feedback_event`]，SDK 与桌面窗体共用）、
//!   蒙层挡输入、合成入口
//! - `geometry`：提示框与 Modal 的盒子几何（逻辑像素；绘制、命中、原生组件遮挡共用一份）
//! - `tip` / `modal`：两者的绘制；`icons`：对勾、感叹号、转圈、自定义图标
//! - `draw`：直接写输出缓冲的光栅化小工具
//! - `modal_hit`：Modal 的命中与按钮回调
//!
//! 视觉对齐微信 8.0 的原生控件（WeUI 2.x），数值与出处写在 `geometry.rs` 的文件头。

use crate::runtime::UiEvent;
use crate::text::TextRenderer;
use crate::Color;
use std::cell::Cell;
use std::time::Instant;

mod draw;
mod geometry;
mod icons;
mod modal;
mod modal_hit;
mod tip;
/// `wx.showActionSheet` 的底部操作菜单（状态在线程局部，SDK 与桌面共用）
pub mod action_sheet;
/// `wx.previewImage` 的全屏图片预览（同上）
pub mod image_preview;

/// 操作菜单 / 图片预览这两层宿主浮层：有没有弹着（宿主据此把指针交给它们）
pub fn sheet_overlay_visible() -> bool {
    action_sheet::visible() || image_preview::visible()
}

/// 两层浮层的动画在走（宿主据此续帧）
pub fn sheet_overlay_animating() -> bool {
    action_sheet::animating() || image_preview::animating()
}

/// 换页 / 销毁时收掉两层浮层
pub fn clear_sheet_overlays() {
    action_sheet::clear();
    image_preview::clear();
}

/// 每帧：取走逻辑层的 showActionSheet / previewImage 请求、回收走完退场动画的浮层。返回是否要重新合成
pub fn take_sheet_requests(app: &mut crate::runtime::MiniApp) -> bool {
    let a = action_sheet::take_requests(app);
    let b = image_preview::take_requests(app);
    a || b
}

/// 两层浮层的按下 / 移动 / 抬起（逻辑坐标，`vp` 是逻辑视口）。返回是否消费
pub fn sheet_overlay_press(vp: (f32, f32), x: f32, y: f32) -> bool {
    action_sheet::on_press(vp, x, y) || image_preview::on_press(x)
}

pub fn sheet_overlay_move(x: f32) -> bool {
    action_sheet::visible() || image_preview::on_move(x)
}

pub fn sheet_overlay_release(app: &mut crate::runtime::MiniApp, vp: (f32, f32), x: f32, y: f32) -> bool {
    action_sheet::on_release(app, vp, x, y) || image_preview::on_release(x)
}

/// 合成：图片预览盖在页面上，操作菜单在它上面
pub fn render_sheet_overlays(buf: &mut [u32], w: u32, h: u32, sf: f32, text: Option<&TextRenderer>) {
    image_preview::render(buf, w, h, sf, text);
    action_sheet::render(buf, w, h, sf, text);
}

pub use geometry::{modal_geom, tip_box, ModalGeom, TipBox, TipIcon};
pub use icons::{spinner_step, SPINNER_STEPS};
pub use modal_hit::{modal_button_at, modal_layout, modal_press, modal_release, ModalLayout};

/// Toast 状态（`wx.showToast`）
#[derive(Clone)]
pub struct ToastState {
    pub title: String,
    /// success / error / loading / none
    pub icon: String,
    /// 自定义图标路径（非空时优先于 icon）
    pub image: String,
    /// 透明蒙层：弹着时页面点不动
    pub mask: bool,
    pub visible: bool,
    pub start_time: Instant,
    pub duration_ms: u32,
}

/// Loading 状态（`wx.showLoading`）
#[derive(Clone)]
pub struct LoadingState {
    pub title: String,
    pub mask: bool,
    pub visible: bool,
}

/// Modal 状态
#[derive(Clone)]
pub struct ModalState {
    pub title: String,
    pub content: String,
    pub show_cancel: bool,
    pub cancel_text: String,
    pub confirm_text: String,
    pub cancel_color: Color,
    pub confirm_color: Color,
    pub visible: bool,
    pub pressed_button: Option<String>, // "cancel" or "confirm"
}

impl ModalState {
    /// 其余字段取微信的缺省值（取消 / 确定、#000000 / #576B95）
    pub fn new(title: &str, content: &str) -> Self {
        Self {
            title: title.to_string(),
            content: content.to_string(),
            show_cancel: true,
            cancel_text: "取消".to_string(),
            confirm_text: "确定".to_string(),
            cancel_color: Color::from_hex(0x000000),
            confirm_color: Color::from_hex(0x576B95),
            visible: true,
            pressed_button: None,
        }
    }
}

/// `#RGB` / `#RRGGBB`，不合法时用 `def`
fn hex_color(s: &str, def: u32) -> Color {
    let h = s.trim().trim_start_matches('#');
    let full: String = if h.len() == 3 { h.chars().flat_map(|c| [c, c]).collect() } else { h.to_string() };
    match (full.len(), u32::from_str_radix(&full, 16)) {
        (6, Ok(v)) => Color::from_hex(v),
        _ => Color::from_hex(def),
    }
}

/// 逻辑层的提示框 / 弹窗指令落到宿主状态上（SDK 与桌面窗体共用一份规则）。
/// 不是这几类事件（下拉刷新）就原样还回去，调用方自己处理。
///
/// Toast 与 Loading 是**同一个槽位**（官方：「wx.showLoading 和 wx.showToast 同时只能显示一个」）：
/// show 一个就顶掉另一个；hide 两个接口默认互相能关，`noConflict` 才只关自己那种。
/// 从前两者各有一份状态、互不相干，于是 `showLoading` 之后再 `showToast`，Loading 盖着 Toast
/// 一直转（绘制时 Loading 优先），Toast 根本看不见。
pub fn apply_feedback_event(
    e: UiEvent,
    toast: &mut Option<ToastState>,
    loading: &mut Option<LoadingState>,
    modal: &mut Option<ModalState>,
) -> Option<UiEvent> {
    match e {
        UiEvent::ShowToast { title, icon, duration, mask, image } => {
            *loading = None;
            *toast = Some(ToastState {
                title,
                icon,
                image,
                mask,
                visible: true,
                start_time: Instant::now(),
                duration_ms: duration,
            });
        }
        UiEvent::HideToast { no_conflict } => {
            *toast = None;
            if !no_conflict {
                *loading = None;
            }
        }
        UiEvent::ShowLoading { title, mask } => {
            *toast = None;
            *loading = Some(LoadingState { title, mask, visible: true });
        }
        UiEvent::HideLoading { no_conflict } => {
            *loading = None;
            if !no_conflict {
                *toast = None;
            }
        }
        UiEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text, cancel_color, confirm_color } => {
            *modal = Some(ModalState {
                title,
                content,
                show_cancel,
                cancel_text,
                confirm_text,
                cancel_color: hex_color(&cancel_color, 0x000000),
                confirm_color: hex_color(&confirm_color, 0x576B95),
                visible: true,
                pressed_button: None,
            });
        }
        UiEvent::HideModal => *modal = None,
        other => return Some(other),
    }
    None
}

fn shown<T>(s: &Option<T>, visible: impl Fn(&T) -> bool) -> Option<&T> {
    s.as_ref().filter(|v| visible(v))
}

/// 提示框带着 `mask: true`：透明蒙层挡住页面（与 Modal 一样，触摸不往下传）。
///
/// 不带 mask 时**不挡**：微信的 Toast / Loading 默认可以点穿（mask 的文档原话是
/// 「是否显示透明蒙层，防止触摸穿透」）。从前反过来 —— Loading 一律挡、Toast 一律不挡。
pub fn tip_blocks_input(toast: &Option<ToastState>, loading: &Option<LoadingState>) -> bool {
    shown(loading, |l| l.visible).map(|l| l.mask).unwrap_or(false)
        || shown(toast, |t| t.visible).map(|t| t.mask).unwrap_or(false)
}

/// 提示框里有个在转的圈（Loading，或 `icon: 'loading'` 的 Toast）
pub fn tip_spinning(toast: &Option<ToastState>, loading: &Option<LoadingState>) -> bool {
    shown(loading, |l| l.visible).is_some()
        || shown(toast, |t| t.visible).map(|t| TipIcon::of(&t.icon, &t.image) == TipIcon::Loading).unwrap_or(false)
}

thread_local! {
    /// 上一次合成画出来的转圈步数：宿主据此判断「这一帧要不要重新合成」
    static DRAWN_STEP: Cell<Option<u32>> = const { Cell::new(None) };
}

/// 圈在转、而且转到了新的一步（一秒 60 步）：要重新合成一帧。
/// 高刷屏上 144Hz 出帧、圈一秒只走 60 步，其余帧不必合成。
pub fn tip_needs_recompose(anim_secs: f32, toast: &Option<ToastState>, loading: &Option<LoadingState>) -> bool {
    tip_spinning(toast, loading) && DRAWN_STEP.with(|s| s.get()) != Some(spinner_step(anim_secs))
}

/// 本帧 [`render_ui_overlay`] 会画出的覆盖层：(不透明盒子, 全屏遮罩不透明度)，
/// 盒子是**物理像素**，按画的顺序（Modal 在下、提示框在上）。
/// 宿主原生组件的遮挡计算（`host::native_occlusion`）靠它知道「上面压着什么」。
pub fn overlay_cover(
    width: u32,
    height: u32,
    sf: f32,
    toast: &Option<ToastState>,
    loading: &Option<LoadingState>,
    modal: &Option<ModalState>,
    text_renderer: Option<&TextRenderer>,
) -> Vec<(crate::Rect, f32)> {
    let vp = (width as f32 / sf, height as f32 / sf);
    let phys = |x: f32, y: f32, w: f32, h: f32| crate::Rect::new(x * sf, y * sf, w * sf, h * sf);
    let mut out = Vec::new();
    if let Some(m) = shown(modal, |m| m.visible) {
        let g = modal_geom(m, vp, text_renderer);
        out.push((phys(g.x, g.y, g.w, g.h), 1.0 - modal::MASK_KEEP));
    }
    if let Some((title, icon)) = current_tip(toast, loading) {
        // 提示框的蒙层是透明的：挡触摸，不挡视线
        let b = tip_box(title, &icon, vp, text_renderer);
        out.push((phys(b.x, b.y, b.w, b.h), 0.0));
    }
    out
}

/// 槽位里现在显示的提示框：(标题, 图标)
fn current_tip<'a>(toast: &'a Option<ToastState>, loading: &'a Option<LoadingState>) -> Option<(&'a str, TipIcon)> {
    if let Some(l) = shown(loading, |l| l.visible) {
        return Some((&l.title, TipIcon::Loading));
    }
    shown(toast, |t| t.visible).map(|t| (t.title.as_str(), TipIcon::of(&t.icon, &t.image)))
}

/// 渲染 UI 覆盖层：Modal（连同遮罩）在下，提示框在上（微信的提示框是浮在一切之上的 HUD）。
///
/// `anim_secs` 是转圈的时钟（秒）：交互时取进程动画时钟，快照 / 测试取固定值
/// （与 CSS 动画同一个时钟，`--time` / `set_animation_time` 定住它，像素才可比）。
pub fn render_ui_overlay(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    sf: f32,
    anim_secs: f32,
    toast: &Option<ToastState>,
    loading: &Option<LoadingState>,
    modal: &Option<ModalState>,
    text_renderer: Option<&TextRenderer>,
) {
    if let Some(m) = shown(modal, |m| m.visible) {
        modal::render_modal(buffer, width, height, sf, m, text_renderer);
    }
    if let Some((title, icon)) = current_tip(toast, loading) {
        let step = spinner_step(anim_secs);
        if icon == TipIcon::Loading {
            DRAWN_STEP.with(|s| s.set(Some(step)));
        }
        tip::render_tip(buffer, width, height, sf, title, &icon, step, text_renderer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show_toast(icon: &str, mask: bool) -> UiEvent {
        UiEvent::ShowToast { title: "t".into(), icon: icon.into(), duration: 1500, mask, image: String::new() }
    }

    #[test]
    fn toast_与_loading_共用一个槽位() {
        let (mut t, mut l, mut m) = (None, None, None);
        apply_feedback_event(UiEvent::ShowLoading { title: "加载中".into(), mask: false }, &mut t, &mut l, &mut m);
        apply_feedback_event(show_toast("success", false), &mut t, &mut l, &mut m);
        assert!(t.is_some() && l.is_none(), "showToast 顶掉 Loading");
        apply_feedback_event(UiEvent::ShowLoading { title: "x".into(), mask: false }, &mut t, &mut l, &mut m);
        assert!(t.is_none() && l.is_some(), "showLoading 顶掉 Toast");
        // 混用：hideToast 也能关 Loading
        apply_feedback_event(UiEvent::HideToast { no_conflict: false }, &mut t, &mut l, &mut m);
        assert!(l.is_none());
        // noConflict：hideToast 不碰 Loading
        apply_feedback_event(UiEvent::ShowLoading { title: "x".into(), mask: false }, &mut t, &mut l, &mut m);
        apply_feedback_event(UiEvent::HideToast { no_conflict: true }, &mut t, &mut l, &mut m);
        assert!(l.is_some());
    }

    #[test]
    fn 只有带_mask_的提示框挡触摸() {
        let (mut t, mut l, mut m) = (None, None, None);
        apply_feedback_event(UiEvent::ShowLoading { title: "x".into(), mask: false }, &mut t, &mut l, &mut m);
        assert!(!tip_blocks_input(&t, &l), "不带 mask 的 Loading 可以点穿");
        apply_feedback_event(UiEvent::ShowLoading { title: "x".into(), mask: true }, &mut t, &mut l, &mut m);
        assert!(tip_blocks_input(&t, &l));
        apply_feedback_event(show_toast("none", true), &mut t, &mut l, &mut m);
        assert!(tip_blocks_input(&t, &l), "带 mask 的 Toast 也挡");
        apply_feedback_event(show_toast("none", false), &mut t, &mut l, &mut m);
        assert!(!tip_blocks_input(&t, &l));
    }

    #[test]
    fn 弹窗颜色按逻辑层给的来_坏值用缺省() {
        let (mut t, mut l, mut m) = (None, None, None);
        let ev = UiEvent::ShowModal {
            title: "a".into(),
            content: "b".into(),
            show_cancel: true,
            cancel_text: "取消".into(),
            confirm_text: "确定".into(),
            cancel_color: "#f00".into(),
            confirm_color: "nope".into(),
        };
        apply_feedback_event(ev, &mut t, &mut l, &mut m);
        let m = m.unwrap();
        assert_eq!((m.cancel_color.r, m.cancel_color.g, m.cancel_color.b), (255, 0, 0));
        assert_eq!((m.confirm_color.r, m.confirm_color.g, m.confirm_color.b), (0x57, 0x6B, 0x95));
    }

    #[test]
    fn 转圈的帧闸门_同一步只合成一次() {
        let (mut t, mut l, mut m) = (None, None, None);
        apply_feedback_event(UiEvent::ShowLoading { title: "加载中".into(), mask: false }, &mut t, &mut l, &mut m);
        let (w, h) = (750u32, 1334u32);
        let mut buf = vec![0xF5F5F5u32; (w * h) as usize];
        assert!(tip_needs_recompose(0.10, &t, &l));
        render_ui_overlay(&mut buf, w, h, 2.0, 0.10, &t, &l, &m, None);
        assert!(!tip_needs_recompose(0.105, &t, &l), "还在第 6 步");
        assert!(tip_needs_recompose(0.12, &t, &l), "到第 7 步了");
        // 收起之后不再要帧
        apply_feedback_event(UiEvent::HideLoading { no_conflict: false }, &mut t, &mut l, &mut m);
        assert!(!tip_needs_recompose(0.5, &t, &l));
    }
}
