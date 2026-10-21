//! 小程序应用

use crate::js::{JsRuntime, MiniAppApi, JsBridge, BridgeEvent};
use crate::event::Event;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::HashMap;

/// UI 事件（用于通知窗口显示 Toast/Loading/Modal）
///
/// Toast 与 Loading 在微信里是**同一个提示框**（官方：「wx.showLoading 和 wx.showToast
/// 同时只能显示一个」，hide 两个接口默认可以互相关掉，`noConflict: true` 才各管各的）。
/// 这份「一个槽位」的规则落在 `host::ui_overlay::apply_feedback_event`，两个宿主共用。
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// `icon`：success / error / loading / none（逻辑层已把非法值归成 success）；
    /// `image`：自定义图标路径，非空时优先于 icon；`mask`：透明蒙层，挡住页面触摸
    ShowToast { title: String, icon: String, duration: u32, mask: bool, image: String },
    /// `no_conflict`：只关 Toast，不连带关掉 Loading
    HideToast { no_conflict: bool },
    ShowLoading { title: String, mask: bool },
    /// `no_conflict`：只关 Loading，不连带关掉 Toast
    HideLoading { no_conflict: bool },
    /// 颜色是逻辑层校验过的 16 进制串（缺省 #000000 / #576B95）
    ShowModal {
        title: String,
        content: String,
        show_cancel: bool,
        cancel_text: String,
        confirm_text: String,
        cancel_color: String,
        confirm_color: String,
    },
    HideModal,
    /// `wx.startPullDownRefresh()`：由逻辑层主动进入下拉刷新态
    StartPullDownRefresh,
    /// `wx.stopPullDownRefresh()`：刷新完成，收起指示器并把内容归位
    StopPullDownRefresh,
}

/// 小程序应用
pub struct MiniApp {
    runtime: Arc<Mutex<JsRuntime>>,
    bridge: Arc<JsBridge>,
    api: MiniAppApi,
    width: u32,
    height: u32,
    running: bool,
    last_frame: Instant,
    timers: HashMap<u32, TimerState>,
    ui_events: Vec<UiEvent>,
    /// `wx.pageScrollTo` 的请求（JSON 对象），宿主每帧取走
    page_scroll_requests: Vec<serde_json::Value>,
    /// 通用宿主请求 (种类, 参数)，按发生顺序；宿主按种类取走（见 `take_host_requests`）
    host_requests: Vec<(String, serde_json::Value)>,
}

struct TimerState {
    delay_ms: u32,
    repeat: bool,
    last_trigger: Instant,
}

impl MiniApp {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let runtime = Arc::new(Mutex::new(JsRuntime::new()?));
        let bridge = Arc::new(JsBridge::new(runtime.clone()));
        let api = MiniAppApi::new(runtime.clone());
        
