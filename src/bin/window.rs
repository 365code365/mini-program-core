//! 带窗口的小程序运行器 - 支持多页面导航和原生 TabBar

mod app_window;

use app_window::*;
use app_window::ui_overlay::{ToastState, LoadingState, ModalState, render_ui_overlay};
use app_window::event_handler as evt;
use app_window::click_handler as click;

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::WxmlRenderer;
use mini_render::ui::interaction::InteractionManager;
use mini_render::{Canvas, Color};
use mini_render::text::TextRenderer;
use serde_json::json;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
use mini_render::ui::ScrollController;

struct MiniAppWindow {
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    app: MiniApp,
    canvas: Option<Canvas>,
    tabbar_canvas: Option<Canvas>,
    fixed_canvas: Option<Canvas>,
    renderer: Option<WxmlRenderer>,
    tabbar_renderer: Option<WxmlRenderer>,
    text_renderer: Option<TextRenderer>,
    page_stack: Vec<PageInstance>,
    pages: HashMap<String, PageInfo>,
    app_config: AppConfig,
    custom_tabbar: Option<CustomTabBar>,
    mouse_pos: (f32, f32),
    needs_redraw: bool,
    scale_factor: f64,
    scroll: ScrollController,
    last_frame: Instant,
    click_start_pos: (f32, f32),
    click_start_time: Instant,
    pending_navigation: Option<NavigationRequest>,
    interaction: InteractionManager,
    modifiers: winit::keyboard::ModifiersState,
    clipboard: Option<arboard::Clipboard>,
    toast: Option<ToastState>,
    loading: Option<LoadingState>,
    modal: Option<ModalState>,
}

impl MiniAppWindow {
    fn new(app_path: Option<std::path::PathBuf>) -> Result<Self, String> {
        // 设置小程序路径
        if let Some(path) = app_path {
            page_loader::set_app_path(path);
        }
        
        let mut app = MiniApp::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)?;
        app.init()?;
        
        // 动态加载 app.js
        let app_js = page_loader::load_app_js();
        app.load_script(&app_js)?;
        println!("📱 App.js loaded");
        
        // 动态加载 app.json
        let app_json_str = page_loader::load_app_json();
        let app_config: AppConfig = serde_json::from_str(&app_json_str)
            .map_err(|e| format!("Failed to parse app.json: {}", e))?;
        
