//! 按选择器在当前布局树里找节点并给出位置：`wx.pageScrollTo({selector})`，
//! 之后的 `wx.createSelectorQuery()` 也走这里。
//!
//! 选择器按官方 SelectorQuery 文档支持的那几种：`#id`、`.class`（可多个）、标签、
//! 后代 `a b`、子代 `a > b`、多选 `a, b`；跨组件的 `>>>` 当后代处理。
//! 坐标是**页面内容坐标**（逻辑像素，不减页面滚动）；scroll-view 里的节点按它当前的
//! 滚动偏移算，position:fixed 的节点给视口坐标（与微信 boundingClientRect 一致时再减页面滚动）。

use super::*;

/// 选择器里的一个复合段：`tag#id.a.b`
#[derive(Debug, Default)]
struct Compound {
    tag: String,
    id: String,
    classes: Vec<String>,
}

/// 一组复合段 + 它们之间的关系（true = 子代 `>`，false = 后代）
#[derive(Debug)]
struct Complex {
    parts: Vec<(Compound, bool)>,
}

/// 一个命中的节点
#[derive(Debug, Clone)]
pub struct QueryHit {
    pub rect: GeoRect,
    pub id: String,
    pub dataset: HashMap<String, String>,
    pub is_fixed: bool,
    pub tag: String,
    /// 与 `get_component_id` 一致（组件事件按它找绑定，见 `host::own_events`）
    pub component_id: String,
    /// 节点属性（模板求值之后；SelectorQuery 的 `fields({properties})` 与 video 的 `src` 都从这里取）
    pub attrs: HashMap<String, String>,
}

fn parse_compound(s: &str) -> Option<Compound> {
    let mut c = Compound::default();
    let mut cur = String::new();
    let mut kind = 't';
    let flush = |kind: char, cur: &mut String, c: &mut Compound| {
        let v = std::mem::take(cur);
        if v.is_empty() {
            return;
        }
        match kind {
            '#' => c.id = v,
            '.' => c.classes.push(v),
            _ => c.tag = v,
        }
    };
    for ch in s.chars() {
        match ch {
            '#' | '.' => {
                flush(kind, &mut cur, &mut c);
                kind = ch;
            }
            ch if ch.is_alphanumeric() || ch == '-' || ch == '_' => cur.push(ch),
            _ => return None,
        }
    }
    flush(kind, &mut cur, &mut c);
    (!c.tag.is_empty() || !c.id.is_empty() || !c.classes.is_empty()).then_some(c)
}

fn parse(sel: &str) -> Vec<Complex> {
    sel.split(',')
        .filter_map(|g| {
            let g = g.replace(">>>", " ").replace('>', " > ");
            let mut parts = Vec::new();
            let mut child = false;
            for tok in g.split_whitespace() {
                if tok == ">" {
                    child = true;
                    continue;
                }
                parts.push((parse_compound(tok)?, child));
                child = false;
            }
            (!parts.is_empty()).then_some(Complex { parts })
        })
        .collect()
}

fn compound_matches(c: &Compound, n: &RenderNode) -> bool {
    if !c.tag.is_empty() && c.tag != n.tag {
        return false;
    }
    if !c.id.is_empty() && n.attrs.get("id").map(|s| s.as_str()) != Some(c.id.as_str()) {
        return false;
    }
    if !c.classes.is_empty() {
        let have = n.attrs.get("class").map(|s| s.as_str()).unwrap_or("");
        if !c.classes.iter().all(|w| have.split_whitespace().any(|h| h == w)) {
            return false;
        }
    }
    true
}

/// 从最后一段往前对祖先链
fn complex_matches(cx: &Complex, node: &RenderNode, ancestors: &[&RenderNode]) -> bool {
    let n = cx.parts.len();
    if !compound_matches(&cx.parts[n - 1].0, node) {
        return false;
    }
    let mut need_child = cx.parts[n - 1].1;
    let mut ai = ancestors.len();
    for (part, child_of_prev) in cx.parts[..n - 1].iter().rev() {
        let mut found = false;
        while ai > 0 {
            ai -= 1;
            if compound_matches(part, ancestors[ai]) {
                found = true;
                break;
            }
            if need_child {
                return false;
            }
        }
        if !found {
            return false;
        }
        need_child = *child_of_prev;
    }
    true
}

