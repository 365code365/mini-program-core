//! 布局树的两个诊断（增量布局，renderer/wxml_renderer/layout_incremental.rs）：测试与排查用
use super::engine::MiniEngine;

impl MiniEngine {
    /// 上一次 setData 增量重建：留下了几棵没变的子树、重建了几个
    pub fn layout_reuse(&self) -> (usize, usize) {
        self.renderer.as_ref().map(|r| r.last_reuse()).unwrap_or((0, 0))
    }

    /// 页面布局树里一共几个节点、从根能走到几个（不相等就是增量重建漏了没拿掉的节点）
    pub fn layout_node_counts(&self) -> (usize, usize) {
        self.renderer.as_ref().map(|r| r.layout_node_counts()).unwrap_or((0, 0))
    }
}
