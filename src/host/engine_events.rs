//! 引擎 → 宿主的事件队列：宿主（移动端 SDK / Flutter 插件）每次 `pump` 之后用
//! `mr_app_poll_event` 取到 NULL 为止。每条是一个 JSON 对象，`type` 区分：
//!
//! | type | 何时 | 字段 |
//! |---|---|---|
//! | `page` | 当前页或栈深变了 | `route` `depth` `isTabBar` `window`（app.json 与页面 json 合并，含导航栏改动） |
//! | `navigationBar` | `wx.setNavigationBarTitle/Color`、导航栏 loading | `title` / `frontColor` / `backgroundColor` / `loading` |
//! | `inputFocus` | 输入框聚焦（已聚焦时再点一下挪光标也会再发） | `id` `tag` `value` `cursor`（字符下标） `inputType` `password` `maxlength` `confirmType`（textarea 默认 `return`） `confirmHold` `placeholder` `rect` |
//! | `inputBlur` | 失焦 / 确认（`confirm-hold` 的确认不发） | `id` `value` |
//! | `nativeViews` | 原生组件的摆放或遮挡变了 | `views[]`：`key` `tag` `x` `y` `width` `height` `attrs` `fixed` `clip` `holes` `mask`（遮挡三项见 `native_occlusion.rs`） |
//! | `hostCall` | 调了宿主实现的 `wx.*` | `id` `api` `args` |
//! | `clipboard` | `wx.setClipboardData`、输入框里 `copy` / `cut`（宿主写系统剪贴板） | `data` |
//! | `vibrate` | `wx.vibrateShort` / `vibrateLong` | `duration`（`short` / `long`）`style` |
//! | `app` | 启动后第一帧（只发一次） | `about`：`name` `appId` `description` `entryPage` `pageCount` `engine`（见 `app_about.rs`） |
//! | `menuButton` | 点了引擎画的右上角胶囊（选项 `menuButton`，custom 导航栏的页面；见 engine_capsule.rs） | `action`：`menu`（•••，查看关于）/ `close`（⊙，返回宿主） |
//! | `log` | 逻辑层 `console.*` | `text` |
//!
//! 坐标一律是**视口逻辑坐标**（左上原点，已减滚动；`nativeViews` 在侧滑返回时还加上了整帧右移）。
//!
//! ## 为什么是轮询而不是回调
//! C ABI 一直是「没有回调注册」：回调意味着要约定在哪条线程上回、回调里能不能再调引擎。
//! 轮询把这些全留给宿主自己的线程模型，Flutter 插件、原生 View、单测都一样用。
use super::engine::MiniEngine;
use serde_json::{json, Map, Value};
use std::collections::VecDeque;

/// 队列上限：宿主不取时丢最旧的（日志刷屏不能把内存吃光）
pub const EVENT_CAP: usize = 512;

#[derive(Default)]
pub struct HostEvents {
    queue: VecDeque<String>,
    /// 上一次报给宿主的页面（路由、栈深）：变了才发 `page`
    last_page: Option<(String, usize)>,
    /// 页面栈每一层被 `setNavigationBarTitle/Color` 改过的字段（与 page_stack 对齐）。
    /// 返回上一页时宿主要恢复**那一页**的标题，所以按层存，不是一个全局值。
    nav: Vec<(String, Map<String, Value>)>,
    /// 当前聚焦的输入框（发 `inputBlur` 用）
    focused: Option<String>,
    /// 上一次报的原生组件摆放（变了才发，免得滚动静止时每帧刷一条）
    last_native: Option<String>,
    /// `app` 事件（关于信息）发过了：启动后只发一次
    app_sent: bool,
}

impl HostEvents {
    pub fn push(&mut self, v: Value) {
        self.push_raw(v.to_string());
    }

    fn push_raw(&mut self, s: String) {
        if self.queue.len() >= EVENT_CAP {
            self.queue.pop_front();
        }
        self.queue.push_back(s);
    }

