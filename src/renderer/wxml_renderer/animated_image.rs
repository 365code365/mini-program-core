//! 动图（GIF）的续帧：页面画布上每张动图画的是第几帧、什么时候该换下一帧。
//!
//! GIF 的当前帧按墙上时钟取（`components::image::load_image`），但只有「有人重画它那块」
//! 时才生效。从前没有任何东西为它重画：页面静止时宿主停帧；滑动时 SDK 只合成、或只补
//! 新露出的条带（`host/engine_damage.rs`），已经画好的动图就停在最后一次被画到的那一帧
//! —— Flutter 宿主里表现为「一滑 GIF 就停」，桌面窗体上静止页面的 GIF 也根本不动。
//!
//! ## 为什么不直接 `mark_animating()` 走 CSS 动画那条路
//! 那条路每个 vsync 都重画动画盒。`loading.gif` 一帧 120ms，144Hz 屏上 17 次重画里有
//! 16 次画的还是同一帧，滑动时还要每帧多走一趟节点树。这里记下「画布上是第几帧」，
//! 宿主每帧问一次 [`WxmlRenderer::animated_images_due`]，只重画帧号变了的那几张，
//! 其余帧照旧只合成 / 空转。
//!
//! ## 记录什么时候算数
//! - 只有**整张（夹进这一趟的裁剪窗后）都落在损伤区里**才算画上了新帧；只走到没画、
//!   或只画了一部分的，沿用上一次的记录，没有记录就记成「未知」（下一帧必重画它）。
//!   损伤区帧的绘制判据带 400px 余量（`outside_damage`），「发起了绘制」不等于「像素换了」。
//! - 帧号在**绘制之前**取：画的那一刻要是正好跨了帧，记下的是旧帧号，下一帧多重画一次；
//!   反过来（画完再取）会记成新帧号而画布上是旧帧，要等再下一次换帧才补上。
//!   第一次画时动图还没解码（解码就发生在这次绘制里），只能画完再取 —— 刚解码完是第 0 帧。
//! - 一趟只走「视口 ± 裁剪余量」：窗里的记录整体换成这一趟的，窗外的原样留着 ——
//!   桌面窗体的已绘制条带可能比当前裁剪窗长，滑回去时还要靠它们判换帧。
//! - 画在离屏画布（transform / opacity 组合成）或 fixed 覆盖层上的：坐标不是页面画布坐标，
//!   退回 CSS 动画那条路（`mark_animating`，宿主按刷新率重画），正确但不省。
//! - 顶层 scroll-view 的离屏缓存只在数据变了时重画，里面的动图会冻住：子树里有动图的
//!   顶层 scroll-view 改走逐帧画可见子节点那条路（`scroll_live.rs`，与子树里有 swiper 同一条）。

use super::*;

/// 画布上那块的帧号不确定（只画了一部分 / 只走到没画）：下一次一定判成「该换帧」
const UNKNOWN_FRAME: u32 = u32::MAX;

/// 一张画进页面画布的动图
#[derive(Clone, Debug)]
pub(super) struct AnimatedImageMark {
    src: String,
    /// 物理像素、页面画布坐标，外沿取整到整像素（重画时整块清掉，不留半像素的旧边）
    rect: GeoRect,
    /// 画布上这块现在是第几帧
    frame: u32,
}

/// `inner` 是否整块落在 `outer` 里（半个像素的浮点误差算在里面）
fn contains(outer: &GeoRect, inner: &GeoRect) -> bool {
    const EPS: f32 = 0.5;
    inner.x >= outer.x - EPS
        && inner.y >= outer.y - EPS
        && inner.x + inner.width <= outer.x + outer.width + EPS
        && inner.y + inner.height <= outer.y + outer.height + EPS
}

/// 纵向夹进 `[y0, y1)`，夹完是空的返回 None
fn clamp_y(r: &GeoRect, y0: f32, y1: f32) -> Option<GeoRect> {
    let (a, b) = (r.y.max(y0), (r.y + r.height).min(y1));
    (b > a && r.width > 0.0).then(|| GeoRect::new(r.x, a, r.width, b - a))
}

impl WxmlRenderer {
    /// 这个节点若是已解码的动图，取它此刻该显示的帧号（绘制**之前**调，原因见模块注释）
    pub(super) fn animated_image_frame(node: &RenderNode) -> Option<u32> {
        if node.tag != "image" {
            return None;
        }
        node.attrs
            .get("src")
            .filter(|s| !s.is_empty())
            .and_then(|s| crate::renderer::components::animated_frame(s))
    }

