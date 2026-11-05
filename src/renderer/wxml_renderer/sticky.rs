//! `position: sticky` 与 Skyline `<sticky-header>` / `<sticky-section>`：吸住、被推走、只重画自己那一块。
//!
//! ## 选的哪条路（三种候选）
//! ① 像 `position: fixed` 那样放进覆盖层；② 像动画包围盒那样，滚动时在页面画布上按新位置重画它那一块；③ 新开一层合成。
//! 用的是 ②：
//! - 覆盖层压在 tabBar 之上、整层按视口重排；吸顶元素没吸住时要跟着内容走、还要被父容器底边推走，放进覆盖层层叠就错了；
//! - ③ 要改 CPU / GPU 两个合成器和四份宿主；
//! - 页面画布是内容坐标、滚动只换切片：吸住的元素在内容坐标里每帧都在动，宿主拿「上次画在哪 ∪ 这次画在哪」当损伤区
//!   （[`WxmlRenderer::sticky_damage`]，SDK `engine_damage.rs` / 桌面 `frame.rs`），只清、只画这两块。
//!   没吸住（随内容走、位置没变）的帧什么都不加，照旧只合成 —— 没写 sticky 的页面一个字节都不变。
//! scroll-view 里的：scroll-view 一动本来就重画它整个盒子（分块缓存时是贴块），吸住的元素在内容画完 / 贴完之后按吸住的位置
//! 画上去，不另加损伤区；块里与离屏缓存里不画它（否则块里留着它在原位的影子）。
//!
//! ## 怎么画
//! 布局照 relative 排（占原位），top / bottom 不进 taffy（base_style.rs）。每次布局之后按树建一张索引：哪些节点吸、归哪个滚动容器
//! （最近的、真能滚的纵向 scroll-view，没有就是页面）。绘制时正常流遇到受管的节点跳过，等它所在的滚动容器把内容画完，
//! 再按吸住的位置画（`draw_*` 的原点加上位移，子树、命中区、事件绑定都跟着走）—— 所以它压在后面滚上来的内容上面
//! （浏览器里定位元素画在非定位的流内容之后）。
//!
//! 不受管（画在原位、不吸）的：祖先带 transform / 动画 / opacity<1 / overflow 非 visible（CSS 里那个祖先就是它的滚动容器，
//! 不滚就不吸）/ fixed / swiper / 横向 scroll-view；吸附元素里面再套吸附元素（里面那个当普通内容）。
//!
//! ## sticky-header 的几条（官方没写死的标「待验」）
//! - `offset-top` = CSS 的 top；在 `sticky-section` 里（`push-pinned-header` 默认 true）被本节底边推走；
//!   `push-pinned-header=false` 不被推（一直吸到容器内容末尾）；
//! - 同一个滚动容器里的几个 sticky-header（直接放在 scroll-view 下，或分在几个 sticky-section 里）：
//!   `allow-overlapping=false`（默认，「不与前一个 sticky-header 重叠」）时后一个顶到前一个的下边就吸在那里（叠成一摞），
//!   true 时吸在 offset-top 处、盖住前一个。待验：按 Flutter 的 pinned header 吃 `SliverConstraints.overlap` 的摞法；
//!   「后一个把前一个推走」是 sticky-section 的 `push-pinned-header`（本节底边推），不是 allow-overlapping；
//! - `bindstickontopchange {isStickOnTop}`：吸住（位移 > 0 且还露在视窗里）与否变了才发，第一次见到时按「没吸住」起算。
//!
//! 已知近似：吸住时 SelectorQuery 报的仍是原位（浏览器报吸住的位置）；没写 id 的节点在吸住期间组件 id 随位置变（按压态跟不住，
//! 与带平移动画的节点一样）；横向吸附（left / right）不支持，按 relative 平移。
use super::*;

/// 一个吸附元素在节点树里的位置与配置
#[derive(Clone)]
pub(super) struct StickyEntry {
    /// 从页面根列表起的孩子下标
    path: Vec<u32>,
    /// 所属滚动容器：None 是页面；Some((深度, 节点)) 是 `path[..深度]` 那个纵向 scroll-view
    container: Option<(usize, NodeId)>,
    /// `<sticky-header>` 的配置（CSS sticky 为 None）
    header: Option<HeaderCfg>,
    /// 上一次报给逻辑层的吸顶状态
    stuck: bool,
}

#[derive(Clone, Copy)]
struct HeaderCfg {
    allow_overlap: bool,
    /// 在 `<sticky-section>` 里且 `push-pinned-header` 不是 false：被本节底边推走
    pushed_by_section: bool,
    /// 绑了 `stickontopchange`
    notify: bool,
}

