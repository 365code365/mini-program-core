//! 平台无关的小程序引擎实例：**移动端 SDK 的核心**。
//!
//! 它把「一个小程序目录」跑成「一块 RGBA 像素缓冲 + 一组事件入口」，不碰任何窗口
//! 系统。iOS/Android/鸿蒙的宿主只需要做三件事：给尺寸、每帧调 [`MiniEngine::pump`]
//! 拿像素、把触摸/键盘塞进来。
//!
//! ## 为什么不直接复用桌面窗体
//! 桌面窗体（`src/bin/window.rs`）绑着 winit + softbuffer，而这两个依赖在移动端
//! 要么编不过（winit 的 android 后端）、要么不该编进去（剪贴板/声卡）。所以把
//! **与平台无关的宿主逻辑**沉到 `crate::host`（页面加载、页面栈、覆盖层、触摸
//! 状态机、像素合成都在那里），窗体和本引擎共用同一份，避免行为分叉。
//!
//! ## 与桌面窗体的差异（v1 已知范围）
//! 每帧走**整帧重绘**，没有接桌面那套损伤区/局部重绘；picker 面板与侧滑返回
//! 也还没接进来。像素一致性由 `tools/sdk-parity.sh` 对照桌面基线守着。

use crate::host::{
    component_mount, load_all_pages, page_loader, remove_manual_tabbar, tabbar_height,
    ui_overlay::{self, LoadingState, ModalState, ToastState},
    AppConfig, PageInstance, CONTENT_HEIGHT, LOGICAL_HEIGHT, LOGICAL_WIDTH,
};
use crate::parser::{WxmlParser, WxssParser};
use crate::renderer::WxmlRenderer;
use crate::runtime::{MiniApp, UiEvent};
use crate::text::TextRenderer;
use crate::ui::interaction::InteractionManager;
use crate::ui::ScrollController;
use crate::{Canvas, Color};
use std::collections::HashMap;
use std::sync::Arc;

pub struct MiniEngine {
    pub(crate) app: MiniApp,
    pub(crate) config: AppConfig,
    pub(crate) pages: HashMap<String, crate::host::PageInfo>,
    pub(crate) page_stack: Vec<PageInstance>,
    pub(crate) app_wxss: String,
    pub(crate) custom_tabbar: Option<crate::host::CustomTabBar>,

    pub(crate) renderer: Option<WxmlRenderer>,
    pub(crate) tabbar_renderer: Option<WxmlRenderer>,
    pub(crate) text: Option<Arc<TextRenderer>>,
    pub(crate) canvas: Canvas,
    pub(crate) fixed_canvas: Canvas,
    pub(crate) tabbar_canvas: Canvas,

    pub(crate) interaction: InteractionManager,
    pub(crate) scroll: ScrollController,
    pub(crate) touch: crate::host::touch::TouchTracker,
    pub(crate) toast: Option<ToastState>,
    pub(crate) loading: Option<LoadingState>,
    pub(crate) modal: Option<ModalState>,

    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) dpr: f32,
    pub(crate) page_data: serde_json::Value,
    pub(crate) page_data_dirty: bool,
    pub(crate) needs_redraw: bool,
    pub(crate) caret_visible: bool,
    /// 固定的动画时钟（仅测试用，见 set_animation_time）
    pub(crate) anim_time: Option<f32>,
    /// fixed 覆盖层实际画过的行区间（设备像素）
    fixed_rows: Option<(u32, u32)>,
    /// 合成缓冲（0xRRGGBB）与交给宿主的 RGBA
    compose: Vec<u32>,
    rgba: Vec<u8>,
    started_ms: u64,
    last_pump_ms: u64,
    page_opened_ms: u64,
}

