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
use std::time::{Instant, Duration};
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent, StartCause};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};
use mini_render::ui::ScrollController;
use mini_render::ui::scroll_controller::PULL_REFRESH_HEIGHT;
use mini_render::Rect as GeoRect;

struct MiniAppWindow {
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    app: MiniApp,
    canvas: Option<Canvas>,
    tabbar_canvas: Option<Canvas>,
    fixed_canvas: Option<Canvas>,
    renderer: Option<WxmlRenderer>,
    tabbar_renderer: Option<WxmlRenderer>,
    text_renderer: Option<std::sync::Arc<TextRenderer>>,
    /// 全局样式（app.wxss 原文），每次解析页面样式时并入
    app_wxss: String,
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
    /// 每帧时间间隔（依据显示器刷新率，默认 60Hz，支持 120/144Hz 高刷）
    frame_interval: Duration,
    /// 上一帧呈现的时间戳，用于按刷新率节流
    last_present: Instant,
    /// 帧率诊断（`MINI_FPS=1` 开启）：统计窗口内的实际出帧数与单帧最长耗时
    fps_log: bool,
    fps_window_start: Instant,
    fps_frames: u32,
    fps_worst_ms: f32,
    /// 最慢那一帧的分段耗时（逻辑, 渲染, 上屏），单位 ms
    fps_worst_parts: (f32, f32, f32),
    /// 上一次 render() 内部分段耗时（页面, fixed 覆盖层, tabBar），单位 ms
    render_parts: (f32, f32, f32),
    /// 上一次整帧重绘记录的动画元素包围盒（物理像素，画布坐标）
    last_animated_bounds: Vec<GeoRect>,
    /// fixed 覆盖层里是否有动画（有的话不能走局部重绘）
    fixed_layer_animates: bool,
    /// 进程启动时刻：用作与帧无关的动画相位时钟
    started_at: Instant,
    /// 页面画布上「已经画过内容」的行区间（物理像素，画布坐标）。
    /// 滚动只要还落在里面就不必重绘，直接换切片上屏。
    drawn_band: Option<(f32, f32)>,
    /// 当前是否处于下拉刷新态（内容被按住、指示器转圈，等 `wx.stopPullDownRefresh()`）
    pull_refreshing: bool,
    /// 最慢一帧对应的 render 内部分段
    fps_worst_render_parts: (f32, f32, f32),
    /// 当前页面数据快照。逻辑层 setData 之后才重新取，避免每帧一次全量 JSON 往返。
    page_data: std::sync::Arc<serde_json::Value>,
    page_data_dirty: bool,
}

