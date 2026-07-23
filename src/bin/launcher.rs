//! 小程序启动器 - 从 sample 目录加载并运行小程序
//! 
//! 功能：
//! 1. 扫描 sample 目录下的所有小程序
//! 2. 显示小程序列表供用户选择
//! 3. 点击后加载并运行选中的小程序

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, ElementState, MouseButton};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use softbuffer::Surface;

use mini_render::{Canvas, Color, Paint, PaintStyle, Rect};
use mini_render::text::TextRenderer;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::parser::wxml::WxmlNode;
use mini_render::parser::wxss::StyleSheet;
use mini_render::runtime::MiniApp;
use mini_render::ui::ScrollController;

const WINDOW_WIDTH: u32 = 375;
const WINDOW_HEIGHT: u32 = 667;

/// 小程序信息
#[derive(Clone, Debug)]
struct MiniAppInfo {
    name: String,
    path: PathBuf,
    description: String,
}

/// 页面信息
struct PageInfo {
    path: String,
    wxml: String,
    wxss: String,
    js: String,
}

/// 自定义 TabBar
struct CustomTabBar {
    wxml_nodes: Vec<WxmlNode>,
    stylesheet: StyleSheet,
}

/// 启动器状态
enum LauncherState {
    /// 显示小程序列表
    List,
    /// 运行小程序
    Running(RunningApp),
}

/// 运行中的小程序
struct RunningApp {
    #[allow(dead_code)]
    app_path: PathBuf,
    pages: HashMap<String, PageInfo>,
    current_page: String,
    wxml_nodes: Vec<WxmlNode>,
    stylesheet: StyleSheet,
    mini_app: MiniApp,
    page_data: serde_json::Value,
    renderer: mini_render::renderer::WxmlRenderer,
    interaction: mini_render::ui::interaction::InteractionManager,
    scroll: ScrollController,
    #[allow(dead_code)]
    custom_tabbar: Option<CustomTabBar>,
}

/// 应用程序
struct LauncherApp {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    canvas: Canvas,
    text_renderer: Option<TextRenderer>,
    mini_apps: Vec<MiniAppInfo>,
    state: LauncherState,
    scale_factor: f32,
    mouse_pos: (f32, f32),
    list_scroll: ScrollController,
    last_frame: Instant,
    click_start_pos: (f32, f32),
    click_start_time: Instant,
    frame_interval: std::time::Duration,
    last_present: Instant,
}