    pub fn pop(&mut self) -> Option<String> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl MiniEngine {
    /// 取一条事件（JSON），没有了返回 None
    pub fn poll_event(&mut self) -> Option<String> {
        self.events.pop()
    }

    /// 每次 pump 末尾：把逻辑层与引擎这一帧产生的事件收拢进队列
    pub(super) fn sync_host_events(&mut self) {
        self.sync_app_event();
        self.sync_nav_slots();
        self.drain_js_host_events();
        self.drain_js_logs();
        self.sync_page_event();
        // 换到有 / 没有导航栏的页面：逻辑层的 windowHeight 跟着变（engine_navbar.rs）
        self.sync_window_height();
        self.sync_native_views();
    }

    /// 启动后第一帧：小程序的关于信息（胶囊「•••」由宿主展示，见 `app_about.rs`）
    fn sync_app_event(&mut self) {
        if self.events.app_sent || self.page_stack.is_empty() {
            return;
        }
        self.events.app_sent = true;
        let root = super::page_loader::get_app_path();
        let app_json = serde_json::from_str::<Value>(&super::page_loader::load_app_json()).unwrap_or(Value::Null);
        let about = crate::app_about::about(&root, &app_json);
        self.events.push(json!({ "type": "app", "about": about }));
    }

    /// 导航栏改动按页面栈层对齐：换页（同层换了路由）清空，出栈截断
    fn sync_nav_slots(&mut self) {
        let routes: Vec<String> = self.page_stack.iter().map(|p| p.path.clone()).collect();
        let nav = &mut self.events.nav;
        nav.truncate(routes.len());
        for (i, r) in routes.iter().enumerate() {
            match nav.get_mut(i) {
                Some(slot) if &slot.0 == r => {}
                Some(slot) => *slot = (r.clone(), Map::new()),
                None => nav.push((r.clone(), Map::new())),
            }
        }
    }

    fn drain_js_host_events(&mut self) {
        for raw in self.app.take_host_events() {
            let Ok(v) = serde_json::from_str::<Value>(&raw) else { continue };
            if v.get("type").and_then(|t| t.as_str()) == Some("navigationBar") {
                if let (Some(slot), Some(o)) = (self.events.nav.last_mut(), v.as_object()) {
                    for (k, val) in o {
                        if k != "type" {
                            slot.1.insert(k.clone(), val.clone());
                        }
                    }
                }
                // 前景色决定胶囊用深色还是浅色那套
                if v.get("frontColor").is_some() {
                    self.capsule_restyle();
                }
            }
            self.events.push_raw(raw);
        }
    }

    /// `console.*` 的输出。SDK 从前从不取 `__print_buffer` —— 日志宿主看不见，
    /// 缓冲还一直涨（只在加载脚本时清一次）。
    fn drain_js_logs(&mut self) {
        let Ok(raw) = self
            .app
            .eval("__print_buffer.length ? JSON.stringify(__print_buffer.splice(0)) : ''")
        else {
            return;
        };
        if raw.is_empty() {
            return;
        }
        if let Ok(Value::Array(lines)) = serde_json::from_str::<Value>(&raw) {
            for l in lines {
                let text = l.as_str().map(str::to_string).unwrap_or_else(|| l.to_string());
                self.events.push(json!({ "type": "log", "text": text }));
            }
        }
    }

    /// 当前页的窗口配置：app.json 的 `window` ← 页面 json ← 运行时导航栏改动
    pub fn page_window(&self) -> Value {
        let mut w = self.app_window.as_object().cloned().unwrap_or_default();
        let route = self.current_route();
        if let Some(Value::Object(page)) = self.pages.get(&route).map(|p| &p.window) {
            for (k, v) in page {
                // 只合并窗口配置，组件声明不是宿主外壳关心的
                if k != "usingComponents" && k != "componentGenerics" {
                    w.insert(k.clone(), v.clone());
                }
            }
        }
        if let Some((_, nav)) = self.events.nav.last() {
            fn map(k: &str) -> &str {
                match k {
                    "title" => "navigationBarTitleText",
                    "frontColor" => "navigationBarTextStyle",
                    "backgroundColor" => "navigationBarBackgroundColor",
                    "loading" => "navigationBarLoading",
                    other => other,
                }
            }
            for (k, v) in nav {
                // frontColor 是 #ffffff / #000000，窗口配置里的同义字段是 white / black
                let v = if k == "frontColor" {
                    match v.as_str() {
                        Some("#ffffff") | Some("#FFFFFF") => json!("white"),
                        Some(_) => json!("black"),
                        None => v.clone(),
                    }
                } else {
                    v.clone()
                };
                w.insert(map(k).to_string(), v);
            }
        }
        Value::Object(w)
    }

    fn sync_page_event(&mut self) {
        let sig = (self.current_route(), self.page_stack.len());
        if sig.1 == 0 || self.events.last_page.as_ref() == Some(&sig) {
            return;
        }
        let is_tab = self.is_tabbar_page(&sig.0);
        let ev = json!({
            "type": "page",
            "route": sig.0,
            "depth": sig.1,
            "isTabBar": is_tab,
            "window": self.page_window(),
        });
        self.events.last_page = Some(sig);
        self.events.push(ev);
    }

    /// 输入框聚焦（`apply_interaction` 调）。`bounds` 是逻辑像素，正常流为内容坐标。
    pub(super) fn emit_input_focus(&mut self, id: &str, value: &str, bounds: crate::Rect, is_fixed: bool) {
        let region = self.renderer.as_ref().and_then(|r| r.input_region(id)).cloned();
        let attr = |k: &str| region.as_ref().and_then(|r| r.attrs.get(k)).cloned();
        let truthy = |k: &str| attr(k).map(|v| v == "true" || v == "{{true}}" || v.is_empty()).unwrap_or(false);
        // 画面坐标：引擎画了导航栏时页面视口在它下面（宿主按这个矩形算键盘顶起多少）
        let y = self.nav_height() + if is_fixed { bounds.y } else { bounds.y - self.scroll.get_position() };
        let tag = region.as_ref().map(|r| r.tag.clone()).unwrap_or_else(|| "input".into());
        // 点中位置的光标（字符下标）。宿主的输入法要从这里开始编辑，
        // 而不是假设光标在末尾 —— 否则在文字中间点一下再打字，字会落到别处
        let cursor = self.interaction.focused_input.as_ref().map(|f| f.cursor_pos).unwrap_or(0);
        let ev = json!({
            "type": "inputFocus",
            "id": id,
            "value": value,
            "cursor": cursor,
            "inputType": attr("type").unwrap_or_else(|| "text".into()),
            "password": truthy("password"),
            "maxlength": attr("maxlength").and_then(|v| v.parse::<i64>().ok()).unwrap_or(140),
            // textarea 默认 `return`：右下角就是换行键，不触发 confirm（微信 textarea 的默认值）
            "confirmType": attr("confirm-type")
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| if tag == "textarea" { "return".into() } else { "done".into() }),
            "tag": tag,
            "confirmHold": truthy("confirm-hold"),
            "placeholder": attr("placeholder").unwrap_or_default(),
            "rect": { "x": bounds.x, "y": y, "width": bounds.width, "height": bounds.height },
        });
        self.events.focused = Some(id.to_string());
        self.events.push(ev);
    }

