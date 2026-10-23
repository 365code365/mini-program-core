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
    // 只弹着操作菜单：白底把屏幕下面那一截整行盖死，那几行不用先合成页面、再压暗（贴面板时整行覆盖）；
    // 上面那截的底图留一份，各层没变就不再拼（sheet_base.rs）。一加 PLY110 的 CPU 后端上，
    // 弹出那 0.3s 每帧都是整屏合成 + 整屏压暗
    match sheet_cover_from(layers) {
        Some(end) => super::sheet_base::compose(buf, layers, end),
        None => {
            super::sheet_base::release();
            compose_rows(buf, layers, (0, layers.height))
        }
    }
}

/// 操作菜单是这一帧唯一的覆盖层时，面板从第几行起把整行盖死（这一行往下合成页面也会被盖掉）；
/// 面板整个在屏幕外（入场第一帧、收起最后几帧）时是屏幕高 —— 那一帧照样走底图缓存，
/// 入场第一帧拼好的底图留给后面几帧（面板缓存也在这一帧画，两样不挤在第一个看得见的帧里）。
/// 与 GPU 直通帧同一份范围（[`overlay_reach`]：它顺带定住这一帧的时刻，紧接着的合成按同一刻画）；
/// 逐字节相同由 `action_sheet` 的「分窗与整帧逐字节相同」、`cpu_compose_skips_rows_under_sheet`
/// 与 `sheet_base` 的逐帧对照守着
pub(super) fn sheet_cover_from(layers: &FrameLayers<'_>) -> Option<u32> {
    if !sheet_only(layers) {
        return None;
    }
    let h = layers.height;
    Some(match overlay_reach(layers) {
        Some((rows, base)) if rows.1 == h => base.1,
        _ => h,
    })
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
    // 全屏视频把每个像素都写一遍：底下的页面 / 导航栏 / 下拉指示器不用先拼出来（拼了也被盖掉，
    // 一加上整屏拷一遍页面要好几毫秒，全屏时每来一帧视频都要合成一次）
    let covered = crate::renderer::components::fullscreen_video().is_some();
    if !covered {
        compose_page(buf, layers, rows);
    }
    // 全屏视频（控制条的全屏键 / requestFullScreen）：盖住页面各层，Toast / Modal / picker 仍在它上面
    let o = &layers.overlays;
    let mut fp = if covered { Footprint::full() } else { Footprint::NONE };
    crate::renderer::components::draw_fullscreen(buf, w, h, layers.scale, o.text);
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

/// 第 1、2 步：页面 → tabBar → fixed 覆盖层、顶部导航栏、下拉刷新指示器（从 `compose_rows` 原样搬出）
pub(super) fn compose_page(buf: &mut [u32], layers: &FrameLayers<'_>, rows: (u32, u32)) {
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

/// 操作菜单是这一帧唯一的覆盖层、而且面板缓存在：GPU 可以把面板当纹理挪（见 `ui_overlay::sheet_raster`）。
/// 返回 `(顶边整像素行, 圆角行数, 缓存行数, 缓存 id)`。要在 [`overlay_reach`] 之后调（它定住这一帧的位置）
pub fn sheet_layer(layers: &FrameLayers<'_>) -> Option<(i32, u32, u32, u64)> {
    if !sheet_only(layers) {
        return None;
    }
    ui_overlay::action_sheet::gpu_layer()
}

/// 这一帧弹着的只有操作菜单（没有别的覆盖层、侧滑、下拉、全屏视频）
fn sheet_only(layers: &FrameLayers<'_>) -> bool {
    let o = &layers.overlays;
    let others = layers.pull.is_some()
        || layers.edge_back.is_some()
        || crate::renderer::components::fullscreen_video().is_some()
        || o.picker.is_some()
        || o.modal.is_some()
        || o.toast.is_some()
        || o.loading.is_some()
        || ui_overlay::image_preview::visible();
    !others && ui_overlay::action_sheet::visible()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::{OverlayRefs, PageCanvas, TopBand};
    use crate::host::ui_overlay::action_sheet;
    use crate::{Canvas, Color};

    fn noise(c: &mut Canvas, seed: u32, alpha: impl Fn(usize) -> u8) {
        for (i, p) in c.pixels_mut().iter_mut().enumerate() {
            let v = (i as u32 ^ seed).wrapping_mul(2_654_435_761);
            *p = Color::new((v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8, alpha(i));
        }
    }

    /// 只弹着操作菜单时，CPU 合成不碰面板白底整行盖死的那几行（页面不拷、不压暗）：入场 / 退场任何时刻
    /// 都与整帧合成逐字节相同。缓冲先填一个怪色 —— 跳过的行要是没被面板盖满，怪色会漏出来
    #[test]
    fn cpu_compose_skips_rows_under_sheet() {
        let (w, h, sf) = (120u32, 640u32, 2.0f32);
        let mut page = Canvas::new(w, 1500);
        noise(&mut page, 0x51, |_| 255);
        let mut tab = Canvas::new(w, 50);
        noise(&mut tab, 0x7A, |_| 255);
        let mut fixed = Canvas::new(w, h);
        noise(&mut fixed, 0x3C, |i| [0, 255, 128][i % 3]);
        let mut band = Canvas::new(w, 64);
        noise(&mut band, 0x9E, |i| if i % 5 == 0 { 90 } else { 255 });
        let (page, tab, fixed, band) = (PageCanvas::new(page), LayerCanvas::new(tab), LayerCanvas::new(fixed), LayerCanvas::new(band));
        let layers = FrameLayers {
            width: w,
            height: h,
            scale: sf,
            page: &page,
            scroll_px: 300,
            page_bg: 0x12_34_56,
            has_tabbar: true,
            tabbar: Some(&tab),
            tabbar_px: 50,
            fixed: Some(&fixed),
            fixed_rows: Some((100, 560)),
            pull: None,
            overlays: OverlayRefs::none(),
            edge_back: None,
            top: Some(TopBand { rows: 60, canvas: &band }),
        };
        let mut skipped_any = false;
        for (ago, closing) in [(0, false), (30, false), (120, false), (900, false), (40, true), (200, true)] {
            action_sheet::open_at(ago, closing);
            // 两次合成按同一刻画（动画按帧时钟取样）
            crate::frame_clock::set(Some(std::time::Instant::now()));
            let end = sheet_cover_from(&layers);
            skipped_any |= end.is_some_and(|e| e < h);
            // 两边同一个初值（导航栏半透明的那几行本来就叠在缓冲原值上）
            let mut got = vec![0x00AB_CDEF; (w * h) as usize];
            compose(&mut got, &layers);
            let mut want = vec![0x00AB_CDEF; (w * h) as usize];
            compose_rows(&mut want, &layers, (0, h));
            let bad = got.iter().zip(&want).position(|(a, b)| a != b);
            assert!(bad.is_none(), "{ago}ms{}：跳过的行与整帧合成不同 @{:?}（从第 {end:?} 行起跳过）", if closing { " 收起" } else { "" }, bad.map(|i| (i as u32 % w, i as u32 / w)));
            crate::frame_clock::set(None);
        }
        assert!(skipped_any, "一次都没跳过：测的不是那条路");
        action_sheet::clear();
    }
}