        let custom_tabbar = if app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) {
            load_custom_tabbar()?
        } else { None };
        
        let pages = load_all_pages();
        
        // 获取首页路径
        let first_page = app_config.pages.first()
            .cloned()
            .unwrap_or_else(|| "pages/index/index".to_string());
        
        let has_tabbar = app_config.tab_bar.as_ref()
            .map(|tb| tb.list.iter().any(|item| item.page_path == first_page))
            .unwrap_or(false);
        
        let now = Instant::now();
        let mut window = Self {
            window: None, surface: None, app, canvas: None, tabbar_canvas: None, fixed_canvas: None,
            renderer: None, tabbar_renderer: None, text_renderer: None,
            page_stack: Vec::new(), pages, app_config, custom_tabbar,
            mouse_pos: (0.0, 0.0), needs_redraw: true, scale_factor: 1.0,
            scroll: ScrollController::new(CONTENT_HEIGHT as f32, (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32),
            last_frame: now, click_start_pos: (0.0, 0.0), click_start_time: now,
            pending_navigation: None, interaction: InteractionManager::new(),
            modifiers: winit::keyboard::ModifiersState::empty(),
            clipboard: arboard::Clipboard::new().ok(),
            toast: None, loading: None, modal: None,
        };
        
        window.navigate_to(&first_page, HashMap::new())?;
        Ok(window)
    }
    
    fn is_tabbar_page(&self, path: &str) -> bool {
        self.app_config.tab_bar.as_ref().map(|tb| tb.list.iter().any(|item| item.page_path == path)).unwrap_or(false)
    }
    
    fn get_tabbar_index(&self, path: &str) -> Option<usize> {
        self.app_config.tab_bar.as_ref().and_then(|tb| tb.list.iter().position(|item| item.page_path == path))
    }
    
    fn is_custom_tabbar(&self) -> bool {
        self.app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) && self.custom_tabbar.is_some()
    }

    fn navigate_to(&mut self, path: &str, query: HashMap<String, String>) -> Result<(), String> {
        let path = path.trim_start_matches('/');
        let page_info = self.pages.get(path).ok_or_else(|| format!("Page not found: {}", path))?;
        
        let mut wxml_parser = WxmlParser::new(&page_info.wxml);
        let wxml_nodes = remove_manual_tabbar(&wxml_parser.parse().map_err(|e| format!("WXML error: {}", e))?);
        
        let mut wxss_parser = WxssParser::new(&page_info.wxss);
        let stylesheet = wxss_parser.parse().map_err(|e| format!("WXSS error: {}", e))?;
        
        self.app.load_script(&page_info.js)?;
        let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
        self.app.eval(&format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json)).ok();
        self.app.eval("if(__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
        print_js_output(&self.app);
        
        self.page_stack.push(PageInstance { path: path.to_string(), query, wxml_nodes, stylesheet });
        
        let has_tabbar = self.is_tabbar_page(path);
        self.scroll = ScrollController::new(CONTENT_HEIGHT as f32, (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32);
        self.needs_redraw = true;
        println!("✅ Page loaded: {}", path);
        Ok(())
    }
    
    fn navigate_back(&mut self) -> Result<(), String> {
        if self.page_stack.len() <= 1 { return Ok(()); }
        self.page_stack.pop();
        
        if let Some(page) = self.page_stack.last() {
            let (path, query) = (page.path.clone(), page.query.clone());
            if let Some(page_info) = self.pages.get(&path) {
                self.app.load_script(&page_info.js)?;
                self.app.eval(&format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", 
                    serde_json::to_string(&query).unwrap_or("{}".to_string()))).ok();
                print_js_output(&self.app);
            }
            let has_tabbar = self.is_tabbar_page(&path);
            self.scroll = ScrollController::new(CONTENT_HEIGHT as f32, (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32);
        }
        self.needs_redraw = true;
        Ok(())
    }
    
    fn switch_tab(&mut self, path: &str) -> Result<(), String> {
        self.page_stack.clear();
        self.interaction.clear_page_state();
        self.navigate_to(path.trim_start_matches('/'), HashMap::new())
    }
    
    fn setup_canvas(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        let (pw, ph) = ((LOGICAL_WIDTH as f64 * scale_factor) as u32, (CONTENT_HEIGHT as f64 * scale_factor) as u32);
        self.canvas = Some(Canvas::new(pw, ph));
        self.tabbar_canvas = Some(Canvas::new(pw, (TABBAR_HEIGHT as f64 * scale_factor) as u32));
        self.fixed_canvas = Some(Canvas::new(pw, (LOGICAL_HEIGHT as f64 * scale_factor) as u32));
        self.text_renderer = TextRenderer::load_system_font()
            .or_else(|_| TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf"))).ok();
    }
    
    fn update_renderers(&mut self) {
        if let Some(page) = self.page_stack.last() {
            self.renderer = Some(WxmlRenderer::new_with_scale(page.stylesheet.clone(), LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32, self.scale_factor as f32));
            if let Some(ref ct) = self.custom_tabbar {
                self.tabbar_renderer = Some(WxmlRenderer::new_with_scale(ct.stylesheet.clone(), LOGICAL_WIDTH as f32, TABBAR_HEIGHT as f32, self.scale_factor as f32));
            }
        }
    }

    fn render(&mut self) {
        let page_data = self.app.eval("__getPageData()").map(|s| serde_json::from_str(&s).unwrap_or(json!({}))).unwrap_or(json!({}));
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let (current_path, has_tabbar) = (page.path.clone(), self.is_tabbar_page(&page.path));
        let viewport_height = (LOGICAL_HEIGHT - if has_tabbar { TABBAR_HEIGHT } else { 0 }) as f32;
        let scroll_offset = self.scroll.get_position();
        
        let mut content_height = 0.0f32;
        if let Some(canvas) = &mut self.canvas {
            canvas.clear(Color::from_hex(0xF5F5F5));
            if let Some(renderer) = &mut self.renderer {
                content_height = renderer.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
            }
        }
        
        if content_height > 0.0 {
            self.scroll.update_content_height(content_height, viewport_height);
            let required_height = (content_height * self.scale_factor as f32).ceil() as u32;
            if self.canvas.as_ref().map(|c| c.height()).unwrap_or(0) != required_height && required_height > 0 {
                self.canvas = Some(Canvas::new((LOGICAL_WIDTH as f64 * self.scale_factor) as u32, required_height));
                if let Some(page) = self.page_stack.last() {
                    if let (Some(canvas), Some(renderer)) = (&mut self.canvas, &mut self.renderer) {
                        canvas.clear(Color::from_hex(0xF5F5F5));
                        renderer.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
                    }
                }
            }
        }
        
        if let Some(page) = self.page_stack.last() {
            if let (Some(fc), Some(r)) = (&mut self.fixed_canvas, &mut self.renderer) {
                fc.clear(Color::new(0, 0, 0, 0));
                r.render_fixed_elements(fc, &page.wxml_nodes, &page_data, &mut self.interaction, viewport_height);
            }
        }
        
        if has_tabbar {
            if self.is_custom_tabbar() { self.render_custom_tabbar(&current_path); }
            else { self.render_native_tabbar(&current_path); }
        }
    }
    
    fn render_custom_tabbar(&mut self, current_path: &str) {
        let tb = match &self.app_config.tab_bar { Some(tb) => tb.clone(), None => return };
        let list: Vec<serde_json::Value> = tb.list.iter().map(|i| json!({"pagePath": i.page_path, "text": i.text})).collect();
        let data = json!({"selected": self.get_tabbar_index(current_path).unwrap_or(0), "list": list});
        let ct = match &self.custom_tabbar { Some(ct) => ct, None => return };
        if let (Some(c), Some(r)) = (&mut self.tabbar_canvas, &mut self.tabbar_renderer) {
            c.clear(Color::WHITE);
            r.render(c, &ct.wxml_nodes, &data);
        }
    }
    
    fn render_native_tabbar(&mut self, current_path: &str) {
        let tb = match &self.app_config.tab_bar { Some(tb) => tb.clone(), None => return };
        if let (Some(c), Some(tr)) = (&mut self.tabbar_canvas, &self.text_renderer) {
            render_native_tabbar(c, tr, &tb, current_path, self.scale_factor);
        }
    }
    
    fn present(&mut self) {
        let canvas = match &self.canvas { Some(c) => c, None => return };
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        let (toast_state, loading_state, modal_state) = (self.toast.clone(), self.loading.clone(), self.modal.clone());
        
        if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
            let size = window.inner_size();
            if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                surface.resize(w, h).ok();
                if let Ok(mut buffer) = surface.buffer_mut() {
                    present_to_buffer(&mut buffer, size.width, size.height, canvas, self.fixed_canvas.as_ref(), self.tabbar_canvas.as_ref(),
                        (self.scroll.get_position() * self.scale_factor as f32) as i32, has_tabbar,
                        if has_tabbar { (TABBAR_HEIGHT as f64 * self.scale_factor) as u32 } else { 0 });
                    render_ui_overlay(&mut buffer, size.width, size.height, self.scale_factor as f32, self.last_frame,
                        &toast_state, &loading_state, &modal_state, self.text_renderer.as_ref());
                    buffer.present().ok();
                }
            }
        }
    }
    
    fn handle_click(&mut self, x: f32, y: f32) {
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) { self.handle_modal_click(x, y); return; }
        if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) { return; }
        
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let has_tabbar = self.is_tabbar_page(&page.path);
        let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
        
        if has_tabbar && y >= tabbar_y {
            let nav = if self.is_custom_tabbar() {
                click::handle_custom_tabbar_click(x, y - tabbar_y, self.tabbar_renderer.as_ref(), &page.path)
            } else {
                self.app_config.tab_bar.as_ref().and_then(|tb| click::handle_native_tabbar_click_wrapper(x, tb, &page.path))
            };
            if let Some(n) = nav { self.pending_navigation = Some(n); if let Some(w) = &self.window { w.request_redraw(); } }
        } else {
            if let Some(nav) = click::handle_content_click(x, y, &self.scroll, has_tabbar, &mut self.interaction,
                self.renderer.as_ref(), &mut self.app, self.scale_factor, self.text_renderer.as_ref(), self.window.as_ref(), &mut self.clipboard) {
                self.pending_navigation = Some(nav);
            }
            self.needs_redraw = true;
        }
    }
    
    fn handle_modal_press(&mut self, x: f32, y: f32) -> bool {
        let modal = match &self.modal { Some(m) if m.visible => m, _ => return false };
        let layout = click::calculate_modal_layout(modal, self.scale_factor as f32, self.text_renderer.as_ref());
        if let Some(btn) = click::detect_modal_button(x, y, &layout, modal.show_cancel) {
            if let Some(m) = &mut self.modal { m.pressed_button = Some(btn); }
            self.needs_redraw = true;
            if let Some(w) = &self.window { w.request_redraw(); }
            return true;
        }
        false
    }
    
    fn handle_modal_release(&mut self, x: f32, y: f32) {
        let pressed = self.modal.as_ref().and_then(|m| m.pressed_button.clone());
        if let Some(m) = &mut self.modal { m.pressed_button = None; }
        let modal = match &self.modal { Some(m) if m.visible => m, _ => return };
        let layout = click::calculate_modal_layout(modal, self.scale_factor as f32, self.text_renderer.as_ref());
        if let Some(btn) = click::detect_modal_button(x, y, &layout, modal.show_cancel) {
            if pressed.as_deref() == Some(&btn) {
                let code = if btn == "cancel" { "if(__modalCallback) __modalCallback({ confirm: false, cancel: true })" }
                           else { "if(__modalCallback) __modalCallback({ confirm: true, cancel: false })" };
                self.app.eval(code).ok();
                self.modal = None;
            }
        }
        self.needs_redraw = true;
        if let Some(w) = &self.window { w.request_redraw(); }
    }
    
    fn handle_modal_click(&mut self, x: f32, y: f32) { self.handle_modal_release(x, y); }
    
    fn process_navigation(&mut self) {
        if let Some(nav) = self.pending_navigation.take() {
            match nav {
                NavigationRequest::NavigateTo { url } => { let (p, q) = parse_url(&url); self.navigate_to(&p, q).ok(); }
                NavigationRequest::NavigateBack => { self.navigate_back().ok(); }
                NavigationRequest::SwitchTab { url } => { let (p, _) = parse_url(&url); self.switch_tab(&p).ok(); }
            }
            self.update_renderers();
        }
    }
    
    fn update_scroll(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        
        let (animating, event) = self.scroll.update_with_events(dt);
        if let Some(e) = event { evt::handle_scroll_event(e, &mut self.app); self.needs_redraw = true; }
        
        let mut changed = animating;
        for c in self.interaction.scroll_controllers.values_mut() { if c.update(dt) { changed = true; } }
        if changed { if let Some(w) = &self.window { w.request_redraw(); } }
    }
}