impl LauncherApp {
    /// 是否需要连续出帧（列表惯性滚动 / 运行中的小程序有滚动或定时器）
    fn is_active(&self) -> bool {
        match &self.state {
            LauncherState::List => self.list_scroll.is_animating() || self.list_scroll.is_dragging,
            LauncherState::Running(app) => {
                app.scroll.is_animating() || app.scroll.is_dragging || app.mini_app.has_active_timers()
            }
        }
    }
    fn new() -> Self {
        let canvas = Canvas::new(WINDOW_WIDTH * 2, WINDOW_HEIGHT * 2);
        let text_renderer = TextRenderer::load_system_font()
            .or_else(|_| TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf")))
            .ok();
        
        // 扫描 sample 目录
        let mini_apps = scan_sample_directory();
        
        // 计算列表内容高度
        let list_content_height = 88.0 + 16.0 + mini_apps.len() as f32 * 96.0;
        let now = Instant::now();
        
        Self {
            window: None,
            surface: None,
            canvas,
            text_renderer,
            mini_apps,
            state: LauncherState::List,
            scale_factor: 2.0,
            mouse_pos: (0.0, 0.0),
            list_scroll: ScrollController::new(list_content_height, WINDOW_HEIGHT as f32),
            last_frame: now,
            click_start_pos: (0.0, 0.0),
            click_start_time: now,
            frame_interval: std::time::Duration::from_micros(16_667),
            last_present: now,
        }
    }
    
    /// 渲染小程序列表
    fn render_list(&mut self) {
        self.canvas.clear(Color::from_hex(0xF5F5F5));
        
        let sf = self.scale_factor;
        let scroll_offset = self.list_scroll.get_position();
        
        // 标题栏
        let header_height = 88.0 * sf;
        let header_paint = Paint::new().with_color(Color::from_hex(0xFF6B35));
        self.canvas.draw_rect(&Rect::new(0.0, 0.0, WINDOW_WIDTH as f32 * sf, header_height), &header_paint);
        
        if let Some(tr) = &self.text_renderer {
            let title_paint = Paint::new().with_color(Color::WHITE);
            tr.draw_text(&mut self.canvas, "小程序启动器", 20.0 * sf, 55.0 * sf, 18.0 * sf, &title_paint);
        }
        
        // 小程序列表
        let item_height = 80.0 * sf;
        let padding = 16.0 * sf;
        let start_y = header_height + padding - scroll_offset * sf;
        
        for (i, app) in self.mini_apps.iter().enumerate() {
            let y = start_y + i as f32 * (item_height + padding);
            
            // 跳过不可见的项
            if y + item_height < header_height || y > WINDOW_HEIGHT as f32 * sf {
                continue;
            }
            
            // 卡片背景
            let card_paint = Paint::new().with_color(Color::WHITE);
            self.canvas.draw_rect(&Rect::new(padding, y, (WINDOW_WIDTH as f32 - 32.0) * sf, item_height), &card_paint);
            
            // 图标占位
            let icon_size = 48.0 * sf;
            let icon_x = padding + 16.0 * sf;
            let icon_y = y + (item_height - icon_size) / 2.0;
            let icon_paint = Paint::new().with_color(Color::from_hex(0xFF6B35));
            self.canvas.draw_rect(&Rect::new(icon_x, icon_y, icon_size, icon_size), &icon_paint);
            
            // 应用名称和描述
            if let Some(tr) = &self.text_renderer {
                let text_x = icon_x + icon_size + 16.0 * sf;
                let name_paint = Paint::new().with_color(Color::from_hex(0x333333));
                tr.draw_text(&mut self.canvas, &app.name, text_x, y + 30.0 * sf, 16.0 * sf, &name_paint);
                
                let desc_paint = Paint::new().with_color(Color::from_hex(0x999999));
                tr.draw_text(&mut self.canvas, &app.description, text_x, y + 55.0 * sf, 12.0 * sf, &desc_paint);
            }
            
            // 启动按钮
            let btn_width = 60.0 * sf;
            let btn_height = 32.0 * sf;
            let btn_x = (WINDOW_WIDTH as f32 - 32.0 - 16.0) * sf - btn_width;
            let btn_y = y + (item_height - btn_height) / 2.0;
            let btn_paint = Paint::new().with_color(Color::from_hex(0x07C160));
            self.canvas.draw_rect(&Rect::new(btn_x, btn_y, btn_width, btn_height), &btn_paint);
            
            if let Some(tr) = &self.text_renderer {
                let btn_text_paint = Paint::new().with_color(Color::WHITE);
                tr.draw_text(&mut self.canvas, "启动", btn_x + 12.0 * sf, btn_y + 22.0 * sf, 14.0 * sf, &btn_text_paint);
            }
        }
        
        // 底部提示
        if self.mini_apps.is_empty() {
            if let Some(tr) = &self.text_renderer {
                let hint_paint = Paint::new().with_color(Color::from_hex(0x999999));
                tr.draw_text(&mut self.canvas, "sample 目录下没有找到小程序", 
                    60.0 * sf, 300.0 * sf, 14.0 * sf, &hint_paint);
                tr.draw_text(&mut self.canvas, "请在 sample 目录下创建小程序项目", 
                    50.0 * sf, 330.0 * sf, 14.0 * sf, &hint_paint);
            }
        }
    }
    
    /// 处理列表点击
    fn handle_list_click(&mut self, x: f32, y: f32) -> bool {
        let sf = self.scale_factor;
        let header_height = 88.0;
        let item_height = 80.0;
        let padding = 16.0;
        let scroll_offset = self.list_scroll.get_position();
        
        // 转换为逻辑坐标
        let lx = x / sf;
        let ly = y / sf;
        
        if ly < header_height {
            return false;
        }
        
        let start_y = header_height + padding - scroll_offset;
        
        for (i, app) in self.mini_apps.iter().enumerate() {
            let item_y = start_y + i as f32 * (item_height + padding);
            
            if ly >= item_y && ly < item_y + item_height {
                // 检查是否点击了启动按钮
                let btn_width = 60.0;
                let btn_x = WINDOW_WIDTH as f32 - 32.0 - 16.0 - btn_width;
                
                if lx >= btn_x && lx < btn_x + btn_width {
                    println!("🚀 启动小程序: {}", app.name);
                    self.launch_mini_app(app.path.clone());
                    return true;
                }
            }
        }
        
        false
    }
    
    /// 启动小程序
    fn launch_mini_app(&mut self, app_path: PathBuf) {
        println!("📂 加载小程序: {:?}", app_path);
        
        match load_mini_app(&app_path, self.scale_factor) {
            Ok(running_app) => {
                self.state = LauncherState::Running(running_app);
            }
            Err(e) => {
                eprintln!("❌ 加载小程序失败: {}", e);
            }
        }
    }
    
    /// 返回列表
    fn back_to_list(&mut self) {
        println!("🔙 返回小程序列表");
        self.state = LauncherState::List;
        // 重新扫描目录
        self.mini_apps = scan_sample_directory();
        // 重置列表滚动
        let list_content_height = 88.0 + 16.0 + self.mini_apps.len() as f32 * 96.0;
        self.list_scroll = ScrollController::new(list_content_height, WINDOW_HEIGHT as f32);
    }
    
    /// 渲染运行中的小程序
    fn render_running_app(&mut self) {
        if let LauncherState::Running(ref mut app) = self.state {
            let scroll_offset = app.scroll.get_position();
            
            // 渲染页面内容
            let content_height = app.renderer.render_with_scroll_and_viewport(
                &mut self.canvas,
                &app.wxml_nodes,
                &app.page_data,
                &mut app.interaction,
                scroll_offset,
                WINDOW_HEIGHT as f32,
            );
            
            // 更新滚动控制器的内容高度
            app.scroll.update_content_height(content_height, WINDOW_HEIGHT as f32);
            
            // 渲染返回按钮
            self.render_back_button();
        }
    }
    
    /// 渲染返回按钮
    fn render_back_button(&mut self) {
        let sf = self.scale_factor;
        let btn_size = 36.0 * sf;
        let btn_x = 10.0 * sf;
        let btn_y = 40.0 * sf;
        
        // 半透明背景
        let bg_paint = Paint::new().with_color(Color::new(0, 0, 0, 180));
        self.canvas.draw_circle(btn_x + btn_size / 2.0, btn_y + btn_size / 2.0, btn_size / 2.0, &bg_paint);
        
        // 返回箭头
        let arrow_paint = Paint::new()
            .with_color(Color::WHITE)
            .with_style(PaintStyle::Stroke)
            .with_stroke_width(2.0 * sf);
        let cx = btn_x + btn_size / 2.0;
        let cy = btn_y + btn_size / 2.0;
        let arrow_size = 10.0 * sf;
        self.canvas.draw_line(cx + arrow_size / 3.0, cy - arrow_size / 2.0, cx - arrow_size / 3.0, cy, &arrow_paint);
        self.canvas.draw_line(cx - arrow_size / 3.0, cy, cx + arrow_size / 3.0, cy + arrow_size / 2.0, &arrow_paint);
    }
    
    /// 检查是否点击了返回按钮
    fn check_back_button_click(&self, x: f32, y: f32) -> bool {
        let sf = self.scale_factor;
        let btn_size = 36.0 * sf;
        let btn_x = 10.0 * sf;
        let btn_y = 40.0 * sf;
        let cx = btn_x + btn_size / 2.0;
        let cy = btn_y + btn_size / 2.0;
        
        let dx = x - cx;
        let dy = y - cy;
        (dx * dx + dy * dy).sqrt() <= btn_size / 2.0
    }
}

impl ApplicationHandler for LauncherApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        
        let window_attrs = Window::default_attributes()
            .with_title("Mini Program Launcher")
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_resizable(false);
        