        Ok(Self {
            runtime,
            bridge,
            api,
            width,
            height,
            running: false,
            last_frame: Instant::now(),
            timers: HashMap::new(),
            ui_events: Vec::new(),
            page_scroll_requests: Vec::new(),
            host_requests: Vec::new(),
        })
    }
    
    /// 初始化应用
    pub fn init(&mut self) -> Result<(), String> {
        // 先初始化桥接（注册 native 函数）
        println!("  Initializing Bridge...");
        self.bridge.init().map_err(|e| format!("Bridge init failed: {}", e))?;
        
        // 再初始化 API（使用 native 函数）
        println!("  Initializing API...");
        self.api.init().map_err(|e| format!("API init failed: {}", e))?;
        // 本线程上的渲染层从此能调 wxs 模块（`{{m.f(x)}}`）；渲染与逻辑层同线程
        crate::parser::expr::wxs::set_host(Some(std::rc::Rc::new(super::wxs_host::AppWxsHost::new(&self.runtime))));
        
        println!("Mini App Engine initialized");
        Ok(())
    }
    
    /// 加载并运行 JS 代码
    pub fn load_script(&self, code: &str) -> Result<(), String> {
        println!("  Loading script...");
        let rt = self.runtime.lock().unwrap();
        rt.eval(code).map_err(|e| format!("Script error: {}", e))?;
        
        // 打印 JS 输出
        let output = rt.eval("__print_buffer.join('\\n')").unwrap_or_default();
        if !output.is_empty() && output != "undefined" {
            println!("\n--- JS Output ---");
            println!("{}", output);
            println!("-----------------\n");
        }
        
        // 清空缓冲区
        rt.eval("__print_buffer = [];").ok();
        
        println!("  Script loaded successfully");
        Ok(())
    }
    
    /// 以模块作用域加载并运行 JS（app.js / 页面 js 走这条）。
    ///
    /// 小程序里每个 .js 都是 CommonJS 模块，所以 `module` / `exports` / 相对 `require`
    /// 必须在作用域里 —— 否则 TS / uni-app 这类编译产物第一行就报
    /// `'exports' is not defined`。`path` 决定相对 require 的基准目录。
    pub fn load_module_script(&self, path: &str, code: &str) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.run_as_module(path, code)
            .map_err(|e| format!("Script error ({}): {}", path, e))?;
        let output = rt.eval("__print_buffer.join('\\n')").unwrap_or_default();
        if !output.is_empty() && output != "undefined" {
            println!("\n--- JS Output ---");
            println!("{}", output);
            println!("-----------------\n");
        }
        rt.eval("__print_buffer = [];").ok();
        Ok(())
    }

    /// 加载 JS 文件
    pub fn load_file(&self, path: &str) -> Result<(), String> {
        let code = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        self.load_script(&code)
    }
    
    /// 注册一个 CommonJS 模块（延迟执行）
    pub fn define_module(&self, path: &str, source: &str) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.define_module(path, source)
    }
    
    /// require 一个模块，触发其执行（例如页面/组件的注册）
    pub fn require_module(&self, path: &str) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.require_module(path)
    }
    
    /// 递归扫描目录，把其中所有 .js 文件注册为 CommonJS 模块。
    ///
    /// key 为相对 `root` 的路径（去掉 .js 后缀），例如 `utils/util`、
    /// `pages/index/index`。注册后可用 `require_module` 触发执行。
    pub fn register_all_modules(&self, root: &std::path::Path) -> Result<usize, String> {
        let mut count = 0usize;
        self.register_modules_recursive(root, root, &mut count)?;
        Ok(count)
    }
    
    fn register_modules_recursive(
        &self,
        root: &std::path::Path,
        dir: &std::path::Path,
        count: &mut usize,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                self.register_modules_recursive(root, &path, count)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("js") {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let key = rel.trim_end_matches(".js");
                let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                self.define_module(key, &source)?;
                *count += 1;
            }
        }
        Ok(())
    }
    
    /// 泵出 JS 微任务队列（Promise / async 回调）
    pub fn pump_jobs(&self) {
        let rt = self.runtime.lock().unwrap();
        rt.pump_jobs();
    }
    
    /// 启动应用
    pub fn start(&mut self) -> Result<(), String> {
        self.running = true;
        
        // 触发 App.onLaunch
        self.bridge.dispatch_event(&Event::AppShow)?;
        
        Ok(())
    }
    
    /// 停止应用
    pub fn stop(&mut self) {
        self.running = false;
        self.bridge.dispatch_event(&Event::AppHide).ok();
    }
    
    /// 检查是否有活动定时器
    pub fn has_active_timers(&self) -> bool {
        !self.timers.is_empty()
    }

    /// 取走「逻辑层改过数据」标记（读后清零），宿主据此决定本帧是否重绘
    pub fn take_data_dirty(&self) -> bool {
        self.bridge.take_data_dirty()
    }
    
    /// 更新一帧
    pub fn update(&mut self) -> Result<(), String> {
        let now = Instant::now();
        let _dt = now.duration_since(self.last_frame);
        self.last_frame = now;
        
        // 处理定时器
        self.process_timers()?;
        
        // 处理桥接事件
        self.process_bridge_events()?;
        
        // 取回已完成的网络响应并回调 JS。放在这里（而不是各宿主里）是为了让
        // 窗体、devserver、快照、launcher 都自动拿到网络能力。
        self.process_network_responses()?;
        self.process_font_loads();
        self.process_io_events();
        
        // 泵出微任务队列（Promise / async 回调），确保异步链得以推进
        self.pump_jobs();
        
        Ok(())
    }

    /// 把后台线程完成的 HTTP 响应喂回 JS 的 `__resolveRequest`
    fn process_network_responses(&mut self) -> Result<(), String> {
        let done = crate::net::take_completed(self.bridge.net_owner());
        if done.is_empty() {
            return Ok(());
        }
        if std::env::var("MINI_NET_LOG").is_ok() {
            eprintln!("🌐 取回 {} 条响应，回调 JS", done.len());
        }
        let rt = self.runtime.lock().unwrap();
        for r in done {
            let headers: serde_json::Map<String, serde_json::Value> = r
                .headers
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            // 全部走 JSON 字面量注入：正文里可能有引号、换行、反斜杠，
            // 手拼字符串一定会在某个响应上炸掉。
            let code = format!(
                "__resolveRequest({}, {}, {}, {}, {})",
                r.id,
                r.status,
                serde_json::Value::String(serde_json::Value::Object(headers).to_string()),
                serde_json::Value::String(r.body),
                match r.error {
                    Some(e) => serde_json::Value::String(e),
                    None => serde_json::Value::Null,
                },
            );
            if let Err(e) = rt.eval(&code) {
                // 别把回调里的异常吞掉：请求明明成功了却什么也没发生，
                // 没有这条日志根本无从下手
                eprintln!("⚠️  网络回调执行失败 (id={}): {}", r.id, e);
            }
        }
        // 数据可能因此变化，通知宿主重绘
        crate::net::mark_dirty();
        Ok(())
    }

    /// `wx.loadFontFace` 有结论了：回给逻辑层（id 为 0 的是 `@font-face` 自己登记的，不回调）
    fn process_font_loads(&mut self) {
        for (id, family, r) in crate::text_faces::take_completed() {
            if id == 0 {
                continue;
            }
            let err = match &r {
                Ok(()) => String::new(),
                Err(e) => e.clone(),
            };
            let js = format!(
                "__resolveFontFace({id}, {}, {}, {})",
                r.is_ok(),
                serde_json::to_string(&family).unwrap_or_default(),
                serde_json::to_string(&err).unwrap_or_default()
            );
            self.eval(&js).ok();
        }
    }

    /// 是否有网络请求在飞或刚回来（宿主据此决定要不要继续出帧等回调）。
    /// 下载中的字体也算（下完要有一帧去回调、重排）
    pub fn has_pending_network(&self) -> bool {
        crate::net::has_pending(self.bridge.net_owner()) || crate::text_faces::has_pending() || self.has_pending_io()
    }

    /// 逻辑层交给宿主、还没办完的活，宿主据此续帧：
    /// - 桥上的事件（画布指令、宿主请求、定时器登记…）要等下一次 `update()` 才处理。
    ///   帧中间跑的 JS（SelectorQuery 回调、tap 处理函数）发的指令都落在这里；
    /// - `selectorQuery` / `scrollView` 在页面还没布局时被留到下一帧（见 `host::selector_query`）。
    ///
    /// 不续的话：`onReady` 里 `createSelectorQuery().exec()` 拿 canvas 节点，进页面那一帧
    /// 还没布局、留到下一帧 —— 而宿主看「没动画」就停了 vsync，`type="2d"` 画布一直空白，
    /// 依赖它的按钮也全成了空操作（`if (!this.canvas) return`）。
    /// 别的种类每帧都被取空，不算进来：没人取的种类会让宿主一直空转
    pub fn has_pending_host_work(&self) -> bool {
        self.bridge.has_pending_events()
            || self.host_requests.iter().any(|(k, _)| k == "selectorQuery" || k == "scrollView")
    }

    /// 取走「网络响应导致需要重绘」的标记
    pub fn take_network_dirty(&self) -> bool {
        crate::net::take_dirty()
    }
    
    fn process_timers(&mut self) -> Result<(), String> {
        let now = Instant::now();
        let mut to_trigger = Vec::new();
        let mut to_remove = Vec::new();
        
        for (id, state) in &self.timers {
            let elapsed = now.duration_since(state.last_trigger).as_millis() as u32;
            if elapsed >= state.delay_ms {
                to_trigger.push(*id);
                if !state.repeat {
                    to_remove.push(*id);
                }
            }
        }
        
        // 触发定时器
        for id in to_trigger {
            self.bridge.trigger_timer(id)?;
            if let Some(state) = self.timers.get_mut(&id) {
                state.last_trigger = now;
            }
        }
        
        // 移除一次性定时器
        for id in to_remove {
            self.timers.remove(&id);
        }
        
        Ok(())
    }
    
    fn process_bridge_events(&mut self) -> Result<(), String> {
        let events = self.bridge.drain_events();
        
        for event in events {
            match event {
                BridgeEvent::SetTimer { id, delay, repeat } => {
                    self.timers.insert(id, TimerState {
                        delay_ms: delay,
                        repeat,
                        last_trigger: Instant::now(),
                    });
                }
                BridgeEvent::ClearTimer(id) => {
                    self.timers.remove(&id);
                }
                BridgeEvent::ShowToast { title, icon, duration, mask, image } => {
                    println!("[Toast] {} ({})", title, icon);
                    self.ui_events.push(UiEvent::ShowToast { title, icon, duration, mask, image });
                }
                BridgeEvent::HideToast { no_conflict } => {
                    self.ui_events.push(UiEvent::HideToast { no_conflict });
                }
                BridgeEvent::ShowLoading { title, mask } => {
                    println!("[Loading] {}", title);
                    self.ui_events.push(UiEvent::ShowLoading { title, mask });
                }
                BridgeEvent::HideLoading { no_conflict } => {
                    println!("[HideLoading]");
                    self.ui_events.push(UiEvent::HideLoading { no_conflict });
                }
                BridgeEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text, cancel_color, confirm_color } => {
                    println!("[Modal] {}: {}", title, content);
                    self.ui_events.push(UiEvent::ShowModal {
                        title,
                        content,
                        show_cancel,
                        cancel_text,
                        confirm_text,
                        cancel_color,
                        confirm_color,
                    });
                }
                BridgeEvent::StartPullDownRefresh => {
                    self.ui_events.push(UiEvent::StartPullDownRefresh);
                }
                BridgeEvent::StopPullDownRefresh => {
                    self.ui_events.push(UiEvent::StopPullDownRefresh);
                }
                // 不进 UiEvent：两个宿主都通过 `host::scroll_view::apply_page_scroll` 取，
                // 免得各自的 UiEvent 分发再多一个分支
                BridgeEvent::PageScrollTo(json) => {
                    if let Ok(v) = serde_json::from_str(&json) {
                        self.page_scroll_requests.push(v);
                    }
                }
                BridgeEvent::HostRequest { kind, json } => {
                    let v = serde_json::from_str(&json).unwrap_or(serde_json::Value::Null);
                    self.host_requests.push((kind, v));
                }
                BridgeEvent::NavigateTo(url) => {
                    println!("[Navigate] {}", url);
                }
                BridgeEvent::CanvasDraw { canvas_id, commands } => {
                    crate::renderer::components::execute_canvas_draw(&canvas_id, &commands);
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// 获取并清空 UI 事件
    pub fn drain_ui_events(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut self.ui_events)
    }

    /// 把一条宿主请求放回队列（这一帧还答不了，比如页面还没布局）
    pub fn push_host_request(&mut self, kind: &str, v: serde_json::Value) {
        self.host_requests.push((kind.to_string(), v));
    }

    /// 取走某一种宿主请求（按发生顺序），别的种类留着给各自的处理方
    pub fn take_host_requests(&mut self, kind: &str) -> Vec<serde_json::Value> {
        if !self.host_requests.iter().any(|(k, _)| k == kind) {
            return Vec::new();
        }
        let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.host_requests).into_iter().partition(|(k, _)| k == kind);
        self.host_requests = rest;
        mine.into_iter().map(|(_, v)| v).collect()
    }

    /// 取走 `wx.pageScrollTo` 的请求（`{scrollTop, selector, offsetTop, duration}`）
    pub fn take_page_scroll_requests(&mut self) -> Vec<serde_json::Value> {
        std::mem::take(&mut self.page_scroll_requests)
    }
    
    
    
    
    
    
    pub fn width(&self) -> u32 {
        self.width
    }
    
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 宿主改了尺寸（`MiniEngine::resize`）。只是记账：逻辑层看到的尺寸走 `__hostSystemInfo`
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// 取走逻辑层交给宿主外壳的事件（导航栏、hostCall；JSON 串，按发生顺序）
    pub fn take_host_events(&self) -> Vec<String> {
        self.bridge.take_host_events()
    }

    /// 把宿主实现的 API 装进 `wx`（`app.js` 执行之前调）
    pub fn install_host_apis(&self, names: &[String]) {
        if names.is_empty() {
            return;
        }
        let list = serde_json::to_string(names).unwrap_or_else(|_| "[]".into());
        if let Err(e) = self.eval(&format!("__installHostApis({list})")) {
            eprintln!("⚠️  安装宿主 API 失败: {e}");
        }
    }

    /// 宿主回 hostCall 的结果。`result_json` 是交给回调的 res（对象），解析不了的原样包成 `{data}`。
    /// 返回 false 表示 id 不认识（已经回过、或页面早就换了）。
    pub fn resolve_host_call(&self, id: u32, ok: bool, result_json: &str) -> bool {
        let code = format!(
            "__resolveHostCall({id}, {ok}, {})",
            serde_json::Value::String(result_json.to_string())
        );
        let r = self.eval(&code);
        self.pump_jobs();
        matches!(r.as_deref(), Ok("true"))
    }
    
    /// 执行 JS 代码
    pub fn eval(&self, code: &str) -> Result<String, String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(code)
    }
}