impl MiniEngine {
    /// 创建实例。`app_dir` 是解包后的小程序目录，`data_dir` 是宿主沙盒里可写的目录
    /// （storage 与图片磁盘缓存落在这里；不传的话缓存会**静默失效**）。
    pub fn new(
        app_dir: &str,
        data_dir: Option<&str>,
        width: u32,
        height: u32,
        dpr: f32,
    ) -> Result<Self, String> {
        if let Some(d) = data_dir {
            crate::data_dir::set_data_dir(d);
        }
        page_loader::set_app_path(std::path::PathBuf::from(app_dir));
        let (width, height) = (width.max(1), height.max(1));
        let dpr = if dpr > 0.1 { dpr } else { 1.0 };

        let mut app = MiniApp::new(width, height)?;
        app.init()?;
        // 先把包内所有 .js 注册成 CommonJS 模块：uni-app / TS 产物第一行就 require
        // 一个几百 KB 的 vendor 包，不预注册直接失败。
        let root = page_loader::get_app_path();
        app.register_all_modules(&root).ok();
        app.load_module_script("app", &page_loader::load_app_js())?;

        let config: AppConfig = serde_json::from_str(&page_loader::load_app_json())
            .map_err(|e| format!("app.json 解析失败: {e}"))?;
        let app_wxss = page_loader::load_app_wxss();
        let custom_tabbar = if config.tab_bar.as_ref().map(|t| t.custom).unwrap_or(false) {
            crate::host::load_custom_tabbar_with_app_wxss(&app_wxss)?
        } else {
            None
        };
        // tabBar 高度以组件 WXSS 实测为准（微信也是由组件自身决定）
        if let Some(ct) = &custom_tabbar {
            let probe = WxmlRenderer::new(ct.stylesheet.clone(), width as f32, height as f32);
            let measured = probe.measure_content_height(&ct.wxml_nodes, &ct.data);
            if measured > 1.0 {
                crate::host::set_tabbar_height(measured.round() as u32);
            }
        }

        let px = |v: u32| (v as f32 * dpr) as u32;
        let vp = height as f32;
        Ok(Self {
            app,
            pages: load_all_pages(),
            config,
            page_stack: Vec::new(),
            app_wxss,
            custom_tabbar,
            renderer: None,
            tabbar_renderer: None,
            text: crate::text::shared_fonts(),
            canvas: Canvas::new(px(width), px(CONTENT_HEIGHT.max(height))),
            fixed_canvas: Canvas::new(px(width), px(height)),
            tabbar_canvas: Canvas::new(px(width), px(tabbar_height()).max(1)),
            interaction: InteractionManager::new(),
            scroll: ScrollController::new(vp, vp),
            touch: crate::host::touch::TouchTracker::new(),
            toast: None,
            loading: None,
            modal: None,
            width,
            height,
            dpr,
            page_data: serde_json::json!({}),
            page_data_dirty: true,
            needs_redraw: true,
            caret_visible: true,
            anim_time: None,
            fixed_rows: None,
            compose: Vec::new(),
            rgba: Vec::new(),
            started_ms: 0,
            last_pump_ms: 0,
            page_opened_ms: 0,
        })
    }

    /// 首页路径（`app.json` 的 `pages[0]`）
    pub fn home_route(&self) -> String {
        self.config
            .pages
            .first()
            .cloned()
            .unwrap_or_else(|| "pages/index/index".to_string())
    }

    /// 打开一个页面（不传就开首页）
    pub fn launch(&mut self, route: Option<&str>) -> Result<(), String> {
        let route = route
            .map(|r| r.trim_start_matches('/').to_string())
            .unwrap_or_else(|| self.home_route());
        self.navigate_to(&route, HashMap::new())
    }

    pub fn current_route(&self) -> String {
        self.page_stack.last().map(|p| p.path.clone()).unwrap_or_default()
    }

    pub fn page_depth(&self) -> usize {
        self.page_stack.len()
    }

    pub(crate) fn is_tabbar_page(&self, path: &str) -> bool {
        self.config
            .tab_bar
            .as_ref()
            .map(|tb| tb.list.iter().any(|i| i.page_path == path))
            .unwrap_or(false)
    }

    pub(crate) fn viewport_height(&self) -> f32 {
        (self.height - if self.is_tabbar_page(&self.current_route()) { tabbar_height() } else { 0 })
            as f32
    }