        let window = Rc::new(event_loop.create_window(window_attrs).unwrap());
        // 按显示器刷新率设定帧间隔（支持高刷屏）
        let millihertz = window.current_monitor()
            .and_then(|m| m.refresh_rate_millihertz())
            .filter(|&hz| hz > 0)
            .unwrap_or(60_000);
        self.frame_interval = std::time::Duration::from_secs_f64(1000.0 / millihertz as f64);
        
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();
        
        self.window = Some(window);
        self.surface = Some(surface);
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            
            WindowEvent::RedrawRequested => {
                // 更新滚动动画
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;
                
                let mut needs_redraw = false;
                
                match &mut self.state {
                    LauncherState::List => {
                        if self.list_scroll.update(dt) {
                            needs_redraw = true;
                        }
                        self.render_list();
                    }
                    LauncherState::Running(app) => {
                        if app.scroll.update(dt) {
                            needs_redraw = true;
                        }
                        // 更新 MiniApp
                        app.mini_app.update().ok();
                    }
                }
                
                match &self.state {
                    LauncherState::List => self.render_list(),
                    LauncherState::Running(_) => self.render_running_app(),
                }
                
                // 输出到窗口
                if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
                    let size = window.inner_size();
                    
                    surface.resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    ).unwrap();
                    
