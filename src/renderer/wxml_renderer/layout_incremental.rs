//! setData 的增量布局：没变的子树（`RenderNode` + 它的 taffy 节点 + taffy 的布局缓存）原样留着，
//! 只重建变了的那几块，taffy 也只重排脏了的那条路径。
//!
//! 从前每次 setData 都是「模板展开 → 整棵建树（选择器匹配、量字）→ 一棵新 taffy 树整页求解」。
//! 一加 PLY110「节点与滚动」页：回调里改一个悬浮按钮上的数字，重建 3.5~5ms，其中建树与 taffy 求解七成。
//!
//! ## 什么算「没变」
//! 新旧两次模板展开在同一位置的节点**整棵逐字段相等**（标签、属性、文字、孩子），并且它的上下文没变：
//! 只往下走「自己的标签与属性没变、孩子个数没变」的容器（[`descendable`]），所以走到的每个节点的
//! 祖先链、兄弟位置、继承来的文字样式、有没有定位祖先都与上次相同 —— 建树的全部输入都相同，
//! 建出来的东西也就相同，原样留着就行。子树里只能有建树只看属性与上下文的标签（[`pure_tag`]）：
//! 图片的固有尺寸、输入框的值这类别处的状态不进键，那种子树每次照旧重建。
//!
//! ## 还原「布局之后才改的样式」
//! 第二遍修正（换行补高、绝对定位包含块）与覆盖层重排会改树上节点的样式。整棵重建时新树从建树那份样式起排；
//! 留下来的节点也得从那份起排，结果才一样 —— 所以那几处改样式之前把原样记在 `layout_touched`，
//! 这里先还原（只记缓存那棵树，`record_touch`）。
//!
//! ## 失效分析
//! 新旧现在是同一棵 taffy 树：改之前先把旧布局拍下来（layout_snapshot.rs），失效分析读那份。
//! 留下来的子树在旧 `RenderNode` 树里换成一个占位（[`REUSED_TAG`]），比对时旧的就是新的这一份（[`resolve`]），
//! 只比位置尺寸变没变。
//!
//! `MINI_NO_INCREMENTAL_LAYOUT=1` 关掉（每次整棵重建，对照用）；`MINI_LAYOUT_VERIFY=1` 每次增量重建之后
//! 再整棵重建一遍逐节点比布局，不一致就打出来（测试用，很慢）。
use super::invalidate::OldLayout;
use super::layout_snapshot::LayoutSnapshot;
use super::*;

/// 旧 `RenderNode` 树里「这棵子树已经搬到新树里去了」的占位标签（NUL 开头，不会是真标签）
pub(super) const REUSED_TAG: &str = "\u{0}reused";

pub(super) fn is_placeholder(n: &RenderNode) -> bool {
    n.tag == REUSED_TAG
}

/// 失效分析里旧树的节点：是占位就换回搬到新树里的那个真节点（内容与从前逐字段相同，布局从旧快照取），
/// 失效分析因此看到的旧树与整棵重建时一模一样。找不到（不该发生）就原样给占位，标签对不上会退回整帧
pub(super) fn resolve<'a>(old: &'a RenderNode, moved: &HashMap<NodeId, &'a RenderNode>) -> &'a RenderNode {
    if is_placeholder(old) { moved.get(&old.taffy_node).copied().unwrap_or(old) } else { old }
}

/// 建树读了属性与上下文以外状态的标签：它们的子树每次都重建。
///
/// 逐个看过各组件的 `build`（components/registry.rs 那张表）：
/// - `video`：静音态从播放器的界面状态里取（`video::ui::effective_muted`），不在属性里；
/// - `grid-view`：瀑布流在布局**之后**改孩子的样式（`masonry_pass`），那几处不在 `layout_touched` 的记录里；
/// - 宿主承载的直播 / 相机组件：交给宿主，不猜。
/// 其余（view / text / image / input / button / picker / swiper / scroll-view / canvas / rich-text / 表单控件 /
/// 没登记的标签按 view 建）只读属性、样式表与字体，同样的输入建出同样的东西。
/// 图片的固有尺寸**不进**布局（`ImageComponent::build` 只用样式里的尺寸，没写就是 150×100 的默认值），所以图片能留
fn pure_tag(tag: &str) -> bool {
    !matches!(tag, "video" | "grid-view" | "live-player" | "live-pusher" | "camera" | "voip-room")
}

/// 可以「重建自己、孩子各自判」的容器：build_tree 走容器那条路（不是叶子组件），并且建树不读别处的状态
fn container_tag(tag: &str) -> bool {
    pure_tag(tag) && !crate::renderer::components::spec_for(tag).leaf
}

/// 自己这一层相等（不看孩子的个数与内容）：往下走的前提
fn same_self(a: &WxmlNode, b: &WxmlNode) -> bool {
    a.node_type == b.node_type && a.tag_name == b.tag_name && a.text_content == b.text_content && a.attributes == b.attributes
}

