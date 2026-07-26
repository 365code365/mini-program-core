//! JS 与 Native 桥接层

use super::JsRuntime;
use crate::ui::ComponentTree;
use crate::event::{Event, Touch};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// JS 桥接器
pub struct JsBridge {
    runtime: Arc<Mutex<JsRuntime>>,
    component_tree: Arc<Mutex<ComponentTree>>,
    storage: Arc<Mutex<HashMap<String, String>>>,
    event_queue: Arc<Mutex<Vec<BridgeEvent>>>,
    /// 逻辑层调用过 `setData`（宿主每帧取走一次，决定要不要重绘）
    data_dirty: Arc<std::sync::atomic::AtomicBool>,
}

/// 桥接事件
#[derive(Debug, Clone)]
pub enum BridgeEvent {
    ConsoleLog(String),
    ConsoleError(String),
    ConsoleWarn(String),
    ShowToast { title: String, icon: String, duration: u32, mask: bool },
    HideToast,
    ShowLoading { title: String, mask: bool },
    HideLoading,
    ShowModal { title: String, content: String, show_cancel: bool, cancel_text: String, confirm_text: String },
    NavigateTo(String),
    NavigateBack(u32),
    SetTimer { id: u32, delay: u32, repeat: bool },
    ClearTimer(u32),
    StartPullDownRefresh,
    StopPullDownRefresh,
    CanvasDraw { canvas_id: String, commands: String },
    StorageSet { key: String, value: String },
    StorageGet { key: String },
    StorageRemove { key: String },
    StorageClear,
}

impl JsBridge {
    pub fn new(runtime: Arc<Mutex<JsRuntime>>) -> Self {
        Self {
            runtime,
            component_tree: Arc::new(Mutex::new(ComponentTree::new())),
            storage: Arc::new(Mutex::new(HashMap::new())),
            event_queue: Arc::new(Mutex::new(Vec::new())),
            data_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
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
    
    fn register_storage_functions(&self) -> Result<(), String> {
        let storage = self.storage.clone();
        let rt = self.runtime.lock().unwrap();
        
        let s = storage.clone();
        rt.register_function("__native_storage_set", move |args| {
            if args.len() >= 2 {
                s.lock().unwrap().insert(args[0].clone(), args[1].clone());
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
                s.lock().unwrap().remove(key);
            }
            "undefined".to_string()
        })?;
        
        let s = storage.clone();
        rt.register_function("__native_storage_clear", move |_args| {
            s.lock().unwrap().clear();
            "undefined".to_string()
        })?;
        
        Ok(())
    }
    
    fn register_ui_functions(&self) -> Result<(), String> {
        let queue = self.event_queue.clone();
        let rt = self.runtime.lock().unwrap();
        
        // showToast
        let q = queue.clone();
        rt.register_function("__native_show_toast", move |args| {
            let title = args.get(0).cloned().unwrap_or_default();
            let icon = args.get(1).cloned().unwrap_or_else(|| "success".to_string());
            let duration: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1500);
            let mask = args.get(3).map(|s| s == "true").unwrap_or(false);
            q.lock().unwrap().push(BridgeEvent::ShowToast { title, icon, duration, mask });
            "undefined".to_string()
        })?;
        
        let q = queue.clone();
        rt.register_function("__native_hide_toast", move |_args| {
            q.lock().unwrap().push(BridgeEvent::HideToast);
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
        rt.register_function("__native_hide_loading", move |_args| {
            q.lock().unwrap().push(BridgeEvent::HideLoading);
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
        
        // showModal
        let q = queue.clone();
        rt.register_function("__native_show_modal", move |args| {
            let title = args.get(0).cloned().unwrap_or_default();
            let content = args.get(1).cloned().unwrap_or_default();
            let show_cancel = args.get(2).map(|s| s == "true").unwrap_or(true);
            let cancel_text = args.get(3).cloned().unwrap_or_else(|| "取消".to_string());
            let confirm_text = args.get(4).cloned().unwrap_or_else(|| "确定".to_string());
            q.lock().unwrap().push(BridgeEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text });
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
    
    /// 获取组件树
    pub fn component_tree(&self) -> Arc<Mutex<ComponentTree>> {
        self.component_tree.clone()
    }
    
    /// 获取存储
    pub fn storage(&self) -> Arc<Mutex<HashMap<String, String>>> {
        self.storage.clone()
    }
}
