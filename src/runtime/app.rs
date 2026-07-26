//! 小程序应用

use crate::{Canvas, Color};
use crate::js::{JsRuntime, MiniAppApi, JsBridge, BridgeEvent};
use crate::event::{Event, TouchEvent, Touch, TapEvent};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::HashMap;

/// UI 事件（用于通知窗口显示 Toast/Loading/Modal）
#[derive(Debug, Clone)]
pub enum UiEvent {
    ShowToast { title: String, icon: String, duration: u32 },
    HideToast,
    ShowLoading { title: String },
    HideLoading,
    ShowModal { title: String, content: String, show_cancel: bool, cancel_text: String, confirm_text: String },
    HideModal,
}

/// 小程序应用
pub struct MiniApp {
    runtime: Arc<Mutex<JsRuntime>>,
    bridge: Arc<JsBridge>,
    api: MiniAppApi,
    canvas: Canvas,
    width: u32,
    height: u32,
    running: bool,
    last_frame: Instant,
    timers: HashMap<u32, TimerState>,
    ui_events: Vec<UiEvent>,
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
            canvas: Canvas::new(width, height),
            width,
            height,
            running: false,
            last_frame: Instant::now(),
            timers: HashMap::new(),
            ui_events: Vec::new(),
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
        
        // 泵出微任务队列（Promise / async 回调），确保异步链得以推进
        self.pump_jobs();
        
        Ok(())
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
                BridgeEvent::ShowToast { title, icon, duration, .. } => {
                    println!("[Toast] {} ({})", title, icon);
                    self.ui_events.push(UiEvent::ShowToast { title, icon, duration });
                }
                BridgeEvent::HideToast => {
                    self.ui_events.push(UiEvent::HideToast);
                }
                BridgeEvent::ShowLoading { title, .. } => {
                    println!("[Loading] {}", title);
                    self.ui_events.push(UiEvent::ShowLoading { title });
                }
                BridgeEvent::HideLoading => {
                    println!("[HideLoading]");
                    self.ui_events.push(UiEvent::HideLoading);
                }
                BridgeEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text } => {
                    println!("[Modal] {}: {}", title, content);
                    self.ui_events.push(UiEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text });
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
    
    /// 渲染
    pub fn render(&mut self) {
        // 清空画布
        self.canvas.clear(Color::WHITE);
        
        // 渲染组件树
        let tree = self.bridge.component_tree();
        let tree = tree.lock().unwrap();
        tree.render(&mut self.canvas);
    }
    
    /// 处理触摸事件
    pub fn on_touch(&mut self, x: f32, y: f32, touch_type: &str) -> Result<(), String> {
        let touch = Touch::new(0, x, y);
        let touch_event = TouchEvent {
            touches: vec![touch.clone()],
            changed_touches: vec![touch],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        
        let event = match touch_type {
            "start" => Event::TouchStart(touch_event),
            "move" => Event::TouchMove(touch_event),
            "end" => Event::TouchEnd(touch_event),
            "cancel" => Event::TouchCancel(touch_event),
            _ => return Ok(()),
        };
        
        // 分发给组件树
        {
            let tree = self.bridge.component_tree();
            let mut tree = tree.lock().unwrap();
            tree.dispatch_event(&event);
        }
        
        // 分发给 JS
        self.bridge.dispatch_event(&event)?;
        
        Ok(())
    }
    
    /// 处理点击事件
    pub fn on_tap(&mut self, x: f32, y: f32) -> Result<(), String> {
        let tap = TapEvent {
            x,
            y,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        
        let event = Event::Tap(tap);
        
        // 分发给组件树
        {
            let tree = self.bridge.component_tree();
            let mut tree = tree.lock().unwrap();
            tree.dispatch_event(&event);
        }
        
        // 分发给 JS
        self.bridge.dispatch_event(&event)?;
        
        Ok(())
    }
    
    /// 获取画布像素数据
    pub fn pixels(&self) -> &Canvas {
        &self.canvas
    }
    
    /// 获取画布 RGBA 数据
    pub fn to_rgba(&self) -> Vec<u8> {
        self.canvas.to_rgba()
    }
    
    pub fn width(&self) -> u32 {
        self.width
    }
    
    pub fn height(&self) -> u32 {
        self.height
    }
    
    pub fn is_running(&self) -> bool {
        self.running
    }
    
    /// 执行 JS 代码
    pub fn eval(&self, code: &str) -> Result<String, String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(code)
    }
}
