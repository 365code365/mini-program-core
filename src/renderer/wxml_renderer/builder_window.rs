//! list-builder / grid-builder 的「可见窗」：哪几项落在视口裁剪窗里（宿主据此派发 `bind:itembuild` /
//! `bind:itemdispose`，host/builder_events.rs）。
//!
//! 裁剪窗 = 页面视口 ∩ 一路上每个 scroll-view 的盒子（scroll-view 写了 `cache-extent` 就上下各外扩这么多 px：
//! Skyline scroll-view 的「视口外渲染区域」，默认 0）。只按纵向判（官方：构造器只支持纵向列表）。
//!
//! 每帧代价：构造器在树里的位置（孩子下标路径）**每次布局只找一遍**（整棵走一趟，记在线程局部里，按渲染器 + 布局代号作废）；
//! 之后每帧只沿路径往下走（树深那么多步）、在项里二分（项按文档顺序排，顶边不减）—— 不遍历整个列表。
//! 瀑布流的项顶边不单调，只能逐项看（一帧 O(项数)，写明）。
use super::*;
use crate::renderer::components::builder::{is_builder, per_item};
use crate::renderer::components::EventBind;
use std::cell::RefCell;

/// 一个构造器这一帧的可见窗
pub struct BuilderWindow {
    /// 跨帧认同一个构造器的键（`#id`，没 id 时是孩子下标路径）
    pub key: String,
    /// 可见的项 `[start, end)`（项序号，不是孩子下标）；一项都不可见是 None
    pub range: Option<(usize, usize)>,
    pub id: String,
    /// itembuild / itemdispose 的绑定
    pub binds: Vec<EventBind>,
}

thread_local! {
    /// (渲染器地址, 布局代号, 构造器的孩子下标路径)
    static PATHS: RefCell<(usize, u64, Vec<Vec<usize>>)> = const { RefCell::new((0, u64::MAX, Vec::new())) };
}

fn find(nodes: &[RenderNode], path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    for (i, n) in nodes.iter().enumerate() {
        path.push(i);
        if is_builder(&n.tag) {
            out.push(path.clone());
        } else {
            find(&n.children, path, out);
        }
        path.pop();
    }
}

impl WxmlRenderer {
    /// 各构造器这一帧的可见窗。`page_scroll` / `view_h` 是页面滚动与视口高（逻辑像素）。没有构造器的页面
    /// 每帧只是一次线程局部的比较
    pub fn builder_windows(&self, interaction: &InteractionManager, page_scroll: f32, view_h: f32) -> Vec<BuilderWindow> {
        let Some(cache) = self.cache.as_ref() else { return Vec::new() };
        let me = self as *const _ as usize;
        let paths = PATHS.with(|p| {
            let mut p = p.borrow_mut();
            if p.0 != me || p.1 != self.layout_generation {
                let mut out = Vec::new();
                find(&cache.render_nodes, &mut Vec::new(), &mut out);
                *p = (me, self.layout_generation, out);
            }
            p.2.clone()
        });
        let sf = self.scale_factor;
        let taffy = &cache.taffy;
        let mut res = Vec::new();
        'paths: for path in &paths {
            let mut nodes: &[RenderNode] = &cache.render_nodes;
            let (mut ox, mut oy) = (0.0f32, 0.0f32);
            // 裁剪窗（物理像素，页面内容坐标）
            let (mut top, mut bottom) = (page_scroll * sf, (page_scroll + view_h) * sf);
            for (depth, &i) in path.iter().enumerate() {
                let Some(n) = nodes.get(i) else { continue 'paths };
                let Ok(l) = taffy.layout(n.taffy_node) else { continue 'paths };
                let (x, y) = (ox + l.location.x, oy + l.location.y);
                if depth + 1 == path.len() {
                    let _ = x;
                    res.push(window_of(n, taffy, path, y, top, bottom));
                    break;
                }
                let (mut cx, mut cy) = (x, y);
                if n.tag == "scroll-view" {
                    let ext = n.attrs.get("cache-extent").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0).max(0.0) * sf;
                    top = top.max(y - ext);
                    bottom = bottom.min(y + l.size.height + ext);
                    // 与 SelectorQuery 同一个算式（query.rs）：子节点随滚动偏移移动
                    let bounds = GeoRect::new(x / sf, y / sf, l.size.width / sf, l.size.height / sf);
                    let sid = Self::get_component_id(n, &bounds);
                    if let Some(c) = interaction.get_scroll_controller(&sid) {
                        if super::scroll_views::scroll_axes(&n.attrs).0 {
                            cx -= c.get_position() * sf;
                        } else {
                            cy -= c.get_position() * sf;
                            if super::scroll_views::scroll_both(&n.attrs) {
                                cx -= interaction.scroll_x_of(&sid) * sf;
                            }
                        }
                    }
                }
                ox = cx;
                oy = cy;
                nodes = &n.children;
            }
        }
        res
    }
}

#[cfg(test)]
impl WxmlRenderer {
    /// 逐个看布局树的顶层节点（断言渲染节点上的样式用）
    pub fn for_each_root_for_test(&self, mut f: impl FnMut(&RenderNode)) {
        if let Some(c) = self.cache.as_ref() {
            c.render_nodes.iter().for_each(&mut f);
        }
    }

    /// 布局树里的渲染节点总数（长 rich-text 的节点数断言用）
    pub fn render_node_count(&self) -> usize {
        fn count(n: &[RenderNode]) -> usize {
            n.iter().map(|c| 1 + count(&c.children)).sum()
        }
        self.cache.as_ref().map(|c| count(&c.render_nodes)).unwrap_or(0)
    }
}

fn window_of(n: &RenderNode, taffy: &Tree, path: &[usize], y: f32, top: f32, bottom: f32) -> BuilderWindow {
    let id = n.attrs.get("id").cloned().unwrap_or_default();
    let binds: Vec<EventBind> = n.events.iter().filter(|e| e.event_type == "itembuild" || e.event_type == "itemdispose").cloned().collect();
    let key = if id.is_empty() { format!("{}@{path:?}", n.tag) } else { format!("#{id}") };
    let span = |c: &RenderNode| taffy.layout(c.taffy_node).ok().map(|l| (y + l.location.y, y + l.location.y + l.size.height));
    let kids = &n.children;
    let per = per_item(&n.attrs);
    let range = if top >= bottom || kids.is_empty() {
        None
    } else if crate::renderer::components::grid_view::is_masonry(&n.attrs) {
        // 瀑布流：顶边不单调，逐项看
        let vis: Vec<usize> = (0..kids.len()).filter(|&i| span(&kids[i]).is_some_and(|(t, b)| b > top && t < bottom)).collect();
        vis.first().zip(vis.last()).map(|(a, b)| (*a, *b + 1))
    } else {
        // 第一个底边进到窗里的、第一个顶边出了窗的（项按文档顺序排，顶边 / 底边都不减）
        let first = kids.partition_point(|c| span(c).is_some_and(|(_, b)| b <= top));
        let end = kids.partition_point(|c| span(c).is_some_and(|(t, _)| t < bottom));
        (first < end).then_some((first, end))
    };
    let range = range.map(|(a, b)| (a / per, (b - 1) / per + 1));
    BuilderWindow { key, range, id, binds }
}
