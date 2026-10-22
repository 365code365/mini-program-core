//! [`MiniEngine`] 进入页面：解析 → 样式 → 页面 js 与生命周期 → 渲染器。
//!
//! 从 `engine.rs` 原样搬出（那边只留结构体、构造与查询）。

use super::engine::MiniEngine;
use crate::host::{component_mount, remove_manual_tabbar, tabbar_height, PageInstance};
use crate::parser::{WxmlParser, WxssParser};
use crate::renderer::WxmlRenderer;
use crate::ui::ScrollController;
use std::collections::HashMap;

impl MiniEngine {
    /// 进入一个页面（与桌面窗体 `navigate_to` 同序：组件登记 → 路由登记 →
    /// 页面 js → onLoad/onShow/onReady → 挂组件实例）
    pub(crate) fn navigate_to(
        &mut self,
        path: &str,
        query: HashMap<String, String>,
    ) -> Result<(), String> {
        let path = path.trim_start_matches('/').to_string();
        // 页面 js 看到的 windowHeight 是这一页的（有没有引擎画的导航栏，engine_navbar.rs）
        if self.pages.contains_key(&path) {
            self.sync_window_height_for(&path);
        }
        let info = self
            .pages
            .get(&path)
            .ok_or_else(|| format!("页面不存在: {path}"))?;
        let nodes = remove_manual_tabbar(
            &WxmlParser::new(&info.wxml)
                .parse()
                .map_err(|e| format!("WXML 解析失败: {e}"))?,
        );
        let (templates, stylesheet) = self.page_styles(info)?;

        self.app.eval("__resetPageComponents()").ok();
        self.app
            .eval(&format!(
                "__setPendingRoute({})",
                serde_json::to_string(&path).unwrap_or_else(|_| "''".into())
            ))
            .ok();
        self.app.load_module_script(&path, &info.js)?;
        let q = serde_json::to_string(&query).unwrap_or_else(|_| "{}".into());
        self.app
            .eval(&format!("if(__currentPage&&__currentPage.onLoad)__currentPage.onLoad({q})"))
            .ok();
        self.app.eval("if(__currentPage&&__currentPage.onShow)__currentPage.onShow()").ok();
        self.app.eval("if(__currentPage&&__currentPage.onReady)__currentPage.onReady()").ok();
        component_mount::mount_page_components(&mut self.app, &info.components, &nodes);

        self.page_stack.push(PageInstance {
            path: path.clone(),
            query,
            wxml_nodes: nodes,
            stylesheet,
            component_templates: templates,
            saved_scroll: None,
        });
        self.page_opened_ms = self.last_pump_ms;
        self.page_data_dirty = true;
        let vp = self.viewport_height();
        self.scroll = ScrollController::new(vp, vp);
        self.interaction.clear_page_state();
        self.update_renderers();
        self.mark_full();
        Ok(())
    }

    /// 页面的组件模板与合并后的样式表（app.wxss → 页面 → 组件）。
    ///
    /// 换页时用；安全区 / 屏宽变了（`env()` 与 rpx 在解析期求值）也要对栈上每一页重来一遍。
    pub(crate) fn page_styles(
        &self,
        info: &crate::host::PageInfo,
    ) -> Result<(crate::parser::template::ComponentTemplates, crate::parser::wxss::StyleSheet), String> {
        let mut templates = crate::parser::template::ComponentTemplates::new();
        let mut comp_wxss = String::new();
        for c in &info.components {
            if let Ok(n) = WxmlParser::new(&c.wxml).parse() {
                templates.insert(c.tag.clone(), n);
                comp_wxss.push('\n');
                comp_wxss.push_str(&c.wxss);
            }
        }
        // app.wxss 在前、页面在后：同特异性时页面样式因书写顺序更靠后而胜出。
        // 解析前把本实例的安全区放进本线程（`env(safe-area-inset-*)` 在解析期求值）。
        // 顶上有引擎画的导航栏的页面不与状态栏重叠：`safe-area-inset-top` 是 0
        Self::apply_env_context(&self.env_opts_for(&info.path), self.width, self.height);
        let merged = format!("{}\n{}\n{}", self.app_wxss, info.wxss, comp_wxss);
        let stylesheet = WxssParser::new(&merged)
            .parse()
            .map_err(|e| format!("WXSS 解析失败: {e}"))?;
        Ok((templates, stylesheet))
    }

    pub(crate) fn update_renderers(&mut self) {
        // 宿主承载的 `<video>`：引擎不解码不画（见 components::video）
        crate::renderer::components::set_host_owns_video(
            self.opts.native_components.iter().any(|t| t == "video"),
        );
        let Some(page) = self.page_stack.last() else { return };
        // 屏高 = 页面窗口高（`100vh`、fixed 层的视口）：引擎画了导航栏时减掉它
        let mut r = WxmlRenderer::new_with_scale(
            page.stylesheet.clone(),
            self.width as f32,
            self.page_window_height(),
            self.dpr,
        );
        r.set_component_templates(page.component_templates.clone());
        r.set_native_components(&self.opts.native_components);
        if let Some(t) = self.anim_time {
            r.set_animation_time(t);
        }
        self.renderer = Some(r);
        if let Some(ct) = &self.custom_tabbar {
            let mut tr = WxmlRenderer::new_with_scale(
                ct.stylesheet.clone(),
                self.width as f32,
                tabbar_height() as f32,
                self.dpr,
            );
            if let Some(t) = self.anim_time {
                tr.set_animation_time(t);
            }
            self.tabbar_renderer = Some(tr);
        }
    }
}
