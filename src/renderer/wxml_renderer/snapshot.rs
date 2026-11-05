//! `<snapshot>` 的离屏截图（官方 component/snapshot、api/skyline/Snapshot.takeSnapshot）：
//! 把 snapshot 节点自己连同子树画进一张独立的画布，宿主再编成 PNG / 原样 RGBA（`host::snapshot`）。
//!
//! 走静态绘制路径（`draw_with_color`，与离屏仿射同一套）：不登记交互、不记动画包围盒、不碰页面画布，
//! 所以截图不会让页面重画任何一块。尺寸是节点的布局尺寸（物理像素），官方「导出图片没有尺寸限制」——
//! 这里给一个 8192 的边长上限，超了按失败回（一张 8192² 的 RGBA 就是 256MB）。
//! 取舍：节点自己的 transform / 动画按此刻的值画；原生组件（video / map …）只有占位（官方也不支持子节点含原生组件）。
use super::*;
use crate::ui::InteractionManager;

/// 截图的边长上限（物理像素）
const MAX_SIDE: u32 = 8192;

impl WxmlRenderer {
    /// `component_id`（与 `get_component_id` 一致）对应的节点画成 RGBA。返回 (宽, 高, 像素)，物理像素
    pub fn snapshot_subtree(&mut self, component_id: &str, interaction: &InteractionManager) -> Result<(u32, u32, Vec<u8>), String> {
        let cache = self.cache.take().ok_or("页面还没布局")?;
        let mut found = None;
        for n in &cache.render_nodes {
            if let Some(f) = self.find_snapshot(n, &cache.taffy, (0.0, 0.0), interaction, component_id) {
                found = Some(f);
                break;
            }
        }
        let out = match found {
            None => Err(format!("找不到节点 {component_id}")),
            Some(node) => self.draw_snapshot(&cache.taffy, node),
        };
        self.cache = Some(cache);
        out
    }

    fn find_snapshot<'a>(
        &self,
        n: &'a RenderNode,
        taffy: &Tree,
        origin: (f32, f32),
        interaction: &InteractionManager,
        want: &str,
    ) -> Option<&'a RenderNode> {
        let (_, id, child_origin) = self.form_geom(n, taffy, origin, interaction)?;
        if id == want {
            return Some(n);
        }
        n.children.iter().find_map(|c| self.find_snapshot(c, taffy, child_origin, interaction, want))
    }

    /// 也给 `map_callout`（地图自定义气泡出图）用
    pub(super) fn draw_snapshot(&mut self, taffy: &Tree, node: &RenderNode) -> Result<(u32, u32, Vec<u8>), String> {
        let l = taffy.layout(node.taffy_node).map_err(|e| format!("{e:?}"))?;
        let (w, h) = (l.size.width.ceil() as u32, l.size.height.ceil() as u32);
        if w == 0 || h == 0 {
            return Err("节点没有尺寸".into());
        }
        if w > MAX_SIDE || h > MAX_SIDE {
            return Err(format!("截图太大（{w}×{h}）"));
        }
        let mut canvas = Canvas::new(w, h);
        canvas.clear(Color::TRANSPARENT);
        let (bindings, offscreen, clip) = (self.event_bindings.len(), self.drawing_offscreen, self.damage_clip.take());
        self.drawing_offscreen = true;
        // 节点按自己的左上角落在 (0, 0)：draw_with_color 把 ox / oy 加在节点的布局位置上
        self.draw_with_color(&mut canvas, taffy, node, -l.location.x, -l.location.y, Color::BLACK);
        self.drawing_offscreen = offscreen;
        self.damage_clip = clip;
        self.event_bindings.truncate(bindings);
        Ok((w, h, canvas.to_rgba()))
    }
}
