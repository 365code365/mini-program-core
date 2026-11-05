//! `<scroll-view>` 的属性：滚动方向、受控位置（`scroll-top` / `scroll-left` / `scroll-into-view`）、
//! 阈值与下拉刷新配置。
//!
//! 绘制期在 `register_interactive_element` 里登记（与 picker / 原生组件同一套：只登记画到的、
//! 每帧随绘制重建），宿主每帧据此派发 `bindscroll` / `scrolltolower` / `refresher*`
//! （`host::scroll_view`）。
//!
//! **受控位置在这里落地**而不是在宿主：登记发生在画 scroll-view 子节点之前，这一帧画出来的
//! 就已经是新位置 —— 页面一打开就该停在 `scroll-top` 处，不能先画一帧顶部再跳。
//! 只有属性值**变了**才动位置（上一次的值记在 `InteractionManager::scroll_views`），否则用户
//! 一拖，下一帧又被属性拽回去。
//!
//! 依据官方 component/scroll-view.html：长度属性默认单位 px，支持 `rpx` / `px`；
//! `scroll-into-view` 优先于 `scroll-top`；`scroll-with-animation` 时滑过去。

use super::*;
use crate::renderer::components::parse_color_str;
use crate::ui::interaction::InteractionManager;

/// 改受控位置时的动画时长（秒）。文档没给数，取与 `wx.pageScrollTo` 默认值相同的 300ms
pub const SCROLL_WITH_ANIMATION_SECS: f32 = 0.3;

/// 一个 scroll-view 在本帧的登记信息（逻辑像素）
#[derive(Debug, Clone)]
pub struct ScrollViewInfo {
    /// 与 `get_component_id` 一致，也是滚动控制器的键
    pub component_id: String,
    /// 声明它的组件实例键（页面里是空串）：判断 `refresher-triggered` 是不是被 setData 过
    pub owner: String,
    pub horizontal: bool,
    /// 横竖两个方向都能滚（[`scroll_both`]；`horizontal` 此时为 false，横向在 `"{id}#x"` 控制器上）
    pub both: bool,
    /// 有没有写 `scroll-x` / `scroll-y`（都没写就不能滚）
    pub scrollable: bool,
    pub bounds: GeoRect,
    pub content_width: f32,
    pub content_height: f32,
    pub is_fixed: bool,
    pub upper_threshold: f32,
    pub lower_threshold: f32,
    /// `enhanced`：`dragstart` / `dragging` / `dragend` 只在它开着时发
    pub enhanced: bool,
    /// `enable-back-to-top`：点状态栏（iOS）/ 双击标题栏（安卓）时滚回顶部，只对纵向生效
    pub back_to_top: bool,
    /// 画滚动条（`enhanced` + `show-scrollbar`，或 Context 的 showScrollbar；scrollbar.rs）
    pub scrollbar: bool,
    pub refresher: Option<RefresherConfig>,
    /// `scroll-into-view` 绑定的根字段名（`INTO_VIEW_KEYS_ATTR`）：同一帧里改过又改回原值也要再滚（host/scroll_view.rs）
    pub into_keys: Vec<String>,
}

/// `refresher-*` 配置（`refresher-enabled` 时才有）
#[derive(Debug, Clone)]
pub struct RefresherConfig {
    /// 触发阈值（逻辑 px，默认 45）
    pub threshold: f32,
    /// `black` / `white` / `none`
    pub default_style: String,
    pub background: Option<crate::Color>,
    /// `refresher-triggered` 的当前值
    pub triggered: bool,
    /// `refresher-triggered` 绑定表达式里的根字段名（模板展开时记下，见 `REFRESHER_KEYS_ATTR`）
    pub triggered_keys: Vec<String>,
    /// 下拉二级（Skyline `refresher-two-level-enabled`）；没开为 None
    pub two_level: Option<TwoLevelConfig>,
}