/// 两次模板展开的同一个节点：自己这一层相等（不看孩子）
fn same_shallow(a: &WxmlNode, b: &WxmlNode) -> bool {
    a.children.len() == b.children.len() && same_self(a, b)
}

/// 整棵相等，并且整棵里都是 [`pure_tag`]（文字节点也算）
fn reusable(a: &WxmlNode, b: &WxmlNode) -> bool {
    if !same_shallow(a, b) {
        return false;
    }
    if a.node_type == WxmlNodeType::Element && !pure_tag(&a.tag_name) {
        return false;
    }
    a.children.iter().zip(&b.children).all(|(x, y)| reusable(x, y))
}

/// build_tree 会不会给这个展开节点建出一个 `RenderNode`（与 build_tree 开头的判断一致）
fn produces(n: &WxmlNode) -> bool {
    match n.node_type {
        WxmlNodeType::Text => !n.text_content.trim().is_empty(),
        WxmlNodeType::Element => true,
        _ => false,
    }
}

/// 把 `id` 和**挂在它名下**的子孙从树里拿掉（已经改挂到别处的孩子不动）
pub(super) fn remove_owned(t: &mut Tree, id: NodeId) {
    let kids: Vec<NodeId> = t.children(id).unwrap_or_default();
    for c in kids {
        if t.parent(c) == Some(id) {
            remove_owned(t, c);
        }
    }
    let _ = t.remove(id);
}

/// 旧的那一份：展开节点、它建出来的 RenderNode（留下来时换成占位）、它当时的兄弟序号与兄弟数
type Old<'a> = Option<(&'a WxmlNode, &'a mut RenderNode, usize, usize)>;

/// 把一串旧孩子对到新孩子上（`out[新序号]` = 那个位置要比的旧孩子）。
///
/// 个数没变：按位置一一对（与从前一样）。个数变了（列表追加 / 删一条 / 根上的 `wx:if`）：
/// 头上一段、尾上一段标签相同的按位置对（追加只动尾巴、删中间一条只动那一截），中间那段新的从头建、旧的拿掉。
/// 旧孩子的「展开节点 → RenderNode」对不上（某个元素当时没建出节点）就返回 None：这一层整个重建
fn align_olds<'a>(old_x: &'a [WxmlNode], old_r: &'a mut [RenderNode], new_x: &[WxmlNode]) -> Option<Vec<Old<'a>>> {
    if old_x.iter().filter(|c| produces(c)).count() != old_r.len() {
        return None;
    }
    let n_old = old_x.len();
    let mut it = old_r.iter_mut();
    let mut olds: Vec<Old<'a>> =
        old_x.iter().enumerate().map(|(k, x)| if produces(x) { it.next().map(|r| (x, r, k, n_old)) } else { None }).collect();
    let mut out: Vec<Old<'a>> = (0..new_x.len()).map(|_| None).collect();
    if n_old == new_x.len() {
        for (k, o) in olds.into_iter().enumerate() {
            out[k] = o;
        }
        return Some(out);
    }
    let like = |a: &WxmlNode, b: &WxmlNode| a.node_type == b.node_type && a.tag_name == b.tag_name;
    let mut head = 0;
    while head < n_old.min(new_x.len()) && like(&old_x[head], &new_x[head]) {
        head += 1;
    }
    let mut tail = 0;
    while tail < n_old.min(new_x.len()) - head && like(&old_x[n_old - 1 - tail], &new_x[new_x.len() - 1 - tail]) {
        tail += 1;
    }
    for k in 0..head {
        out[k] = olds[k].take();
    }
    for t in 0..tail {
        out[new_x.len() - 1 - t] = olds[n_old - 1 - t].take();
    }
    // 中间那段旧孩子不再对给谁：留在 `olds` 里，由调用方拿掉
    STRAYS.with(|v| v.borrow_mut().extend(olds.into_iter().flatten().map(|(_, r, _, _)| r.taffy_node)));
    Some(out)
}