/// 每次布局之后建一次的索引 + 跨帧状态
#[derive(Default)]
pub(super) struct StickyIndex {
    gen: Option<u64>,
    entries: Vec<StickyEntry>,
    /// 这一趟受管的节点（只拿地址比身份）：正常流画到它们时跳过。趟末清空 —— 布局重建后地址会被别的节点复用
    ptrs: Vec<usize>,
    /// 页面级的上一次画在哪（物理像素，画布坐标，含阴影余量）
    page_drawn: Vec<GeoRect>,
    /// 待派发的 `stickontopchange`：(组件 id, isStickOnTop)
    events: Vec<(String, bool)>,
    /// 这一趟吸住（挪了位置）的元素：画在哪、原位、它的事件绑定在 `event_bindings` 里的下标范围（逻辑坐标）
    occluders: Vec<Occluder>,
}

/// 吸住的元素压住了底下的内容：点在它上面时，底下先登记、又不是它祖先的绑定收不到（hit_test.rs）
struct Occluder {
    drawn: GeoRect,
    natural: GeoRect,
    start: usize,
}

/// 按当前滚动算出的一个吸附元素
struct Placed<'a> {
    entry: usize,
    node: &'a RenderNode,
    /// 父节点的原点（交给 draw_* 的 ox / oy，位移另加）
    origin: (f32, f32),
    color: Color,
    rect: GeoRect,
    shift: f32,
}

impl Placed<'_> {
    /// 画到的范围（含阴影 / 溢出余量）
    fn drawn(&self) -> GeoRect {
        let pad = self
            .node
            .style
            .box_shadow
            .map(|s| s.blur + s.spread.max(0.0) + s.offset_x.abs().max(s.offset_y.abs()))
            .unwrap_or(0.0)
            + 2.0;
        GeoRect::new(self.rect.x - pad, self.rect.y + self.shift - pad, self.rect.width + pad * 2.0, self.rect.height + pad * 2.0)
    }
}

/// 纵向 scroll-view 的内容是否真比盒子高（uni-app 那种「内容多高它就多高、滚的是页面」的根 scroll-view 不算滚动容器）
fn really_scrolls(taffy: &Tree, n: &RenderNode) -> bool {
    let Ok(l) = taffy.layout(n.taffy_node) else { return false };
    let bottom = n
        .children
        .iter()
        .filter_map(|c| taffy.layout(c.taffy_node).ok().map(|cl| cl.location.y + cl.size.height))
        .fold(0.0f32, f32::max);
    bottom > l.size.height + 0.5
}

fn collect(taffy: &Tree, n: &RenderNode, path: &mut Vec<u32>, cont: Option<(usize, NodeId)>, section: Option<bool>, out: &mut Vec<StickyEntry>) {
    let s = &n.style;
    if s.is_fixed {
        return;
    }
    if s.is_sticky {
        let header = (n.tag == "sticky-header").then(|| HeaderCfg {
            allow_overlap: super::scroll_views::bool_attr(&n.attrs, "allow-overlapping"),
            pushed_by_section: section.unwrap_or(true),
            notify: n.events.iter().any(|e| e.event_type == "stickontopchange"),
        });
        out.push(StickyEntry { path: path.clone(), container: cont, header, stuck: false });
        return;
    }
    if s.transform.is_some() || s.animation.is_some() || s.opacity < 0.999 || n.attrs.get("animation").is_some_and(|a| a.contains("actions")) {
        return;
    }
    let mut cont = cont;
    if n.tag == "scroll-view" {
        let (horizontal, scrollable) = super::scroll_views::scroll_axes(&n.attrs);
        if horizontal {
            return;
        }
        if scrollable && really_scrolls(taffy, n) {
            cont = Some((path.len(), n.taffy_node));
        }
    } else if s.overflow != crate::renderer::components::Overflow::Visible || matches!(n.tag.as_str(), "swiper" | "picker-view") {
        return;
    }
    if !WxmlRenderer::draws_children(n) {
        return;
    }
    let section = (n.tag == "sticky-section").then(|| n.attrs.get("push-pinned-header").map(|v| crate::renderer::components::attr_is_true(v)).unwrap_or(true));
    for (i, c) in n.children.iter().enumerate() {
        path.push(i as u32);
        collect(taffy, c, path, cont, section, out);
        path.pop();
    }
}