/// 下拉二级的配置（官方 scroll-view「下拉二级」一节，都是逻辑 px）
#[derive(Debug, Clone, Copy)]
pub struct TwoLevelConfig {
    /// `refresher-two-level-threshold`（默认 150）：拉过它松手打开二级
    pub threshold: f32,
    /// `refresher-two-level-close-threshold`（默认 80）：二级里往上推过它松手关闭
    pub close_threshold: f32,
    /// `refresher-two-level-scroll-enabled`（默认 false）：二级状态下能不能拖（能拖才能滑动关闭）
    pub scroll_enabled: bool,
    /// `refresher-two-level-triggered` 的当前值（页面受控开 / 关）
    pub triggered: bool,
    /// 盒子高（逻辑 px）：二级打开时内容推下去这么多
    pub height: f32,
}

pub use crate::parser::template::REFRESHER_KEYS_ATTR;

/// 布尔属性（`<scroll-view scroll-y>` 不带值解析成 "true"；`{{flag}}` 求出空是假），规则见 `attr_is_true`
pub fn bool_attr(attrs: &HashMap<String, String>, name: &str) -> bool {
    attrs.get(name).map(|v| crate::renderer::components::attr_is_true(v)).unwrap_or(false)
}

/// Skyline `clip`（默认 true）：false 时内容不裁到盒子，滚到盒子外、溢出盒子的部分照画
pub fn clips(attrs: &HashMap<String, String>) -> bool {
    attrs.get("clip").map(|v| crate::renderer::components::attr_is_true(v)).unwrap_or(true)
}

/// 滚动方向：`(横向, 可滚)`。两个都写时按纵向（单轴控制器）
pub fn scroll_axes(attrs: &HashMap<String, String>) -> (bool, bool) {
    let x = bool_attr(attrs, "scroll-x");
    let y = bool_attr(attrs, "scroll-y");
    (x && !y, x || y)
}

/// `scroll-x` 与 `scroll-y` 都写了：横竖两个方向同时滚（主控制器管纵向，横向另一个，ui/interaction/scroll_2d.rs）。
/// 布局照纵向（子节点竖排，宽出盒子的那部分横着滚出来），与 WebView 下块级子节点的排法一致
pub fn scroll_both(attrs: &HashMap<String, String>) -> bool {
    bool_attr(attrs, "scroll-x") && bool_attr(attrs, "scroll-y")
}

/// Skyline `padding="[上, 右, 下, 左]"`（px）：scroll-view 内容区的内边距（不是四个数就不动）
pub fn apply_padding_attr(attrs: &HashMap<String, String>, ts: &mut taffy::style::Style, sf: f32) {
    let Some(p) = attrs.get("padding") else { return };
    let v = crate::renderer::components::parse_attr_json(p);
    let Some(a) = v.as_array().filter(|a| a.len() == 4) else { return };
    let n = |i: usize| taffy::style::LengthPercentage::length(a[i].as_f64().unwrap_or(0.0) as f32 * sf);
    ts.padding = taffy::geometry::Rect { top: n(0), right: n(1), bottom: n(2), left: n(3) };
}

/// 长度属性：数字按 px，带 `rpx` / `px` 单位的换算（rpx 以 750 为屏宽）
pub fn parse_len(v: &str, screen_width: f32) -> Option<f32> {
    let v = v.trim();
    if let Some(n) = v.strip_suffix("rpx") {
        return n.trim().parse::<f32>().ok().map(|n| n * screen_width / 750.0);
    }
    v.strip_suffix("px").unwrap_or(v).trim().parse::<f32>().ok().filter(|n| n.is_finite())
}

impl WxmlRenderer {
    /// 本帧登记的 scroll-view
    pub fn scroll_views(&self) -> &[ScrollViewInfo] {
        &self.scroll_views
    }

