//! 合成层：把一帧的各个图层按固定顺序合成到输出缓冲。
//!
//! 内容（布局、命中、事件、`setData` 增量失效、动画时钟、光栅化）都在别处，
//! 而且与后端无关；这里只拿到**本帧各图层的只读引用**（[`FrameLayers`]），
//! 拿不到 `MiniApp` / `InteractionManager` / `ScrollController`。所以「合成后端
//! 不碰布局、命中、事件」是类型层面保证的，不靠约定（设计见
//! `.agents/tasks/gpu-dual-backend-2026-10-12/design.md` §2）。
//!
//! 现在只有 CPU 合成器 [`cpu::compose`]：它是参考实现，永远可用。
//!
//! ## 为什么字段与 `present_to_buffer` 的参数一一对应
//! 合成序列原来在四个宿主调用点各抄了一份（桌面上屏、无头快照、侧滑留图、SDK）。
//! 把它们收拢成「构造 `FrameLayers` → 调 `cpu::compose`」时，字段只是把原来的实参
//! 原样装进来，宿主怎么算（截断方式、f32/f64、相位时钟）都不变，逐字节可证。
pub mod cpu;

use crate::host::edge_back::PageShot;
use crate::host::picker_sheet::PickerSheetState;
use crate::host::ui_overlay::{LoadingState, ModalState, ToastState};
use crate::text::TextRenderer;
use crate::Canvas;
use std::time::Instant;

/// 本帧要合成的全部图层（只读引用 + 覆盖层状态）
pub struct FrameLayers<'a> {
    /// 输出缓冲的像素尺寸（窗口 / 引擎像素尺寸）
    pub width: u32,
    pub height: u32,
    /// 设备像素比（覆盖层、下拉指示器按它缩放）
    pub scale: f32,
    /// 页面画布（内容坐标，整页高）
    pub page: &'a Canvas,
    /// 页面滚动偏移（设备像素）。由宿主按 `(scroll * scale) as i32` 截断，这里不再取整
    pub scroll_px: i32,
    /// 画布之外区域的填充色（0xRRGGBB，取 `page { background-color }`）
    pub page_bg: u32,
    /// 当前页是否是 tabBar 页。与 `tabbar` 分开：`present_to_buffer` 在 tabBar 页上
    /// 即使画布缺失也会扣掉 `tabbar_px` 的高度
    pub has_tabbar: bool,
    pub tabbar: Option<&'a Canvas>,
    /// tabBar 占用的设备像素高度（非 tabBar 页为 0）
    pub tabbar_px: u32,
    /// fixed 覆盖层画布（与视口同大）
    pub fixed: Option<&'a Canvas>,
    /// fixed 覆盖层真正画过的行区间（含端点）；`None` 表示整层透明、跳过
    pub fixed_rows: Option<(u32, u32)>,
    /// 下拉刷新指示器。宿主只在「顶部露出空白 > 0.5 或正在刷新」时给 `Some`
    pub pull: Option<PullState>,
    /// Toast / Loading / Modal 与 picker 面板
    pub overlays: OverlayRefs<'a>,
    /// 左边缘侧滑返回进行中（只有桌面窗体有这一层）
    pub edge_back: Option<EdgeBackFrame<'a>>,
}

/// 下拉刷新指示器的参数（与 `render_pull_indicator` 的参数一一对应）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PullState {
    /// 顶部露出的空白高度（逻辑像素）
    pub gap_logical: f32,
    pub refreshing: bool,
    /// 下拉进度 0~1
    pub progress: f32,
    /// 刷新期间三点呼吸的相位 0~1（快照固定成 0.25，交互窗体按进程时钟走）
    pub phase: f32,
}

/// 直接画进输出缓冲的覆盖层（Toast / Loading / Modal、picker 面板）
pub struct OverlayRefs<'a> {
    /// Loading 动画的时钟基准（与 `render_ui_overlay` 的同名参数一致）
    pub last_frame: Instant,
    pub toast: &'a Option<ToastState>,
    pub loading: &'a Option<LoadingState>,
    pub modal: &'a Option<ModalState>,
    pub picker: Option<&'a PickerSheetState>,
    pub text: Option<&'a TextRenderer>,
}

impl OverlayRefs<'static> {
    /// 没有任何覆盖层。侧滑留图要的是「页面 + tabBar + fixed」那一张，不带弹窗。
    pub fn none(last_frame: Instant) -> Self {
        Self { last_frame, toast: &None, loading: &None, modal: &None, picker: None, text: None }
    }
}

/// 侧滑返回这一帧的参数（与 `edge_back::compose` 的参数一一对应）
pub struct EdgeBackFrame<'a> {
    /// 当前页右移的距离（设备像素）
    pub offset_px: u32,
    /// 被覆盖的上一页留下的视口图
    pub shot: Option<&'a PageShot>,
    /// 0~1 的推出进度
    pub progress: f32,
}