impl StickyIndex {
    fn ensure(&mut self, taffy: &Tree, roots: &[RenderNode], gen: u64) {
        if self.gen == Some(gen) {
            return;
        }
        let old = std::mem::take(&mut self.entries);
        let mut path = Vec::new();
        for (i, n) in roots.iter().enumerate() {
            path.push(i as u32);
            collect(taffy, n, &mut path, None, None, &mut self.entries);
            path.pop();
        }
        // setData 重建了树：同一个位置上的吸顶状态接着用（不重发 stickontopchange）
        for e in &mut self.entries {
            if let Some(o) = old.iter().find(|o| o.path == e.path) {
                e.stuck = o.stuck;
            }
        }
        self.gen = Some(gen);
    }

    fn resolve(&mut self, roots: &[RenderNode]) {
        self.ptrs.clear();
        for e in &self.entries {
            let mut list = roots;
            let mut node = None;
            for &i in &e.path {
                let Some(n) = list.get(i as usize) else { node = None; break };
                node = Some(n);
                list = &n.children;
            }
            if let Some(n) = node {
                self.ptrs.push(n as *const RenderNode as usize);
            }
        }
    }

    /// 某个容器的吸附元素按当前滚动画在哪。`list` / `origin`：容器的孩子与它们的原点；
    /// `cont_box`：容器内容的上下边；`view`：视窗的上下边（都是物理像素，画布坐标）
    #[allow(clippy::too_many_arguments)]
    fn place<'a>(
        &self,
        key: Option<NodeId>,
        taffy: &Tree,
        list: &'a [RenderNode],
        origin: (f32, f32),
        cont_box: (f32, f32),
        (vy0, vy1): (f32, f32),
        color: Color,
        out: &mut Vec<Placed<'a>>,
    ) {
        for (ei, e) in self.entries.iter().enumerate() {
            if e.container.map(|c| c.1) != key {
                continue;
            }
            let depth0 = e.container.map(|c| c.0).unwrap_or(0);
            let steps = &e.path[depth0..];
            let (mut nodes, (mut ox, mut oy), mut cb, mut color) = (list, origin, cont_box, color);
            let mut found = None;
            for (k, &ix) in steps.iter().enumerate() {
                let Some(n) = nodes.get(ix as usize) else { break };
                let Ok(l) = taffy.layout(n.taffy_node) else { break };
                let (x, y) = (ox + l.location.x, oy + l.location.y);
                if k + 1 == steps.len() {
                    found = Some((n, GeoRect::new(x, y, l.size.width, l.size.height), (l.margin.top, l.margin.bottom)));
                    break;
                }
                cb = (y + l.padding.top + l.border.top, y + l.size.height - l.padding.bottom - l.border.bottom);
                color = n.style.text_color.unwrap_or(color);
                (ox, oy) = (x, y);
                nodes = &n.children;
            }
            let Some((node, rect, (mt, mb))) = found else { continue };
            let mut top = node.style.fixed_top;
            if let Some(h) = e.header {
                if !h.pushed_by_section {
                    cb.1 = cont_box.1;
                }
                // 同一个滚动容器里的前一个 sticky-header（`out` 里只有这个容器的，文档顺序）：不许重叠就吸在它下边。
                // 跨 sticky-section 也算：`push-pinned-header=false` 的节头一直吸着，下一节的头吸在它下面
                // （搜索栏 + 分类 tab 两节都不推的写法）；推的时候前一个已经被本节底边顶上去，自然不碍事
                if !h.allow_overlap {
                    if let Some(prev) = out.iter().rev().find(|p| self.entries[p.entry].header.is_some()) {
                        let below = prev.rect.y + prev.shift + prev.rect.height - vy0;
                        top = Some(top.unwrap_or(0.0).max(below));
                    }
                }
            }
            let (y, h) = (rect.y, rect.height);
            let mut shift = 0.0f32;
            if let Some(t) = top {
                let pin = vy0 + t;
                if y < pin {
                    shift = (pin - y).min(cb.1 - mb - (y + h)).max(0.0);
                }
            }
            if shift == 0.0 && e.header.is_none() {
                if let Some(b) = node.style.fixed_bottom {
                    let pin = vy1 - b;
                    if y + h > pin {
                        shift = -((y + h - pin).min(y - mt - cb.0).max(0.0));
                    }
                }
            }
            out.push(Placed { entry: ei, node, origin: (ox, oy), color, rect, shift });
        }
    }
}

impl WxmlRenderer {
    /// 正常流画到它时跳过（等它的滚动容器画完再按吸住的位置画）。每个节点只多读一个 bool
    #[inline]
    pub(super) fn sticky_skips(&self, node: &RenderNode) -> bool {
        node.style.is_sticky && self.sticky.ptrs.contains(&(node as *const RenderNode as usize))
    }