thread_local! {
    /// [`align_olds`] 没对上的旧孩子的节点（这一趟结束前从树里拿掉）
    static STRAYS: std::cell::RefCell<Vec<NodeId>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 这一趟增量重建的上下文
struct Ctx {
    reused: usize,
    rebuilt: usize,
    /// 留下来的子树根（失效分析里占位要换回真节点，见 [`moved_nodes`]）
    moved: Vec<NodeId>,
    /// 会被按兄弟位置选的规则选中的类名 / 标签：兄弟序号 / 兄弟数变了的节点，只有不在这里面的才能留
    positional: crate::parser::wxss::PositionalSubjects,
}

impl Ctx {
    /// 兄弟序号 / 兄弟数从 `(was_i, was_n)` 变成 `(i, n)` 之后，这个节点（作为自己、也作为子孙的祖先）的样式还一样吗
    fn position_ok(&self, node: &WxmlNode, was: (usize, usize), now: (usize, usize)) -> bool {
        was == now
            || node.node_type != WxmlNodeType::Element
            || !self.positional.may_match(&node.tag_name, node.get_attr("class"))
    }
}

/// 新树里留下来的那几棵子树根：编号 → 节点。失效分析把旧树里的占位换回它（旧布局照样从快照取）
fn moved_nodes<'a>(roots: &'a [RenderNode], ids: &[NodeId]) -> HashMap<NodeId, &'a RenderNode> {
    fn walk<'a>(nodes: &'a [RenderNode], want: &std::collections::HashSet<NodeId>, out: &mut HashMap<NodeId, &'a RenderNode>) {
        for n in nodes {
            if want.contains(&n.taffy_node) {
                out.insert(n.taffy_node, n);
            } else {
                walk(&n.children, want, out);
            }
        }
    }
    let want: std::collections::HashSet<NodeId> = ids.iter().copied().collect();
    let mut out = HashMap::with_capacity(ids.len());
    walk(roots, &want, &mut out);
    out
}

impl WxmlRenderer {
    /// 上一次增量重建留下了几棵子树、重建了几个（整棵重建不改它）
    pub fn last_reuse(&self) -> (usize, usize) {
        self.reuse_stats
    }

    /// 缓存那棵布局树里一共几个节点、从根能走到几个
    pub fn layout_node_counts(&self) -> (usize, usize) {
        let Some(c) = self.cache.as_ref() else { return (0, 0) };
        fn reach(t: &Tree, id: NodeId) -> usize {
            1 + t.children(id).unwrap_or_default().into_iter().map(|c| reach(t, c)).sum::<usize>()
        }
        (c.taffy.total_node_count(), reach(&c.taffy, c.root))
    }

    /// 增量重建。做不了（没有上一棵树、顶层对不上、关掉了）就把展开结果原样还回去，调用方整棵重建
    pub(super) fn rebuild_incremental(
        &mut self,
        rendered: Vec<WxmlNode>,
        data: &JsonValue,
        viewport: Option<(f32, f32)>,
        offset_changed: bool,
        t_start: std::time::Instant,
        t_template: std::time::Instant,
        log_timing: bool,
    ) -> Result<FramePlan, Vec<WxmlNode>> {
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *OFF.get_or_init(|| std::env::var("MINI_NO_INCREMENTAL_LAYOUT").is_ok()) {
            return Err(rendered);
        }
        let Some(mut cache) = self.cache.take() else { return Err(rendered) };
        if cache.rendered.iter().filter(|c| produces(c)).count() != cache.render_nodes.len() {
            self.cache = Some(cache);
            return Err(rendered);
        }
        // 1) 布局之后改过的样式先还原成建树时的样子（见模块注释）
        for (id, st) in self.layout_touched.borrow_mut().drain() {
            let _ = cache.taffy.set_style(id, st);
        }
        // 2) 旧布局拍下来给失效分析（下面要在同一棵树上改）
        let snapshot = LayoutSnapshot::take(&cache.taffy, &cache.render_nodes);
        let old_height = cache.content_height;
        // 3) 顶层逐个判：留着 / 往下走 / 重建
        let page = self.page_style();
        let mut old_roots = std::mem::take(&mut cache.render_nodes);
        let old_rendered = std::mem::take(&mut cache.rendered);
        let mut stats = Ctx { reused: 0, rebuilt: 0, moved: Vec::new(), positional: self.stylesheet.positional_subjects() };
        let mut new_roots = Vec::with_capacity(rendered.len());
        {
            let mut olds = align_olds(&old_rendered, &mut old_roots, &rendered).expect("上面数过");
            for (sib_i, node) in rendered.iter().enumerate() {
                let old = olds[sib_i].take();
                if let Some(rn) = self.rebuild_node(&mut cache.taffy, node, old, &[], &page.inherited, sib_i, rendered.len(), false, &mut stats) {
                    new_roots.push(rn);
                }
            }
        }
        // 没对上的旧孩子（列表删掉的那几条）从树里拿掉
        for id in STRAYS.with(|v| std::mem::take(&mut *v.borrow_mut())) {
            remove_owned(&mut cache.taffy, id);
        }
        let t_build = std::time::Instant::now();
        // 4) 根换孩子（根自己的样式只跟视口有关，视口没变）、排版、修正
        let child_ids: Vec<NodeId> = new_roots.iter().map(|n| n.taffy_node).collect();
        let root = cache.root;
        let _ = cache.taffy.set_children(root, &child_ids);
        let viewport_h = self.root_min_height(viewport);
        let t_layout1 = self.layout_pass(&mut cache.taffy, root, &new_roots, viewport_h, true);
        let t_fix = std::time::Instant::now();
        let content_height = cache.taffy.layout(root).map(|l| l.size.height).unwrap_or(0.0) / self.scale_factor;
        // 5) 失效分析：旧的读快照，新的读树
        let moved = moved_nodes(&new_roots, &stats.moved);
        // 滚动偏移也变了：重画范围与整棵重建时一样按整帧（layout.rs 里 `(Some(old), false)` 那条）
        let plan = if offset_changed { self.full_because(|| "增量布局：滚动偏移与上次布局时不同".into()) } else { self.plan_from_diff(
            OldLayout { nodes: &old_roots, layouts: &snapshot, content_height: old_height, moved: &moved },
            &new_roots,
            &cache.taffy,
            content_height,
        ) };
        drop(moved);
        drop(old_roots);
        let ms = |a: std::time::Instant, b: std::time::Instant| (b - a).as_secs_f32() * 1000.0;
        if log_timing {
            eprintln!(
                "⏱  增量重建：模板 {:.2}ms  建树 {:.2}ms（留 {} 棵子树 / 重建 {} 个）  布局 {:.2}ms  修正 {:.2}ms",
                ms(t_start, t_template),
                ms(t_template, t_build),
                stats.reused,
                stats.rebuilt,
                ms(t_build, t_layout1),
                ms(t_layout1, t_fix),
            );
        }
        self.rebuild_laps = Some([
            ms(t_start, t_template),
            ms(t_template, t_build),
            ms(t_build, t_layout1),
            ms(t_layout1, t_fix),
            ms(t_fix, std::time::Instant::now()),
        ]);
        self.reuse_stats = (stats.reused, stats.rebuilt);
        cache.render_nodes = new_roots;
        cache.content_height = content_height;
        cache.data = data.clone();
        cache.rendered = rendered;
        self.cache = Some(cache);
        self.layout_generation += 1;
        super::layout_verify::check(self, viewport);
        Ok(plan)
    }

