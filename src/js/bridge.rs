//! JS 与 Native 桥接层

use super::JsRuntime;
use crate::event::{Event, Touch};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// JS 桥接器
pub struct JsBridge {
    runtime: Arc<Mutex<JsRuntime>>,
    storage: Arc<Mutex<HashMap<String, String>>>,
    event_queue: Arc<Mutex<Vec<BridgeEvent>>>,
    /// 逻辑层调用过 `setData`（宿主每帧取走一次，决定要不要重绘）
    data_dirty: Arc<std::sync::atomic::AtomicBool>,
    /// 交给宿主外壳的事件（导航栏、hostCall），JSON 串。**每个实例一份**，不是进程级：
    /// 进程里同时有两个引擎（测试并行跑）时互不串台。没人取时按 [`HOST_EVENT_CAP`] 丢最旧的。
    host_events: Arc<Mutex<std::collections::VecDeque<String>>>,
    /// 网络请求的归属号：响应按它取回，不会被同进程里别的实例 `update()` 取走（见 `net.rs` 的 DONE）
    net_owner: u64,
}

/// 宿主事件队列上限：桌面窗体不消费这些事件，不封顶就是一条慢慢涨的内存
pub const HOST_EVENT_CAP: usize = 256;

/// 桥接事件
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    ConsoleLog(String),
    ConsoleError(String),
    ConsoleWarn(String),
    ShowToast { title: String, icon: String, duration: u32, mask: bool, image: String },
    HideToast { no_conflict: bool },
    ShowLoading { title: String, mask: bool },
    HideLoading { no_conflict: bool },
    ShowModal {
        title: String,
        content: String,
        show_cancel: bool,
        cancel_text: String,
        confirm_text: String,
        cancel_color: String,
        confirm_color: String,
    },
    NavigateTo(String),
    NavigateBack(u32),
    SetTimer { id: u32, delay: u32, repeat: bool },
    ClearTimer(u32),
    StartPullDownRefresh,
    StopPullDownRefresh,
    /// `wx.pageScrollTo` 的请求（JSON：scrollTop / selector / offsetTop / duration）
    PageScrollTo(String),
    /// 要宿主（布局树 / 组件状态）来办的请求：`kind` 是请求种类（`video` / `selectorQuery` …），
    /// `json` 是参数。宿主每帧按种类取走（`MiniApp::take_host_requests`），新 API 不必再加变体
    HostRequest { kind: String, json: String },
    CanvasDraw { canvas_id: String, commands: String },
    StorageSet { key: String, value: String },
    StorageGet { key: String },
    StorageRemove { key: String },
    StorageClear,
}

impl Drop for JsBridge {
    /// 实例没了：它没取的网络响应丢掉，迟到的也不再入队
    fn drop(&mut self) {
        crate::net::forget_owner(self.net_owner);
    }
}

impl JsBridge {
    pub fn new(runtime: Arc<Mutex<JsRuntime>>) -> Self {
        Self {
            runtime,
            storage: Arc::new(Mutex::new(HashMap::new())),
            event_queue: Arc::new(Mutex::new(Vec::new())),
            data_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            host_events: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            net_owner: crate::net::new_owner(),
        }
    }

    /// 本实例的网络归属号（`MiniApp::update` 按它取回自己的响应）
    pub fn net_owner(&self) -> u64 {
        self.net_owner
    }

    /// 取走逻辑层交给宿主外壳的事件（JSON 串，按发生顺序）
    pub fn take_host_events(&self) -> Vec<String> {
        self.host_events.lock().unwrap().drain(..).collect()
    }

    /// 取走「逻辑层改过数据」标记（读后清零）。宿主用它决定本帧是否重绘：
    /// 定时器、网络回调里的 `setData` 全靠这个上屏。
    pub fn take_data_dirty(&self) -> bool {
        self.data_dirty.swap(false, std::sync::atomic::Ordering::Relaxed)
    }
    
