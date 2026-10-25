//! [`MiniEngine`] 的路由：返回、逻辑层发起的跳转、宿主深链。
//!
//! 从 `engine_input.rs` 按职责拆出（那边只留指针 / 键盘 / 输入法）。被盖住的页留视图、
//! tab 页留实例在 `engine_keep.rs`，转场动画在 `route_anim.rs`。

use super::engine::MiniEngine;
use super::navigation::{self, parse_url, NavigationRequest};
use std::collections::HashMap;

impl MiniEngine {
    /// 系统返回键 / 导航栏返回键 / `wx.navigateBack`。宿主开了 `routeAnimation` 时带转场。
    ///
    /// 返回 `false` 表示**页面栈已经只剩一页**，宿主该关掉这个小程序
    /// （Android 的 `onBackPressed` 要在这时才调 `finish()`）。
    ///
    /// 视频全屏时先退出全屏（与微信一致：返回键不离开页面），返回 `true`。
    /// `wx.navigateBack` 不走这里（逻辑层要离开页面就离开，全屏跟着页面收掉）
    pub fn back(&mut self) -> bool {
        if crate::renderer::components::video_fullscreen::exit() {
            self.dmg.compose_dirty = true;
            return true;
        }
        // 返回询问（wx.enableAlertBeforeUnload）/ page-container 先关：接住了就当这次返回已处理（宿主别退出小程序）。
        // 侧滑返回不问（走 back_with）：页面已经跟着手指滑出去了，微信 iOS 也不弹
        if self.back_intercepted() {
            return true;
        }
        self.back_with(self.opts.route_animation)
    }

    /// `animate`：要不要转场。侧滑返回松手判定返回时不要 —— 页面已经跟着手指滑出去了
    pub(crate) fn back_with(&mut self, animate: bool) -> bool {
        // 先取走 wx.navigateBack 记下的层数：栈只剩一页时什么都不做，也不能把它留到下一次
        let logic_popped = navigation::take_logic_popped(&self.app);
        if self.page_stack.len() <= 1 {
            return false;
        }
        self.route.finish();
        // 转场：退栈之前留一张当前画面（要离开的那页），退栈之后它盖在恢复出来的上一页上滑走
        let t = self.pump_notes.start();
        let leaving = if animate { self.snapshot_frame() } else { None };
        self.pump_notes.add("留离开那页的图", t);
        let t = self.pump_notes.start();
        self.pop_page(logic_popped);
        self.pump_notes.add("退栈（onUnload + 放回视图）", t);
        if let Some(shot) = leaving {
            self.route.start_pop(shot);
        }
        true
    }

    /// 退栈。`logic_popped`：`wx.navigateBack({ delta })` 已经替宿主出过的逻辑层层数
    /// （见 [`navigation::take_logic_popped`]）—— 大于 0 时宿主退这么多层、逻辑层不动；
    /// 0 是宿主自己发起的返回（返回键、侧滑），宿主退一层，逻辑层补 onUnload + 出栈。
    pub(crate) fn pop_page(&mut self, logic_popped: usize) {
        let mut returned = None;
        for _ in 0..logic_popped.max(1) {
            if self.page_stack.len() <= 1 {
                break;
            }
            self.page_stack.pop();
            // 回到的这一页要重新渲染，它离开时留下的那张图用完即弃
            self.back_shots.pop();
            // 最后一次弹出的就是回到的那一页被盖住时留的视图（滚动位置、scroll-view 位置）
            returned = self.keep.saved.pop();
        }
        self.unpack_last_shot();
        // **不重载**上一页的 js：否则用户在上一页的筛选/表单/滚动位置全丢，
        // 还会重复触发「仅首次进入」的逻辑
        if logic_popped == 0 {
            navigation::pop_logic_page(&self.app);
        }
        self.app.eval("if(__currentPage&&__currentPage.onShow)__currentPage.onShow()").ok();
        // 自定义组件的实例要重建（与桌面窗体 `page_host::navigate_back` 同一处理，见 engine_keep.rs）
        self.remount_top_components();
        self.put_back_view(returned);
    }

    /// 取一次逻辑层挂起的路由请求
    pub(crate) fn take_navigation(&mut self) -> Option<NavigationRequest> {
        let raw = self.app.eval("JSON.stringify(__pendingNavigation||null)").ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let obj = v.as_object()?;
        let kind = obj.get("type")?.as_str()?.to_string();
        let url = obj.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
        self.app.eval("__pendingNavigation=null").ok();
        Some(match kind.as_str() {
            "navigateTo" => NavigationRequest::NavigateTo { url },
            "navigateBack" => NavigationRequest::NavigateBack,
            "switchTab" => NavigationRequest::SwitchTab { url },
            "redirectTo" => NavigationRequest::RedirectTo { url },
            "reLaunch" => NavigationRequest::ReLaunch { url },
            _ => return None,
        })
    }

    pub(crate) fn apply_navigation(&mut self, nav: NavigationRequest) {
        // 上一次转场还没走完又来一次跳转：直接收手（新的转场从当前画面起）
        self.route.finish();
        match nav {
            NavigationRequest::NavigateTo { url } => {
                let (p, q) = parse_url(&url);
                self.push_page(&p, q).ok();
            }
            NavigationRequest::NavigateBack => {
                self.back_with(self.opts.route_animation);
            }
            NavigationRequest::SwitchTab { url } => {
                let (p, _) = parse_url(&url);
                self.switch_tab(&p);
            }
            NavigationRequest::RedirectTo { url } => {
                // 替换当前页：栈深不变
                let (p, q) = parse_url(&url);
                self.app
                    .eval("if(__currentPage&&__currentPage.onUnload)__currentPage.onUnload()")
                    .ok();
                self.page_stack.pop();
                self.app.eval("__dropTopPage()").ok();
                self.interaction.clear_page_state();
                self.navigate_to(&p, q).ok();
            }
            NavigationRequest::ReLaunch { url } => {
                // query 要带上：`wx.reLaunch({url:'/pages/a?id=1'})` 的 onLoad 收到 {id:'1'}（从前丢了，收到 {}）
                let (p, q) = parse_url(&url);
                self.relaunch(&p, q);
            }
        }
    }

    /// 入栈一页：盖住当前页（onHide、留图、留视图）→ 进新页 → 转场
    fn push_page(&mut self, path: &str, query: HashMap<String, String>) -> Result<(), String> {
        let path = path.trim_start_matches('/');
        // 页面不存在就什么都别动（不派 onHide、不留图）
        if !self.pages.contains_key(path) {
            return Err(format!("页面不存在: {path}"));
        }
        let t = self.pump_notes.start();
        self.cover_current_page();
        self.pump_notes.add("盖住旧页（onHide + 留图）", t);
        let t = self.pump_notes.start();
        let opened = self.navigate_to(path, query);
        self.pump_notes.add("建新页（加载 + onLoad）", t);
        match opened {
            Ok(()) => {
                if self.opts.route_animation && self.page_stack.len() > 1 {
                    self.route.start_push();
                }
                Ok(())
            }
            Err(e) => {
                self.uncover_current_page();
                Err(e)
            }
        }
    }

    /// 主动跳页（宿主侧的深链：扫码进来直接落到某个页面）
    pub fn navigate(&mut self, url: &str) -> Result<(), String> {
        self.route.finish();
        let (p, q) = parse_url(url);
        self.push_page(&p, q)
    }
}
