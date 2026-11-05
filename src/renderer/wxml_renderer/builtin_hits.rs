//! `<navigator>` 与 `<label>` 的点击命中：点在它们里面时，找出要跳转的地址 / 要触发的控件。
//!
//! 与 `form.rs` 同理走布局树（而不是本帧的交互元素表）：navigator / label 自己通常没有任何绑定，
//! 不会出现在元素表里。坐标、scroll-view 偏移、组件 id 的算法都复用 `form_geom`，
//! 算出来的 id 与交互元素表里的一致（label 要据此找到控件在表里的位置）。
//!
//! 依据官方 component/navigator.html、component/label.html：
//! - navigator：`open-type` 为 navigate（默认）/ redirect / switchTab / reLaunch / navigateBack（配 `delta`）/
//!   exit；`target="miniProgram"` 跳其它小程序，交给宿主（`navigateToMiniProgram`）。
//!   点在里面、且这一路上没有被 `catchtap` 截住才跳。
//! - label：有 `for` 时触发 `id` 等于它的控件，否则触发 label 里的第一个控件；
//!   能绑定的控件是 button / checkbox / radio / switch / input / textarea。

use super::*;

/// 命中的 navigator
#[derive(Clone, Debug, PartialEq)]
pub struct NavigatorHit {
    pub url: String,
    pub open_type: String,
    pub delta: u32,
    /// `target="miniProgram"` 时的 app-id / path / extra-data / version
    pub mini_program: Option<HashMap<String, String>>,
}

const LABEL_CONTROLS: &[&str] = &["button", "checkbox", "radio", "switch", "input", "textarea"];

/// 一个单选 / 多选组：(组的组件 id, 是不是 radio-group, 成员 (组件 id, value, 属性里是否选中))
type ChoiceGroup = (String, bool, Vec<(String, String, bool)>);

fn catches_tap(n: &RenderNode) -> bool {
    n.attrs.keys().any(|k| k == "catchtap" || k == "catch:tap" || k == "capture-catch:tap" || k == "mut-bind:tap")
}

impl WxmlRenderer {
    /// (x, y) 上的 navigator（最内层的那个）。`fixed` 与命中坐标系一致：覆盖层是视口坐标，正常流是内容坐标
    pub fn navigator_at(&self, x: f32, y: f32, fixed: bool, interaction: &InteractionManager) -> Option<NavigatorHit> {
        let cache = self.cache.as_ref()?;
        let mut best = None;
        for n in &cache.render_nodes {
            self.find_navigator(n, &cache.taffy, (0.0, 0.0), false, (x, y, fixed), interaction, None, &mut best);
        }
        best
    }

    #[allow(clippy::too_many_arguments)]
    fn find_navigator(
        &self,
        n: &RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        in_fixed: bool,
        pt: (f32, f32, bool),
        interaction: &InteractionManager,
        // 外层最近的 navigator，以及它之后这一路上是否遇到过 catchtap
        outer: Option<(&RenderNode, bool)>,
        best: &mut Option<NavigatorHit>,
    ) {
        let Some((rect, _, child_origin)) = self.form_geom(n, taffy, origin, interaction) else { return };
        let fixed = in_fixed || n.style.is_fixed;
        // scroll-view 会裁掉子节点：点不在盒子里的不往下找（navigator 本身可能比内容大，照样往下看子节点）
        let inside = fixed == pt.2 && rect.contains(&crate::Point::new(pt.0, pt.1));
        let mut ctx = outer.map(|(nav, caught)| (nav, caught || (inside && catches_tap(n))));
        if n.tag == "navigator" && inside {
            // navigator 自己写了 catchtap 只是不再往外冒泡，自己的跳转照样发生
            ctx = Some((n, false));
        }
        if inside {
            if let Some((nav, caught)) = ctx {
                // 越深越靠后：最后一次赋值就是最内层命中点所在的 navigator
                *best = if caught { None } else { Some(navigator_hit(nav)) };
            }
        }
        for c in &n.children {
            self.find_navigator(c, taffy, child_origin, fixed, pt, interaction, ctx, best);
        }
    }

    /// (x, y) 落在一个 label 里、且不是直接点在它要触发的那个控件上时，返回那个控件的组件 id
    pub fn label_target_at(&self, x: f32, y: f32, fixed: bool, interaction: &InteractionManager) -> Option<String> {
        self.label_target(x, y, fixed, interaction).map(|t| t.0)
    }

    /// 同上，另给控件的标签名（`for` 指向的控件可能不在视口里、不在本帧的交互元素表里，
    /// 转发方按标签直接改它的状态）
    pub fn label_target(&self, x: f32, y: f32, fixed: bool, interaction: &InteractionManager) -> Option<(String, String)> {
        let cache = self.cache.as_ref()?;
        let mut label: Option<(&RenderNode, (f32, f32), bool)> = None;
        for n in &cache.render_nodes {
            self.find_label(n, &cache.taffy, (0.0, 0.0), false, (x, y, fixed), interaction, &mut label);
        }
        let (node, origin, in_fixed) = label?;
        // for：整棵树里 id 等于它的控件；否则 label 里的第一个控件
        let target = match node.attrs.get("for").map(|s| s.trim()).filter(|s| !s.is_empty()) {
            Some(for_id) => {
                let mut found = None;
                for n in &cache.render_nodes {
                    self.find_control(n, &cache.taffy, (0.0, 0.0), interaction, Some(for_id), &mut found);
                }
                found
            }
            None => {
                let mut found = None;
                for c in &node.children {
                    let child_origin = self.form_geom(node, &cache.taffy, origin, interaction).map(|g| g.2).unwrap_or(origin);
                    self.find_control(c, &cache.taffy, child_origin, interaction, None, &mut found);
                    if found.is_some() {
                        break;
                    }
                }
                found
            }
        };
        let (id, rect, tag) = target?;
        // 点的就是控件本身：它自己已经响应过这次点击
        if in_fixed == fixed && rect.contains(&crate::Point::new(x, y)) {
            return None;
        }
        Some((id, tag))
    }