    /// 带交互的两条绘制路径上，每个节点走到时调一次（画没画都调：只走不画的也要记）。
    /// `before`：绘制前取的帧号
    pub(super) fn note_animated_image(&mut self, node: &RenderNode, before: Option<u32>, (x, y, w, h): (f32, f32, f32, f32)) {
        if node.tag != "image" || w <= 0.0 || h <= 0.0 {
            return;
        }
        let Some(frame) = before.or_else(|| Self::animated_image_frame(node)) else { return };
        if self.drawing_offscreen || self.registering_fixed {
            self.mark_animating();
            return;
        }
        let Some(src) = node.attrs.get("src") else { return };
        let (x0, y0) = (x.floor(), y.floor());
        let rect = GeoRect::new(x0, y0, (x + w).ceil() - x0, (y + h).ceil() - y0);
        let painted = match self.damage_clip {
            None => true,
            // 宿主给的损伤区是夹进裁剪窗、夹进画布宽度的：拿同样夹过的那块来判
            Some(d) => {
                let (wy0, wy1) = self.gif_window;
                self.visible_part(&rect, wy0, wy1).map_or(false, |v| contains(&d, &v))
            }
        };
        let frame = if painted {
            frame
        } else {
            let same = |m: &&AnimatedImageMark| m.src == *src && m.rect == rect;
            self.gif_marks.iter().find(same).map(|m| m.frame).unwrap_or(UNKNOWN_FRAME)
        };
        self.gif_pass.push(AnimatedImageMark { src: src.clone(), rect, frame });
    }

    /// 页面那一趟开始：记下这一趟走的纵向窗口（与 `cull_outside_viewport` 同一个算式）
    pub(super) fn begin_animated_image_pass(&mut self, scroll_offset: f32, viewport_height_px: f32) {
        self.gif_pass.clear();
        self.gif_window = if viewport_height_px <= 0.0 {
            // 不按视口裁剪的那一趟（视口高为 0）：整棵树都走到了
            (f32::NEG_INFINITY, f32::INFINITY)
        } else {
            let top = scroll_offset * self.scale_factor;
            (top - VIEWPORT_CULL_MARGIN_PX, top + viewport_height_px + VIEWPORT_CULL_MARGIN_PX)
        };
    }

    /// 页面那一趟结束：窗里的记录换成这一趟的，窗外的留着
    pub(super) fn finish_animated_image_pass(&mut self) {
        let (y0, y1) = self.gif_window;
        self.gif_marks.retain(|m| clamp_y(&m.rect, y0, y1).is_none());
        self.gif_marks.append(&mut self.gif_pass);
    }

    /// 夹进画布宽度与纵向 `[y0, y1)` 之后还剩的那块
    fn visible_part(&self, r: &GeoRect, y0: f32, y1: f32) -> Option<GeoRect> {
        let cw = self.screen_width * self.scale_factor;
        let (x0, x1) = (r.x.max(0.0), (r.x + r.width).min(cw));
        clamp_y(r, y0, y1).filter(|_| x1 > x0).map(|v| GeoRect::new(x0, v.y, x1 - x0, v.height))
    }

    /// 换了帧的动图（物理像素、页面画布坐标，夹进画布宽度与纵向 `[y0, y1)` —— 传宿主的裁剪窗）。
    /// 宿主把它们当损伤区重画；重画那一趟会记下新帧号，下一次换帧之前不会再报。
    pub fn animated_images_due(&self, y0: f32, y1: f32) -> Vec<GeoRect> {
        self.gif_marks
            .iter()
            .filter(|m| {
                // 被逐出动图缓存的（取不到帧号）不算：下一次画到它时会重新解码、重新登记
                crate::renderer::components::animated_frame(&m.src).map_or(false, |f| f != m.frame)
            })
            .filter_map(|m| self.visible_part(&m.rect, y0, y1))
            .collect()
    }

    /// 裁剪窗里有没有动图。有的话宿主要按刷新率醒着等它换帧
    /// （大多数帧问下来不用重画，见 [`Self::animated_images_due`]）。
    pub fn has_animated_images(&self, y0: f32, y1: f32) -> bool {
        self.gif_marks.iter().any(|m| {
            self.visible_part(&m.rect, y0, y1).is_some()
                && crate::renderer::components::animated_frame(&m.src).is_some()
        })
    }

    /// 顶层 scroll-view 的子树里有没有已解码的动图（有就不走离屏缓存，见模块注释）
    pub(super) fn subtree_has_animated_image(node: &RenderNode) -> bool {
        node.children
            .iter()
            .any(|c| Self::animated_image_frame(c).is_some() || Self::subtree_has_animated_image(c))
    }
}