                    let mut buffer = surface.buffer_mut().unwrap();
                    
                    // 获取滚动偏移
                    let scroll_offset = match &self.state {
                        LauncherState::List => 0.0, // 列表滚动已在渲染时处理
                        LauncherState::Running(app) => app.scroll.get_position() * self.scale_factor,
                    };
                    
                    // 像素复制（带滚动偏移）
                    let pixels = self.canvas.pixels();
                    let canvas_width = self.canvas.width() as usize;
                    let canvas_height = self.canvas.height() as usize;
                    
                    for y in 0..size.height.min(canvas_height as u32) {
                        for x in 0..size.width.min(canvas_width as u32) {
                            let src_y = match &self.state {
                                LauncherState::List => y as usize,
                                LauncherState::Running(_) => (y as f32 + scroll_offset).min(canvas_height as f32 - 1.0).max(0.0) as usize,
                            };
                            let src_idx = src_y * canvas_width + x as usize;
                            let dst_idx = y as usize * size.width as usize + x as usize;
                            if src_idx < pixels.len() && dst_idx < buffer.len() {
                                let color = &pixels[src_idx];
                                buffer[dst_idx] = ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32);
                            }
                        }
                    }
                    
                    buffer.present().unwrap();
                }
                
                self.last_present = Instant::now();
                let _ = needs_redraw; // 后续帧调度交给 about_to_wait（按刷新率节流）
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                self.mouse_pos = (x, y);
                
                // 更新拖拽滚动
                let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                let logical_y = y / self.scale_factor;
                
                match &mut self.state {
                    LauncherState::List => {
                        if self.list_scroll.is_dragging {
                            self.list_scroll.update_drag(logical_y, ts);
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                    LauncherState::Running(app) => {
                        if app.scroll.is_dragging {
                            app.scroll.update_drag(logical_y, ts);
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                }
            }
            
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                self.click_start_pos = self.mouse_pos;
                self.click_start_time = Instant::now();
                
                let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                let (_, y) = self.mouse_pos;
                let logical_y = y / self.scale_factor;
                
                match &mut self.state {
                    LauncherState::List => {
                        self.list_scroll.begin_drag(logical_y, ts);
                    }
                    LauncherState::Running(app) => {
                        app.scroll.begin_drag(logical_y, ts);
                    }
                }
            }
            
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {
                let (x, y) = self.mouse_pos;
                let (start_x, start_y) = self.click_start_pos;
                let dx = (x - start_x).abs();
                let dy = (y - start_y).abs();
                let is_click = dx < 10.0 && dy < 10.0 && self.click_start_time.elapsed().as_millis() < 300;
                
                // 先处理滚动结束和收集需要的信息
                let mut should_back = false;
                let mut click_info: Option<(f32, f32)> = None;
                
                match &mut self.state {
                    LauncherState::List => {
                        self.list_scroll.end_drag();
                    }
                    LauncherState::Running(app) => {
                        app.scroll.end_drag();
                        if is_click {
                            let sf = self.scale_factor;
                            let scroll_pos = app.scroll.get_position();
                            click_info = Some((x / sf, y / sf + scroll_pos));
                        }
                    }
                }
                
                // 检查返回按钮
                if is_click {
                    if let LauncherState::Running(_) = &self.state {
                        if self.check_back_button_click(x, y) {
                            should_back = true;
                        }
                    }
                }
                
                // 处理返回
                if should_back {
                    self.back_to_list();
                } else if is_click {
                    match &mut self.state {
                        LauncherState::List => {
                            self.handle_list_click(x, y);
                        }
                        LauncherState::Running(app) => {
                            if let Some((logical_x, logical_y)) = click_info {
                                // 检查事件绑定
                                if let Some(binding) = app.renderer.hit_test(logical_x, logical_y) {
                                    println!("👆 点击事件: {} -> {}", binding.event_type, binding.handler);
                                }
                            }
                        }
                    }
                }
                
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => -y * 30.0,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
                };
                
                let is_precise = matches!(delta, winit::event::MouseScrollDelta::PixelDelta(_));
                
                match &mut self.state {
                    LauncherState::List => {
                        self.list_scroll.handle_scroll(scroll_delta, is_precise);
                    }
                    LauncherState::Running(app) => {
                        app.scroll.handle_scroll(scroll_delta, is_precise);
                    }
                }
                
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            
            _ => {}
        }
    }

    /// WaitUntil 到期唤醒 -> 触发下一帧
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if let winit::event::StartCause::ResumeTimeReached { .. } = cause {
            if let Some(window) = &self.window { window.request_redraw(); }
        }
    }

    /// 有动画则按刷新率安排下一帧，空闲则休眠（0 CPU）
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.is_active() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.last_present + self.frame_interval));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

