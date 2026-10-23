//! SDK 出帧的「这一帧要重画多少」：只合成 / 局部重画 / 补条带 / 整帧，以及每种的归因。
//!
//! steering 10 要求「凡是让某一帧退回整屏重绘的路径，都要能说清为什么」。桌面窗体的
//! `MINI_FPS` 早有整帧归因，SDK 一直只有一个 `needs_redraw`：Flutter 宿主里滑动掉帧时，
//! 连「是动画逼的、setData 逼的、还是滑出条带逼的」都分不出来。
//!
//! ## 为什么 SDK 要自己做一遍（桌面窗体早就有）
//! 帧调度是两边各写的（桌面 `app_window/frame*.rs`，SDK `engine_frame.rs`）。SDK 那份只有
//! 「整帧 / 只合成」两档：页面上一个无限旋转的小图标就让**每一帧**都把视口 ± 400px 整条带
//! 重新光栅化，再陪着把覆盖层、tabBar 也重画一遍。411×786@2.625 的视口上一帧 8~9ms（M3），
//! 手机上 20ms 往上 —— Flutter 宿主里往上一滑就顿（`examples/sdk_scroll_bench.rs` 量的就是它）。
//!
//! ## 比桌面多走的两步：滚动中也能局部重画，滑出条带只补新露出的那截
//! 桌面窗体在「滚动 + 动画」同时发生的帧里退回整帧（`layout.rs` 注释里那段：视口裁剪按
//! **当前**偏移剪子树，偏移变了之后只重画损伤区，裁剪窗外那部分会被清成底色却没人重画）。
//! 这里守一条不变量：**局部帧之后，已绘制条带 ⊆ 当前裁剪窗**（窗 = 视口上下各加 400 物理
//! 像素，绘制期的视口裁剪按它剪子树）。于是：
//! - 损伤区（动画包围盒 / setData 失效范围 / scroll-view 的盒子）夹进裁剪窗再清、再画，
//!   清掉的每一块都有人重画；窗外那截不清不画，但它也不在条带里了；
//! - 窗外的东西（没画的一次性变化、祖先被裁剪剪掉而没记下包围盒的动画节点）要被看见，
//!   视口得先滑出条带 —— 那一帧补画条带，按新的窗重新走一遍；
//! - 滚动中窗里没画的行攒够 100 物理像素就补（`FrameKind::Band`），只补窗里条带没盖到的那一两截：
//!   不等视口真滑出条带（那时一次要补 400 多行），更不整条带 3M 像素重新光栅化。
//!   一下跳出很远（`scroll_to`、大幅甩动）才整条带重画。
//!
//! 判据是 `tests/sdk_damage_equivalence.rs`：同一串输入喂给开 / 关局部重画的两个引擎，
//! 每一帧的合成结果逐字节相同。
use super::engine::MiniEngine;
use super::engine_damage_rects::{clamp_rect, merge_rects, union, EAGER_STRIP_PX};
use crate::renderer::{EventBinding, FramePlan, VIEWPORT_CULL_MARGIN_PX};
use crate::Rect as GeoRect;
use std::collections::HashMap;

/// 要重画的几块加起来超过裁剪窗面积的这个比例，就干脆整条带画一趟
const BAND_AREA_LIMIT: f32 = 0.6;

/// 一次产出新画面的 pump 做了哪一种活（便宜 → 贵）
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameKind {
    /// 没出新画面
    #[default]
    Idle,
    /// 一个像素都没重画，只按新的滚动位置 / 覆盖层重新合成
    Compose,
    /// 只重画了一块矩形（CSS 动画、setData 失效范围、scroll-view 的盒子）
    Damage,
    /// 补条带：滚动让裁剪窗里露出了没画的行（或视口已滑出条带），只补那几截
    /// （跳得太远时整条带），覆盖层与 tabBar 沿用
    Band,
    /// 整帧（页面条带 + 覆盖层 + tabBar）
    Full(FullReason),
}

/// 整帧的原因
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FullReason {
    /// 结构性：首帧、换页、按压态、尺寸变化、picker / 弹窗以外的 `needs_redraw`
    Structural,
    /// setData 的失效范围算不出来（结构变了）或大于视口的三成
    Data,
    /// CSS 动画的包围盒太大（超过视口面积的三成），或页面在动却没记下包围盒
    Animation,
    /// swiper 翻页的滑动过程 / 引擎自己画的视频
    Media,
    /// scroll-view 动了但不知道它画在哪（滚出视口 / 在覆盖层里）
    ScrollView,
    /// `MINI_NO_DAMAGE=1`：一律整帧（逐像素对照用）
    NoDamage,
}

