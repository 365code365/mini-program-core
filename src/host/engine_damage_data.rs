//! setData 之后的失效分析入口（从 `engine_damage.rs` 拆出，**纯搬迁**：那个文件超了 500 行）。
use super::engine::MiniEngine;
use crate::renderer::wxml_renderer::FramePlan;

impl MiniEngine {
    /// setData 之后：先建好新布局，问出「只影响哪一块」（要在清屏之前知道）
    pub(super) fn plan_data_damage(&mut self) -> FramePlan {
        let t = self.pump_notes.start();
        self.refresh_page_data();
        self.pump_notes.add("取页面数据（JS → JSON）", t);
        let t = self.pump_notes.start();
        let plan = self.plan_data_layout();
        self.pump_notes.add("重建布局 + 比对", t);
        if let Some(l) = self.renderer.as_mut().and_then(|r| r.take_rebuild_laps()) {
            self.pump_notes.note(&format!(
                "〔模板 {:.1} 建树 {:.1} 布局 {:.1} 换行 {:.1} 比对 {:.1}〕",
                l[0], l[1], l[2], l[3], l[4]
            ));
        }
        plan
    }

    fn plan_data_layout(&mut self) -> FramePlan {
        let vp = self.viewport_height();
        // 用布局缓存上次被问的同一个偏移：偏移不进布局，换了偏移只会被判成「视口变了」
        let scroll = self.dmg.layout_scroll.unwrap_or_else(|| self.scroll.get_position());
        let Self { renderer, page_stack, page_data, .. } = self;
        match (renderer.as_mut(), page_stack.last()) {
            (Some(r), Some(page)) => r.plan_frame(&page.wxml_nodes, page_data, Some((scroll, vp))),
            _ => FramePlan::Full,
        }
    }
}
