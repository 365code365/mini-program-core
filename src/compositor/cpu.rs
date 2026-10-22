//! CPU 合成器：参考实现，永远可用、永远不删。
//!
//! 合成顺序（从下到上）与原来四个宿主调用点里抄的那一串**完全相同**：
//!
//! 1. 页面 → tabBar → fixed 覆盖层（`present_to_buffer`）
//! 2. 下拉刷新指示器：画在页面之上、Toast/Modal 之下
//! 2½. 全屏视频（没有全屏时什么也不写，见 `components::video::fullscreen`），
//!    然后图片预览与操作菜单（`ui_overlay::render_sheet_overlays`，同样没弹着就不写）
//! 3. Modal（连同遮罩），其上是 Toast / Loading 提示框
//! 4. picker 面板：在所有覆盖层之上
//! 5. 左边缘侧滑返回：整帧右移、左边露出上一页。放在最后 —— 被推走的是「整个页面」，
//!    页面自己的弹窗、Toast 跟着一起走才对，不然弹窗会诡异地钉在原处
//!
//! 每一步调用的函数本身一行没改，只是换了调用者。`Option` 为 `None` 的层跳过；
//! `render_ui_overlay` 在三个状态都是 `None` 时什么也不写，所以只要页面那一层的
//! 调用方（侧滑留图）照样传 [`OverlayRefs::none`](super::OverlayRefs::none)，结果与只调
//! `present_to_buffer` 逐字节相同。
//!
//! **不初始化输出缓冲**：tabBar 画布比 `tabbar_px` 矮时，那几行 `present_to_buffer`
//! 不写，保留缓冲原值。原来各调用点的初值各不相同（softbuffer 借出的缓冲、快照的 0、
//! 侧滑留图的页面底色），这里不能替它们决定。
use super::{FrameLayers, Footprint, LayerCanvas};
use crate::host::render::{present_to_buffer, render_pull_indicator};
use crate::host::{edge_back, picker_sheet, ui_overlay};

/// 把 `layers` 合成进 `buf`（0x00RRGGBB，行宽 = `layers.width`）。
///
/// 返回覆盖层（第 2½ 步起）写了哪些像素：GPU 直通帧据此只传那几行（见 [`Footprint`]）。
/// 页面 / tabBar / fixed / 导航栏 / 下拉指示器不算在内 —— 那几层 GPU 自己会合成
pub fn compose(buf: &mut [u32], layers: &FrameLayers<'_>) -> Footprint {
    compose_rows(buf, layers, (0, layers.height))
}

/// 只合成 `rows` 那几行 [a, b)，其余行不写（保留缓冲原值）。(0, 高) 就是 [`compose`]。
///
/// GPU 直通帧用：覆盖层只占屏幕一截时，CPU 只合成那一截（[`overlay_reach`] 给出），
/// 其余行 GPU 自己合成再查表压暗。一加 PLY110 上操作菜单整帧合成要 5~8ms，大半花在
/// 「整屏拷页面 + 整屏查表压暗」上，而真正要的只有面板那三成。
///
/// 只对「每一步都逐行独立」的情形成立：下拉指示器、全屏视频、侧滑会让 [`overlay_reach`]
/// 返回整帧，这里照常整帧画它们
pub fn compose_rows(buf: &mut [u32], layers: &FrameLayers<'_>, rows: (u32, u32)) -> Footprint {
    let (w, h) = (layers.width, layers.height);
    present_to_buffer(
        buf,
        w,
        h,
        layers.page.read(),
        layers.fixed.map(LayerCanvas::read),
        layers.tabbar.map(LayerCanvas::read),
        layers.scroll_px,
        layers.has_tabbar,
        layers.tabbar_px,
        layers.fixed_rows,
        layers.page_bg,
        layers.top.as_ref().map(|t| t.rows).unwrap_or(0),
        rows,
    );
    // 顶部宿主外壳（导航栏）：盖在页面与 fixed 层之上、所有浮层之下 —— Modal / picker 的遮罩
    // 连导航栏一起压暗（与微信一致：弹窗是盖住整个窗口的）
    if let Some(t) = &layers.top {
        paint_top_band(buf, w, h, t.rows, t.canvas.read(), rows);
    }
    if let Some(p) = layers.pull {
        render_pull_indicator(
            buf,
            w,
            h,
            layers.scale,
            p.gap_logical,
            p.refreshing,
            p.progress,
            p.phase,
        );
    }
    // 全屏视频（VideoContext.requestFullScreen）：盖住页面各层，Toast / Modal / picker 仍在它上面
    let mut fp = if crate::renderer::components::fullscreen_video().is_some() { Footprint::full() } else { Footprint::NONE };
    crate::renderer::components::draw_fullscreen(buf, w, h, layers.scale);
    let o = &layers.overlays;
    // 图片预览（wx.previewImage）与操作菜单（wx.showActionSheet）：没有弹着时什么也不写
    fp.then(ui_overlay::render_sheet_overlays(buf, w, h, layers.scale, o.text, rows));
    fp.then(ui_overlay::render_ui_overlay(
        buf,
        w,
        h,
        layers.scale,
        o.anim_secs,
        o.toast,
        o.loading,
        o.modal,
        o.text,
        rows,
    ));
    if let Some(sheet) = o.picker {
        fp.then(picker_sheet::render(buf, w, h, layers.scale, sheet, o.text, rows));
    }
    if let Some(eb) = &layers.edge_back {
        // 侧滑 / 转场把整帧连同覆盖层一起平移：覆盖层画过东西的话行区间就不成立了。
        // 只有侧滑没有覆盖层时 GPU 着色器自己做（不走直通），这里不用记
        if !fp.is_none() {
            fp = Footprint::full();
        }
        if eb.shot_on_top {
            edge_back::compose_shot_over(buf, w, h, eb.offset_px, eb.shot, eb.progress);
        } else {
            edge_back::compose(buf, w, h, eb.offset_px, eb.shot, eb.progress);
        }
    }
    fp
}