    /// 进入一个页面（与桌面窗体 `navigate_to` 同序：组件登记 → 路由登记 →
    /// 页面 js → onLoad/onShow/onReady → 挂组件实例）
    pub(crate) fn navigate_to(
        &mut self,
        path: &str,
        query: HashMap<String, String>,
    ) -> Result<(), String> {
        let path = path.trim_start_matches('/').to_string();
        let info = self
            .pages
            .get(&path)
            .ok_or_else(|| format!("页面不存在: {path}"))?
            .clone();
        let nodes = remove_manual_tabbar(
            &WxmlParser::new(&info.wxml)
                .parse()
                .map_err(|e| format!("WXML 解析失败: {e}"))?,
        );
        let mut templates = crate::parser::template::ComponentTemplates::new();
        let mut comp_wxss = String::new();
        for c in &info.components {
            if let Ok(n) = WxmlParser::new(&c.wxml).parse() {
                templates.insert(c.tag.clone(), n);
                comp_wxss.push('\n');
                comp_wxss.push_str(&c.wxss);
            }
        }
        // app.wxss 在前、页面在后：同特异性时页面样式因书写顺序更靠后而胜出
        let merged = format!("{}\n{}\n{}", self.app_wxss, info.wxss, comp_wxss);
        let stylesheet = WxssParser::new(&merged)
            .parse()
            .map_err(|e| format!("WXSS 解析失败: {e}"))?;

        self.app.eval("__resetPageComponents()").ok();
        self.app
            .eval(&format!(
                "__setPendingRoute({})",
                serde_json::to_string(&path).unwrap_or_else(|_| "''".into())
            ))
            .ok();
        self.app.load_module_script(&path, &info.js)?;
        let q = serde_json::to_string(&query).unwrap_or_else(|_| "{}".into());
        self.app
            .eval(&format!("if(__currentPage&&__currentPage.onLoad)__currentPage.onLoad({q})"))
            .ok();
        self.app.eval("if(__currentPage&&__currentPage.onShow)__currentPage.onShow()").ok();
        self.app.eval("if(__currentPage&&__currentPage.onReady)__currentPage.onReady()").ok();
        component_mount::mount_page_components(&mut self.app, &info.components, &nodes);

        self.page_stack.push(PageInstance {
            path: path.clone(),
            query,
            wxml_nodes: nodes,
            stylesheet,
            component_templates: templates,
        });
        self.page_opened_ms = self.last_pump_ms;
        self.page_data_dirty = true;
        let vp = self.viewport_height();
        self.scroll = ScrollController::new(vp, vp);
        self.interaction.clear_page_state();
        self.update_renderers();
        self.needs_redraw = true;
        Ok(())
    }

    pub(crate) fn update_renderers(&mut self) {
        let Some(page) = self.page_stack.last() else { return };
        let mut r = WxmlRenderer::new_with_scale(
            page.stylesheet.clone(),
            self.width as f32,
            self.height as f32,
            self.dpr,
        );
        r.set_component_templates(page.component_templates.clone());
        if let Some(t) = self.anim_time {
            r.set_animation_time(t);
        }
        self.renderer = Some(r);
        if let Some(ct) = &self.custom_tabbar {
            let mut tr = WxmlRenderer::new_with_scale(
                ct.stylesheet.clone(),
                self.width as f32,
                tabbar_height() as f32,
                self.dpr,
            );
            if let Some(t) = self.anim_time {
                tr.set_animation_time(t);
            }
            self.tabbar_renderer = Some(tr);
        }
    }