impl FrameKind {
    /// 基准与日志里的短标签
    pub fn label(self) -> &'static str {
        match self {
            FrameKind::Idle => "空转",
            FrameKind::Compose => "只合成",
            FrameKind::Damage => "局部",
            FrameKind::Band => "补条带",
            FrameKind::Full(FullReason::Structural) => "整帧·结构",
            FrameKind::Full(FullReason::Data) => "整帧·数据",
            FrameKind::Full(FullReason::Animation) => "整帧·动画",
            FrameKind::Full(FullReason::Media) => "整帧·媒体",
            FrameKind::Full(FullReason::ScrollView) => "整帧·滚动区",
            FrameKind::Full(FullReason::NoDamage) => "整帧·对照",
        }
    }
}

/// 局部重画要记住的跨帧状态（与桌面窗体 `MiniAppWindow` 上同名的那几个字段一一对应）
#[derive(Default)]
pub(crate) struct DamageState {
    /// 最近一次产出新画面的 pump 做了什么
    pub(crate) last_kind: FrameKind,
    /// `MINI_NO_DAMAGE`：一律整帧（逐像素对照的参照物）
    pub(crate) no_damage: bool,
    /// `MINI_DAMAGE_LOG`：每一帧重画了多少打一行（logcat / 控制台）
    pub(crate) log: bool,
    /// 只有覆盖层（Toast / Modal / picker 面板）变了：重新合成就够，页面一个像素不用动。
    /// 从前它们全走 `needs_redraw`，picker 面板滑入的 0.22s 里每帧整页重画。
    pub(crate) compose_dirty: bool,
    /// 上一次完整走过动画节点时记下的动画包围盒（物理像素，画布坐标）
    pub(crate) last_animated_bounds: Vec<GeoRect>,
    /// 页面画布（正常流）里还有 CSS 动画在跑（上一次页面那一趟求出来的）
    pub(crate) page_animates: bool,
    /// fixed 覆盖层自己带动画：每帧都要重画覆盖层（它是整张重画的），页面可以不动
    pub(crate) fixed_layer_animates: bool,
    /// 覆盖层内容可能变了（setData 的失效分析说覆盖层有变化 / 结构性重画）
    pub(crate) fixed_dirty: bool,
    /// 覆盖层上次重画时登记的事件绑定与遮挡区域：页面重画会清空全部绑定，
    /// 覆盖层跳过重画的帧里要补回来，否则弹窗点不动、点击穿到下层
    pub(crate) cached_fixed_bindings: Vec<EventBinding>,
    pub(crate) cached_fixed_regions: Vec<GeoRect>,
    /// 各 scroll-view 上次画进页面画布时的位置：变了的那几个就是损伤区
    pub(crate) drawn_area_positions: HashMap<String, f32>,
    /// 布局缓存最近一次是按哪个滚动偏移问的。setData 的失效分析要拿**同一个**偏移去问，
    /// 否则会被当成「视口变了」直接整帧（滚动中的每一次 setData 都整帧）
    pub(crate) layout_scroll: Option<f32>,
    /// tabBar 画布上画的是哪个页面的选中态
    pub(crate) tabbar_route: Option<String>,
    /// 页面画布上画成点击态的那几个元素（`engine_press.rs` 每帧比对，变了才重画它们自己）
    pub(crate) press_drawn: Vec<String>,
    /// 这一帧额外要重画的页面画布区域（物理像素，画布坐标）：点击态变了的元素
    pub(crate) extra: Option<GeoRect>,
    /// 覆盖层里的元素点击态变了：覆盖层要重画
    pub(crate) press_fixed: bool,
    /// 这一帧第一个要整页重画的源码位置（`MINI_DAMAGE_LOG`，见 `engine_press.rs::mark_full`）
    pub(crate) full_why: Option<&'static std::panic::Location<'static>>,
    /// 这一帧 GPU 上屏的账（只在 `log` 时填，接在 🩹 行尾）
    pub(crate) gpu_note: Option<String>,
}