    /// 绘制期登记，并把受控位置落到滚动控制器上（见模块注释）
    #[allow(clippy::too_many_arguments)]
    pub(super) fn register_scroll_view(
        &mut self,
        node: &RenderNode,
        taffy: &Tree,
        component_id: &str,
        bounds: &GeoRect,
        content: (f32, f32),
        is_fixed: bool,
        interaction: &mut InteractionManager,
    ) {
        if self.scroll_views.iter().any(|s| s.component_id == component_id) {
            return;
        }
        let a = &node.attrs;
        let sw = self.screen_width;
        let len = |k: &str, d: f32| a.get(k).and_then(|v| parse_len(v, sw)).unwrap_or(d);
        let (horizontal, scrollable) = scroll_axes(a);
        let refresher = bool_attr(a, "refresher-enabled").then(|| RefresherConfig {
            threshold: len("refresher-threshold", 45.0).max(1.0),
            default_style: a.get("refresher-default-style").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| "black".into()),
            background: a.get("refresher-background").and_then(|s| parse_color_str(s)),
            triggered: bool_attr(a, "refresher-triggered"),
            triggered_keys: a
                .get(REFRESHER_KEYS_ATTR)
                .map(|s| s.split(',').filter(|k| !k.is_empty()).map(str::to_string).collect())
                .unwrap_or_default(),
            two_level: bool_attr(a, "refresher-two-level-enabled").then(|| TwoLevelConfig {
                threshold: len("refresher-two-level-threshold", 150.0).max(1.0),
                close_threshold: len("refresher-two-level-close-threshold", 80.0).max(1.0),
                scroll_enabled: bool_attr(a, "refresher-two-level-scroll-enabled"),
                triggered: bool_attr(a, "refresher-two-level-triggered"),
                height: bounds.height,
            }),
        });
        let info = ScrollViewInfo {
            component_id: component_id.to_string(),
            owner: a.get(crate::parser::template::COMPONENT_OWNER_ATTR).cloned().unwrap_or_default(),
            horizontal,
            both: scroll_both(a),
            scrollable,
            bounds: *bounds,
            content_width: content.0,
            content_height: content.1,
            is_fixed,
            upper_threshold: len("upper-threshold", 50.0),
            lower_threshold: len("lower-threshold", 50.0),
            enhanced: bool_attr(a, "enhanced"),
            // 没显式写 false、占屏 90% 以上时自动开（scroll_behavior.rs）
            back_to_top: self.back_to_top(a, bounds, horizontal),
            scrollbar: self.apply_scroll_behavior(node, component_id, interaction),
            refresher,
            into_keys: a
                .get(crate::parser::template::INTO_VIEW_KEYS_ATTR)
                .map(|s| s.split(',').filter(|k| !k.is_empty()).map(str::to_string).collect())
                .unwrap_or_default(),
        };
        if scrollable {
            if info.both {
                // 双向：横向那一半的受控位置（scroll-left / scroll-into-view 的横向，scroll_2d_attrs.rs）
                self.apply_controlled_left(node, taffy, &info, interaction);
            }
            let controlled = self.apply_controlled_position(node, taffy, &info, interaction);
            // scroll-anchoring / reverse 贴底（scroll_behavior.rs）
            self.keep_scroll_anchor(node, taffy, component_id, horizontal, controlled, interaction);
        }
        self.scroll_views.push(info);
    }

    /// `scroll-into-view`（优先）/ `scroll-top` / `scroll-left`：值变了才动。返回这一帧动没动
    fn apply_controlled_position(
        &self,
        node: &RenderNode,
        taffy: &Tree,
        info: &ScrollViewInfo,
        interaction: &mut InteractionManager,
    ) -> bool {
        let a = &node.attrs;
        let sw = self.screen_width;
        let into = a.get("scroll-into-view").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let pos_attr = if info.horizontal { "scroll-left" } else { "scroll-top" };
        let pos = a.get(pos_attr).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let animate = bool_attr(a, "scroll-with-animation");
        let st = interaction.scroll_views.entry(info.component_id.clone()).or_default();
        st.refresher_enabled = info.refresher.is_some();
        st.nested_outer = a.get("type").is_some_and(|t| t.trim() == "nested") && !info.horizontal;
        let first = !st.seen;
        st.seen = true;
        let into_changed = into.is_some() && st.last_into_view != into;
        let last_pos = if info.horizontal { &mut st.last_scroll_left } else { &mut st.last_scroll_top };
        let pos_changed = pos.is_some() && *last_pos != pos;
        *last_pos = pos.clone();
        st.last_into_view = into.clone();
        let target = if into_changed {
            into.as_deref().and_then(|id| self.into_view_target(node, taffy, id, info, interaction))
        } else if pos_changed {
            pos.as_deref().and_then(|v| parse_len(v, sw))
        } else {
            None
        };
        // Skyline `reverse`：初始停在末尾（聊天列表那种从底部开始的写法）
        let reverse_first = first && target.is_none() && bool_attr(a, "reverse");
        if target.is_none() && !reverse_first {
            return false;
        }
        if let Some(c) = interaction.get_scroll_controller_mut(&info.component_id) {
            // 手指正拖着的时候不跟它抢（受控值在松手后的下一次变化才生效）
            if c.is_dragging {
                return false;
            }
            let target = target.unwrap_or_else(|| c.get_max_scroll());
            if animate && !first {
                c.animate_to(target, SCROLL_WITH_ANIMATION_SECS);
            } else {
                c.set_position(target);
            }
            return true;
        }
        false
    }

    /// `scroll-into-view` 的目标位置：在子树里找 `id`，按 `scroll-into-view-alignment`
    /// （Skyline：start / center / end / nearest，默认 start）与 `scroll-into-view-offset` 换算。
    /// 找不到返回 None（微信里同样不动）
    fn into_view_target(
        &self,
        node: &RenderNode,
        taffy: &Tree,
        id: &str,
        info: &ScrollViewInfo,
        interaction: &InteractionManager,
    ) -> Option<f32> {
        let sf = self.scale_factor;
        let (x, y, w, h) = find_anchor(&node.children, taffy, id, 0.0, 0.0)?;
        let (start, size, view) = if info.horizontal {
            (x / sf, w / sf, info.bounds.width)
        } else {
            (y / sf, h / sf, info.bounds.height)
        };
        let offset = node.attrs.get("scroll-into-view-offset").and_then(|v| parse_len(v, self.screen_width)).unwrap_or(0.0);
        let align = node.attrs.get("scroll-into-view-alignment").map(|s| s.trim()).unwrap_or("start");
        let cur = interaction.get_scroll_controller(&info.component_id).map(|c| c.get_position()).unwrap_or(0.0);
        let target = match align {
            "center" => start + size / 2.0 - view / 2.0,
            "end" => start + size - view,
            "nearest" => {
                if start >= cur && start + size <= cur + view {
                    return None; // 已经整个在视口里：不动
                } else if start < cur {
                    start
                } else {
                    start + size - view
                }
            }
            _ => start,
        };
        Some(target + offset)
    }
}