    /// 单选 / 多选组：`member` 是组里某个 radio / checkbox 的组件 id（或组自己的组件 id），
    /// 给出这个组的全部成员 `(组件 id, value, 属性里是否选中)`，文档顺序。不在组里返回 None。
    ///
    /// 走布局树（与 form.rs 同理）：本帧的交互元素表只有视口附近画到的那几个，长列表里滚出去的成员
    /// 不在表里 —— 从前 radio 的互斥与组的 `detail.value` 都按元素表算，滚出去的那几项既不会被取消选中、
    /// 也不进 change 的值。代价：只在点了 radio / checkbox 时走一遍布局树
    pub fn choice_group(&self, member: &str, interaction: &InteractionManager) -> Option<Vec<(String, String, bool)>> {
        let cache = self.cache.as_ref()?;
        let mut groups: Vec<ChoiceGroup> = Vec::new();
        for n in &cache.render_nodes {
            self.collect_choice_groups(n, &cache.taffy, (0.0, 0.0), interaction, None, &mut groups);
        }
        groups.into_iter().find(|(gid, _, items)| gid == member || items.iter().any(|m| m.0 == member)).map(|g| g.2)
    }

    fn collect_choice_groups(
        &self,
        n: &RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        interaction: &InteractionManager,
        group: Option<usize>,
        out: &mut Vec<ChoiceGroup>,
    ) {
        let Some((_, id, child_origin)) = self.form_geom(n, taffy, origin, interaction) else { return };
        let mut group = group;
        match n.tag.as_str() {
            "radio-group" | "checkbox-group" => {
                out.push((id, n.tag == "radio-group", Vec::new()));
                group = Some(out.len() - 1);
            }
            // 成员只认对应的那一种（radio-group 里的 checkbox 不算）
            "radio" | "checkbox" => {
                if let Some(g) = group.filter(|&g| out[g].1 == (n.tag == "radio")) {
                    out[g].2.push((id, Self::input_value_attr(n), n.style.custom_data > 0.5));
                }
                return;
            }
            _ => {}
        }
        for c in &n.children {
            self.collect_choice_groups(c, taffy, child_origin, interaction, group, out);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn find_label<'a>(
        &self,
        n: &'a RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        in_fixed: bool,
        pt: (f32, f32, bool),
        interaction: &InteractionManager,
        best: &mut Option<(&'a RenderNode, (f32, f32), bool)>,
    ) {
        let Some((rect, _, child_origin)) = self.form_geom(n, taffy, origin, interaction) else { return };
        let fixed = in_fixed || n.style.is_fixed;
        if n.tag == "label" && fixed == pt.2 && rect.contains(&crate::Point::new(pt.0, pt.1)) {
            *best = Some((n, origin, fixed));
        }
        for c in &n.children {
            self.find_label(c, taffy, child_origin, fixed, pt, interaction, best);
        }
    }

    /// 第一个（`want_id` 给了时是 id 匹配的那个）可被 label 触发的控件：(组件 id, 逻辑包围盒)
    fn find_control(
        &self,
        n: &RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        interaction: &InteractionManager,
        want_id: Option<&str>,
        found: &mut Option<(String, GeoRect, String)>,
    ) {
        if found.is_some() {
            return;
        }
        let Some((rect, id, child_origin)) = self.form_geom(n, taffy, origin, interaction) else { return };
        if LABEL_CONTROLS.contains(&n.tag.as_str()) {
            let matches = match want_id {
                Some(w) => n.attrs.get("id").map(|s| s.trim()) == Some(w),
                None => true,
            };
            if matches {
                *found = Some((id, rect, n.tag.clone()));
                return;
            }
        }
        for c in &n.children {
            self.find_control(c, taffy, child_origin, interaction, want_id, found);
        }
    }
}

fn navigator_hit(n: &RenderNode) -> NavigatorHit {
    let a = |k: &str| n.attrs.get(k).map(|s| s.trim().to_string()).unwrap_or_default();
    let open_type = match a("open-type").as_str() {
        "" => "navigate".to_string(),
        t => t.to_string(),
    };
    let mini_program = (a("target") == "miniProgram").then(|| {
        ["app-id", "path", "extra-data", "version", "short-link"]
            .iter()
            .filter_map(|k| n.attrs.get(*k).map(|v| (k.to_string(), v.clone())))
            .collect()
    });
    NavigatorHit { url: a("url"), open_type, delta: a("delta").parse().unwrap_or(1).max(1), mini_program }
}
