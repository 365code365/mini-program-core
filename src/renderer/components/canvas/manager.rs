//! 画布上下文管理器：canvas_id → 离屏位图与上下文，以及 JS 下来的命令回放
//!
//! `canvas` 模块的一片。**纯搬迁**，一行逻辑没改。
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
}

impl CanvasContextManager {
    pub fn new() -> Self {
        Self { contexts: HashMap::new(), last_commands: HashMap::new() }
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
        if let Some(commands) = self.last_commands.get(canvas_id).cloned() {
            self.execute_commands(canvas_id, &commands);
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
    }
    
    /// 执行绘制命令
    pub fn execute_commands(&mut self, canvas_id: &str, commands_json: &str) {
        // 记住指令流：后备缓冲按元素真实尺寸重建后要重放一遍（见 `sync_backing`）
        if self.last_commands.get(canvas_id).map(|s| s.as_str()) != Some(commands_json) {
            self.last_commands.insert(canvas_id.to_string(), commands_json.to_string());
        }
        // 解析命令
        let commands: Vec<serde_json::Value> = serde_json::from_str(commands_json).unwrap_or_default();
        
        // 获取或创建上下文（元素尺寸未知时先给个较大的默认值，随后由 sync_backing 校正）
        let ctx = self.contexts.entry(canvas_id.to_string())
            .or_insert_with(|| Canvas2DContext::new(canvas_id, 400, 300));
        
        // 执行每个命令
        for cmd in commands {
            let cmd_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match cmd_type {
                "setFillStyle" => {
                    if let Some(color) = cmd.get("color").and_then(|v| v.as_str()) {
                        ctx.set_fill_style(color);
                    }
                }
                "setStrokeStyle" => {
                    if let Some(color) = cmd.get("color").and_then(|v| v.as_str()) {
                        ctx.set_stroke_style(color);
                    }
                }
                "setLineWidth" => {
                    if let Some(width) = cmd.get("width").and_then(|v| v.as_f64()) {
                        ctx.set_line_width(width as f32);
                    }
                }
                "fillRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.fill_rect(x, y, w, h);
                }
                "strokeRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.stroke_rect(x, y, w, h);
                }
                "clearRect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.clear_rect(x, y, w, h);
                }
                "beginPath" => ctx.begin_path(),
                "closePath" => ctx.close_path(),
                "moveTo" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.move_to(x, y);
                }
                "lineTo" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.line_to(x, y);
                }
                "arc" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let r = cmd.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let s = cmd.get("sAngle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let e = cmd.get("eAngle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let cc = cmd.get("counterclockwise").and_then(|v| v.as_bool()).unwrap_or(false);
                    ctx.arc(x, y, r, s, e, cc);
                }
                "fill" => ctx.fill(),
                "stroke" => ctx.stroke(),
                "save" => ctx.save(),
                "restore" => ctx.restore(),
                "translate" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.translate(x, y);
                }
                "rotate" => {
                    let a = cmd.get("angle").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.rotate(a);
                }
                "scale" => {
                    let sx = cmd.get("scaleX").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    let sy = cmd.get("scaleY").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    ctx.scale(sx, sy);
                }
                "setFontSize" => {
                    if let Some(s) = cmd.get("size").and_then(|v| v.as_f64()) { ctx.font_size = s as f32; }
                }
                "setTextAlign" => {
                    if let Some(a) = cmd.get("align").and_then(|v| v.as_str()) { ctx.set_text_align(a); }
                }
                "setTextBaseline" => {
                    if let Some(b) = cmd.get("baseline").and_then(|v| v.as_str()) { ctx.set_text_baseline(b); }
                }
                "setGlobalAlpha" => {
                    if let Some(a) = cmd.get("alpha").and_then(|v| v.as_f64()) { ctx.set_global_alpha(a as f32); }
                }
                "setLineCap" => {
                    if let Some(c) = cmd.get("cap").and_then(|v| v.as_str()) { ctx.set_line_cap(c); }
                }
                "setLineJoin" => {
                    if let Some(j) = cmd.get("join").and_then(|v| v.as_str()) { ctx.set_line_join(j); }
                }
                "rect" => {
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = cmd.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = cmd.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.rect(x, y, w, h);
                }
                "quadraticCurveTo" => {
                    let cpx = cmd.get("cpx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let cpy = cmd.get("cpy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.quadratic_curve_to(cpx, cpy, x, y);
                }
                "bezierCurveTo" => {
                    let a = |k: &str| cmd.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.bezier_curve_to(a("cp1x"), a("cp1y"), a("cp2x"), a("cp2y"), a("x"), a("y"));
                }
                "fillText" => {
                    let text = cmd.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.fill_text(text, x, y);
                }
                "strokeText" => {
                    let text = cmd.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    ctx.stroke_text(text, x, y);
                }
                "drawImage" => {
                    let src = cmd.get("src").and_then(|v| v.as_str()).unwrap_or("");
                    let dx = cmd.get("dx").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let dy = cmd.get("dy").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let dw = cmd.get("dWidth").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
                    let dh = cmd.get("dHeight").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
                    ctx.draw_image(src, dx, dy, dw, dh);
                }
                _ => {}
            }
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
    println!("[Canvas] execute_canvas_draw: {} commands for '{}'", 
        commands_json.len(), canvas_id);
    if let Ok(mut manager) = CANVAS_MANAGER.lock() {
        manager.execute_commands(canvas_id, commands_json);
    }
}