/// 自定义下拉刷新节点（`slot="refresher"` 的子节点，`refresher-default-style="none"` 时用）：
/// 不占内容流，绝对定位在内容**上方**（底边贴着内容顶边）。下拉时内容整体下移，
/// 它就从上面露出来 —— 与微信「刷新区域在列表上方，被拉出来」一致。
pub(super) fn place_refresher_slot(node: &crate::parser::WxmlNode, children: &[RenderNode], taffy: &mut Tree) {
    if !bool_attr(&node.attributes, "refresher-enabled") {
        return;
    }
    for c in children {
        if c.attrs.get("slot").map(|s| s.trim() == "refresher").unwrap_or(false) {
            if let Ok(mut st) = taffy.style(c.taffy_node).cloned() {
                st.position = taffy::style::Position::Absolute;
                st.inset.left = taffy::style::LengthPercentageAuto::length(0.0);
                st.inset.right = taffy::style::LengthPercentageAuto::length(0.0);
                st.inset.top = taffy::style::LengthPercentageAuto::auto();
                st.inset.bottom = taffy::style::LengthPercentageAuto::percent(1.0);
                let _ = taffy.set_style(c.taffy_node, st);
            }
        }
    }
}

/// 在子树里按 `id` 找节点，返回相对 scroll-view 内容原点的 (x, y, w, h)（物理像素）。
/// 不进嵌套的 scroll-view —— 那是它自己的内容
pub(super) fn find_anchor(list: &[RenderNode], taffy: &Tree, id: &str, ox: f32, oy: f32) -> Option<(f32, f32, f32, f32)> {
    for n in list {
        let Ok(l) = taffy.layout(n.taffy_node) else { continue };
        let (x, y) = (ox + l.location.x, oy + l.location.y);
        if n.attrs.get("id").map(|s| s == id).unwrap_or(false) {
            return Some((x, y, l.size.width, l.size.height));
        }
        if n.tag != "scroll-view" {
            if let Some(hit) = find_anchor(&n.children, taffy, id, x, y) {
                return Some(hit);
            }
        }
    }
    None
}