    /// 输入框失焦 / 确认（键盘该收起了）
    pub(super) fn emit_input_blur(&mut self, id: &str, value: &str) {
        if self.events.focused.take().is_some() {
            self.events.push(json!({ "type": "inputBlur", "id": id, "value": value }));
        }
    }

    /// 输入框里复制 / 剪切：与 `wx.setClipboardData` 同一条 `clipboard` 事件（宿主写系统剪贴板）
    pub(super) fn emit_clipboard(&mut self, text: &str) {
        crate::clipboard::set(text);
        self.events.push(json!({ "type": "clipboard", "data": text }));
    }

    /// 宿主承载的原生组件：摆放或遮挡变了才发（滚动中、picker 面板滑入滑出时每帧都会变，
    /// 静止时不发）。遮挡（`clip` / `holes` / `mask`）见 `native_occlusion.rs`。
    fn sync_native_views(&mut self) {
        if self.opts.native_components.is_empty() {
            return;
        }
        let scroll = self.scroll.get_position();
        let vw = self.width as f32;
        // 报给宿主的是**画面坐标**：引擎画了导航栏时页面视口整体在它下面（engine_navbar.rs）
        let top = self.nav_height();
        // 正常流画在 tabBar 之上的那一截；fixed 覆盖层盖满整个视口（含 tabBar，见 present_to_buffer）
        let flow_area = crate::Rect::new(0.0, top, vw, self.viewport_height());
        let fixed_area = crate::Rect::new(0.0, top, vw, self.page_window_height());
        // 侧滑返回 / 页面转场时整帧平移（合成最后一步搬像素）：控件不在那张图里，
        // 不报这段位移它就停在原处，盖在露出来的另一页上。返回转场时活的是下面那一页：
        // 它按视差左移、右边被离开的那页盖住（可见区右沿 = 那页的左沿）
        let (dx, visible_right) = self.route.live_shift(vw).unwrap_or((self.edge_back_shift(), vw));
        let regions = self.renderer.as_ref().map(|r| r.native_regions()).unwrap_or(&[]);
        let covers = if regions.is_empty() { Default::default() } else { self.native_covers() };
        let rect = |r: &crate::Rect| json!({ "x": r.x, "y": r.y, "width": r.width, "height": r.height });
        let views: Vec<Value> = regions
            .iter()
            .filter_map(|n| {
                let y = top + if n.is_fixed { n.bounds.y } else { n.bounds.y - scroll };
                let b = &n.bounds;
                let view = crate::Rect::new(b.x, y, b.width, b.height);
                let area = if n.is_fixed { fixed_area } else { flow_area };
                // 滚出自己那一层的不报（宿主就把控件收起来）。被覆盖层**挡住**的照报，
                // 只是 clip 为空 —— 面板一关就要原样露出来，控件不能被销毁重建。
                if !view.intersects(&area) {
                    return None;
                }
                // 遮挡在平移之前算：覆盖层也跟着整帧一起挪，相对关系不变
                let vis = covers.visibility(view, area, n.is_fixed).shifted(dx, visible_right);
                Some(json!({
                    "key": n.key, "tag": n.tag,
                    "x": b.x + dx, "y": y, "width": b.width, "height": b.height,
                    "fixed": n.is_fixed, "attrs": n.attrs,
                    "clip": rect(&vis.clip),
                    "holes": vis.holes.iter().map(rect).collect::<Vec<_>>(),
                    "mask": vis.mask,
                }))
            })
            .collect();
        let sig = serde_json::to_string(&views).unwrap_or_default();
        if self.events.last_native.as_deref() == Some(sig.as_str()) {
            return;
        }
        self.events.last_native = Some(sig);
        self.events.push(json!({ "type": "nativeViews", "views": views }));
    }