/// 这一帧页面画布要画多少
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PagePass {
    /// 视口 ± 余量整条带
    Band,
    /// 只清并重画这几块（物理像素，画布坐标），一块一趟。`anim_complete`：这一帧走到了
    /// 全部动画节点，收集到的动画包围盒可以替换旧记录
    Rects { rects: Vec<GeoRect>, anim_complete: bool },
    /// 不动页面画布
    Skip,
}

/// 一帧重画的范围：页面 / 覆盖层 / tabBar 各自要不要画
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RenderScope {
    pub page: PagePass,
    pub fixed: bool,
    pub tabbar: bool,
}

impl RenderScope {
    /// 从前每次重画都是这样：三层全画
    pub(crate) const FULL: Self = Self { page: PagePass::Band, fixed: true, tabbar: true };
}

/// scroll-view 动了之后要重画多少
enum AreaDamage {
    Nothing,
    Rect(GeoRect),
    Full,
}

impl MiniEngine {
    /// 最近一次产出新画面的 pump 做了哪种活（基准与诊断用）
    pub fn last_frame_kind(&self) -> FrameKind {
        self.dmg.last_kind
    }

    /// 绘制期视口裁剪的纵向窗口（物理像素，画布坐标）：窗外的子树整棵不画。
    /// 与 `WxmlRenderer::cull_outside_viewport` 同一个算式。
    fn cull_window(&self) -> (f32, f32) {
        let top = self.scroll.get_position() * self.dpr;
        let vp = self.viewport_height() * self.dpr;
        (top - VIEWPORT_CULL_MARGIN_PX, top + vp + VIEWPORT_CULL_MARGIN_PX)
    }

    /// 裁剪窗里换了帧的动图（`wxml_renderer/animated_image.rs`），已夹进窗与画布
    fn animated_images_due(&self) -> Vec<GeoRect> {
        let (y0, y1) = self.cull_window();
        let ph = self.canvas.read().height() as f32;
        self.renderer.as_ref().map(|r| r.animated_images_due(y0.max(0.0), y1.min(ph))).unwrap_or_default()
    }

    /// 裁剪窗里有动图：宿主要按刷新率醒着等它换帧（`wants_next_frame`）
    pub(super) fn animated_images_visible(&self) -> bool {
        let (y0, y1) = self.cull_window();
        self.renderer.as_ref().map(|r| r.has_animated_images(y0, y1)).unwrap_or(false)
    }

    /// 相对上次画进页面画布时，哪些 scroll-view 的位置变了（与桌面 `scroll_area_damage` 一致）
    fn scroll_area_damage(&self) -> AreaDamage {
        let mut acc = None;
        let mut moved_any = false;
        for (id, c) in &self.interaction.scroll_controllers {
            let pos = c.get_position();
            let moved = self.dmg.drawn_area_positions.get(id).map(|p| (p - pos).abs() > 0.001).unwrap_or(true);
            if !moved {
                continue;
            }
            moved_any = true;
            // 覆盖层里的要连覆盖层一起重画；本帧没登记（滚出视口了）的不知道画在哪
            let Some((b, false)) = self.interaction.scroll_area_bounds(id) else {
                return AreaDamage::Full;
            };
            let s = self.dpr;
            acc = union(acc, GeoRect::new(b.x * s, b.y * s, b.width * s, b.height * s));
        }
        match acc {
            Some(r) => AreaDamage::Rect(r),
            // 动了但宽高为 0（还没布局出来）：保守整帧
            None if moved_any => AreaDamage::Full,
            None => AreaDamage::Nothing,
        }
    }

