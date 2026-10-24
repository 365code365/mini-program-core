//! 失效分析读布局的两种来源。
//!
//! 整棵重建时新旧是两棵 taffy 树，旧树原样留着给失效分析比；增量重建（layout_incremental.rs）
//! 是在**同一棵树**上改，改之前先把旧树里每个节点的布局拍下来（[`LayoutSnapshot`]），
//! 失效分析读旧布局时读这份。两种来源都实现 [`Layouts`]，失效分析不关心是哪一种。
use super::*;
use taffy::tree::Layout;

/// 「这个节点上一次排到哪」
pub(super) trait Layouts {
    fn layout(&self, id: NodeId) -> Result<&Layout, ()>;
}

impl Layouts for Tree {
    fn layout(&self, id: NodeId) -> Result<&Layout, ()> {
        taffy::TaffyTree::layout(self, id).map_err(|_| ())
    }
}

/// 增量重建之前旧树里每个节点的布局（按节点编号）
pub(super) struct LayoutSnapshot(HashMap<NodeId, Layout>);

impl Layouts for LayoutSnapshot {
    fn layout(&self, id: NodeId) -> Result<&Layout, ()> {
        self.0.get(&id).ok_or(())
    }
}

impl LayoutSnapshot {
    /// 把 `nodes` 这几棵子树里每个节点（`RenderNode` 能走到的）的当前布局拍下来
    pub(super) fn take(taffy: &Tree, nodes: &[RenderNode]) -> Self {
        fn walk(t: &Tree, nodes: &[RenderNode], out: &mut HashMap<NodeId, Layout>) {
            for n in nodes {
                if let Ok(l) = taffy::TaffyTree::layout(t, n.taffy_node) {
                    out.insert(n.taffy_node, *l);
                }
                walk(t, &n.children, out);
            }
        }
        let mut out = HashMap::new();
        walk(taffy, nodes, &mut out);
        Self(out)
    }
}
