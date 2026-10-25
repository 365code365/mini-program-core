//! SDK 引擎：页面被盖住时留视图、返回时放回、tab 页切走时留实例（共用部分见 `page_keep.rs`）。
//!
//! 与微信对齐的生命周期：
//! - navigateTo：当前页 `onHide`（含组件 `pageLifetimes.hide`）→ 新页 onLoad / onShow / onReady；
//! - 返回：离开的页 onUnload → 回到的页 onShow，滚动位置与 scroll-view 位置原样；
//! - switchTab：非 tab 页 onUnload，离开的 tab 页 onHide 并留着；切回留过的 tab 页只有 onShow；
//! - reLaunch：所有页（含留着的 tab 页）onUnload。
use super::engine::MiniEngine;
use super::page_keep::{self, SavedView};
use crate::host::component_mount;
use std::collections::HashMap;

impl MiniEngine {
    /// navigateTo 之前：当前页 onHide、留一张图（侧滑 / 转场露出来的就是它）、留视图
    pub(super) fn cover_current_page(&mut self) {
        if self.page_stack.is_empty() {
            return;
        }
        self.app.eval("__dispatchPage('onHide')").ok();
        let t = self.pump_notes.start();
        self.capture_back_shot();
        self.pump_notes.add("留图", t);
        let vp = self.viewport_height();
        let v = SavedView::take(&mut self.scroll, &mut self.renderer, &mut self.interaction, vp);
        self.keep.saved.push(v);
    }

    /// navigateTo 没成：把刚留的放回去（页面又露出来了，补一次 onShow）
    pub(super) fn uncover_current_page(&mut self) {
        if self.page_stack.is_empty() || self.keep.saved.len() < self.page_stack.len() {
            return; // cover 没做（栈是空的）
        }
        self.back_shots.pop();
        self.unpack_last_shot();
        if let Some(v) = self.keep.saved.pop() {
            self.put_back_view(Some(v));
        }
        self.app.eval("__dispatchPage('onShow')").ok();
    }

    /// 回到某一页之后放回它的视图；没有留（或渲染器没留）就按新页处理
    pub(super) fn put_back_view(&mut self, view: Option<SavedView>) {
        match view {
            Some(v) => v.restore(&mut self.scroll, &mut self.renderer, &mut self.interaction),
            None => {
                self.interaction.clear_page_state();
                let vp = self.viewport_height();
                self.scroll = crate::ui::ScrollController::new(vp, vp);
                self.renderer = None;
            }
        }
        if self.renderer.is_none() {
            self.update_renderers();
        }
        self.page_data_dirty = true;
        self.mark_full();
        self.dmg.fixed_dirty = true;
        self.drawn_band = None;
    }

    /// 页面自定义组件的实例重建（往前走时 `navigate_to` 的 `__resetPageComponents()` 把
    /// 盖住那页的组件实例作废了，不重建的话回来后组件模板里全是空值）
    pub(super) fn remount_top_components(&mut self) {
        if let Some(top) = self.page_stack.last() {
            if let Some(info) = self.pages.get(&top.path).filter(|i| !i.components.is_empty()) {
                self.app.eval("__resetPageComponents()").ok();
                component_mount::mount_page_components(&mut self.app, &info.components, &top.wxml_nodes);
            }
        }
    }

    /// switchTab：tab 页留着、切回来不重载（见文件头）
    pub(super) fn switch_tab(&mut self, path: &str) {
        let path = path.trim_start_matches('/');
        if self.page_stack.len() == 1 && self.current_route() == path {
            return; // 已经就在这个 tab 上
        }
        let tabs = page_keep::tab_routes(&self.config);
        // 栈上的 tab 页（只可能在栈底）连同视图留进 keep.tabs；栈顶的视图是活的，这时才拿
        if !self.page_stack.is_empty() {
            let vp = self.viewport_height();
            let live = SavedView::take(&mut self.scroll, &mut self.renderer, &mut self.interaction, vp);
            let mut views = std::mem::take(&mut self.keep.saved);
            views.push(live);
            let stack = std::mem::take(&mut self.page_stack);
            for (page, view) in stack.into_iter().zip(views) {
                if tabs.contains(&page.path) {
                    self.keep.tabs.insert(page.path.clone(), (page, view));
                }
            }
        }
        self.back_shots.clear();
        self.keep.saved.clear();
        self.edge_back = None;
        self.route.finish();
        let restored = page_keep::switch_tab_js(&self.app, path, &tabs);
        match (restored, self.keep.tabs.remove(path)) {
            (true, Some((page, view))) => {
                self.page_stack.push(page);
                self.remount_top_components();
                self.put_back_view(Some(view));
            }
            (js, _) => {
                // 第一次来（或两边留的对不上）：逻辑层不留实例，照常加载
                if js {
                    page_keep::drop_tab_js(&self.app, path);
                }
                self.interaction.clear_page_state();
                self.renderer = None;
                self.navigate_to(path, HashMap::new()).ok();
            }
        }
    }

    /// reLaunch：所有页（含留着的 tab 页）onUnload，再开目标页
    pub(super) fn relaunch(&mut self, path: &str, query: HashMap<String, String>) {
        page_keep::relaunch_js(&self.app);
        self.page_stack.clear();
        self.back_shots.clear();
        self.keep.clear();
        self.edge_back = None;
        self.route.finish();
        self.interaction.clear_page_state();
        self.navigate_to(path, query).ok();
    }
}