impl ApplicationHandler for MiniAppWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let window = Arc::new(event_loop.create_window(WindowAttributes::default()
            .with_title("Mini App").with_inner_size(winit::dpi::LogicalSize::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)).with_resizable(false)).unwrap());
        window.set_ime_allowed(true);
        self.setup_canvas(window.scale_factor());
        self.update_renderers();
        let ctx = softbuffer::Context::new(window.clone()).unwrap();
        self.surface = Some(softbuffer::Surface::new(&ctx, window.clone()).unwrap());
        self.window = Some(window);
        self.render();
        self.present();
        println!("\n🎮 Ready!\n");
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),
            
            WindowEvent::KeyboardInput { event, .. } => {
                let (nr, pn, ex) = evt::handle_keyboard_event(event, self.modifiers, &mut self.interaction, &mut self.clipboard,
                    self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.scroll, self.scale_factor);
                if ex { event_loop.exit(); }
                if let Some(n) = pn { self.pending_navigation = Some(n); }
                if nr { self.needs_redraw = true; if let Some(w) = &self.window { w.request_redraw(); } }
            }
            
            WindowEvent::Ime(ime) => {
                if evt::handle_ime_event(ime, &mut self.interaction, self.window.as_ref(), self.renderer.as_ref(),
                    &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor) {
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
            
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.setup_canvas(scale_factor);
                self.update_renderers();
                self.render();
                self.needs_redraw = false;
            }
            
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32 / self.scale_factor as f32, position.y as f32 / self.scale_factor as f32);
                self.mouse_pos = (x, y);
                if evt::handle_cursor_moved(x, y, &mut self.interaction, &mut self.scroll, self.text_renderer.as_ref(),
                    self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.clipboard, self.scale_factor) {
                    self.needs_redraw = true;
                }
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseWheel { delta, .. } => {
                if evt::handle_mouse_wheel(delta, self.mouse_pos, &mut self.interaction, &mut self.scroll, self.scale_factor) {
                    self.needs_redraw = true;
                }
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
                let (x, y) = self.mouse_pos;
                
                if state == ElementState::Pressed {
                    self.click_start_pos = self.mouse_pos;
                    self.click_start_time = Instant::now();
                    
                    if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) { self.handle_modal_press(x, y); return; }
                    if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) { return; }
                    
                    let has_tabbar = self.page_stack.last().map(|p| self.is_tabbar_page(&p.path)).unwrap_or(false);
                    let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - TABBAR_HEIGHT) as f32 } else { LOGICAL_HEIGHT as f32 };
                    if has_tabbar && y >= tabbar_y { return; }
                    
                    let actual_y = y + self.scroll.get_position();
                    
                    // 输入框内点击
                    if let Some(focused) = &self.interaction.focused_input {
                        let b = focused.bounds;
                        if (x >= b.x && x <= b.x + b.width && y >= b.y - self.scroll.get_position() && y <= b.y + b.height - self.scroll.get_position()) ||
                           (x >= b.x && x <= b.x + b.width && actual_y >= b.y && actual_y <= b.y + b.height) {
                            if let Some(tr) = &self.text_renderer {
                                let sf = self.scale_factor as f32;
                                let cw: Vec<f32> = focused.value.chars().map(|c| tr.measure_text(&c.to_string(), 16.0 * sf)).collect();
                                let cp = mini_render::ui::interaction::calculate_cursor_position(&focused.value, &cw, (x - b.x) * sf, 12.0 * sf, focused.text_offset);
                                self.interaction.prepare_text_selection(cp);
                                self.needs_redraw = true;
                                if let Some(w) = &self.window { w.request_redraw(); }
                                return;
                            }
                        }
                    }
                    
                    // 交互元素
                    if let Some(el) = self.interaction.hit_test(x, y).or_else(|| self.interaction.hit_test(x, actual_y)).cloned() {
                        use mini_render::ui::interaction::InteractionType;
                        match el.interaction_type {
                            InteractionType::Slider if !el.disabled => {
                                let ty = if el.is_fixed { y } else { actual_y };
                                if let Some(r) = self.interaction.handle_click(x, ty) {
                                    handle_interaction_result(&r, self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor);
                                    self.needs_redraw = true;
                                    if let Some(w) = &self.window { w.request_redraw(); }
                                }
                                return;
                            }
                            InteractionType::Button if !el.disabled => {
                                self.interaction.set_button_pressed(el.id.clone(), el.bounds);
                                self.needs_redraw = true;
                                if let Some(w) = &self.window { w.request_redraw(); }
                            }
                            InteractionType::ScrollArea => {
                                if let Some(c) = self.interaction.get_scroll_controller_mut(&el.id) {
                                    c.begin_drag(y, ts);
                                    self.interaction.dragging_scroll_area = Some(el.id.clone());
                                    return;
                                }
                            }
                            _ => {}
                        }
                    }
                    
                    if !self.interaction.is_dragging_slider() { self.scroll.begin_drag(y, ts); }
                } else {
                    // Released
                    if self.modal.as_ref().map(|m| m.visible && m.pressed_button.is_some()).unwrap_or(false) {
                        self.handle_modal_release(x, y);
                        return;
                    }
                    
                    self.interaction.clear_button_pressed();
                    let was_sel = self.interaction.is_dragging_selection();
                    self.interaction.end_text_selection();
                    if was_sel { self.needs_redraw = true; if let Some(w) = &self.window { w.request_redraw(); } return; }
                    
                    if let Some(id) = self.interaction.dragging_scroll_area.take() {
                        if let Some(c) = self.interaction.get_scroll_controller_mut(&id) { c.end_drag(); }
                        self.needs_redraw = true;
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                    
                    if let Some(r) = self.interaction.handle_mouse_release() {
                        handle_interaction_result(&r, self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.clipboard, self.scroll.get_position(), self.scale_factor);
                    }
                    
                    let anim = self.scroll.end_drag();
                    let (dx, dy) = ((x - self.click_start_pos.0).abs(), (y - self.click_start_pos.1).abs());
                    if dx < 10.0 && dy < 10.0 && self.click_start_time.elapsed().as_millis() < 300 { self.handle_click(x, y); }
                    
                    self.needs_redraw = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                    if anim { if let Some(w) = &self.window { w.request_redraw(); } }
                }
            }
            
            WindowEvent::RedrawRequested => {
                self.app.update().ok();
                print_js_output(&self.app);
                
                if evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal) { self.needs_redraw = true; }
                if evt::update_toast_timeout(&mut self.toast) { self.needs_redraw = true; }
                
                self.update_scroll();
                self.process_navigation();
                
                let scrolling = self.scroll.is_animating() || self.scroll.is_dragging;
                let sv_scroll = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
                if self.needs_redraw || mini_render::renderer::components::has_playing_video() || sv_scroll || self.interaction.has_focused_input() || scrolling {
                    self.render();
                    self.needs_redraw = false;
                }
                self.present();
                
                if scrolling || sv_scroll || self.interaction.has_focused_input() || self.app.has_active_timers() ||
                   self.toast.as_ref().map(|t| t.visible).unwrap_or(false) || self.loading.as_ref().map(|l| l.visible).unwrap_or(false) ||
                   self.modal.as_ref().map(|m| m.visible).unwrap_or(false) || mini_render::renderer::components::has_playing_video() {
                    if let Some(w) = &self.window { w.request_redraw(); }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mini App Engine\n");
    
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();
    let app_path = if args.len() > 1 {
        let path = std::path::PathBuf::from(&args[1]);
        if path.exists() && path.is_dir() {
            println!("📂 加载小程序: {}", path.display());
            Some(path)
        } else {
            eprintln!("❌ 目录不存在: {}", path.display());
            return Err(format!("目录不存在: {}", path.display()).into());
        }
    } else {
        println!("📂 使用内置 sample-app");
        None
    };
    
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut MiniAppWindow::new(app_path)?)?;
    Ok(())
}
