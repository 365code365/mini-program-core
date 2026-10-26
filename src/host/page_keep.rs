//! 页面被盖住（navigateTo）或切走（switchTab 离开的 tab 页）时留着它的视图，回来时原样放回。
//!
//! 微信里返回上一页，页面还停在原来的滚动位置、里面的横向列表也还在原处；tab 页切走再切回来
//! 也是同一个实例（onLoad 只有第一次，之后只有 onHide / onShow）。引擎从前两样都不对：
//! 返回时滚动归零、换一个新渲染器从头建；switchTab 把目标 tab 页的 js 重跑一遍（onLoad
//! 再来一次，页面里的状态全丢）。
//!
//! 留的东西三样（[`SavedView`]）：页面滚动、渲染器（布局缓存、已派发过的 `bindload`、
//! `wx.createAnimation` 落定的状态）、交互状态（scroll-view 位置、组件状态，
//! [`InteractionManager::take_page_state`]）。逻辑层的页面实例由 `app-page.js` 留
//! （`__switchTabTo` / `__reLaunchAll`）。SDK 引擎与桌面窗体共用这一份。
use crate::host::config::AppConfig;
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::ui::interaction::{InteractionManager, PageUiState};
use crate::ui::ScrollController;

/// 一页的视图状态
pub struct SavedView {
    pub scroll: ScrollController,
    pub renderer: Option<WxmlRenderer>,
    pub ui: PageUiState,
    /// 盖住那一刻的页面数据（分栏时另一栏要按它重画，engine_frameset.rs）。只有 SDK 引擎填
    pub data: Option<serde_json::Value>,
}

impl SavedView {
    /// 把当前页的视图拿走，换成一个空的（`vp` = 新页的视口高）
    pub fn take(
        scroll: &mut ScrollController,
        renderer: &mut Option<WxmlRenderer>,
        ui: &mut InteractionManager,
        vp: f32,
    ) -> Self {
        // 惯性 / 回弹停在离开那一刻：回来时不该接着滑
        if scroll.is_animating() {
            scroll.stop();
        }
        let mut r = renderer.take();
        if let Some(r) = r.as_mut() {
            r.drop_offscreen_caches();
        }
        Self {
            scroll: std::mem::replace(scroll, ScrollController::new(vp, vp)),
            renderer: r,
            ui: ui.take_page_state(),
            data: None,
        }
    }

    /// 放回去。渲染器没留（尺寸变过、留的时候就没有）时 `renderer` 是 None，调用方照常新建
    pub fn restore(self, scroll: &mut ScrollController, renderer: &mut Option<WxmlRenderer>, ui: &mut InteractionManager) {
        *scroll = self.scroll;
        *renderer = self.renderer;
        ui.restore_page_state(self.ui);
    }
}

/// 栈上每一页（底下几层）的视图 + 切走的 tab 页
#[derive(Default)]
pub struct Keep {
    /// 与页面栈底下几层对齐：`saved[i]` 是 `page_stack[i]` 被盖住时留的
    pub saved: Vec<SavedView>,
    /// 切走的 tab 页：路由 → （页面、视图）
    pub tabs: std::collections::HashMap<String, (crate::host::PageInstance, SavedView)>,
}

impl Keep {
    /// 尺寸 / 安全区变了：留着的渲染器按旧尺寸建的，丢掉（回来时新建；滚动与交互状态照留）
    pub fn forget_renderers(&mut self) {
        for v in &mut self.saved {
            v.renderer = None;
        }
        for (_, v) in self.tabs.values_mut() {
            v.renderer = None;
        }
    }

    pub fn clear(&mut self) {
        self.saved.clear();
        self.tabs.clear();
    }
}

/// app.json `tabBar.list` 里的页面路由
pub fn tab_routes(config: &AppConfig) -> Vec<String> {
    config.tab_bar.as_ref().map(|t| t.list.iter().map(|i| i.page_path.clone()).collect()).unwrap_or_default()
}

/// 逻辑层的 switchTab：栈上的非 tab 页 onUnload，tab 页留下（顶上那页是 tab 页时 onHide）。
/// 目标 tab 页留过实例就放回去、派 onShow，返回 true —— 宿主**不要**再加载它的 js
pub fn switch_tab_js(app: &MiniApp, route: &str, tabs: &[String]) -> bool {
    let r = serde_json::to_string(route).unwrap_or_else(|_| "''".into());
    let t = serde_json::to_string(tabs).unwrap_or_else(|_| "[]".into());
    app.eval(&format!("__switchTabTo({r}, {t})")).map(|s| s.trim() == "true").unwrap_or(false)
}

/// 逻辑层丢掉某个 tab 页留着的实例（宿主那边没有它的视图，只能重新加载）
pub fn drop_tab_js(app: &MiniApp, route: &str) {
    let r = serde_json::to_string(route).unwrap_or_else(|_| "''".into());
    app.eval(&format!("__dropTab({r})")).ok();
}

/// 逻辑层的 reLaunch：栈上每一页与留着的 tab 页全部 onUnload，清空
pub fn relaunch_js(app: &MiniApp) {
    app.eval("__reLaunchAll()").ok();
}