impl WxmlRenderer {
    /// `clip="{{false}}"`：滚动时盒子外也画着的范围（逻辑 px，页面内容坐标）与「画到哪算不准」（里面有 transform / 动画）。
    ///
    /// 主轴：内容在盒子两侧各多画一个盒子长（scroll_live.rs / draw_interactive.rs 的 `ext`，与 swiper clip=false 同一个约定）；
    /// 交叉轴：子树的溢出并集（阴影那圈也算，自己裁剪的子节点只算它的盒子）。与滚动位置无关，一个矩形管前后两帧。
    /// 裁到盒子的（默认）返回 `(None, false)`
    pub(super) fn clip_false_paint(&self, taffy: &Tree, node: &RenderNode, bounds: &GeoRect) -> (Option<GeoRect>, bool) {
        if clips(&node.attrs) {
            return (None, false);
        }
        fn walk(t: &Tree, n: &RenderNode, ox: f32, oy: f32, e: &mut [f32; 4], unbounded: &mut bool) {
            if n.style.is_fixed {
                return;
            }
            if n.style.transform.is_some() || n.style.animation.is_some() {
                *unbounded = true;
                return;
            }
            let Ok(l) = t.layout(n.taffy_node) else { return };
            let (x, y) = (ox + l.location.x, oy + l.location.y);
            let r = super::invalidate_grow::inflate(GeoRect::new(x, y, l.size.width, l.size.height), &n.style);
            *e = [e[0].min(r.x), e[1].min(r.y), e[2].max(r.x + r.width), e[3].max(r.y + r.height)];
            let clips = n.style.overflow != crate::renderer::components::Overflow::Visible
                || crate::renderer::components::spec_for(n.tag.as_str()).clips;
            if !clips {
                for c in &n.children {
                    walk(t, c, x, y, e, unbounded);
                }
            }
        }
        let mut e = [0.0f32; 4];
        let mut unbounded = false;
        for c in &node.children {
            walk(taffy, c, 0.0, 0.0, &mut e, &mut unbounded);
        }
        let sf = self.scale_factor;
        let (bw, bh) = (bounds.width, bounds.height);
        // 双向又不裁：横向不剔除子节点，盒子外画到哪随两个轴的位置变，不算这一块，滚动按整帧（少见的组合）
        if scroll_both(&node.attrs) {
            return (Some(*bounds), true);
        }
        let (x0, x1, y0, y1) = if scroll_axes(&node.attrs).0 {
            (bounds.x - bw, bounds.x + 2.0 * bw, bounds.y + e[1] / sf, bounds.y + e[3] / sf)
        } else {
            (bounds.x + e[0] / sf, bounds.x + e[2] / sf, bounds.y - bh, bounds.y + 2.0 * bh)
        };
        (Some(GeoRect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))), unbounded)
    }
}