    /// 初始化 native 函数
    pub fn init(&self) -> Result<(), String> {
        println!("    register_print_function...");
        self.register_print_function().map_err(|e| format!("print: {}", e))?;
        println!("    register_timer_functions...");
        self.register_timer_functions().map_err(|e| format!("timer: {}", e))?;
        println!("    register_storage_functions...");
        self.register_storage_functions().map_err(|e| format!("storage: {}", e))?;
        println!("    register_ui_functions...");
        self.register_ui_functions().map_err(|e| format!("ui: {}", e))?;
        println!("    register_network_functions...");
        self.register_network_functions().map_err(|e| format!("network: {}", e))?;
        self.register_host_functions().map_err(|e| format!("host: {}", e))?;
        super::bridge_io::register(&self.runtime.lock().unwrap(), self.event_queue.clone()).map_err(|e| format!("io: {}", e))?;
        super::bridge_ext::register(&self.runtime.lock().unwrap(), self.storage.clone()).map_err(|e| format!("ext: {}", e))?;
        Ok(())
    }
    
    fn register_print_function(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            var __print_buffer = [];
            function __native_print(msg) {
                __print_buffer.push(String(msg));
            }
        "#)?;
        // setData 的脏标记走原生回调：宿主每帧取走一次决定要不要重绘。
        // 从前这里是 JS 侧 `__print_buffer.push('[PageUpdate] ' + JSON.stringify(data))`，
        // 没有任何消费方 —— 每次 setData 白付一次全量序列化，还把日志刷成一片。
        // 可注入的固定时钟：`MINI_FAKE_NOW=<epoch 毫秒>` 或 ISO 日期（`2024-03-15`）。
        // 只为**快照回归**服务 —— 有些页面用 `new Date()` 高亮当天（news-app 的签到
        // 日历），基线跨天就失效。不设时返回 0，JS 侧原样使用系统 Date。
        rt.register_function("__native_fake_now", move |_| fake_now_ms().to_string())?;

        let dirty = self.data_dirty.clone();
        rt.register_function("__native_page_update", move |_| {
            dirty.store(true, std::sync::atomic::Ordering::Relaxed);
            "undefined".to_string()
        })?;
        Ok(())
    }
    
