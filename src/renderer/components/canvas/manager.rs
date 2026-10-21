//! 画布上下文管理器：canvas_id → 离屏位图与上下文，以及 JS 下来的命令回放
//!
//! `canvas` 模块的一片。从前是纯搬迁；之后加了 `type="2d"` 的追加式日志（`logs_2d`）
//! 与「画过了要重画页面」的标记（`take_canvas_dirty`）。
//!
//! 注意这里是**进程级**状态（`CANVAS_MANAGER`）：同进程跑两个小程序会互相看见对方的
//! 画布，测试之间要用 `clear()` 隔离。详见 doc/架构与实现原理.md。
use super::*;

/// Canvas 上下文管理器 - 全局管理所有 canvas 实例
pub struct CanvasContextManager {
    contexts: HashMap<String, Canvas2DContext>,
    /// 每个 canvas 最近一次收到的指令流。元素真实尺寸与设备像素比只有布局后才知道，
    /// 而 `ctx.draw()` 一般发生在 onLoad —— 后备缓冲重建后要靠它把内容重画一遍。
    last_commands: HashMap<String, String>,
    /// `type="2d"` 画布（键 `2d#<id>`）自上次重置以来的全部指令。2d 上下文是「画了就生效」的
    /// 追加语义（逻辑层每个微任务交一批），与旧接口「每次 draw 交一整份」不同 ——
    /// 后备缓冲重建时要重放**全部**，只放最后一批会丢掉前面画的。
    logs_2d: HashMap<String, Vec<serde_json::Value>>,
}

/// 2d 日志的上限（条）。动画每帧 clearRect 重画时日志只增不减；超过上限就不再记，
/// 代价是之后元素尺寸变了（后备缓冲重建）那一帧画布是空的，直到下一帧动画重画
const LOG_2D_CAP: usize = 50_000;

impl CanvasContextManager {
    pub fn new() -> Self {
        Self { contexts: HashMap::new(), last_commands: HashMap::new(), logs_2d: HashMap::new() }
    }

    /// 按元素的逻辑尺寸与设备像素比对齐后备缓冲；尺寸/DPR 变了就重建并重放指令。
    ///
    /// 没有这一步时上下文一律是写死的 400x300 逻辑缓冲，又被 1:1 拷进 2 倍分辨率的
    /// 页面画布 —— 内容只落在元素左上角的四分之一里（看起来就是「canvas 画的不居中」）。
    pub fn sync_backing(&mut self, canvas_id: &str, logical_w: u32, logical_h: u32, dpr: f32) {
        if logical_w == 0 || logical_h == 0 {
            return;
        }
        let needs_rebuild = match self.contexts.get(canvas_id) {
            Some(ctx) => {
                ctx.logical_size() != (logical_w, logical_h)
                    || (ctx.device_pixel_ratio() - dpr).abs() > 0.01
            }
            None => true,
        };
        if !needs_rebuild {
            return;
        }
        self.contexts.insert(
            canvas_id.to_string(),
            Canvas2DContext::new_with_dpr(canvas_id, logical_w, logical_h, dpr),
        );
        if let Some(log) = self.logs_2d.get(canvas_id) {
            if let Some(ctx) = self.contexts.get_mut(canvas_id) {
                for cmd in log {
                    super::commands::run_command(ctx, cmd);
                }
            }
        } else if let Some(commands) = self.last_commands.get(canvas_id).cloned() {
            self.execute_commands(canvas_id, &commands);
        }
    }

    /// `type="2d"` 的一批指令：追加执行并记进日志。`__reset`（改了 canvas.width / height、
    /// 或第一次拿到节点）= 清空画布与状态、日志从头记
    fn execute_2d(&mut self, key: &str, commands_json: &str) {
        let commands: Vec<serde_json::Value> = serde_json::from_str(commands_json).unwrap_or_default();
        for cmd in commands {
            if cmd.get("type").and_then(|v| v.as_str()) == Some("__reset") {
                let (w, h, dpr) = self
                    .contexts
                    .get(key)
                    .map(|c| (c.logical_size().0, c.logical_size().1, c.device_pixel_ratio()))
                    .unwrap_or((400, 300, 1.0));
                self.contexts.insert(key.to_string(), Canvas2DContext::new_with_dpr(key, w, h, dpr));
                self.logs_2d.insert(key.to_string(), Vec::new());
                continue;
            }
            let ctx = self.contexts.entry(key.to_string()).or_insert_with(|| Canvas2DContext::new(key, 400, 300));
            super::commands::run_command(ctx, &cmd);
            let log = self.logs_2d.entry(key.to_string()).or_default();
            if log.len() < LOG_2D_CAP {
                log.push(cmd);
            }
        }
    }
    