    /// 一个节点：整棵没变就把旧的搬过来；自己没变的容器就重建自己、孩子各自判；否则拿掉旧的、重建。
    /// 兄弟序号 / 兄弟数变了的，只有不会被按位置选的规则选中时才留 / 往下走（[`Ctx::position_ok`]）
    #[allow(clippy::too_many_arguments)]
    fn rebuild_node(
        &self,
        taffy: &mut Tree,
        node: &WxmlNode,
        old: Old<'_>,
        ancestors: &[ElementDesc],
        inherited: &InheritedText,
        sib: usize,
        count: usize,
        positioned: bool,
        cx: &mut Ctx,
    ) -> Option<RenderNode> {
        if let Some((ox, orn, was_i, was_n)) = old {
            let position_ok = cx.position_ok(node, (was_i, was_n), (sib, count));
            if position_ok && !is_placeholder(orn) && reusable(node, ox) {
                cx.reused += 1;
                cx.moved.push(orn.taffy_node);
                let hole = RenderNode {
                    tag: REUSED_TAG.to_string(),
                    text: String::new(),
                    attrs: HashMap::new(),
                    taffy_node: orn.taffy_node,
                    style: NodeStyle { is_fixed: orn.style.is_fixed, ..Default::default() },
                    children: Vec::new(),
                    events: Vec::new(),
                };
                return Some(std::mem::replace(orn, hole));
            }
            let plain = position_ok
                && node.node_type == WxmlNodeType::Element
                && container_tag(&node.tag_name)
                && same_self(node, ox)
                // 带 transform / 动画的子树失效分析要按整棵的绘制几何算（invalidate_transform.rs），里面不能有占位
                && orn.style.transform.is_none()
                && orn.style.animation.is_none()
                && !orn.attrs.contains_key("animation");
            if plain {
                if let Some(mut olds) = align_olds(&ox.children, &mut orn.children, &node.children) {
                    // 旧容器节点先拿掉（它的孩子摘下来，留下来的那几个马上挂到新容器上）
                    let _ = taffy.remove(orn.taffy_node);
                    let rn = self.build_tree_with(taffy, node, ancestors, inherited, sib, count, positioned, &mut |r, t, c, anc, inh, k, n, pos| {
                        r.rebuild_node(t, c, olds[k].take(), anc, inh, k, n, pos, cx)
                    });
                    // 建树没走到的旧孩子（理论上没有：每个孩子都会问一次）也拿掉，不留在树里
                    for (_, o, _, _) in olds.into_iter().flatten() {
                        if !is_placeholder(o) {
                            remove_owned(taffy, o.taffy_node);
                        }
                    }
                    return rn;
                }
            }
            remove_owned(taffy, orn.taffy_node);
        }
        cx.rebuilt += 1;
        self.build_tree(taffy, node, ancestors, inherited, sib, count, positioned)
    }
}