    /// 跑一帧：逻辑层 → 事件 → 渲染 → 合成。返回是否产出了新画面。
    ///
    /// `now_ms` 用宿主的单调时钟（iOS `CACurrentMediaTime`、Android
    /// `System.nanoTime`）。传 0 表示让引擎自己取。
    pub fn pump(&mut self, now_ms: u64) -> bool {
        let now = if now_ms > 0 { now_ms } else { self.mono_ms() };
        let dt = if self.last_pump_ms == 0 {
            0.016
        } else {
            ((now.saturating_sub(self.last_pump_ms)) as f32 / 1000.0).clamp(0.001, 0.1)
        };
        self.last_pump_ms = now;

        self.app.update().ok();
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
            self.needs_redraw = true;
        }
        self.drain_ui_events();
        // 滚动推进（惯性/回弹），到底事件回调页面
        let (moved, event) = self.scroll.update_with_events(dt);
        if moved {
            self.needs_redraw = true;
        }
        if let Some(crate::ui::scroll_controller::ScrollEvent::ReachBottom) = event {
            self.app
                .eval("if(__currentPage&&__currentPage.onReachBottom)__currentPage.onReachBottom()")
                .ok();
        }
        // 逻辑层发起的路由
        if let Some(nav) = self.take_navigation() {
            self.apply_navigation(nav);
        }
        if crate::renderer::components::advance_due_swipers() {
            self.needs_redraw = true;
        }
        if self.app.take_network_dirty() || crate::renderer::components::image_net::take_dirty() {
            self.page_data_dirty = true;
            self.needs_redraw = true;
        }
        // 长按由帧驱动
        if self.touch.is_active() {
            let outs = self.touch.tick(now);
            if let Some((x, y)) = self.touch.pos() {
                self.dispatch_touch(&outs, x, y);
            }
        }
        self.caret_tick();
        if !self.needs_redraw && !self.interaction.has_focused_input() {
            return false;
        }
        self.render();
        self.needs_redraw = false;
        true
    }

    fn caret_tick(&mut self) {
        if !self.interaction.has_focused_input() {
            return;
        }
        let vis = crate::renderer::components::cursor_blink_visible();
        if vis != self.caret_visible {
            self.caret_visible = vis;
            self.needs_redraw = true;
        }
    }

    fn drain_ui_events(&mut self) {
        for e in self.app.drain_ui_events() {
            match e {
                UiEvent::ShowToast { title, icon, duration } => {
                    self.toast = Some(ToastState {
                        title,
                        icon,
                        visible: true,
                        start_time: std::time::Instant::now(),
                        duration_ms: duration,
                    });
                }
                UiEvent::HideToast => self.toast = None,
                UiEvent::ShowLoading { title } => {
                    self.loading = Some(LoadingState { title, visible: true })
                }
                UiEvent::HideLoading => self.loading = None,
                UiEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text } => {
                    self.modal = Some(ModalState {
                        title,
                        content,
                        show_cancel,
                        cancel_text,
                        confirm_text,
                        visible: true,
                        pressed_button: None,
                    });
                }
                UiEvent::HideModal => self.modal = None,
                UiEvent::StartPullDownRefresh | UiEvent::StopPullDownRefresh => {}
            }
            self.needs_redraw = true;
        }
    }

    /// 渲染 + 合成到内部缓冲
    pub(crate) fn render(&mut self) {
        let Some(page) = self.page_stack.last() else { return };
        let route = page.path.clone();
        if self.page_data_dirty {
            self.page_data = self
                .app
                .eval("__getRenderData()")
                .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::json!({})))
                .unwrap_or(serde_json::json!({}));
            self.page_data_dirty = false;
        }
        let nodes = self
            .page_stack
            .last()
            .map(|p| p.wxml_nodes.clone())
            .unwrap_or_default();
        let data = self.page_data.clone();
        let has_tabbar = self.is_tabbar_page(&route);
        let vp = self.viewport_height();
        let bg = self
            .renderer
            .as_ref()
            .and_then(|r| r.page_style().background)
            .unwrap_or(Color::from_hex(0xF5F5F5));
        self.canvas.clear(bg);
        self.fixed_canvas.clear(Color::new(0, 0, 0, 0));
        let scroll = self.scroll.get_position();
        let mut content_h = 0.0;
        if let Some(r) = &mut self.renderer {
            content_h = r.render_with_scroll_and_viewport(
                &mut self.canvas,
                &nodes,
                &data,
                &mut self.interaction,
                scroll,
                vp,
            );
            r.render_fixed_elements(
                &mut self.fixed_canvas,
                &nodes,
                &data,
                &mut self.interaction,
                vp,
            );
        }
        if content_h > 1.0 {
            self.scroll.update_content_height(content_h, vp);
        }
        // 自定义 tabBar
        if has_tabbar {
            let selected = self
                .config
                .tab_bar
                .as_ref()
                .and_then(|tb| tb.list.iter().position(|i| i.page_path == route));
            // 三个字段各自可变/不可变借用，互不重叠
            if let (Some(ct), Some(tr)) = (self.custom_tabbar.as_ref(), self.tabbar_renderer.as_mut())
            {
                self.tabbar_canvas.clear(Color::new(0, 0, 0, 0));
                let mut d = ct.data.clone();
                if let Some(idx) = selected {
                    d["selected"] = serde_json::json!(idx);
                }
                tr.render(&mut self.tabbar_canvas, &ct.wxml_nodes, &d);
            }
        }
        // 覆盖层真正画过的行区间：`present_to_buffer` 只合成这几行，
        // 传 None 会**整层跳过** —— 底部吸底操作栏、弹窗遮罩就全丢了。
        self.fixed_rows = crate::host::render::opaque_row_span(&self.fixed_canvas);
        self.compose_frame(has_tabbar, bg);
    }

    fn compose_frame(&mut self, has_tabbar: bool, bg: Color) {
        let (pw, ph) = ((self.width as f32 * self.dpr) as u32, (self.height as f32 * self.dpr) as u32);
        let n = (pw as usize) * (ph as usize);
        if self.compose.len() != n {
            self.compose = vec![0u32; n];
        }
        let bg_u32 = ((bg.r as u32) << 16) | ((bg.g as u32) << 8) | bg.b as u32;
        crate::host::present_to_buffer(
            &mut self.compose[..],
            pw,
            ph,
            &self.canvas,
            Some(&self.fixed_canvas),
            if has_tabbar { Some(&self.tabbar_canvas) } else { None },
            (self.scroll.get_position() * self.dpr) as i32,
            has_tabbar,
            if has_tabbar { (tabbar_height() as f32 * self.dpr) as u32 } else { 0 },
            self.fixed_rows,
            bg_u32,
        );
        ui_overlay::render_ui_overlay(
            &mut self.compose[..],
            pw,
            ph,
            self.dpr,
            std::time::Instant::now(),
            &self.toast,
            &self.loading,
            &self.modal,
            self.text.as_deref(),
        );
        // u32(0xRRGGBB) → RGBA8
        if self.rgba.len() != n * 4 {
            self.rgba = vec![255u8; n * 4];
        }
        for (i, px) in self.compose.iter().enumerate() {
            let o = i * 4;
            self.rgba[o] = ((px >> 16) & 0xFF) as u8;
            self.rgba[o + 1] = ((px >> 8) & 0xFF) as u8;
            self.rgba[o + 2] = (px & 0xFF) as u8;
            self.rgba[o + 3] = 255;
        }
    }

    /// 当前帧的 RGBA8 像素（宽 = 逻辑宽 × dpr）
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn pixel_size(&self) -> (u32, u32) {
        (
            (self.width as f32 * self.dpr) as u32,
            (self.height as f32 * self.dpr) as u32,
        )
    }

    /// 固定 CSS 动画时钟（秒）。**只给确定性测试用**：正常运行时动画由真实时间驱动，
    /// 而快照对比需要把 `@keyframes` 求值到同一个时刻，否则两次跑的相位不同、
    /// 逐像素对比永远对不上（桌面侧对应 `--time` 参数）。
    pub fn set_animation_time(&mut self, seconds: f32) {
        self.anim_time = Some(seconds);
        if let Some(r) = &mut self.renderer {
            r.set_animation_time(seconds);
        }
        if let Some(r) = &mut self.tabbar_renderer {
            r.set_animation_time(seconds);
        }
        self.needs_redraw = true;
    }

    pub fn on_show(&mut self) {
        self.app.eval("__dispatchApp('onShow')").ok();
        self.needs_redraw = true;
    }

    pub fn on_hide(&mut self) {
        self.app.eval("__dispatchApp('onHide')").ok();
    }

    pub fn trim_memory(&mut self) {
        crate::trim_memory();
    }

    pub(crate) fn mono_ms(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
            .saturating_sub(self.started_ms)
    }
}
