//! 自定义 tabBar（`custom-tab-bar/`）的活实例：SDK 引擎与桌面窗体共用。
//!
//! 从前 tabBar 只按 [`super::CustomTabBar::data`] 画，那是加载时在**另一个** JS 上下文里跑一遍组件取的快照：
//! - 页面里 `this.getTabBar()` 永远是 null，官方「在 tab 页 onShow 里 `getTabBar().setData({selected})`」的写法没有效果；
//! - 组件自己 `attached` / `pageLifetimes.show` / 定时器里 setData（角标、红点）画面不变；
//! - 组件在 `attached` 里 `getApp()` 是 null（快照那个上下文里没跑 app.js），`require` 也找不到模块。
//!
//! 现在 app.js 执行之后在主上下文里建一个实例（[`mount`]），它的 setData 让 `__tabBarVersion` 加一，
//! 宿主每帧比对版本号（[`take_data`]），变了就拿最新的 data 重画。快照仍用来量 tabBar 高度（建实例之前就要）。
//! `selected` 照旧由宿主按当前路由覆盖：只靠组件自己设，没写 onShow 同步的小程序选中态会停在第一项。
use crate::runtime::MiniApp;

/// 在主 JS 上下文里建自定义 tabBar 的实例（app.js 之后调一次）。返回是否建成
pub fn mount(app: &MiniApp, js: &str) -> bool {
    // 同 component_mount：这个标记告诉 `Component()`「这是组件不是页面」，模块路径决定相对 require 的基准
    app.eval("__setPendingComponentPath('custom-tab-bar')").ok();
    let loaded = app.load_module_script("custom-tab-bar/index", js);
    app.eval("__setPendingComponentPath('')").ok();
    if let Err(e) = loaded {
        eprintln!("⚠️  custom-tab-bar 的 js 执行失败: {e}");
        return false;
    }
    app.eval("__mountCustomTabBar()").map(|v| v == "true").unwrap_or(false)
}

/// 实例的 data 自上次以来变过就返回新的（`last` 记着上次的版本号）
pub fn take_data(app: &MiniApp, last: &mut i64) -> Option<serde_json::Value> {
    let v: i64 = app.eval("__tabBarVersion").ok()?.parse().ok()?;
    if v == *last {
        return None;
    }
    *last = v;
    serde_json::from_str(&app.eval("__customTabBarData()").ok()?).ok()
}

impl super::MiniEngine {
    /// 自定义 tabBar 的实例 setData 过（`getTabBar().setData` / 组件自己改角标）：换数据、重画那一条。
    /// 在 setData 之后调（数据没脏时版本号不会变，不必每帧问）
    pub(crate) fn sync_custom_tabbar(&mut self) {
        if let Some(ct) = self.custom_tabbar.as_mut() {
            if let Some(d) = take_data(&self.app, &mut ct.version) {
                ct.data = d;
                self.dmg.tabbar_route = None;
            }
        }
    }
}
