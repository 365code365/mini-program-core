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
        let (id, rect) = target?;
        // 点的就是控件本身：它自己已经响应过这次点击
        if in_fixed == fixed && rect.contains(&crate::Point::new(x, y)) {
            return None;
        }
        Some(id)
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
        found: &mut Option<(String, GeoRect)>,
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
                *found = Some((id, rect));
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