/// 画之前算出的本帧覆盖层范围：`(会写到的行, 其中要先有页面底图的行)`。
/// 后者之外、前者之内的行整行被不透明面板盖掉（操作菜单的白底），不用先合成页面。
/// `None`：没有覆盖层，或者说不清（图片预览、全屏视频、下拉指示器、侧滑），要整帧合成。
///
/// 操作菜单按这一刻定住时钟（紧接着的合成按同一刻画）；picker 面板给的是不看时间的最大范围
/// （完全展开时顶边以下，入场 / 退场途中只会更低）
pub fn overlay_reach(layers: &FrameLayers<'_>) -> Option<((u32, u32), (u32, u32))> {
    if layers.pull.is_some() || layers.edge_back.is_some() || crate::renderer::components::fullscreen_video().is_some() {
        return None;
    }
    let (w, h, sf, o) = (layers.width, layers.height, layers.scale, &layers.overlays);
    let (mut rows, mut base) = ui_overlay::overlay_reach(w, h, sf, o.toast, o.loading, o.modal, o.text);
    if let Some(sheet) = o.picker {
        rows.then(picker_sheet::reach(h, sf, sheet));
        // picker 的遮罩画在最上面，要压到下面每一层写过的行（同 Modal，见 ui_overlay::overlay_reach）
        base = rows;
    }
    if rows.full || base.full {
        return None;
    }
    let r = rows.rows?;
    // 底图行只在覆盖层范围里有意义（并集可能把两段之间的空隙也算上：多合成几行无妨）
    let b = base.rows.map(|(a, b)| (a.max(r.0), b.min(r.1).max(a.max(r.0)))).unwrap_or((r.0, r.0));
    Some((r, b))
}

/// 导航栏画布的前 `rows` 行按 alpha 叠进缓冲（导航栏底色不透明时就是整行拷贝），只写 `win` 那几行
fn paint_top_band(buf: &mut [u32], w: u32, h: u32, rows: u32, band: &crate::Canvas, win: (u32, u32)) {
    let draw_w = (w as usize).min(band.width() as usize);
    let src_w = band.width() as usize;
    let px = band.pixels();
    for y in win.0 as usize..rows.min(h).min(band.height()).min(win.1) as usize {
        let src = &px[y * src_w..y * src_w + draw_w];
        let dst = &mut buf[y * w as usize..y * w as usize + draw_w];
        for (d, c) in dst.iter_mut().zip(src) {
            match c.a {
                0 => {}
                255 => *d = ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32,
                a => {
                    let (a, inv) = (a as u32, 255 - a as u32);
                    let p = *d;
                    let r = (c.r as u32 * a + ((p >> 16) & 0xFF) * inv) / 255;
                    let g = (c.g as u32 * a + ((p >> 8) & 0xFF) * inv) / 255;
                    let b = (c.b as u32 * a + (p & 0xFF) * inv) / 255;
                    *d = (r << 16) | (g << 8) | b;
                }
            }
        }
    }
}