    /// 获取或创建 canvas 上下文
    pub fn get_context(&mut self, canvas_id: &str, width: u32, height: u32) -> &mut Canvas2DContext {
        self.contexts.entry(canvas_id.to_string())
            .or_insert_with(|| Canvas2DContext::new(canvas_id, width, height))
    }
    
    /// 获取已存在的上下文
    pub fn get_existing_context(&self, canvas_id: &str) -> Option<&Canvas2DContext> {
        self.contexts.get(canvas_id)
    }
    
    /// 移除上下文
    pub fn remove_context(&mut self, canvas_id: &str) {
        self.contexts.remove(canvas_id);
    }
    
    /// 清除所有上下文
    pub fn clear(&mut self) {
        self.contexts.clear();
        self.logs_2d.clear();
    }
    
    /// 执行绘制命令
    pub fn execute_commands(&mut self, canvas_id: &str, commands_json: &str) {
        if canvas_id.starts_with("2d#") {
            return self.execute_2d(canvas_id, commands_json);
        }
        // 记住指令流：后备缓冲按元素真实尺寸重建后要重放一遍（见 `sync_backing`）
        if self.last_commands.get(canvas_id).map(|s| s.as_str()) != Some(commands_json) {
            self.last_commands.insert(canvas_id.to_string(), commands_json.to_string());
        }
        // 解析命令
        let commands: Vec<serde_json::Value> = serde_json::from_str(commands_json).unwrap_or_default();
        
        // 获取或创建上下文（元素尺寸未知时先给个较大的默认值，随后由 sync_backing 校正）
        let ctx = self.contexts.entry(canvas_id.to_string())
            .or_insert_with(|| Canvas2DContext::new(canvas_id, 400, 300));
        
        // 执行每个命令（解释器在 commands.rs）
        for cmd in commands {
            super::commands::run_command(ctx, &cmd);
        }
    }
}

impl Default for CanvasContextManager {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Canvas 组件实现 ==========

use once_cell::sync::Lazy;

/// 全局 Canvas 上下文管理器
pub static CANVAS_MANAGER: Lazy<Mutex<CanvasContextManager>> = Lazy::new(|| {
    Mutex::new(CanvasContextManager::new())
});

/// 预创建指定尺寸的 canvas 上下文（用于静态渲染场景先建好画布再绘制）。
pub fn ensure_canvas_context(canvas_id: &str, width: u32, height: u32) {
    if let Ok(mut manager) = CANVAS_MANAGER.lock() {
        manager.get_context(canvas_id, width, height);
    }
}

/// 执行 Canvas 绘制命令（供外部调用）
pub fn execute_canvas_draw(canvas_id: &str, commands_json: &str) {
    // 2d 画布的动画每帧都来一批，默认不打（`MINI_CANVAS_LOG=1` 打开）
    if std::env::var("MINI_CANVAS_LOG").is_ok() {
        println!("[Canvas] execute_canvas_draw: {} commands for '{}'", commands_json.len(), canvas_id);
    }
    if let Ok(mut manager) = CANVAS_MANAGER.lock() {
        manager.execute_commands(canvas_id, commands_json);
    }
    CANVAS_DIRTY.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// 有画布被画过（宿主据此重画页面：canvas 的像素是在页面绘制时拷进去的）
static CANVAS_DIRTY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 取走「画布被画过」标记（读后清零）
pub fn take_canvas_dirty() -> bool {
    CANVAS_DIRTY.swap(false, std::sync::atomic::Ordering::Relaxed)
}