    /// 决定这一帧重画多少，并画出来。返回是否产出了新画面。
    ///
    /// `page_anim` / `tabbar_anim`：上一次绘制时页面 / tabBar 里还有 CSS 动画在跑；
    /// `media`：swiper 翻页的滑动过程、引擎自己画的视频（这两样只能整帧）。
    pub(super) fn redraw_as_needed(&mut self, tabbar_anim: bool, media: bool, scroll_moved: bool) -> bool {
        let compose_dirty = std::mem::take(&mut self.dmg.compose_dirty);
        let recompose = scroll_moved || compose_dirty || self.present_pending;
        let (kind, scope) = if self.dmg.no_damage {
            self.plan_reference(tabbar_anim || media || compose_dirty, recompose)
        } else {
            self.plan_redraw(tabbar_anim, media, recompose)
        };
        self.dmg.last_kind = kind;
        let mut log_line = None;
        if self.dmg.log && kind != FrameKind::Idle {
            let r = match &scope.page {
                PagePass::Rects { rects, .. } => rects
                    .iter()
                    .map(|r| format!(" {:.0}x{:.0}@({:.0},{:.0})", r.width, r.height, r.x, r.y))
                    .collect::<String>(),
                PagePass::Band if kind != FrameKind::Compose => " 整条带".to_string(),
                _ => String::new(),
            };
            let band = self.drawn_band.map(|(a, b)| format!(" 条带 {a:.0}..{b:.0}")).unwrap_or_default();
            // 转场进度：真机上对着每帧的 t 看起步那一步与后面是不是一样大（route_anim.rs「时钟」）
            let route = self
                .route
                .progress()
                .map(|(t, n, dt, cap)| format!(" 转场 {t:.3}（第 {n} 步 间隔 {dt:.0} 封顶 {cap:.0}）"))
                .unwrap_or_default();
            let why = match (kind, self.dmg.full_why) {
                (FrameKind::Full(FullReason::Structural), Some(l)) => format!(" ← {}:{}", l.file(), l.line()),
                _ => String::new(),
            };
            log_line = Some((
                std::time::Instant::now(),
                format!(
                    "🩹 [SDK] {}{r}{}{}  滚动 {:.1}{band}{route}{why}",
                    kind.label(),
                    if scope.fixed && kind != FrameKind::Compose { " +覆盖层" } else { "" },
                    if scope.tabbar && kind != FrameKind::Compose { " +tabBar" } else { "" },
                    self.scroll.get_position(),
                ),
            ));
        }
        self.dmg.full_why = None;
        match kind {
            FrameKind::Idle => return false,
            FrameKind::Compose => self.compose_current(),
            _ => self.render_scoped(scope),
        }
        self.needs_redraw = false;
        // 画完再打：带上这一帧画 + 合成花了多久（真机上哪种帧慢，一眼看得出来）
        if let Some((t0, line)) = log_line {
            let gpu = self.dmg.gpu_note.take().unwrap_or_default();
            crate::compositor::diag::emit(&format!("{line}{gpu}  {:.1}ms", t0.elapsed().as_secs_f32() * 1000.0));
        }
        true
    }

    /// `MINI_NO_DAMAGE=1` 的参照路径：与引入局部重画之前一模一样 —— 有任何变化就三层全画，
    /// 只有「纯滚动且还在已绘制条带内」才只合成
    fn plan_reference(&mut self, other_work: bool, recompose: bool) -> (FrameKind, RenderScope) {
        let full = (FrameKind::Full(FullReason::NoDamage), RenderScope::FULL);
        if self.needs_redraw {
            return (FrameKind::Full(FullReason::Structural), RenderScope::FULL);
        }
        let sv_moved = !matches!(self.scroll_area_damage(), AreaDamage::Nothing);
        let gif = !self.animated_images_due().is_empty();
        let press = self.dmg.extra.take().is_some() | std::mem::take(&mut self.dmg.press_fixed);
        if other_work || press || self.page_data_dirty || self.dmg.page_animates || self.dmg.fixed_layer_animates || sv_moved || gif {
            return full;
        }
        if !recompose {
            return (FrameKind::Idle, RenderScope::FULL);
        }
        if self.viewport_inside_drawn_band() {
            (FrameKind::Compose, RenderScope::FULL)
        } else {
            full
        }
    }