impl MiniAppWindow {
    fn new(app_path: Option<std::path::PathBuf>) -> Result<Self, String> {
        // 登记小程序根目录：页面/组件加载与包内资源（图片）解析都依赖它
        page_loader::set_app_path(
            app_path.unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample-app")
            }),
        );
        
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
        
        // 全局样式：小程序语义下 app.wxss 对所有页面生效，必须并进每个页面的样式表
        let app_wxss = page_loader::load_app_wxss();
        if !app_wxss.trim().is_empty() {
            println!("🎨 app.wxss loaded ({} 字节)", app_wxss.len());
        }
        
        let custom_tabbar = if app_config.tab_bar.as_ref().map(|tb| tb.custom).unwrap_or(false) {
            load_custom_tabbar_with_app_wxss(&app_wxss)?
        } else { None };
        
        // tabBar 高度以组件 WXSS 实测为准（微信同样由组件自身决定），
        // 写死常量会让内容视口和固定层与 H5 差几个像素。
        if let Some(ct) = &custom_tabbar {
            let probe = WxmlRenderer::new(ct.stylesheet.clone(), LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32);
            let measured = probe.measure_content_height(&ct.wxml_nodes, &ct.data);
            if measured > 1.0 {
                set_tabbar_height(measured.round() as u32);
                println!("📐 自定义 tabBar 高度: {:.0}px", measured);
            }
        }
        
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
            renderer: None, tabbar_renderer: None, text_renderer: None, app_wxss,
            page_stack: Vec::new(), pages, app_config, custom_tabbar,
            mouse_pos: (0.0, 0.0), needs_redraw: true, scale_factor: 1.0,
            scroll: { let vp = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
                // 初始内容高 = 视口高（max_scroll 为 0）：真实内容高要等第一帧渲染后才知道。
                // 从前这里塞的是写死的 1500，于是内容不足一屏的页面在首帧前也能拉出 800+ 像素空白。
                ScrollController::new(vp, vp) },
            last_frame: now, click_start_pos: (0.0, 0.0), click_start_time: now,
            pending_navigation: None, interaction: InteractionManager::new(),
            modifiers: winit::keyboard::ModifiersState::empty(),
            clipboard: arboard::Clipboard::new().ok(),
            toast: None, loading: None, modal: None,
            frame_interval: Duration::from_micros(16_667), // 默认 60Hz，resumed 后按显示器实际刷新率修正
            last_present: now,
            fps_log: std::env::var("MINI_FPS").is_ok(),
            fps_window_start: now,
            fps_frames: 0,
            fps_worst_ms: 0.0,
            fps_worst_parts: (0.0, 0.0, 0.0),
            render_parts: (0.0, 0.0, 0.0),
            fps_worst_render_parts: (0.0, 0.0, 0.0),
            last_animated_bounds: Vec::new(),
            fixed_layer_animates: false,
            started_at: now,
            drawn_band: None,
            pull_refreshing: false,
            page_data: std::sync::Arc::new(json!({})),
            page_data_dirty: true,
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

    /// 是否存在持续动画/需要连续帧的状态（滚动、惯性、视频、光标闪烁、定时器、弹层等）。
    /// 用于决定事件循环是「按刷新率连续出帧」还是「空闲休眠（0 CPU）」。
    fn is_animating(&self) -> bool {
        let scrolling = self.scroll.is_animating() || self.scroll.is_dragging;
        let sv_scroll = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
        // CSS @keyframes / switch 拨动过渡：渲染器在上一帧求值时标记，
        // 有动画在跑就继续按刷新率出帧（否则动画只会画出第一帧然后停住）
        let css_anim = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false)
            || self.tabbar_renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
        scrolling
            || sv_scroll
            || css_anim
            || self.interaction.has_focused_input()
            || self.pull_refreshing // 指示器要持续转
            // 有自动播放的 swiper：即使页面没有 JS 定时器也要按刷新率醒着，
            // 否则到点该翻页时没人来推进它的状态
            || mini_render::renderer::components::has_autoplay_swiper()
            || self.app.has_active_timers()
            || self.toast.as_ref().map(|t| t.visible).unwrap_or(false)
            || self.loading.as_ref().map(|l| l.visible).unwrap_or(false)
            || self.modal.as_ref().map(|m| m.visible).unwrap_or(false)
            || mini_render::renderer::components::has_playing_video()
    }

    /// 按闸门决定这一帧要不要重绘，以及走整帧还是损伤区。
    ///
    /// - `structural`：数据/滚动/交互引起的变化，必须整帧重绘。
    /// - `css_anim`：只有 CSS/JS 动画在跑，走损伤区：只清并只画动画元素的包围盒。
    ///   一个 `infinite` 的小徽标不该逼着整屏每帧重新光栅化 ——
    ///   浏览器靠图层合成避免这件事，这里用裁剪矩形达到同样效果。
    fn render_frame_if_needed(&mut self, structural: bool, css_anim: bool) {
        if !(structural || css_anim) {
            return;
        }
        let damage = if structural { None } else { self.animation_damage_rect() };
        self.render_with_damage(damage);
        self.needs_redraw = false;
    }

    /// 跑一帧「逻辑 + 渲染 + 上屏」，与交互窗体 `RedrawRequested` 走同一套闸门。
    /// 供 `--frames` 用：局部重绘这类只在连续出帧时才暴露的问题，单帧快照测不到。
    fn pump_one_frame(&mut self) {
        self.app.update().ok();
        if self.app.take_data_dirty() {
            self.needs_redraw = true;
            self.page_data_dirty = true;
        }
        let mut pull_req: Option<bool> = None;
        evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req);
        self.apply_pull_down_request(pull_req);
        self.update_scroll();
        if self.scroll.take_pull_trigger() && self.page_enables_pull_down() {
            self.set_pull_refreshing(true);
        }
        if !self.viewport_inside_drawn_band() {
            self.needs_redraw = true;
        }
        let scrolling = self.scroll.is_animating() || self.scroll.is_dragging;
        let sv_scroll = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
        let css_anim = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
        let structural = self.needs_redraw
            || mini_render::renderer::components::has_playing_video()
            || sv_scroll
            || self.interaction.has_focused_input()
            || scrolling
            || mini_render::renderer::components::swiper_needs_frame();
        self.render_frame_if_needed(structural, css_anim);
    }

    /// 上一帧动画元素包围盒的并集，作为本帧的损伤区。
    ///
    /// 返回 None 表示「不值得做局部重绘」：没有记录、或并集已经占到视口的三成以上
    /// （那时局部重绘省不下多少，还要多付一次裁剪判断）。
    fn animation_damage_rect(&self) -> Option<GeoRect> {
        if self.fixed_layer_animates {
            return None; // 覆盖层在动，必须整帧重绘
        }
        let bounds = &self.last_animated_bounds;
        if bounds.is_empty() {
            return None;
        }
        let (mut x0, mut y0) = (f32::MAX, f32::MAX);
        let (mut x1, mut y1) = (f32::MIN, f32::MIN);
        for r in bounds {
            x0 = x0.min(r.x);
            y0 = y0.min(r.y);
            x1 = x1.max(r.x + r.width);
            y1 = y1.max(r.y + r.height);
        }
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let sf = self.scale_factor as f32;
        let viewport_area = (LOGICAL_WIDTH as f32 * sf) * (LOGICAL_HEIGHT as f32 * sf);
        if (x1 - x0) * (y1 - y0) > viewport_area * 0.3 {
            return None;
        }
        Some(GeoRect::new(x0, y0, x1 - x0, y1 - y0))
    }

    /// 上屏要用的那段画布是否已经画过（画布之外的行由上屏填背景色，不算未绘制）
    fn viewport_inside_drawn_band(&self) -> bool {
        let Some((band_top, band_bottom)) = self.drawn_band else { return false };
        let Some(canvas) = &self.canvas else { return false };
        let sf = self.scale_factor as f32;
        let has_tabbar = self
            .page_stack
            .last()
            .map(|p| self.is_tabbar_page(&p.path))
            .unwrap_or(false);
        let viewport = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
        let top = (self.scroll.get_position() * sf).max(0.0);
        let bottom = (self.scroll.get_position() * sf + viewport * sf).min(canvas.height() as f32);
        top >= band_top - 0.01 && bottom <= band_bottom + 0.01
    }

    /// 当前页是否声明了 `enablePullDownRefresh`
    fn page_enables_pull_down(&self) -> bool {
        self.page_stack
            .last()
            .and_then(|p| self.pages.get(&p.path))
            .map(|info| info.enable_pull_down_refresh)
            .unwrap_or(false)
    }

    /// 进入/退出下拉刷新态。
    ///
    /// 进入时把内容按住在露出指示器的位置（等价 iOS 的 contentInset.top），
    /// 并回调 `onPullDownRefresh`；退出时收回 inset，内容回弹归位。
    fn set_pull_refreshing(&mut self, on: bool) {
        if self.pull_refreshing == on {
            return;
        }
        self.pull_refreshing = on;
        self.scroll
            .set_top_inset(if on { PULL_REFRESH_HEIGHT } else { 0.0 });
        self.needs_redraw = true;
        if on {
            println!("📜 onPullDownRefresh");
            self.app
                .eval("if(__currentPage && __currentPage.onPullDownRefresh) __currentPage.onPullDownRefresh()")
                .ok();
            print_js_output(&self.app);
        }
    }

    /// 处理逻辑层的 `wx.startPullDownRefresh` / `wx.stopPullDownRefresh`
    fn apply_pull_down_request(&mut self, request: Option<bool>) {
        match request {
            Some(true) => {
                if self.page_enables_pull_down() {
                    self.set_pull_refreshing(true);
                }
            }
            Some(false) => self.set_pull_refreshing(false),
            None => {}
        }
    }

    fn navigate_to(&mut self, path: &str, query: HashMap<String, String>) -> Result<(), String> {
        let path = path.trim_start_matches('/');
        let page_info = self.pages.get(path).ok_or_else(|| format!("Page not found: {}", path))?;
        
        let mut wxml_parser = WxmlParser::new(&page_info.wxml);
        let wxml_nodes = remove_manual_tabbar(&wxml_parser.parse().map_err(|e| format!("WXML error: {}", e))?);
        
        // app.wxss 在前、页面 WXSS 在后：同特异性时页面样式因书写顺序更靠后而胜出
        let merged_wxss = format!("{}\n{}", self.app_wxss, page_info.wxss);
        let mut wxss_parser = WxssParser::new(&merged_wxss);
        let stylesheet = wxss_parser.parse().map_err(|e| format!("WXSS error: {}", e))?;
        
        // 先登记路由：页面实例的 `this.route` / `getCurrentPages()` 都依赖它，
        // 返回时也靠它判断逻辑层是否已经出过栈
        self.app.eval(&format!("__setPendingRoute({})", serde_json::to_string(path).unwrap_or_else(|_| "''".into()))).ok();
        self.app.load_script(&page_info.js)?;
        let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
        self.app.eval(&format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json)).ok();
        self.app.eval("if(__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
        // onReady：微信在首次渲染完成后触发；编译端取数据快照时也会走这一步，
        // 窗体不调用会导致两端初始数据不同（例如在 onReady 里补数据的页面）。
        self.app.eval("if(__currentPage && __currentPage.onReady) __currentPage.onReady()").ok();
        print_js_output(&self.app);
        
        self.page_stack.push(PageInstance { path: path.to_string(), query, wxml_nodes, stylesheet });
        // 换页必须重取数据快照：新页面的 onLoad 里可能一次 setData 都没有，
        // 那样缓存里还是上一页的数据。
        self.page_data_dirty = true;
        
        let has_tabbar = self.is_tabbar_page(path);
        self.scroll = { let vp = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
                // 初始内容高 = 视口高（max_scroll 为 0）：真实内容高要等第一帧渲染后才知道。
                // 从前这里塞的是写死的 1500，于是内容不足一屏的页面在首帧前也能拉出 800+ 像素空白。
                ScrollController::new(vp, vp) };
        self.needs_redraw = true;
        println!("✅ Page loaded: {}", path);
        Ok(())
    }
    
    /// 返回上一页（微信 navigateBack 语义）。
    ///
    /// 关键点：**不重新加载上一页的 JS**。页面实例还在逻辑层的页面栈里，重载脚本
    /// 会重新执行 `Page({...})` 并再跑一遍 `onLoad`，用户在上一页的状态（筛选、
    /// 滚动位置、已填表单）全部丢失，而且会重复触发"仅首次进入"的逻辑。
    /// 正确做法是让逻辑层出栈、恢复原实例，只补一次 `onShow`。
    fn navigate_back(&mut self) -> Result<(), String> {
        if self.page_stack.len() <= 1 { return Ok(()); }
        // 离开当前页：先给它 onUnload
        self.app.eval("if(__currentPage && __currentPage.onUnload) __currentPage.onUnload()").ok();
        self.page_stack.pop();
        self.interaction.clear_page_state();
        
        if let Some(page) = self.page_stack.last() {
            let path = page.path.clone();
            // 逻辑层出栈并恢复上一页实例（wx.navigateBack 触发时 JS 已自行出栈，
            // 这里用当前路由校验：不一致才补一次出栈，避免重复 pop）
            let current_route = self.app.eval("(__currentPage && __currentPage.route) || ''").unwrap_or_default();
            if current_route.trim_matches('"') != path {
                self.app.eval("__popPage(1)").ok();
            }
            self.app.eval("if(__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
            print_js_output(&self.app);
            
            let has_tabbar = self.is_tabbar_page(&path);
            self.scroll = { let vp = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
                // 初始内容高 = 视口高（max_scroll 为 0）：真实内容高要等第一帧渲染后才知道。
                // 从前这里塞的是写死的 1500，于是内容不足一屏的页面在首帧前也能拉出 800+ 像素空白。
                ScrollController::new(vp, vp) };
            println!("↩️  返回: {}", path);
        }
        // 恢复的是上一页实例，数据快照要重取（返回后可能一次 setData 都没有）
        self.page_data_dirty = true;
        self.needs_redraw = true;
        Ok(())
    }
    
    fn switch_tab(&mut self, path: &str) -> Result<(), String> {
        self.page_stack.clear();
        self.interaction.clear_page_state();
        // 切 tab 会销毁原页面栈（微信语义），逻辑层的栈也要一起清，否则越切越长
        self.app.eval("__resetPageStack()").ok();
        self.navigate_to(path.trim_start_matches('/'), HashMap::new())
    }
    
    fn setup_canvas(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
        let (pw, ph) = ((LOGICAL_WIDTH as f64 * scale_factor) as u32, (CONTENT_HEIGHT as f64 * scale_factor) as u32);
        self.canvas = Some(Canvas::new(pw, ph));
        self.tabbar_canvas = Some(Canvas::new(pw, (tabbar_height() as f64 * scale_factor) as u32));
        self.fixed_canvas = Some(Canvas::new(pw, (LOGICAL_HEIGHT as f64 * scale_factor) as u32));
        // 共享进程内唯一字体实例（加载一次约 0.9s，若每次切页都重载会造成明显卡顿）
        self.text_renderer = mini_render::text::shared_fonts();
    }
    
    fn update_renderers(&mut self) {
        if let Some(page) = self.page_stack.last() {
            self.renderer = Some(WxmlRenderer::new_with_scale(page.stylesheet.clone(), LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32, self.scale_factor as f32));
            if let Some(ref ct) = self.custom_tabbar {
                self.tabbar_renderer = Some(WxmlRenderer::new_with_scale(ct.stylesheet.clone(), LOGICAL_WIDTH as f32, tabbar_height() as f32, self.scale_factor as f32));
            }
        }
    }

    fn render(&mut self) {
        self.render_with_damage(None);
    }

    /// `damage` 为 `Some(rect)` 时只重绘该矩形（动画帧的局部重绘），
    /// 其余像素保留上一帧结果，并且不更新「已绘制条带」。
    fn render_with_damage(&mut self, damage: Option<GeoRect>) {
        // 页面数据只在逻辑层 setData 之后才会变，所以只有脏了才做这趟
        // 「JS 侧 JSON.stringify 整份 data → Rust 侧反序列化」往返。
        // 从前每帧都做一次：动画帧里白付一次全量序列化。
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
        }
        if self.page_data_dirty {
            self.page_data = std::sync::Arc::new(
                self.app
                    .eval("__getPageData()")
                    .map(|s| serde_json::from_str(&s).unwrap_or(json!({})))
                    .unwrap_or(json!({})),
            );
            self.page_data_dirty = false;
        }
        // Arc 克隆：只加引用计数，不复制整棵 JSON（渲染期同时要可变借用 canvas/renderer）
        let page_data = std::sync::Arc::clone(&self.page_data);
        let page_data = &*page_data;
        let page = match self.page_stack.last() { Some(p) => p, None => return };
        let (current_path, has_tabbar) = (page.path.clone(), self.is_tabbar_page(&page.path));
        let viewport_height = (LOGICAL_HEIGHT - if has_tabbar { tabbar_height() } else { 0 }) as f32;
        let scroll_offset = self.scroll.get_position();
        
        // 只清理「渲染器实际会画的那条带」：视口 ± 裁剪余量。
        // 页面画布是整页高的（首页 6870px），每帧全量 clear 白烧几毫秒。
        let sf = self.scale_factor as f32;
        let margin = mini_render::renderer::VIEWPORT_CULL_MARGIN_PX;
        let band_y0 = (scroll_offset * sf - margin).floor() as i32;
        let band_y1 = (scroll_offset * sf + viewport_height * sf + margin).ceil() as i32;

        let t_render_begin = Instant::now();
        let mut content_height = 0.0f32;
        if let Some(canvas) = &mut self.canvas {
            match &damage {
                Some(rect) => canvas.clear_area(rect, Color::from_hex(0xF5F5F5)),
                None => canvas.clear_band(band_y0, band_y1, Color::from_hex(0xF5F5F5)),
            }
            if let Some(renderer) = &mut self.renderer {
                renderer.set_damage_clip(damage);
                content_height = renderer.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
            }
        }
        if let Some(r) = &mut self.renderer {
            let bounds = r.take_animated_bounds();
            if !bounds.is_empty() || damage.is_none() {
                self.last_animated_bounds = bounds;
            }
        }
        
        if content_height > 0.0 {
            self.scroll.update_content_height(content_height, viewport_height);
            let required_height = (content_height * self.scale_factor as f32).ceil() as u32;
            if self.canvas.as_ref().map(|c| c.height()).unwrap_or(0) != required_height && required_height > 0 {
                self.canvas = Some(Canvas::new((LOGICAL_WIDTH as f64 * self.scale_factor) as u32, required_height));
                if let Some(page) = self.page_stack.last() {
                    if let (Some(canvas), Some(renderer)) = (&mut self.canvas, &mut self.renderer) {
                        canvas.clear_band(band_y0, band_y1, Color::from_hex(0xF5F5F5));
                        renderer.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
                    }
                }
            }
        }
        
        let t_page_done = Instant::now();
        // 局部重绘帧里 fixed 覆盖层与 tabBar 不会变，直接沿用上一帧的画布 ——
        // 除非覆盖层自己带动画（那时 animation_damage_rect 会拒绝走局部路径）。
        if damage.is_none() {
            if let Some(page) = self.page_stack.last() {
                if let (Some(fc), Some(r)) = (&mut self.fixed_canvas, &mut self.renderer) {
                    // 在两趟之间清零，才能分辨「动画在页面里」还是「在 fixed 覆盖层里」
                    let page_anim = r.has_active_animations();
                    r.set_animations_active(false);
                    fc.clear(Color::new(0, 0, 0, 0));
                    r.render_fixed_elements(fc, &page.wxml_nodes, &page_data, &mut self.interaction, viewport_height);
                    let fixed_anim = r.has_active_animations();
                    r.set_animations_active(page_anim || fixed_anim);
                    // 覆盖层自己带动画时，局部重绘会让它停住 —— 禁用局部路径
                    self.fixed_layer_animates = fixed_anim;
                }
            }
        }
        let t_fixed_done = Instant::now();
        
        if has_tabbar && damage.is_none() {
            if self.is_custom_tabbar() { self.render_custom_tabbar(&current_path); }
            else { self.render_native_tabbar(&current_path); }
        }
        // 记下这一帧真正画过的行区间（夹到画布内；画布之外由上屏填背景色）
        let canvas_h = self.canvas.as_ref().map(|c| c.height() as f32).unwrap_or(0.0);
        self.drawn_band = Some((
            (band_y0 as f32).max(0.0),
            (band_y1 as f32).min(canvas_h),
        ));
        self.render_parts = (
            (t_page_done - t_render_begin).as_secs_f32() * 1000.0,
            (t_fixed_done - t_page_done).as_secs_f32() * 1000.0,
            t_fixed_done.elapsed().as_secs_f32() * 1000.0,
        );
        if std::env::var("MINI_SCROLL_LOG").is_ok() {
            eprintln!(
                "📐 {} 内容高 {:.1} 视口 {:.1} 画布高 {} 滚动 {:.1}/{:.1}",
                current_path, content_height, viewport_height,
                self.canvas.as_ref().map(|c| c.height()).unwrap_or(0),
                self.scroll.get_position(), self.scroll.get_max_scroll()
            );
        }
    }
    
    fn render_custom_tabbar(&mut self, current_path: &str) {
        let tb = match &self.app_config.tab_bar { Some(tb) => tb.clone(), None => return };
        // 数据以「组件自身 data」为准（微信语义）：iconType 等字段只存在于组件里，
        // 只用 app.json 拼 list 会丢掉图标类型，导致所有 tab 图标退化成默认样式。
        let mut data = self
            .custom_tabbar
            .as_ref()
            .map(|ct| ct.data.clone())
            .unwrap_or_else(|| json!({}));
        if !data.is_object() {
            data = json!({});
        }
        let has_list = data.get("list").and_then(|l| l.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
        if let Some(obj) = data.as_object_mut() {
            if !has_list {
                // 组件没有提供 list 时，退回 app.json 的配置
                let list: Vec<serde_json::Value> = tb.list.iter()
                    .map(|i| json!({"pagePath": i.page_path, "text": i.text}))
                    .collect();
                obj.insert("list".to_string(), json!(list));
            }
            obj.insert(
                "selected".to_string(),
                json!(self.get_tabbar_index(current_path).unwrap_or(0)),
            );
        }
        let ct = match &self.custom_tabbar { Some(ct) => ct, None => return };
        if let (Some(c), Some(r)) = (&mut self.tabbar_canvas, &mut self.tabbar_renderer) {
            c.clear(Color::WHITE);
            r.render(c, &ct.wxml_nodes, &data);
        }
    }
    
    fn render_native_tabbar(&mut self, current_path: &str) {
        let tb = match &self.app_config.tab_bar { Some(tb) => tb.clone(), None => return };
        if let (Some(c), Some(tr)) = (&mut self.tabbar_canvas, self.text_renderer.as_deref()) {
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
                        if has_tabbar { (tabbar_height() as f64 * self.scale_factor) as u32 } else { 0 });
                    // 下拉刷新指示器画在页面之上、Toast/Modal 之下
                    let gap = self.scroll.top_gap();
                    if gap > 0.5 || self.pull_refreshing {
                        app_window::render::render_pull_indicator(
                            &mut buffer, size.width, size.height, self.scale_factor as f32,
                            gap, self.pull_refreshing, self.scroll.pull_progress(),
                            // 相位要用「进程启动至今」的时钟：last_frame 每帧都会被重置，
                            // 拿它算 elapsed 恒为 0，三点看起来是静止的。
                            (self.started_at.elapsed().as_secs_f32() * 1.1).fract(),
                        );
                    }
                    render_ui_overlay(&mut buffer, size.width, size.height, self.scale_factor as f32, self.last_frame,
                        &toast_state, &loading_state, &modal_state, self.text_renderer.as_deref());
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
        let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - tabbar_height()) as f32 } else { LOGICAL_HEIGHT as f32 };
        
        if has_tabbar && y >= tabbar_y {
            let nav = if self.is_custom_tabbar() {
                click::handle_custom_tabbar_click(x, y - tabbar_y, self.tabbar_renderer.as_ref(), &page.path)
            } else {
                self.app_config.tab_bar.as_ref().and_then(|tb| click::handle_native_tabbar_click_wrapper(x, tb, &page.path))
            };
            if let Some(n) = nav { self.pending_navigation = Some(n); if let Some(w) = &self.window { w.request_redraw(); } }
        } else {
            if let Some(nav) = click::handle_content_click(x, y, &self.scroll, has_tabbar, &mut self.interaction,
                self.renderer.as_ref(), &mut self.app, self.scale_factor, self.text_renderer.as_deref(), self.window.as_ref(), &mut self.clipboard) {
                self.pending_navigation = Some(nav);
            }
            self.needs_redraw = true;
        }
    }
    
    fn handle_modal_press(&mut self, x: f32, y: f32) -> bool {
        let modal = match &self.modal { Some(m) if m.visible => m, _ => return false };
        let layout = click::calculate_modal_layout(modal, self.scale_factor as f32, self.text_renderer.as_deref());
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
        let layout = click::calculate_modal_layout(modal, self.scale_factor as f32, self.text_renderer.as_deref());
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
                NavigationRequest::RedirectTo { url } => {
                    // 关闭当前页再开新页：栈深度不变
                    let (p, q) = parse_url(&url);
                    self.app.eval("if(__currentPage && __currentPage.onUnload) __currentPage.onUnload()").ok();
                    self.page_stack.pop();
                    self.app.eval("__popPage(1)").ok();
                    self.interaction.clear_page_state();
                    self.navigate_to(&p, q).ok();
                }
                NavigationRequest::ReLaunch { url } => {
                    let (p, q) = parse_url(&url);
                    self.page_stack.clear();
                    self.interaction.clear_page_state();
                    self.app.eval("__resetPageStack()").ok();
                    self.navigate_to(&p, q).ok();
                }
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
        // 依据显示器刷新率设定每帧间隔，适配 60/120/144Hz 等高刷屏
        let millihertz = window.current_monitor()
            .and_then(|m| m.refresh_rate_millihertz())
            .filter(|&hz| hz > 0)
            .unwrap_or(60_000);
        self.frame_interval = Duration::from_secs_f64(1000.0 / millihertz as f64);
        println!("🖥️  显示器刷新率: {:.1}Hz -> 每帧 {:.2}ms", millihertz as f64 / 1000.0, self.frame_interval.as_secs_f64() * 1000.0);
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
                if evt::handle_cursor_moved(x, y, &mut self.interaction, &mut self.scroll, self.text_renderer.as_deref(),
                    self.window.as_ref(), self.renderer.as_ref(), &mut self.app, &mut self.clipboard, self.scale_factor) {
                    self.needs_redraw = true;
                }
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseWheel { delta, phase, .. } => {
                if evt::handle_mouse_wheel(delta, self.mouse_pos, &mut self.interaction, &mut self.scroll, self.scale_factor) {
                    self.needs_redraw = true;
                }
                // 触控板抬手/取消：越界立即回弹（鼠标滚轮没有这个阶段，由控制器的静默计时兜底）
                if matches!(phase, winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled) {
                    if self.scroll.end_wheel_gesture() { self.needs_redraw = true; }
                    for c in self.interaction.scroll_controllers.values_mut() {
                        c.end_wheel_gesture();
                    }
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
                    let tabbar_y = if has_tabbar { (LOGICAL_HEIGHT - tabbar_height()) as f32 } else { LOGICAL_HEIGHT as f32 };
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
                        // 任何可点元素都进入按压态（`:active` / `hover-class` 靠它生效），
                        // 滚动区域除外 —— 那是拖动不是按压
                        if !el.disabled && el.interaction_type != InteractionType::ScrollArea {
                            self.interaction.set_button_pressed(el.id.clone(), el.bounds);
                            self.needs_redraw = true;
                        }
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
                                    // 根据滚动方向使用 x 或 y
                                    let drag_pos = if el.is_horizontal { x } else { y };
                                    c.begin_drag(drag_pos, ts);
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
                let frame_begin = Instant::now();
                self.app.update().ok();
                print_js_output(&self.app);
                // 逻辑层这一帧改过数据就必须重绘。没有这一步的话，定时器/网络回调
                // 里的 setData 只在「刚好还有 CSS 动画在跑」时才顺带上屏，
                // 纯 JS 驱动的页面（秒杀倒计时、轮询刷新）会一直显示旧值。
                if self.app.take_data_dirty() {
                    self.needs_redraw = true;
                    self.page_data_dirty = true;
                }
                
                let mut pull_req: Option<bool> = None;
                if evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req) { self.needs_redraw = true; }
                self.apply_pull_down_request(pull_req);
                // 每帧轮询一次导航请求：此前只在「内容区被点击」时检查，
                // 于是 setTimeout / 网络回调里发起的 wx.navigateTo / navigateBack
                // 永远不会被宿主取走（登录成功 800ms 后自动返回就是这么失效的）。
                if self.pending_navigation.is_none() {
                    if let Some(nav) = app_window::check_navigation(&mut self.app) {
                        self.pending_navigation = Some(nav);
                        self.needs_redraw = true;
                    }
                }
                if evt::update_toast_timeout(&mut self.toast) { self.needs_redraw = true; }
                
                self.update_scroll();
                // 下拉到位并松手：只有页面声明了 enablePullDownRefresh 才进入刷新态
                // （微信里回弹一直有，指示器与回调由这个开关决定）
                if self.scroll.take_pull_trigger() && self.page_enables_pull_down() {
                    self.set_pull_refreshing(true);
                }
                self.process_navigation();
                // 不变量：上屏要用的那段画布必须是已经画过的。
                // 页面画布只画「视口 ± 裁剪余量」，所以滚动出这条带就必须重画，
                // 否则上屏取到没画过的区域（表现为滑动时一片空白，停下才出现内容）。
                //
                // 反过来说：只要还在这条带内，滚动**不需要重绘** —— 页面画布用的是
                // 内容坐标，滚动只是取不同的切片，上屏本身就是一次逐行拷贝。
                // 这是滑动流畅度的关键：滚动期间大部分帧只花上屏的 3~4ms，
                // 而不是每帧重画整屏的 8~14ms（帧时间抖动就是「卡卡的」来源）。
                if !self.viewport_inside_drawn_band() {
                    self.needs_redraw = true;
                }
                
                let scrolling = self.scroll.is_animating() || self.scroll.is_dragging;
                let sv_scroll = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
                let css_anim = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
                let t_logic = frame_begin.elapsed();
                // swiper 自动播放到点/正在滑动时也要重绘（它的状态在绘制期推进）
                let swiper_frame = mini_render::renderer::components::swiper_needs_frame();
                let structural = self.needs_redraw
                    || mini_render::renderer::components::has_playing_video()
                    || sv_scroll
                    || self.interaction.has_focused_input()
                    || scrolling
                    || swiper_frame;
                self.render_frame_if_needed(structural, css_anim);
                let t_render = frame_begin.elapsed();
                self.present();
                if self.fps_log {
                    let total = frame_begin.elapsed();
                    if total.as_secs_f32() * 1000.0 > self.fps_worst_ms {
                        self.fps_worst_parts = (
                            t_logic.as_secs_f32() * 1000.0,
                            (t_render - t_logic).as_secs_f32() * 1000.0,
                            (total - t_render).as_secs_f32() * 1000.0,
                        );
                        self.fps_worst_render_parts = self.render_parts;
                    }
                }
                self.last_present = Instant::now();
                // 本帧真正花在「逻辑 + 渲染 + 上屏」上的时间（不含为限速而睡的时间）
                let work_ms = frame_begin.elapsed().as_secs_f32() * 1000.0;
                // 动画期间自己续订下一帧：只靠 about_to_wait 的定时唤醒时，
                // 每个周期要多绕一次事件循环，实测在 144Hz 屏上只能跑到 ~47FPS
                // （单帧渲染其实只要 4ms）。
                // 续帧前睡到下一个刷新时点 —— softbuffer 的 present 不阻塞垂直同步，
                // 不限速会以 2~4 倍刷新率空转，白烧一个核画没人看得见的帧。
                // 时点要从**帧开始**算：从 present 之后算的话，每帧会多出一整个
                // 渲染耗时（16.7ms 节拍变成 21ms，只剩 46FPS）。
                if self.is_animating() {
                    let target = frame_begin + self.frame_interval;
                    let now = Instant::now();
                    if now < target {
                        std::thread::sleep(target - now);
                    }
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
                if self.fps_log {
                    let ms = work_ms;
                    self.fps_frames += 1;
                    self.fps_worst_ms = self.fps_worst_ms.max(ms);
                    if self.fps_window_start.elapsed().as_secs_f32() >= 1.0 {
                        let (l, r, p) = self.fps_worst_parts;
                        let (rp, rf, rt) = self.fps_worst_render_parts;
                        println!(
                            "📊 {} 帧/秒，最慢一帧 {:.1}ms（逻辑 {:.1} 渲染 {:.1}[页面 {:.1} 覆盖层 {:.1} tabBar {:.1}] 上屏 {:.1}）{}",
                            self.fps_frames,
                            self.fps_worst_ms,
                            l, r, rp, rf, rt, p,
                            if self.is_animating() { "（动画中）" } else { "" }
                        );
                        self.fps_window_start = Instant::now();
                        self.fps_frames = 0;
                        self.fps_worst_ms = 0.0;
                        self.fps_worst_parts = (0.0, 0.0, 0.0);
                        self.fps_worst_render_parts = (0.0, 0.0, 0.0);
                    }
                }
                // 后续帧的调度交给 about_to_wait：动画中按刷新率 WaitUntil，空闲则 Wait 休眠。
            }
            _ => {}
        }
    }

    /// 定时唤醒（WaitUntil 到期）时触发下一帧重绘，实现按刷新率的稳定出帧节流。
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            if let Some(w) = &self.window { w.request_redraw(); }
        }
    }

    /// 事件处理完毕后决定控制流：
    /// - 有动画：按显示器刷新率安排下一帧（WaitUntil），保证 60/120Hz 平滑且不空转。
    /// - 空闲：Wait 休眠，CPU 占用降为 0，直到下一个输入事件。
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.needs_redraw {
            if let Some(w) = &self.window { w.request_redraw(); }
        }
        if self.is_animating() || self.pending_navigation.is_some() {
            let target = self.last_present + self.frame_interval;
            event_loop.set_control_flow(ControlFlow::WaitUntil(target));
        } else if !self.needs_redraw {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

impl MiniAppWindow {
    /// 无窗口模式：按**与交互窗体完全相同的管线**把每个页面合成成整帧图片。
    ///
    /// 这是「窗体是否忠实还原小程序」的可验证入口 —— 页面加载、样式合并、
    /// 自定义 tabBar、fixed 覆盖层、Toast/Modal 外壳、像素合成顺序都与真实运行一致，
    /// 因此产出的 PNG 可以直接和编译出的 H5 截图做像素级对比。
    fn snapshot_all(&mut self, out_dir: &std::path::Path, scale: f64, time: Option<f32>, settle: Option<f32>, only: Option<&str>, scroll: f32, evals: &[String], frames: u32) -> Result<usize, String> {
        self.setup_canvas(scale);
        let routes: Vec<String> = match only {
            Some(route) => vec![route.trim_start_matches('/').to_string()],
            None => self.app_config.pages.clone(),
        };
        let mut written = 0usize;
        for route in routes {
            if !self.pages.contains_key(&route) {
                eprintln!("⚠️  跳过 {}（页面未加载）", route);
                continue;
            }
            // 构造函数已经加载过首页：重复 navigate 会让 onLoad 跑第二遍，
            // 依赖「首次进入」的逻辑（如只弹一次的新人券）会被吃掉，两端数据就不一致了。
            let already_loaded = self.page_stack.last().map(|p| p.path == route).unwrap_or(false);
            if !already_loaded {
                self.page_stack.clear();
                self.interaction.clear_page_state();
                self.navigate_to(&route, HashMap::new())?;
            }
            self.update_renderers();
            // `--settle`：让**真实时间**流过给定时长，把逻辑层真正跑起来 ——
            // 不等的话 setInterval / setTimeout 一次都不会触发，秒杀倒计时、
            // 延时弹层、轮播自动播放在快照里全停在 0s 的样子（和真机不符）。
            //
            // 和 `--time` 分开是有意的：双端对比时 H5 侧刻意禁用了页面脚本
            // （见 examples/compare.rs 的 inject_capture_override，为了截图确定性），
            // 那条链路只需要把动画时钟拨到位，不能让 JS 状态往前跑。
            if let Some(secs) = settle {
                let deadline = Instant::now() + Duration::from_secs_f32(secs);
                while Instant::now() < deadline {
                    self.app.update().ok();
                    print_js_output(&self.app);
                    { let mut pull_req: Option<bool> = None;
                        evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req);
                        self.apply_pull_down_request(pull_req); }
                    evt::update_toast_timeout(&mut self.toast);
                    if self.pending_navigation.is_none() {
                        self.pending_navigation = app_window::check_navigation(&mut self.app);
                    }
                    if self.pending_navigation.is_some() {
                        self.process_navigation();
                    }
                    std::thread::sleep(Duration::from_millis(8)); // ≈120Hz，与真机出帧节奏同量级
                }
            }
            if let Some(secs) = time {
                if let Some(r) = &mut self.renderer { r.set_animation_time(secs); }
                if let Some(r) = &mut self.tabbar_renderer { r.set_animation_time(secs); }
            }
            // 注入页面状态：把「交互之后」的场景（购物车有商品、开关已打开等）
            // 也纳入可截图、可对比的范围，而不是只能测首次进入的初始态。
            // 放在等待之后执行：页面此时已稳定（入场动画结束、延时弹层已弹出），
            // 脚本里「关掉优惠券弹层」这类操作才不会被随后的 setTimeout 又打开。
            // 逐段执行：每段之后都把导航请求跑完，
            // 于是「点商品进详情 → 返回上一页」这类多步交互可以脚本化验证。
            for script in evals {
                match self.app.eval(script) {
                    Ok(_) => print_js_output(&self.app),
                    Err(e) => eprintln!("⚠️  --eval 执行失败: {}", e),
                }
                for _ in 0..8 {
                    self.app.update().ok();
                    if self.pending_navigation.is_none() {
                        self.pending_navigation = app_window::check_navigation(&mut self.app);
                    }
                    if self.pending_navigation.is_none() {
                        break;
                    }
                    self.process_navigation();
                    print_js_output(&self.app);
                }
            }
            // 真实内容高要渲染一次才知道，而 set_position 会按 max_scroll 夹紧 ——
            // 不先渲染的话 `--scroll` 会被夹到初始上限，深位置截图全都截到同一处。
            if scroll > 0.0 {
                self.render();
            }
            self.scroll.set_position(scroll);
            self.app.update().ok();
            // 先把逻辑层这一批 UI 指令（Toast/Modal/下拉刷新）交给宿主，
            // 再让宿主侧的滚动动画（惯性、边界回弹、下拉刷新的按住/归位）走完 ——
            // 顺序反了的话脚本刚触发的状态在图里还没体现出来。
            {
                let mut pull_req: Option<bool> = None;
                evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req);
                self.apply_pull_down_request(pull_req);
            }
            let anim_deadline = Instant::now() + Duration::from_millis(600);
            while self.scroll.is_animating() && Instant::now() < anim_deadline {
                self.update_scroll();
                self.app.update().ok();
                std::thread::sleep(Duration::from_millis(8));
            }
            // 注入脚本刚触发的动画（`wx.createAnimation`、按压过渡）需要一点真实时间
            // 才能看出效果，否则截到的永远是第 0 帧。只在有 --eval 时等，
            // 双端对比那条链路不受影响（它靠 --time 固定动画时钟）。
            if !evals.is_empty() {
                let deadline = Instant::now() + Duration::from_millis(300);
                while Instant::now() < deadline {
                    self.app.update().ok();
                    // 必须真的出帧：动画的起点是「渲染器第一次看到它」的时刻，
                    // 只 update 不 render 的话最后截到的永远是第 0 帧。
                    self.render();
                    std::thread::sleep(Duration::from_millis(8));
                }
            }
            // `--frames N`：按刷新率跑 N 个**与交互窗体同一套闸门/损伤区逻辑**的帧再截图。
            // 局部重绘这类只在连续出帧时才暴露的问题（比如某个动画元素被漏出损伤区而静止），
            // 只有这样才测得到 —— 单帧快照永远走整帧重绘，看不出来。
            for _ in 0..frames {
                self.pump_one_frame();
                std::thread::sleep(self.frame_interval);
            }
            { let mut pull_req: Option<bool> = None;
                        evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req);
                        self.apply_pull_down_request(pull_req); }
            self.render();

            let (pw, ph) = (
                (LOGICAL_WIDTH as f64 * scale) as u32,
                (LOGICAL_HEIGHT as f64 * scale) as u32,
            );
            let mut buffer = vec![0u32; (pw * ph) as usize];
            // 导航之后当前页可能已不是入口 route
            let route = self.page_stack.last().map(|p| p.path.clone()).unwrap_or(route);
            let has_tabbar = self.is_tabbar_page(&route);
            if let Some(canvas) = &self.canvas {
                present_to_buffer(
                    &mut buffer, pw, ph, canvas,
                    self.fixed_canvas.as_ref(), self.tabbar_canvas.as_ref(),
                    (self.scroll.get_position() * scale as f32) as i32, has_tabbar,
                    if has_tabbar { (tabbar_height() as f64 * scale) as u32 } else { 0 },
                );
            }
            let gap = self.scroll.top_gap();
            if gap > 0.5 || self.pull_refreshing {
                app_window::render::render_pull_indicator(
                    &mut buffer, pw, ph, scale as f32,
                    gap, self.pull_refreshing, self.scroll.pull_progress(), 0.25,
                );
            }
            render_ui_overlay(
                &mut buffer, pw, ph, scale as f32, self.last_frame,
                &self.toast, &self.loading, &self.modal, self.text_renderer.as_deref(),
            );

            let mut rgba = Vec::with_capacity(buffer.len() * 4);
            for px in &buffer {
                rgba.extend_from_slice(&[((px >> 16) & 0xFF) as u8, ((px >> 8) & 0xFF) as u8, (px & 0xFF) as u8, 255]);
            }
            let dir = out_dir.join(&route);
            std::fs::create_dir_all(&dir).map_err(|e| format!("创建 {:?} 失败: {}", dir, e))?;
            let file = dir.join("rust.png");
            image::save_buffer(&file, &rgba, pw, ph, image::ColorType::Rgba8)
                .map_err(|e| format!("写入 {:?} 失败: {}", file, e))?;
            println!("🖼  {} -> {}", route, file.display());
            written += 1;
        }
        Ok(written)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mini App Engine\n");
    
    // 解析命令行参数：<app-dir> [--snapshot <out>] [--scale n] [--time secs] [--route r]
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut app_path: Option<std::path::PathBuf> = None;
    let mut snapshot: Option<std::path::PathBuf> = None;
    let mut scale = 2.0f64;
    let mut anim_time: Option<f32> = None;
    let mut settle: Option<f32> = None;
    let mut frames = 0u32;
    let mut route: Option<String> = None;
    let mut scroll = 0.0f32;
    let mut evals: Vec<String> = Vec::new();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--snapshot" => snapshot = it.next().map(std::path::PathBuf::from),
            "--scale" => scale = it.next().and_then(|v| v.parse().ok()).unwrap_or(2.0),
            "--time" => anim_time = it.next().and_then(|v| v.parse().ok()),
            "--settle" => settle = it.next().and_then(|v| v.parse().ok()),
            "--frames" => frames = it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            "--route" => route = it.next(),
            "--scroll" => scroll = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0),
            "--eval" => { if let Some(v) = it.next() { evals.push(v); } }
            "--help" | "-h" => {
                println!("用法: mini-app-window <小程序目录> [--snapshot <输出目录>] [--scale 2]");
                println!("  --time <秒>    动画时钟位置（CSS @keyframes 求值到该时刻，不消耗真实时间）");
                println!("  --settle <秒>  真实等待该时长，让 setTimeout/setInterval、延时弹层、");
                println!("  --frames <N>   按刷新率跑 N 个与交互窗体同逻辑的帧再截图（验证动画/局部重绘）");
                println!("                 轮播自动播放跑起来（要「和真机一样」的画面时用这个）");
                println!("  --route <页面路径>   --scroll <像素>");
                println!("  --eval <JS>    可重复；在 --settle 之后依次执行，每段跑完导航");
                return Ok(());
            }
            other => {
                let path = std::path::PathBuf::from(other);
                if path.exists() && path.is_dir() {
                    println!("📂 加载小程序: {}", path.display());
                    app_path = Some(path);
                } else {
                    eprintln!("❌ 目录不存在: {}", path.display());
                    return Err(format!("目录不存在: {}", path.display()).into());
                }
            }
        }
    }
    if app_path.is_none() {
        println!("📂 使用内置 sample-app");
    }

    let mut window = MiniAppWindow::new(app_path)?;

    // 快照模式：不开窗口，直接把整帧写成 PNG（用于与 H5 做像素对比）
    if let Some(out) = snapshot {
        let count = window.snapshot_all(&out, scale, anim_time, settle, route.as_deref(), scroll, &evals, frames)?;
        println!("\n✅ 快照完成：{} 个页面 -> {}", count, out.display());
        return Ok(());
    }
    
    let event_loop = EventLoop::new()?;
    // 默认空闲休眠；动画期间由 about_to_wait 按刷新率切换到 WaitUntil。
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut window)?;
    Ok(())
}
