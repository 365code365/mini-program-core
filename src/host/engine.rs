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
//! ## 与桌面窗体的差异（当前已知范围）
//! 页面滚动与桌面一样只换上屏切片（视口滑出已绘制条带才重画），但其余重绘都是
//! 整条带重画，没有接桌面那套动画/scroll-view 损伤区；**左边缘侧滑返回**还没接
//! （它要给被覆盖的页留一张视口图，属于宿主侧的帧管理）。
//!
//! 指针输入、手势仲裁、picker 面板、Modal 命中现在与桌面**共用同一份**
//! （`host::input` / `host::picker_sheet` / `host::ui_overlay`）。这条边界有两道守卫：
//! 静态像素由 `tools/sdk-parity.sh` 对照桌面基线，输入行为由
//! `src/tests/engine_input_tests.rs` 直接驱动本引擎（手势归属、tabBar、picker、Modal）。
//! 只靠像素守不住输入 —— 它们分叉过一次而没人发现：本引擎的指针层曾是简化版
//! （无方向锁定、无嵌套交接、无 picker），静态截图一模一样，手上完全不是一回事。

use crate::host::{
    component_mount, load_all_pages, page_loader, remove_manual_tabbar, tabbar_height,
    ui_overlay::{self, LoadingState, ModalState, ToastState},
    AppConfig, PageInstance, CONTENT_HEIGHT,
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
    /// 拖动手势仲裁（方向锁定 / 嵌套传递 / catchtouchmove），与桌面窗体同一份实现
    pub(crate) gesture: Option<crate::host::gesture::DragGesture>,
    /// `<picker>` 弹出的底部面板（微信里是原生浮层）
    pub(crate) picker_sheet: Option<crate::host::picker_sheet::PickerSheetState>,
    /// 按下瞬间的页面滚动位置：用来判断「滚动是否已经接管这次触摸」
    pub(crate) scroll_pos_at_press: f32,
    /// 这次触摸是用来「停住惯性滚动」的：抬手时不该再算一次点击
    pub(crate) tap_stops_fling: bool,
    pub(crate) toast: Option<ToastState>,
    pub(crate) loading: Option<LoadingState>,
    pub(crate) modal: Option<ModalState>,

    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) dpr: f32,
    pub(crate) page_data: serde_json::Value,
    pub(crate) page_data_dirty: bool,
    pub(crate) needs_redraw: bool,
    /// 页面滚动位置变了。只要视口还落在 `drawn_band` 内，这一帧只需重新合成，不必重画。
    pub(crate) scroll_moved: bool,
    /// 页面画布上最近一次整帧重绘真正画过的行区间（设备像素，已夹到画布内）
    drawn_band: Option<(f32, f32)>,
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
            gesture: None,
            picker_sheet: None,
            scroll_pos_at_press: 0.0,
            tap_stops_fling: false,
            toast: None,
            loading: None,
            modal: None,
            width,
            height,
            dpr,
            page_data: serde_json::json!({}),
            page_data_dirty: true,
            needs_redraw: true,
            scroll_moved: false,
            drawn_band: None,
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

    /// 当前页面的滚动位置（逻辑像素）。宿主用它做「返回时恢复位置」之类的事，
    /// 也是无头测试判断「这一划到底滚没滚」的唯一手段。
    pub fn scroll_position(&self) -> f32 {
        self.scroll.get_position()
    }

    /// 直接把页面滚到某个位置（深链进来要落到某一屏、或恢复上次的位置）
    pub fn scroll_to(&mut self, y: f32) {
        self.scroll.set_position(y);
        self.scroll_moved = true;
    }

    /// 当前是否有 `<picker>` 的底部面板 / `wx.showModal` 的弹窗盖着。
    ///
    /// 宿主的返回键要看它：微信里弹窗盖着时按返回是**关弹窗**，不是退页面。
    pub fn picker_sheet_open(&self) -> bool {
        self.picker_sheet.as_ref().map(|s| s.visible && !s.closing).unwrap_or(false)
    }

    pub fn modal_open(&self) -> bool {
        self.modal.as_ref().map(|m| m.visible).unwrap_or(false)
    }

    /// 在逻辑层执行一段 JS（宿主的深链参数注入、无头测试的断言都要用）
    pub fn eval(&self, code: &str) -> Result<String, String> {
        self.app.eval(code)
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
            .ok_or_else(|| format!("页面不存在: {path}"))?;
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
            self.scroll_moved = true;
        }
        // scroll-view 的惯性与回弹（与桌面 `update_scroll` 一致）：
        // 不推进的话甩一下列表，松手就停在原地。
        for c in self.interaction.scroll_controllers.values_mut() {
            if c.update(dt) {
                self.needs_redraw = true;
            }
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
        // picker 面板的入场/退场动画按帧推进，走完就回收
        if self.picker_sheet.as_ref().map(|s| s.animating()).unwrap_or(false) {
            self.needs_redraw = true;
        }
        if crate::host::picker_sheet::reap(&mut self.picker_sheet) {
            self.needs_redraw = true;
        }
        self.caret_tick();
        let scroll_moved = std::mem::take(&mut self.scroll_moved);
        if self.needs_redraw || self.interaction.has_focused_input() {
            self.render();
        } else if scroll_moved {
            // 页面画布用内容坐标：视口还在已绘制条带内时，滚动只是换一段切片上屏
            if self.viewport_inside_drawn_band() {
                self.compose_current();
            } else {
                self.render();
            }
        } else {
            return false;
        }
        self.needs_redraw = false;
        true
    }

    /// 本帧上屏要用的那段页面画布是否已经画过
    fn viewport_inside_drawn_band(&self) -> bool {
        let Some((band_top, band_bottom)) = self.drawn_band else { return false };
        let top = (self.scroll.get_position() * self.dpr).max(0.0);
        let bottom = (self.scroll.get_position() * self.dpr + self.viewport_height() * self.dpr)
            .min(self.canvas.height() as f32);
        top >= band_top - 0.01 && bottom <= band_bottom + 0.01
    }

    fn page_background(&self) -> Color {
        self.renderer
            .as_ref()
            .and_then(|r| r.page_style().background)
            .unwrap_or(Color::from_hex(0xF5F5F5))
    }

    /// 不重画任何画布，只按当前滚动位置重新合成一帧
    fn compose_current(&mut self) {
        let has_tabbar = self.is_tabbar_page(&self.current_route());
        let bg = self.page_background();
        self.compose_frame(has_tabbar, bg);
    }

    /// 只清理并重画「视口 ± 裁剪余量」这条带（渲染器的视口裁剪用的是同一个余量），
    /// 返回内容高度。整页画布可能上万像素高，全量 clear 是纯浪费。
    fn render_page_band(&mut self, scroll: f32, vp: f32, bg: Color) -> f32 {
        let margin = crate::renderer::VIEWPORT_CULL_MARGIN_PX;
        let band_y0 = (scroll * self.dpr - margin).floor() as i32;
        let band_y1 = (scroll * self.dpr + vp * self.dpr + margin).ceil() as i32;
        self.canvas.clear_band(band_y0, band_y1, bg);
        let Self { renderer, canvas, page_stack, page_data, interaction, .. } = self;
        let (Some(r), Some(page)) = (renderer.as_mut(), page_stack.last()) else { return 0.0 };
        let content_h =
            r.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, page_data, interaction, scroll, vp);
        let canvas_h = self.canvas.height() as f32;
        self.drawn_band = Some(((band_y0 as f32).max(0.0), (band_y1 as f32).min(canvas_h)));
        content_h
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
        let has_tabbar = self.is_tabbar_page(&route);
        let vp = self.viewport_height();
        let bg = self.page_background();
        let content_h = self.render_page_band(self.scroll.get_position(), vp, bg);
        if content_h > 1.0 {
            self.scroll.update_content_height(content_h, vp);
            // 画布按内容高度分配（与桌面窗体一致）：固定高度的画布装不下长页面，
            // 滚到画布之外的部分上屏时只剩一片底色。
            let required = (content_h * self.dpr).ceil() as u32;
            if required > 0 && required != self.canvas.height() {
                self.canvas = Canvas::new(self.canvas.width(), required);
                self.render_page_band(self.scroll.get_position(), vp, bg);
            }
        }
        self.fixed_canvas.clear(Color::new(0, 0, 0, 0));
        {
            let Self { renderer, fixed_canvas, page_stack, page_data, interaction, .. } = self;
            if let (Some(r), Some(page)) = (renderer.as_mut(), page_stack.last()) {
                r.render_fixed_elements(fixed_canvas, &page.wxml_nodes, page_data, interaction, vp);
            }
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
        // picker 面板画在最上层（与桌面窗体同一份绘制）
        if let Some(sheet) = &self.picker_sheet {
            crate::host::picker_sheet::render(
                &mut self.compose[..],
                pw,
                ph,
                self.dpr,
                sheet,
                self.text.as_deref(),
            );
        }
        // u32(0xRRGGBB) → RGBA8
        if self.rgba.len() != n * 4 {
            self.rgba = vec![255u8; n * 4];
        }
        for (dst, px) in self.rgba.chunks_exact_mut(4).zip(&self.compose) {
            dst.copy_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8, 255]);
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