/// 扫描 sample 目录下的小程序
fn scan_sample_directory() -> Vec<MiniAppInfo> {
    let mut apps = Vec::new();
    
    // 获取可执行文件所在目录
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    
    // 尝试多个可能的 sample 目录位置
    let possible_paths = vec![
        PathBuf::from("sample"),
        PathBuf::from("./sample"),
        exe_dir.join("../../../sample"),
        exe_dir.join("../../sample"),
        exe_dir.join("sample"),
    ];
    
    for sample_dir in possible_paths {
        if sample_dir.exists() && sample_dir.is_dir() {
            println!("📁 扫描目录: {:?}", sample_dir);
            
            if let Ok(entries) = fs::read_dir(&sample_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let app_json = path.join("app.json");
                        if app_json.exists() {
                            if let Some(info) = parse_app_info(&path) {
                                println!("  ✅ 发现小程序: {}", info.name);
                                apps.push(info);
                            }
                        }
                    }
                }
            }
            
            if !apps.is_empty() {
                break;
            }
        }
    }
    
    if apps.is_empty() {
        println!("⚠️ 未找到任何小程序，请确保 sample 目录存在");
    }
    
    apps
}

/// 解析小程序信息
fn parse_app_info(path: &Path) -> Option<MiniAppInfo> {
    let app_json_path = path.join("app.json");
    let content = fs::read_to_string(&app_json_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    
    let name = json.get("window")
        .and_then(|w| w.get("navigationBarTitleText"))
        .and_then(|t| t.as_str())
        .unwrap_or_else(|| path.file_name().unwrap().to_str().unwrap())
        .to_string();
    
    let pages_count = json.get("pages")
        .and_then(|p| p.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    
    Some(MiniAppInfo {
        name,
        path: path.to_path_buf(),
        description: format!("{} 个页面", pages_count),
    })
}

/// 加载小程序
fn load_mini_app(app_path: &Path, scale_factor: f32) -> Result<RunningApp, String> {
    // 读取 app.json
    let app_json_path = app_path.join("app.json");
    let app_json_content = fs::read_to_string(&app_json_path)
        .map_err(|e| format!("读取 app.json 失败: {}", e))?;
    let app_json: serde_json::Value = serde_json::from_str(&app_json_content)
        .map_err(|e| format!("解析 app.json 失败: {}", e))?;
    
    // 获取页面列表
    let pages = app_json.get("pages")
        .and_then(|p| p.as_array())
        .ok_or("app.json 中没有 pages 字段")?;
    
    // 加载所有页面
    let mut page_map = HashMap::new();
    for page_path in pages {
        if let Some(page_str) = page_path.as_str() {
            let page_dir = app_path.join(page_str);
            
            let wxml_path = page_dir.with_extension("wxml");
            let wxss_path = page_dir.with_extension("wxss");
            let js_path = page_dir.with_extension("js");
            
            let wxml = fs::read_to_string(&wxml_path).unwrap_or_default();
            let wxss = fs::read_to_string(&wxss_path).unwrap_or_default();
            let js = fs::read_to_string(&js_path).unwrap_or_default();
            
            page_map.insert(page_str.to_string(), PageInfo {
                path: page_str.to_string(),
                wxml,
                wxss,
                js,
            });
        }
    }
    
    // 获取首页
    let first_page = pages.first()
        .and_then(|p| p.as_str())
        .ok_or("没有找到首页")?
        .to_string();
    
    let page_info = page_map.get(&first_page)
        .ok_or("首页不存在")?;
    
    // 解析 WXML
    let mut wxml_parser = WxmlParser::new(&page_info.wxml);
    let wxml_nodes = wxml_parser.parse()
        .map_err(|e| format!("解析 WXML 失败: {}", e))?;
    
    // 解析 WXSS
    let mut wxss_parser = WxssParser::new(&page_info.wxss);
    let stylesheet = wxss_parser.parse()
        .map_err(|e| format!("解析 WXSS 失败: {}", e))?;
    
    // 创建渲染器
    let renderer = mini_render::renderer::WxmlRenderer::new_with_scale(
        stylesheet.clone(),
        WINDOW_WIDTH as f32,
        WINDOW_HEIGHT as f32,
        scale_factor,
    );
    
    // 创建 MiniApp
    let mut mini_app = MiniApp::new(WINDOW_WIDTH, WINDOW_HEIGHT)
        .map_err(|e| format!("创建 MiniApp 失败: {}", e))?;
    mini_app.init()
        .map_err(|e| format!("初始化 MiniApp 失败: {}", e))?;
    
    // 读取 app.js
    let app_js_path = app_path.join("app.js");
    let app_js = fs::read_to_string(&app_js_path).unwrap_or_default();
    let _ = mini_app.load_script(&app_js);
    
    // 执行页面 JS
    let _ = mini_app.load_script(&page_info.js);
    
    // 获取页面数据
    let page_data = mini_app.eval("__getPageData()")
        .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::json!({})))
        .unwrap_or(serde_json::json!({}));
    
    // 创建交互管理器
    let interaction = mini_render::ui::interaction::InteractionManager::new();
    
    // 加载自定义 TabBar
    let custom_tabbar = load_custom_tabbar_from_path(app_path);
    
    Ok(RunningApp {
        app_path: app_path.to_path_buf(),
        pages: page_map,
        current_page: first_page,
        wxml_nodes,
        stylesheet,
        mini_app,
        page_data,
        renderer,
        interaction,
        scroll: ScrollController::new(WINDOW_HEIGHT as f32, WINDOW_HEIGHT as f32),
        custom_tabbar,
    })
}

