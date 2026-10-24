//! `MINI_LAYOUT_VERIFY=1`：每次增量重建之后，再用同一份模板展开结果**整棵重建**一遍，逐节点比
//! 标签 / 文字 / 属性 / 布局位置尺寸，不一致就打出第一处并记一次（[`layout_verify_failures`]）。
//!
//! 只给测试与排查用（每次 setData 多一整棵重建）。判据是逐位相等：增量重建的结果必须与整棵重建一样，
//! 差一点都说明「什么算没变」的判断漏了一种输入（layout_incremental.rs 模块注释）。
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static FAILURES: AtomicUsize = AtomicUsize::new(0);

/// 进程里校验出不一致的次数
pub fn layout_verify_failures() -> usize {
    FAILURES.load(Ordering::Relaxed)
}

fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("MINI_LAYOUT_VERIFY").is_ok())
}

pub(super) fn check(r: &WxmlRenderer, viewport: Option<(f32, f32)>) {
    if !enabled() {
        return;
    }
    let Some(cache) = r.cache.as_ref() else { return };
    let mut t = Tree::new();
    let page = r.page_style();
    let n = cache.rendered.len();
    let roots: Vec<RenderNode> = cache
        .rendered
        .iter()
        .enumerate()
        .filter_map(|(i, x)| r.build_tree(&mut t, x, &[], &page.inherited, i, n, false))
        .collect();
    let ids: Vec<NodeId> = roots.iter().map(|x| x.taffy_node).collect();
    let vh = r.root_min_height(viewport);
    let root = t.new_with_children(r.root_style(vh), &ids).unwrap();
    r.layout_pass(&mut t, root, &roots, vh, false);
    let mut path = String::new();
    let ok = same_layout(&cache.taffy, cache.root, &t, root, "根", &mut path)
        && same_list(&cache.taffy, &cache.render_nodes, &t, &roots, &mut path);
    if !ok {
        FAILURES.fetch_add(1, Ordering::Relaxed);
        eprintln!("❌ 增量布局与整棵重建不一致：{path}");
    }
}

fn same_layout(a: &Tree, ia: NodeId, b: &Tree, ib: NodeId, what: &str, path: &mut String) -> bool {
    let (Ok(la), Ok(lb)) = (a.layout(ia), b.layout(ib)) else {
        *path = format!("{what} 取不到布局");
        return false;
    };
    if la.location != lb.location || la.size != lb.size {
        *path = format!(
            "{what}：增量 ({:.2},{:.2}) {:.2}x{:.2} / 整棵 ({:.2},{:.2}) {:.2}x{:.2}",
            la.location.x, la.location.y, la.size.width, la.size.height, lb.location.x, lb.location.y, lb.size.width, lb.size.height
        );
        return false;
    }
    true
}

fn same_list(a: &Tree, xa: &[RenderNode], b: &Tree, xb: &[RenderNode], path: &mut String) -> bool {
    if xa.len() != xb.len() {
        *path = format!("孩子 {} 个 / {} 个", xa.len(), xb.len());
        return false;
    }
    for (i, (na, nb)) in xa.iter().zip(xb).enumerate() {
        let what = format!("<{}>#{i} {:?}", nb.tag, nb.attrs.get("class"));
        if na.tag != nb.tag || na.text != nb.text || na.attrs != nb.attrs {
            *path = format!("{what} 标签 / 文字 / 属性不同");
            return false;
        }
        if !same_layout(a, na.taffy_node, b, nb.taffy_node, &what, path) || !same_list(a, &na.children, b, &nb.children, path) {
            *path = format!("{what} › {path}");
            return false;
        }
    }
    true
}