    /// 一趟页面绘制开始：布局变过就重建索引，再把受管节点的地址认出来
    pub(super) fn sticky_begin(&mut self, cache: &CachedLayout) {
        // 事件绑定这一趟清空重建（entry.rs），压住关系跟着重建
        self.sticky.occluders.clear();
        self.sticky.ensure(&cache.taffy, &cache.render_nodes, self.layout_generation);
        if self.sticky.entries.is_empty() {
            self.sticky.ptrs.clear();
            return;
        }
        self.sticky.resolve(&cache.render_nodes);
    }

    pub(super) fn sticky_end(&mut self) {
        self.sticky.ptrs.clear();
    }

    /// 第 `i` 条绑定在 `p` 这一点上被吸住的元素压住了：点在吸住的元素上，这条绑定在它之前登记（画在它底下）、
    /// 又不是它的祖先（祖先的盒子包得住它的原位）。命中冒泡是按包围盒面积近似的（hit_test.rs），
    /// 不挡的话点吸顶条会连底下滚上来的列表项一起触发。没有吸住的元素时只是一次判空
    #[inline]
    pub(super) fn sticky_hides(&self, i: usize, b: &EventBinding, p: &crate::Point) -> bool {
        if self.sticky.occluders.is_empty() || b.is_fixed {
            return false;
        }
        let holds = |o: &GeoRect, n: &GeoRect| {
            o.x <= n.x + 0.5 && o.y <= n.y + 0.5 && o.x + o.width >= n.x + n.width - 0.5 && o.y + o.height >= n.y + n.height - 0.5
        };
        self.sticky.occluders.iter().any(|o| i < o.start && o.drawn.contains(p) && !holds(&b.bounds, &o.natural))
    }

    /// 页面视窗（物理像素，画布坐标）：上边按上屏取切片的同一个取整（`scroll_px = (scroll * dpr) as i32`），
    /// 吸住的元素才落在屏幕的整像素上、不随滚动的小数抖
    fn page_view(&self, scroll_offset: f32, viewport_height_px: f32) -> (f32, f32) {
        let top = (scroll_offset * self.scale_factor).trunc();
        (top, top + viewport_height_px)
    }

    /// 页面滚到 `scroll_offset`（逻辑 px）时吸附元素要重画的那几块：上次画的 ∪ 这次要画的（物理像素，画布坐标）。
    /// 没有吸附元素、或位置都没变时为空（宿主照旧只合成）。每帧代价：页面级吸附元素各走一遍自己的路径
    pub fn sticky_damage(&mut self, scroll_offset: f32, viewport_height: f32) -> Vec<GeoRect> {
        let Some(cache) = self.cache.as_ref() else { return Vec::new() };
        self.sticky.ensure(&cache.taffy, &cache.render_nodes, self.layout_generation);
        if self.sticky.entries.iter().all(|e| e.container.is_some()) && self.sticky.page_drawn.is_empty() {
            return Vec::new();
        }
        let view = self.page_view(scroll_offset, viewport_height * self.scale_factor);
        let mut placed = Vec::new();
        let cont = (0.0, cache.content_height * self.scale_factor);
        self.sticky.place(None, &cache.taffy, &cache.render_nodes, (0.0, 0.0), cont, view, Color::BLACK, &mut placed);
        let now: Vec<GeoRect> = placed.iter().map(Placed::drawn).collect();
        let old = &self.sticky.page_drawn;
        let mut out: Vec<GeoRect> = now.iter().filter(|r| !old.contains(r)).copied().collect();
        out.extend(old.iter().filter(|r| !now.contains(r)).copied());
        out
    }

    /// setData 的失效范围碰到吸住的元素原位时，把它吸住的那块也并进来（里面的字变了，画在别处的那份也要重画）
    pub(super) fn sticky_widen(&mut self, rect: GeoRect, viewport: Option<(f32, f32)>) -> GeoRect {
        let Some(cache) = self.cache.as_ref() else { return rect };
        if self.sticky.entries.iter().all(|e| e.container.is_some()) {
            return rect;
        }
        let (scroll, vp) = viewport.unwrap_or((0.0, self.screen_height));
        let view = self.page_view(scroll, vp * self.scale_factor);
        let mut placed = Vec::new();
        let cont = (0.0, cache.content_height * self.scale_factor);
        self.sticky.place(None, &cache.taffy, &cache.render_nodes, (0.0, 0.0), cont, view, Color::BLACK, &mut placed);
        let hits = |a: &GeoRect, b: &GeoRect| a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
        let mut r = rect;
        for p in placed.iter().filter(|p| p.shift != 0.0 && hits(&p.rect, &rect)) {
            let d = p.drawn();
            let (x0, y0) = (r.x.min(d.x), r.y.min(d.y));
            let (x1, y1) = ((r.x + r.width).max(d.x + d.width), (r.y + r.height).max(d.y + d.height));
            r = GeoRect::new(x0, y0, x1 - x0, y1 - y0);
        }
        r
    }

