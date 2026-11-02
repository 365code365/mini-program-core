//! `<web-view>`（官方 component/web-view.html）：宿主把系统 WebView 盖在页面上（`nativeComponents` 含
//! `web-view`，位置 / 遮挡照常走 `nativeViews` 事件，多一个解析好的 `url`），网页里的 JSSDK
//! （`wx.miniProgram.*`、`WeixinJSBridge.invoke('invokeMiniProgramAPI')`）由宿主注入、转回引擎。
//!
//! 宿主经 `mr_app_native_view_event(key, name, detail)` 交来的几种事件：
//!
//! | name | detail | 引擎做什么 |
//! |---|---|---|
//! | `load` | `{src}` | 派给 `bindload`（网页加载完） |
//! | `error` | `{src, errMsg?}` | 派给 `binderror` |
//! | `message` | `{data}` | 网页 `wx.miniProgram.postMessage({data})`：**先攒着**，见下 |
//! | `miniProgram` | `{api, arg}` | 网页里调 `navigateTo` / `redirectTo` / `switchTab` / `reLaunch` / `navigateBack`：照着调 `wx.<api>(arg)` |
//! | `titlechange` | `{title}` | 网页标题当导航栏标题（与微信一致：web-view 页的标题栏显示网页的 `<title>`） |
//!
//! ## 消息为什么要攒
//! 官方：网页 postMessage 的消息**不实时**送达，只在「小程序后退、组件销毁、分享、复制链接」时一次性派给
//! `bindmessage`，`e.detail.data` 是这段时间里每一次 postMessage 的 data 组成的数组。所以这里按页面栈层攒，
//! 离开那一页（返回、`redirectTo`、`reLaunch`、`switchTab`）之前、onUnload 之前派。
use super::engine::MiniEngine;
use crate::renderer::NativeRegion;
use serde_json::{json, Value};

#[derive(Default)]
pub struct WebViewState {
    /// (页面栈层数, 节点键, 攒下的 data)
    pending: Vec<(usize, String, Vec<Value>)>,
}

/// 网页能让小程序做的跳转（JSSDK `wx.miniProgram.*` 里的那几个）
const NAV_APIS: &[&str] = &["navigateTo", "redirectTo", "switchTab", "reLaunch", "navigateBack"];

impl MiniEngine {
    /// `dispatch_native_view_event` 找到的控件是 web-view：网页那边特有的事件在这里处理。
    /// 返回 None 表示按普通组件事件派发（`load` / `error` → `bindload` / `binderror`）
    pub(super) fn webview_event(&mut self, region: &NativeRegion, name: &str, detail: &Value) -> Option<bool> {
        let depth = self.page_stack.len();
        match name {
            "message" => {
                let data = detail.get("data").cloned().unwrap_or(Value::Null);
                let p = &mut self.webview.pending;
                match p.iter_mut().find(|(d, k, _)| *d == depth && *k == region.key) {
                    Some(e) => e.2.push(data),
                    None => p.push((depth, region.key.clone(), vec![data])),
                }
                Some(true)
            }
            "miniProgram" => {
                let api = detail.get("api").and_then(Value::as_str).unwrap_or("");
                if !NAV_APIS.contains(&api) {
                    eprintln!("⚠️ web-view：网页调了不支持的 wx.miniProgram.{api}");
                    return Some(false);
                }
                let arg = detail.get("arg").filter(|a| a.is_object()).cloned().unwrap_or_else(|| json!({}));
                // 要离开这一页的跳转：先派攒下的消息。`wx.navigateBack` 在逻辑层当场就出栈、派 onUnload
                // （引擎之后才退视图），等 back_with 再派就晚了 —— 处理函数所在的页实例已经没了
                if api != "navigateTo" {
                    self.flush_webview_messages();
                }
                self.app.eval(&format!("wx.{api}({arg})")).ok();
                self.app.pump_jobs();
                Some(true)
            }
            "titlechange" => {
                let title = detail.get("title").and_then(Value::as_str).unwrap_or("").trim().to_string();
                if !title.is_empty() {
                    let t = Value::String(title);
                    self.app.eval(&format!("wx.setNavigationBarTitle({{title:{t}}})")).ok();
                    self.app.pump_jobs();
                }
                Some(true)
            }
            _ => None,
        }
    }

    /// 要离开 / 销毁当前页之前（返回、redirectTo、reLaunch、switchTab）：这一页 web-view 攒下的消息
    /// 一次派给 `bindmessage`（`e.detail.data` 是数组）
    pub(super) fn flush_webview_messages(&mut self) {
        let depth = self.page_stack.len();
        if self.webview.pending.is_empty() {
            return;
        }
        let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.webview.pending).into_iter().partition(|(d, _, _)| *d >= depth);
        self.webview.pending = rest;
        for (_, key, data) in mine {
            let region = self.renderer.as_ref().and_then(|r| r.native_regions().iter().find(|n| n.key == key)).cloned();
            match region {
                Some(r) => {
                    self.dispatch_region_event(&r, "message", json!({ "data": data }));
                }
                None => eprintln!("⚠️ web-view：页面上已经没有 {key}，{} 条消息丢弃", data.len()),
            }
        }
    }
}