    fn register_timer_functions(&self) -> Result<(), String> {
        let queue = self.event_queue.clone();
        let rt = self.runtime.lock().unwrap();
        
        let q = queue.clone();
        rt.register_function("__native_set_timer", move |args| {
            if args.len() >= 3 {
                let id: u32 = args[0].parse().unwrap_or(0);
                let delay: u32 = args[1].parse().unwrap_or(0);
                let repeat = args[2] == "true";
                q.lock().unwrap().push(BridgeEvent::SetTimer { id, delay, repeat });
            }
            "undefined".to_string()
        })?;
        
        let q = queue.clone();
        rt.register_function("__native_clear_timer", move |args| {
            if let Some(id_str) = args.first() {
                let id: u32 = id_str.parse().unwrap_or(0);
                q.lock().unwrap().push(BridgeEvent::ClearTimer(id));
            }
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    /// `__native_host_event(json)`：导航栏意图、hostCall 等交给宿主外壳的事件。
    ///
    /// 闭环里**只锁自己的队列**：原生函数是在 `runtime` 锁内被调用的，
    /// 这里再去锁 runtime 或 eval 会死锁。
    fn register_host_functions(&self) -> Result<(), String> {
        let q = self.host_events.clone();
        let rt = self.runtime.lock().unwrap();
        rt.register_function("__native_host_event", move |args| {
            if let Some(json) = args.first() {
                let mut q = q.lock().unwrap();
                if q.len() >= HOST_EVENT_CAP {
                    q.pop_front();
                }
                q.push_back(json.clone());
            }
            "undefined".to_string()
        })?;
        Ok(())
    }

    /// `wx.request` 的原生入口：立刻返回，结果由宿主每帧取回喂给 `__resolveRequest`
    fn register_network_functions(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        let owner = self.net_owner;
        rt.register_function("__native_request", move |args| {
            // (id, method, url, headersJson, body, timeoutMs)
            if args.len() >= 5 {
                let id: u32 = args[0].parse().unwrap_or(0);
                let timeout: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
                crate::net::submit(owner, id, &args[1], &args[2], &args[3], &args[4], timeout);
            }
            "undefined".to_string()
        })?;
        // downloadFile：(id, url, headersJson, filePath, timeoutMs)，结果同样经 __resolveRequest 回填
        rt.register_function("__native_download", move |args| {
            let a = |i: usize| args.get(i).cloned().unwrap_or_default();
            crate::net::submit_download(owner, a(0).parse().unwrap_or(0), &a(1), &a(2), &a(3), a(4).parse().unwrap_or(0));
            "undefined".to_string()
        })?;
        // uploadFile：(id, url, filePath, name, headersJson, formDataJson, timeoutMs)
        rt.register_function("__native_upload", move |args| {
            let a = |i: usize| args.get(i).cloned().unwrap_or_default();
            crate::net::submit_upload(owner, a(0).parse().unwrap_or(0), &a(1), &a(2), &a(3), &a(4), &a(5), a(6).parse().unwrap_or(0));
            "undefined".to_string()
        })?;
        // 剪贴板：引擎里存一份，桌面窗体打开 use_system 后同步系统剪贴板（src/clipboard.rs）
        rt.register_function("__native_clipboard_set", move |args| {
            crate::clipboard::set(args.first().map(|s| s.as_str()).unwrap_or(""));
            "undefined".to_string()
        })?;
        rt.register_function("__native_clipboard_get", move |_args| crate::clipboard::get())?;
        // wx.loadFontFace：(id, family, src)。结果排队，MiniApp::update 每帧回 __resolveFontFace
        rt.register_function("__native_load_font", move |args| {
            let a = |i: usize| args.get(i).cloned().unwrap_or_default();
            crate::text_faces::load(a(0).parse().unwrap_or(0), &a(1), &a(2));
            "undefined".to_string()
        })?;
        Ok(())
    }

    fn register_storage_functions(&self) -> Result<(), String> {
        let storage = self.storage.clone();
        // 先把上次会话存下的内容读回来：微信的 storage 是跨启动持久的，
        // 「首次拉数据存起来、之后走缓存」这类逻辑只有这样才会真的走到缓存分支
        {
            let restored = crate::storage_file::load();
            if !restored.is_empty() {
                storage.lock().unwrap().extend(restored);
            }
        }
        let rt = self.runtime.lock().unwrap();

        // 写操作一律**写穿到磁盘**。storage 的量级是几十 KB，页面写入也不频繁，
        // 换来的是「进程被杀也不丢」，比攒着等退出时再存可靠。
        let s = storage.clone();
        rt.register_function("__native_storage_set", move |args| {
            if args.len() >= 2 {
                let mut map = s.lock().unwrap();
                map.insert(args[0].clone(), args[1].clone());
                crate::storage_file::save(&map);
            }
            "undefined".to_string()
        })?;
        
        let s = storage.clone();
        rt.register_function("__native_storage_get", move |args| {
            if let Some(key) = args.first() {
                s.lock().unwrap().get(key).cloned().unwrap_or_default()
            } else {
                String::new()
            }
        })?;
        
        let s = storage.clone();
        rt.register_function("__native_storage_remove", move |args| {
            if let Some(key) = args.first() {
                let mut map = s.lock().unwrap();
                map.remove(key);
                crate::storage_file::save(&map);
            }
            "undefined".to_string()
        })?;
        
        let s = storage.clone();
        rt.register_function("__native_storage_clear", move |_args| {
            let mut map = s.lock().unwrap();
            map.clear();
            crate::storage_file::save(&map);
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    fn register_ui_functions(&self) -> Result<(), String> {
        let queue = self.event_queue.clone();
        let rt = self.runtime.lock().unwrap();
        
        // showToast(title, icon, duration, mask, image)：参数校验与缺省值在 prelude/ui.js
        let q = queue.clone();
        rt.register_function("__native_show_toast", move |args| {
            let title = args.get(0).cloned().unwrap_or_default();
            let icon = args.get(1).cloned().unwrap_or_else(|| "success".to_string());
            let duration: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1500);
            let mask = args.get(3).map(|s| s == "true").unwrap_or(false);
            let image = args.get(4).cloned().unwrap_or_default();
            q.lock().unwrap().push(BridgeEvent::ShowToast { title, icon, duration, mask, image });
            "undefined".to_string()
        })?;
        
        // hideToast(noConflict) / hideLoading(noConflict)：默认两者互相能关（微信的「混用」）
        let q = queue.clone();
        rt.register_function("__native_hide_toast", move |args| {
            let no_conflict = args.first().map(|s| s == "true").unwrap_or(false);
            q.lock().unwrap().push(BridgeEvent::HideToast { no_conflict });
            "undefined".to_string()
        })?;
        
        // showLoading
        let q = queue.clone();
        rt.register_function("__native_show_loading", move |args| {
            let title = args.get(0).cloned().unwrap_or_default();
            let mask = args.get(1).map(|s| s == "true").unwrap_or(false);
            q.lock().unwrap().push(BridgeEvent::ShowLoading { title, mask });
            "undefined".to_string()
        })?;
        
        let q = queue.clone();
        rt.register_function("__native_hide_loading", move |args| {
            let no_conflict = args.first().map(|s| s == "true").unwrap_or(false);
            q.lock().unwrap().push(BridgeEvent::HideLoading { no_conflict });
            "undefined".to_string()
        })?;

        // 下拉刷新：宿主负责指示器与内容位移，逻辑层只发开始/结束
        let q = queue.clone();
        rt.register_function("__native_start_pull_down_refresh", move |_args| {
            q.lock().unwrap().push(BridgeEvent::StartPullDownRefresh);
            "undefined".to_string()
        })?;
        let q = queue.clone();
        rt.register_function("__native_stop_pull_down_refresh", move |_args| {
            q.lock().unwrap().push(BridgeEvent::StopPullDownRefresh);
            "undefined".to_string()
        })?;
        // pageScrollTo：位置换算（selector）与滚动动画由宿主做，见 host::scroll_view
        let q = queue.clone();
        rt.register_function("__native_page_scroll_to", move |args| {
            q.lock().unwrap().push(BridgeEvent::PageScrollTo(args.first().cloned().unwrap_or_default()));
            "undefined".to_string()
        })?;
        // 通用的宿主请求：`__native_host_request(kind, json)`（VideoContext、SelectorQuery …）
        let q = queue.clone();
        rt.register_function("__native_host_request", move |args| {
            let kind = args.first().cloned().unwrap_or_default();
            let json = args.get(1).cloned().unwrap_or_default();
            q.lock().unwrap().push(BridgeEvent::HostRequest { kind, json });
            "undefined".to_string()
        })?;
        
        // showModal
        let q = queue.clone();
        rt.register_function("__native_show_modal", move |args| {
            let title = args.get(0).cloned().unwrap_or_default();
            let content = args.get(1).cloned().unwrap_or_default();
            let show_cancel = args.get(2).map(|s| s == "true").unwrap_or(true);
            let cancel_text = args.get(3).cloned().unwrap_or_else(|| "取消".to_string());
            let confirm_text = args.get(4).cloned().unwrap_or_else(|| "确定".to_string());
            // 微信缺省：取消 #000000、确定 #576B95
            let cancel_color = args.get(5).cloned().unwrap_or_else(|| "#000000".to_string());
            let confirm_color = args.get(6).cloned().unwrap_or_else(|| "#576B95".to_string());
            q.lock().unwrap().push(BridgeEvent::ShowModal {
                title,
                content,
                show_cancel,
                cancel_text,
                confirm_text,
                cancel_color,
                confirm_color,
            });
            "undefined".to_string()
        })?;
        
        // Canvas 绘制
        let q = queue.clone();
        rt.register_function("__native_canvas_draw", move |args| {
            let canvas_id = args.get(0).cloned().unwrap_or_default();
            let commands = args.get(1).cloned().unwrap_or_else(|| "[]".to_string());
            q.lock().unwrap().push(BridgeEvent::CanvasDraw { canvas_id, commands });
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    /// 队列里还有没处理的事件（下一次 `MiniApp::update` 才会处理）
    pub fn has_pending_events(&self) -> bool {
        !self.event_queue.lock().unwrap().is_empty()
    }

    /// 获取并清空事件队列
    pub fn drain_events(&self) -> Vec<BridgeEvent> {
        let mut queue = self.event_queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }
    
    /// 触发 JS 事件
    pub fn dispatch_event(&self, event: &Event) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        match event {
            Event::Tap(tap) => {
                rt.eval(&format!(
                    "__app && __app.onTap && __app.onTap({{ x: {}, y: {}, timestamp: {} }})",
                    tap.x, tap.y, tap.timestamp
                ))?;
            }
            Event::TouchStart(touch) => {
                let touches_json = self.touches_to_json(&touch.touches);
                rt.eval(&format!(
                    "__app && __app.onTouchStart && __app.onTouchStart({{ touches: {} }})",
                    touches_json
                ))?;
            }
            Event::AppShow => {
                rt.eval("__dispatchApp && __dispatchApp('onShow')")?;
            }
            Event::AppHide => {
                rt.eval("__dispatchApp && __dispatchApp('onHide')")?;
            }
            Event::PageLoad => {
                rt.eval("__dispatchPage && __dispatchPage('onLoad')")?;
            }
            Event::PageShow => {
                // onShow 同时联动组件 pageLifetimes.show
                rt.eval("__dispatchPage && __dispatchPage('onShow')")?;
            }
            Event::PageHide => {
                // onHide 同时联动组件 pageLifetimes.hide
                rt.eval("__dispatchPage && __dispatchPage('onHide')")?;
            }
            Event::PageUnload => {
                // onUnload 触发后回收页面内组件实例（detached）
                rt.eval("__dispatchPage && __dispatchPage('onUnload')")?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn touches_to_json(&self, touches: &[Touch]) -> String {
        let items: Vec<String> = touches
            .iter()
            .map(|t| format!("{{ id: {}, x: {}, y: {} }}", t.id, t.x, t.y))
            .collect();
        format!("[{}]", items.join(", "))
    }
    
    /// 触发定时器
    pub fn trigger_timer(&self, id: u32) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(&format!("__trigger_timer({})", id))?;
        Ok(())
    }
    
}

/// `MINI_FAKE_NOW` 解析成 epoch 毫秒；没设或解析不了返回 0（= 不启用）。
///
/// 支持两种写法：纯数字（epoch 毫秒）与 `YYYY-MM-DD`（按当地时间的零点）。
/// 后者是给人用的 —— 基线脚本里写 `2024-03-15` 比写 1710432000000 可读得多。
fn fake_now_ms() -> u64 {
    let Ok(raw) = std::env::var("MINI_FAKE_NOW") else { return 0 };
    let raw = raw.trim();
    if raw.is_empty() {
        return 0;
    }
    if let Ok(ms) = raw.parse::<u64>() {
        return ms;
    }
    // YYYY-MM-DD → 当地零点。自己算天数：只为测试固定日期，不值得引入 chrono。
    let parts: Vec<&str> = raw.split('-').collect();
    if parts.len() != 3 {
        eprintln!("⚠️ MINI_FAKE_NOW 认不出：{raw}（要 epoch 毫秒或 YYYY-MM-DD）");
        return 0;
    }
    let (Ok(y), Ok(m), Ok(d)) = (
        parts[0].parse::<i64>(),
        parts[1].parse::<i64>(),
        parts[2].parse::<i64>(),
    ) else {
        eprintln!("⚠️ MINI_FAKE_NOW 认不出：{raw}");
        return 0;
    };
    // 民用历法转天数（Howard Hinnant 的 days_from_civil）
    let y_adj = if m <= 2 { y - 1 } else { y };
    let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
    let yoe = y_adj - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days.max(0) as u64) * 86_400_000
}