    /// 宿主承载的原生组件发生了事件（视频播完、地图点了标记…）：按节点上的
    /// `bind<name>` 派发给页面 / 组件实例。返回是否找到了绑定。
    pub fn dispatch_native_view_event(&mut self, key: &str, name: &str, detail: Value) -> bool {
        let Some(region) = self
            .renderer
            .as_ref()
            .and_then(|r| r.native_regions().iter().find(|n| n.key == key))
            .cloned()
        else {
            return false;
        };
        let Some((_, handler, owner, data)) = region.events.iter().find(|e| e.0 == name).cloned() else {
            return false;
        };
        let node = json!({
            "id": region.attrs.get("id").cloned().unwrap_or_default(),
            "dataset": data,
            "offsetLeft": region.bounds.x,
            "offsetTop": region.bounds.y,
        });
        let event = json!({
            "type": name, "timeStamp": self.mono_ms(),
            "target": node, "currentTarget": node, "detail": detail,
            "touches": [], "changedTouches": [],
        });
        let h = serde_json::to_string(&handler).unwrap_or_else(|_| "''".into());
        let o = serde_json::to_string(&owner).unwrap_or_else(|_| "''".into());
        self.app.eval(&format!("__dispatchEvent({h}, {event}, {o})")).ok();
        self.app.pump_jobs();
        // 只标数据脏：重画多大由失效分析定（视频每秒回一次 timeupdate，不该每秒整帧）
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        true
    }

    /// 宿主回 hostCall 的结果
    pub fn host_reply(&mut self, id: u32, ok: bool, result_json: &str) -> bool {
        let found = self.app.resolve_host_call(id, ok, result_json);
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        found
    }
}
