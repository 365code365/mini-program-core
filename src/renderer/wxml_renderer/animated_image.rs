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
//!
//! ## 同层渲染的 `<video>` 也走这里
//! 宿主解码、帧交给引擎（`components::video::host_feed`）：宿主每交来一帧、或播放状态变了
//! （控制条的播放键 / 进度），版本号加一，只重画那个视频框 —— 与动图换帧同一回事。
//! 区别只有一条：视频不让宿主「醒着等」（[`WxmlRenderer::has_animated_images`] 不算它），
//! 新帧到了宿主自己会要一帧，暂停着的视频不该让引擎每个 vsync 空转。
use super::*;

/// 画布上那块的帧号不确定（只画了一部分 / 只走到没画）：下一次一定判成「该换帧」
const UNKNOWN_FRAME: u32 = u32::MAX;

/// 一张画进页面画布的动图（或同层渲染的视频）
#[derive(Clone, Debug)]
pub(super) struct AnimatedImageMark {
    /// 动图是图片地址；视频是 `host_feed::video_key`
    src: String,
    /// 同层渲染的视频（帧号问 `host_feed::seq`，不问动图缓存）或通用同层组件：宿主交帧，不让宿主醒着等
    video: bool,
    /// 通用同层组件（`<map>` 这类，帧号问 `components::host_view`）。为真时 `video` 也为真
    view: bool,
    /// 物理像素、页面画布坐标，外沿取整到整像素（重画时整块清掉，不留半像素的旧边）
    rect: GeoRect,
    /// 画布上这块现在是第几帧
    frame: u32,
}

impl AnimatedImageMark {
    /// 此刻该显示的帧号（被逐出动图缓存的 / 同层渲染关了的取不到）
    fn current(&self) -> Option<u32> {
        if self.view {
            crate::renderer::components::host_view::seq_of(&self.src)
        } else if self.video {
            // 帧 / 播放状态 + 控制条的显隐与拖动（全屏中的那个冻住）
            crate::renderer::components::video_ui::mark_seq(&self.src)
        } else {
            crate::renderer::components::animated_frame(&self.src)
        }
    }
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

/// 这个节点按「动图」记的话，它的身份（地址 / 视频键 / 同层组件键）、是不是宿主交帧、是不是通用同层组件
fn ident(node: &RenderNode) -> Option<(String, bool, bool)> {
    use crate::renderer::components::host_view;
    match node.tag.as_str() {
        "image" => node.attrs.get("src").filter(|s| !s.is_empty()).map(|s| (s.clone(), false, false)),
        "video" if crate::renderer::components::host_feed::host_feeds_video() => {
            Some((crate::renderer::components::host_feed::video_key(&node.attrs), true, false))
        }
        t if host_view::is_same_layer(t) => Some((host_view::key(t, &node.attrs), true, true)),
        _ => None,
    }
}

impl WxmlRenderer {
    /// 这个节点若是已解码的动图（或同层渲染的视频），取它此刻该显示的帧号（绘制**之前**调，原因见模块注释）
    pub(super) fn animated_image_frame(node: &RenderNode) -> Option<u32> {
        match node.tag.as_str() {
            "image" => node
                .attrs
                .get("src")
                .filter(|s| !s.is_empty())
                .and_then(|s| crate::renderer::components::animated_frame(s)),
            "video" => crate::renderer::components::video_ui::mark_seq(&crate::renderer::components::host_feed::video_key(&node.attrs)),
            // 同层渲染的 `<map>` 这类：宿主交来的帧号（components::host_view）。先判标签再拼键：
            // 每画一个节点都走这里，不能为每个 view / text 分配一个字符串
            t if crate::renderer::components::host_view::is_same_layer(t) => {
                crate::renderer::components::host_view::seq(t, &crate::renderer::components::host_view::key(t, &node.attrs))
            }
            _ => None,
        }
    }

    /// 带交互的两条绘制路径上，每个节点走到时调一次（画没画都调：只走不画的也要记）。
    /// `before`：绘制前取的帧号
    pub(super) fn note_animated_image(&mut self, node: &RenderNode, before: Option<u32>, (x, y, w, h): (f32, f32, f32, f32)) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let Some((src, video, view)) = ident(node) else { return };
        let Some(frame) = before.or_else(|| Self::animated_image_frame(node)) else { return };
        if self.drawing_offscreen || self.registering_fixed {
            self.mark_animating();
            return;
        }
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
            let same = |m: &&AnimatedImageMark| m.src == src && m.rect == rect;
            self.gif_marks.iter().find(same).map(|m| m.frame).unwrap_or(UNKNOWN_FRAME)
        };
        self.gif_pass.push(AnimatedImageMark { src, video, view, rect, frame });
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

    /// 换了帧的动图 / 视频（物理像素、页面画布坐标，夹进画布宽度与纵向 `[y0, y1)` —— 传宿主的裁剪窗）。
    /// 宿主把它们当损伤区重画；重画那一趟会记下新帧号，下一次换帧之前不会再报。
    pub fn animated_images_due(&self, y0: f32, y1: f32) -> Vec<GeoRect> {
        self.gif_marks
            .iter()
            // 被逐出动图缓存的（取不到帧号）不算：下一次画到它时会重新解码、重新登记
            .filter(|m| m.current().map_or(false, |f| f != m.frame))
            .filter_map(|m| self.visible_part(&m.rect, y0, y1))
            .collect()
    }

    /// 裁剪窗里有没有动图。有的话宿主要按刷新率醒着等它换帧
    /// （大多数帧问下来不用重画，见 [`Self::animated_images_due`]）。视频不算：新帧到了宿主自己会要一帧
    pub fn has_animated_images(&self, y0: f32, y1: f32) -> bool {
        self.gif_marks.iter().any(|m| !m.video && self.visible_part(&m.rect, y0, y1).is_some() && m.current().is_some())
    }

    /// 顶层 scroll-view 的子树里有没有已解码的动图（有就不走离屏缓存，见模块注释）
    pub(super) fn subtree_has_animated_image(node: &RenderNode) -> bool {
        node.children
            .iter()
            .any(|c| Self::animated_image_frame(c).is_some() || Self::subtree_has_animated_image(c))
    }
}