impl WxmlRenderer {
    /// 已经布局过（有布局树可查）。SelectorQuery 在这之前到达的要留到下一帧再答
    pub fn has_layout(&self) -> bool {
        self.cache.is_some()
    }

    /// 按选择器找节点（文档顺序）。`all` 为假时只要第一个。还没布局过返回空
    pub fn query_selector(&self, selector: &str, all: bool, interaction: &InteractionManager) -> Vec<QueryHit> {
        let Some(cache) = self.cache.as_ref() else { return Vec::new() };
        let groups = parse(selector);
        if groups.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut chain: Vec<&RenderNode> = Vec::new();
        for n in &cache.render_nodes {
            if self.walk_query(n, &cache.taffy, (0.0, 0.0), false, &groups, all, interaction, &mut chain, &mut out) {
                break;
            }
        }
        out
    }

    /// 返回 true 表示已经找够了（`all` 为假时找到第一个就停）
    #[allow(clippy::too_many_arguments)]
    fn walk_query<'a>(
        &self,
        n: &'a RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        in_fixed: bool,
        groups: &[Complex],
        all: bool,
        interaction: &InteractionManager,
        chain: &mut Vec<&'a RenderNode>,
        out: &mut Vec<QueryHit>,
    ) -> bool {
        let Ok(l) = taffy.layout(n.taffy_node) else { return false };
        let sf = self.scale_factor;
        // fixed 子树的根：按覆盖层钉住它的原点算（视口坐标）。覆盖层还没画过这一版布局时退回正常流的位置
        let pinned = (n.style.is_fixed && !in_fixed && self.fixed_origins.0 == self.layout_generation)
            .then(|| self.fixed_origins.1.get(&n.taffy_node).copied())
            .flatten();
        let (x, y) = match pinned {
            Some((bx, by)) => (bx + l.location.x, by + l.location.y),
            None => (origin.0 + l.location.x, origin.1 + l.location.y),
        };
        let fixed = in_fixed || n.style.is_fixed;
        if groups.iter().any(|g| complex_matches(g, n, chain)) {
            let rect = GeoRect::new(x / sf, y / sf, l.size.width / sf, l.size.height / sf);
            out.push(QueryHit {
                rect,
                id: n.attrs.get("id").cloned().unwrap_or_default(),
                dataset: n.events.first().map(|e| e.data.clone()).unwrap_or_else(|| dataset_of(n)),
                is_fixed: fixed,
                tag: n.tag.clone(),
                component_id: Self::get_component_id(n, &rect),
                attrs: n.attrs.clone(),
            });
            if !all {
                return true;
            }
        }
        // scroll-view 的子节点随它的滚动偏移移动
        let (mut cx, mut cy) = (x, y);
        if n.tag == "scroll-view" {
            let bounds = GeoRect::new(x / sf, y / sf, l.size.width / sf, l.size.height / sf);
            let id = Self::get_component_id(n, &bounds);
            if let Some(c) = interaction.get_scroll_controller(&id) {
                if super::scroll_views::scroll_axes(&n.attrs).0 {
                    cx -= c.get_position() * sf;
                } else {
                    cy -= c.get_position() * sf;
                }
            }
        }
        chain.push(n);
        for c in &n.children {
            if self.walk_query(c, taffy, (cx, cy), fixed, groups, all, interaction, chain, out) {
                chain.pop();
                return true;
            }
        }
        chain.pop();
        false
    }
}

/// 节点上的 `data-*`（没有事件绑定的节点，dataset 不在事件表里，直接从属性取）
fn dataset_of(n: &RenderNode) -> HashMap<String, String> {
    n.attrs
        .iter()
        .filter_map(|(k, v)| k.strip_prefix("data-").map(|d| (crate::renderer::components::event_attr::dataset_key(d), v.clone())))
        .collect()
}