/// 从路径加载自定义 TabBar
fn load_custom_tabbar_from_path(app_path: &Path) -> Option<CustomTabBar> {
    let tabbar_dir = app_path.join("custom-tab-bar");
    if !tabbar_dir.exists() {
        return None;
    }
    
    let wxml_path = tabbar_dir.join("index.wxml");
    let wxss_path = tabbar_dir.join("index.wxss");
    
    let wxml = fs::read_to_string(&wxml_path).ok()?;
    let wxss = fs::read_to_string(&wxss_path).ok()?;
    
    let mut wxml_parser = WxmlParser::new(&wxml);
    let wxml_nodes = wxml_parser.parse().ok()?;
    
    let mut wxss_parser = WxssParser::new(&wxss);
    let stylesheet = wxss_parser.parse().ok()?;
    
    Some(CustomTabBar {
        wxml_nodes,
        stylesheet,
    })
}

fn main() {
    println!("🚀 Mini Program Launcher");
    println!("========================");
    println!("扫描 sample 目录下的小程序...\n");
    
    let event_loop = EventLoop::new().unwrap();
    // 默认空闲休眠；动画期间由 about_to_wait 按刷新率切换到 WaitUntil。
    event_loop.set_control_flow(ControlFlow::Wait);
    
    let mut app = LauncherApp::new();
    event_loop.run_app(&mut app).unwrap();
}