    fn plan_redraw(&mut self, tabbar_anim: bool, media: bool, recompose: bool) -> (FrameKind, RenderScope) {
        let route = self.current_route();
        // tabBar 换了选中项（换页）或自己在动：只重画它那一条（很便宜）。
        // 没有 tabBar 的页面永远不画它，`tabbar_route` 也就永远对不上 —— 先判有没有
        let tabbar = self.is_tabbar_page(&route)
            && (tabbar_anim || self.dmg.tabbar_route.as_deref() != Some(route.as_str()));
        let full = |reason: FullReason, fixed: bool, tabbar: bool| {
            (FrameKind::Full(reason), RenderScope { page: PagePass::Band, fixed, tabbar })
        };
        // 点击态变了的元素（engine_press.rs）：这一帧一定消费掉，整帧的路径里它们本来就画到了
        let press_rect = self.dmg.extra.take();
        let press_fixed = std::mem::take(&mut self.dmg.press_fixed);
        if self.needs_redraw {
            return full(FullReason::Structural, true, true);
        }
        let page_anim = self.dmg.page_animates && !self.dmg.last_animated_bounds.is_empty();
        if self.dmg.page_animates && self.dmg.last_animated_bounds.is_empty() {
            // 页面说自己在动却没记下包围盒（动画节点画在离屏画布里）：不知道画在哪，只能整帧。
            // 这一帧要是还有 setData，没走失效分析，覆盖层与 tabBar 也一起重画
            let data = self.page_data_dirty;
            let fixed = self.dmg.fixed_dirty || self.dmg.fixed_layer_animates || data || press_fixed;
            return full(FullReason::Animation, fixed, tabbar || data);
        }
        if media {
            // swiper / 视频可能就在覆盖层里，覆盖层一起画（与从前一样）
            return full(FullReason::Media, true, tabbar);
        }

        // ── 页面画布上要重画的范围：setData 的失效范围 ∪ 动画 ∪ 动了的 scroll-view ∪ 换了帧的动图 ──
        // 动图只在帧号变了的那一帧重画它自己那块：从前它不算「在动」，一滑就停在旧帧上
        let gifs = self.animated_images_due();
        let mut page_work = page_anim || !gifs.is_empty();
        let mut reason = None;
        // 一次性的变化（setData / scroll-view / 点击态）合成一块；动画节点各自一块 ——
        // 页面顶上一个转圈、中间一个呼吸灯，并成一块就是大半屏
        let mut once = None;
        if let Some(r) = press_rect {
            page_work = true;
            once = union(once, r);
        }
        if self.page_data_dirty {
            page_work = true;
            self.dmg.fixed_dirty = true;
            match self.plan_data_damage() {
                FramePlan::Unchanged => self.dmg.fixed_dirty = false,
                FramePlan::Damage { rect, fixed_changed } => {
                    self.dmg.fixed_dirty = fixed_changed;
                    once = union(once, rect);
                }
                FramePlan::Full => reason = Some(FullReason::Data),
            }
        }
        match self.scroll_area_damage() {
            AreaDamage::Nothing => {}
            AreaDamage::Rect(r) => {
                page_work = true;
                once = union(once, r);
            }
            AreaDamage::Full => {
                page_work = true;
                reason = reason.or(Some(FullReason::ScrollView));
            }
        }
        // 覆盖层里的点击态变了（engine_press.rs）：放在 setData 的失效分析之后，不被它改写
        if press_fixed {
            self.dmg.fixed_dirty = true;
        }
        let fixed = self.dmg.fixed_dirty || self.dmg.fixed_layer_animates;
        if let Some(r) = reason {
            // 结构变了（setData 的 Full）覆盖层与 tabBar 跟着重画；动画 / scroll-view 逼出来的整帧
            // 只重画页面（覆盖层自己在动时 `fixed` 本来就是真）
            let data = r == FullReason::Data;
            return full(r, fixed || data || r == FullReason::ScrollView, tabbar || data);
        }
        let inside = self.viewport_inside_drawn_band();
        // 裁剪窗里还有多少行没画（条带边到窗边）。攒到视口真滑出条带才补，一补就是 400 多行
        // 加整窗走一遍节点树，手机上一帧二十毫秒往上；提前在它攒够一小截时就补，尖峰削平
        let (pw, ph) = (self.pixel_size().0 as f32, self.canvas.read().height() as f32);
        let (wy0, wy1) = self.cull_window();
        let (wy0, wy1) = (wy0.max(0.0), wy1.min(ph));
        let missing = match self.drawn_band {
            Some((b0, b1)) if b1 > wy0 && b0 < wy1 => (b0 - wy0).max(wy1 - b1).max(0.0),
            _ => f32::INFINITY,
        };
        let refill = !inside || (recompose && missing > EAGER_STRIP_PX);
        if !page_work && !refill {
            if tabbar || fixed {
                // 只有 tabBar / 覆盖层在变（覆盖层里的动画）：页面画布不动
                return (FrameKind::Damage, RenderScope { page: PagePass::Skip, fixed, tabbar });
            }
            if !recompose {
                return (FrameKind::Idle, RenderScope::FULL);
            }
            return (FrameKind::Compose, RenderScope::FULL);
        }

        // ── 局部重画：损伤区一律夹进当前裁剪窗（见模块注释），条带也只信窗内那段 ──
        let clip = |r: GeoRect| clamp_rect(r, 0.0, pw, wy0, wy1);
        // 补条带：只补画窗里条带没盖到的那几截，不再整条带重画。补完条带 = 整个裁剪窗
        let mut rects: Vec<GeoRect> = Vec::new();
        if refill {
            match self.drawn_band {
                // 条带与窗还连着：窗里条带没盖到的上下两截就是要补的
                Some((b0, b1)) if b1 > wy0 && b0 < wy1 => {
                    if wy0 < b0 {
                        rects.push(GeoRect::new(0.0, wy0, pw, b0 - wy0));
                    }
                    if b1 < wy1 {
                        rects.push(GeoRect::new(0.0, b1, pw, wy1 - b1));
                    }
                }
                // 一下跳出去很远（scroll_to、大幅甩动）：没有可以接着用的，整条带重画
                _ => return (FrameKind::Band, RenderScope { page: PagePass::Band, fixed, tabbar }),
            }
        }
        let strip = !rects.is_empty();
        // 从这一帧起，条带只保证裁剪窗内的像素是对的：
        // 窗外那截要么是这一帧没画的一次性变化，要么是没走到的动画节点（它的祖先被视口裁剪
        // 整棵剪掉了，包围盒也就没记下来）。滑回去之前先判出条带、补画，就不会露出旧像素
        self.drawn_band = Some(match self.drawn_band {
            Some((b0, b1)) if !strip => (b0.max(wy0), b1.min(wy1)),
            _ => (wy0, wy1),
        });
        rects.extend(once.and_then(clip));
        let anim_in: Vec<GeoRect> = if page_anim {
            self.dmg.last_animated_bounds.iter().filter_map(|r| clip(*r)).collect()
        } else {
            Vec::new()
        };
        if page_anim && anim_in.is_empty() {
            // 动画全在裁剪窗外：看不见，也不在已绘制条带里了。先停掉它（不再逼着宿主续帧），
            // 滑回去补条带时会重新走到它
            self.dmg.page_animates = false;
            if let Some(r) = self.renderer.as_mut() {
                r.set_animations_active(self.dmg.fixed_layer_animates);
            }
        }
        let anim_complete = strip || !anim_in.is_empty();
        rects.extend(anim_in);
        rects.extend(gifs);
        let rects = merge_rects(rects);
        let area: f32 = rects.iter().map(|r| r.width * r.height).sum();
        if area > pw * (wy1 - wy0) * BAND_AREA_LIMIT {
            // 要补的加起来快赶上整条带了：一趟整条带比分好几趟各走一遍节点树便宜
            let kind = if strip { FrameKind::Band } else { FrameKind::Full(FullReason::Animation) };
            return (kind, RenderScope { page: PagePass::Band, fixed, tabbar });
        }
        let kind = if strip { FrameKind::Band } else { FrameKind::Damage };
        let page = if rects.is_empty() {
            // 变化全在窗外 / 渲染结果没变：页面不用动
            PagePass::Skip
        } else {
            // 补条带 / 动画节点都被完整走到了，收集到的动画包围盒可以替换旧记录
            PagePass::Rects { rects, anim_complete }
        };
        if page == PagePass::Skip && !fixed && !tabbar {
            // 数据变了但画面一个像素没变（改的字段不参与渲染）：仍算出了一帧，
            // 与从前的整帧重画产出同样的标志位，宿主那边的出帧节奏不变
            return (FrameKind::Compose, RenderScope::FULL);
        }
        (kind, RenderScope { page, fixed, tabbar })
    }

    /// setData 之后：先建好新布局，问出「只影响哪一块」（要在清屏之前知道）
    fn plan_data_damage(&mut self) -> FramePlan {
        self.refresh_page_data();
        let vp = self.viewport_height();
        // 用布局缓存上次被问的同一个偏移：偏移不进布局，换了偏移只会被判成「视口变了」
        let scroll = self.dmg.layout_scroll.unwrap_or_else(|| self.scroll.get_position());
        let Self { renderer, page_stack, page_data, .. } = self;
        match (renderer.as_mut(), page_stack.last()) {
            (Some(r), Some(page)) => r.plan_frame(&page.wxml_nodes, page_data, Some((scroll, vp))),
            _ => FramePlan::Full,
        }
    }
}