    /// 取走待派发的 `stickontopchange`：(组件 id, isStickOnTop)
    pub fn take_sticky_events(&mut self) -> Vec<(String, bool)> {
        std::mem::take(&mut self.sticky.events)
    }

    /// 页面级的吸附元素：所有根画完之后按吸住的位置画（损伤区裁剪照常生效）
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_page_stickies(
        &mut self,
        canvas: &mut Canvas,
        cache: &CachedLayout,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) {
        if self.sticky.entries.iter().all(|e| e.container.is_some()) {
            self.sticky.page_drawn.clear();
            return;
        }
        let view = self.page_view(scroll_offset, viewport_height);
        let cont = (0.0, cache.content_height * self.scale_factor);
        self.draw_stickies_in(canvas, &cache.taffy, &cache.render_nodes, None, (0.0, 0.0), cont, view, Color::BLACK, interaction, scroll_offset, viewport_height);
    }

    /// scroll-view 里的吸附元素：内容画完 / 贴完之后调。`content_origin`：孩子的原点（已减滚动量）
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_scroll_stickies(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        sv: &RenderNode,
        (x, y, w, h): (f32, f32, f32, f32),
        content_origin: (f32, f32),
        color: Color,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) {
        if self.sticky.ptrs.is_empty() || !self.sticky.entries.iter().any(|e| e.container.map(|c| c.1) == Some(sv.taffy_node)) {
            return;
        }
        let Ok(l) = taffy.layout(sv.taffy_node) else { return };
        let bottom = sv
            .children
            .iter()
            .filter_map(|c| taffy.layout(c.taffy_node).ok().map(|cl| cl.location.y + cl.size.height + cl.margin.bottom))
            .fold(0.0f32, f32::max);
        let cont = (content_origin.1 + l.padding.top, content_origin.1 + bottom + l.padding.bottom);
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        self.draw_stickies_in(canvas, taffy, &sv.children, Some(sv.taffy_node), content_origin, cont, (y, y + h), color, interaction, scroll_offset, viewport_height);
        canvas.restore();
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_stickies_in(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        list: &[RenderNode],
        key: Option<NodeId>,
        origin: (f32, f32),
        cont: (f32, f32),
        view: (f32, f32),
        color: Color,
        interaction: &mut InteractionManager,
        scroll_offset: f32,
        viewport_height: f32,
    ) {
        // 索引整个拿出来：画吸附元素的子树时 `sticky_skips` 看到的是空表（子树里没有受管的节点：收集到吸附元素就不往下走）
        let mut idx = std::mem::take(&mut self.sticky);
        let mut placed = Vec::new();
        idx.place(key, taffy, list, origin, cont, view, color, &mut placed);
        for p in &placed {
            if !idx.entries[p.entry].header.is_some_and(|h| h.notify) {
                continue;
            }
            let r = p.rect;
            let stuck = p.shift > 0.0 && r.y + p.shift + r.height > view.0;
            if stuck != idx.entries[p.entry].stuck {
                idx.entries[p.entry].stuck = stuck;
                // 与 draw_* 按吸住的位置算出来的组件 id 一致（事件绑定就登记在那个 id 上）
                let id = Self::get_component_id(p.node, &self.page_bounds(r.x, r.y + p.shift, r.width, r.height));
                idx.events.push((id, stuck));
            }
        }
        if key.is_none() {
            idx.page_drawn = placed.iter().map(Placed::drawn).collect();
        }
        for p in &placed {
            if p.shift != 0.0 {
                let r = p.rect;
                idx.occluders.push(Occluder {
                    drawn: self.page_bounds(r.x, r.y + p.shift, r.width, r.height),
                    natural: self.page_bounds(r.x, r.y, r.width, r.height),
                    start: self.event_bindings.len(),
                });
            }
            self.draw_child_with_interaction(canvas, taffy, p.node, p.origin.0, p.origin.1 + p.shift, p.color, interaction, scroll_offset, viewport_height);
        }
        // 画的时候没人往 self.sticky 里写（见上），直接放回
        self.sticky = idx;
    }
}
