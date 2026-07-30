//! tabBar 的归属判定与绘制：哪些页面有 tabBar、自定义还是原生、画到独立画布上
//!
//! 从 `src/bin/window.rs` 拆出来的一片（`impl crate::MiniAppWindow`）。**纯搬迁**，
//! 一行逻辑没改；判据是 65 张画廊图与逐页整帧快照逐字节不变。
#![allow(clippy::too_many_arguments)]
use super::*;
use crate::*;

impl crate::MiniAppWindow {
    /// 宿主是否要为这个页面画 tabBar（并为它预留高度）。
    ///
    /// `tabBar.custom = true` 的语义是「**由小程序自己画**」。此时宿主既不画原生
    /// tabBar 也不该预留高度 —— 页面会自带一条。uni-app 这类框架就是把 tabBar
    /// 编译进每个页面的，仓库里连 `custom-tab-bar/` 目录都没有。
    ///
    /// 从前只要「声明了 custom 且我们成功加载到 custom-tab-bar 组件」才认；
    /// 组件不存在时就悄悄退回原生 tabBar，于是页面自带的那条和原生那条**叠在一起**
    /// （tea-app 底部因此出现一排巨大的宋体字压在真正的图标标签上）。
    pub(crate) fn is_tabbar_page(&self, path: &str) -> bool {
        let Some(tb) = self.app_config.tab_bar.as_ref() else { return false };
        if tb.custom && self.custom_tabbar.is_none() {
            return false; // 小程序自己画，宿主完全不参与
        }
        tb.list.iter().any(|item| item.page_path == path)
    }

    pub(crate) fn get_tabbar_index(&self, path: &str) -> Option<usize> {
        self.app_config.tab_bar.as_ref().and_then(|tb| tb.list.iter().position(|item| item.page_path == path))
    }

    pub(crate) fn is_custom_tabbar(&self) -> bool {
        self.app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) && self.custom_tabbar.is_some()
    }

    pub(crate) fn render_custom_tabbar(&mut self, current_path: &str) {
        let tb = match &self.app_config.tab_bar { Some(tb) => tb.clone(), None => return };
        // 数据以「组件自身 data」为准（微信语义）：iconType 等字段只存在于组件里，
        // 只用 app.json 拼 list 会丢掉图标类型，导致所有 tab 图标退化成默认样式。
        let mut data = self
            .custom_tabbar
            .as_ref()
            .map(|ct| ct.data.clone())
            .unwrap_or_else(|| json!({}));
        if !data.is_object() {
            data = json!({});
        }
        let has_list = data.get("list").and_then(|l| l.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
        if let Some(obj) = data.as_object_mut() {
            if !has_list {
                // 组件没有提供 list 时，退回 app.json 的配置
                let list: Vec<serde_json::Value> = tb.list.iter()
                    .map(|i| json!({"pagePath": i.page_path, "text": i.text}))
                    .collect();
                obj.insert("list".to_string(), json!(list));
            }
            obj.insert(
                "selected".to_string(),
                json!(self.get_tabbar_index(current_path).unwrap_or(0)),
            );
        }
        let ct = match &self.custom_tabbar { Some(ct) => ct, None => return };
        if let (Some(c), Some(r)) = (&mut self.tabbar_canvas, &mut self.tabbar_renderer) {
            c.clear(Color::WHITE);
            r.render(c, &ct.wxml_nodes, &data);
        }
    }

    pub(crate) fn render_native_tabbar(&mut self, current_path: &str) {
        let tb = match &self.app_config.tab_bar { Some(tb) => tb.clone(), None => return };
        if let (Some(c), Some(tr)) = (&mut self.tabbar_canvas, self.text_renderer.as_deref()) {
            render_native_tabbar(c, tr, &tb, current_path, self.scale_factor);
        }
    }
}
