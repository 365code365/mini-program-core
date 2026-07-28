//! 带窗口的小程序运行器 - 支持多页面导航和原生 TabBar

mod app_window;

use app_window::*;
use app_window::ui_overlay::{ToastState, LoadingState, ModalState, render_ui_overlay};
use app_window::event_handler as evt;
use app_window::click_handler as click;

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::renderer::{WxmlRenderer, FramePlan};
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
    /// 覆盖层上一次产生的事件绑定与遮挡区域。
    /// 事件绑定每帧清空重建，而覆盖层可能跳过重绘 —— 跳过的帧要靠这份缓存补回来，
    /// 否则弹窗在那些帧里点击会穿透到下层。
    cached_fixed_bindings: Vec<mini_render::renderer::EventBinding>,
    cached_fixed_regions: Vec<GeoRect>,
    /// 进程启动时刻：用作与帧无关的动画相位时钟
    started_at: Instant,
    /// 下一帧的目标时点（固定节拍累加，不受单帧耗时抖动影响）
    next_frame_at: Instant,
    /// 最近一次滚动位置发生变化的时刻（裁剪余量按它自适应）
    last_scroll_at: Option<Instant>,
    /// 拖动手势仲裁：方向锁定 / 选定滚动目标 / 嵌套传递（见 app_window::gesture）
    gesture: Option<app_window::gesture::DragGesture>,
    /// 这次触摸是用来「停住惯性滚动」的：抬手时不该再算一次点击
    /// （iOS/微信里滑动列表时点一下只是停住，不会激活那一项）
    tap_stops_fling: bool,
    /// 触摸序列状态机：把窗口指针事件变成微信语义的
    /// touchstart/touchmove/touchend/touchcancel/longpress/tap
    touch: app_window::touch::TouchTracker,
    /// 按下瞬间的页面滚动位置：用来判断「滚动是否已经接管这次触摸」
    scroll_pos_at_press: f32,
    /// 当前页面打开的时刻（事件对象的 `timeStamp` 是「页面打开至今的毫秒数」）
    page_opened_at: Instant,
    /// 正在进行的左边缘侧滑返回（跟手位移 + 松手收尾动画）
    edge_back: Option<app_window::edge_back::EdgeBack>,
    /// 最近一次派发给页面的 `onPageScroll` 位置（避免同一位置反复回调）
    last_page_scroll_sent: f32,
    /// 页面栈里**被覆盖的那些页**离开时留下的视口像素。
    /// 侧滑时下面那一页要真的显示出来，而它已经不是当前页、渲染器里也没有它了。
    /// 约定：`back_shots[i]` 对应 `page_stack[i]`（只对被覆盖的页有值）。
    back_shots: Vec<app_window::edge_back::PageShot>,
    /// 这一帧必须**整帧**重绘，不许被 setData 的增量失效范围收窄。
    ///
    /// 远程图片下载完成属于「内容变了但数据没变」：`setData` 的失效范围算不出它，
    /// 于是在「每秒都有 setData」的页面上（闪屏倒计时、首页秒杀），增量范围只覆盖
    /// 那一小块文字，刚到位的图片永远不会被重画 —— 表现就是图片一直是占位图。
    force_full_redraw: bool,
    /// fixed 覆盖层需要重绘（数据变化 / 按压态变化 / 换页 / 画布重建）。
    /// 滚动不影响它 —— 覆盖层是钉在视口上的，重画纯属浪费。
    fixed_dirty: bool,
    /// 帧间隔抖动统计：本统计窗口内最大/最小帧间隔（ms）
    frame_gap_max_ms: f32,
    frame_gap_min_ms: f32,
    last_frame_begin: Option<Instant>,
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
    /// `<picker>` 弹出的底部选择面板（微信里是原生浮层）。None 表示当前没有面板。
    picker_sheet: Option<picker_sheet::PickerSheetState>,
    /// 覆盖层画布上真正画过的物理行区间。`None` = 全透明（上屏可整层跳过）。
    /// 只在覆盖层重绘那一帧重算，其余帧复用。
    fixed_rows: Option<(u32, u32)>,
    /// 上一次告知 softbuffer 的表面尺寸（物理像素），用于跳过每帧重复的 resize
    surface_size: Option<(u32, u32)>,
    /// 整帧重绘归因计数（needs_redraw, swiper, scroll-view, 动画, 损伤区被否），每秒清零
    gate_counts: (u32, u32, u32, u32, u32),
    /// `MINI_NO_DAMAGE=1`：关掉 setData 的增量失效，一律整帧重绘。
    /// 用于逐像素对照验证「局部重绘和整帧重绘结果一致」。
    no_damage: bool,
}

impl MiniAppWindow {
    fn new(app_path: Option<std::path::PathBuf>) -> Result<Self, String> {
        // 登记小程序根目录：页面/组件加载与包内资源（图片）解析都依赖它
        page_loader::set_app_path(
            app_path.unwrap_or_else(|| {
                mini_render::app_dir::default_app()
            }),
        );
        
        let mut app = MiniApp::new(LOGICAL_WIDTH, LOGICAL_HEIGHT)?;
        app.init()?;
        
        // 先把小程序目录里所有 .js 注册成 CommonJS 模块，`require('./x.js')` 才解析得到。
        // uni-app / TS 编译出来的小程序会 require 一个几百 KB 的 vendor 包，
        // 不预注册的话第一行 require 就失败。
        let app_root = page_loader::get_app_path();
        match app.register_all_modules(&app_root) {
            Ok(n) if n > 0 => println!("📦 已注册 {} 个 JS 模块", n),
            Err(e) => eprintln!("⚠️  模块注册失败: {}", e),
            _ => {}
        }

        // 动态加载 app.js（按模块作用域执行：小程序里每个 .js 都是 CommonJS 模块）
        let app_js = page_loader::load_app_js();
        app.load_module_script("app", &app_js)?;
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
            cached_fixed_bindings: Vec::new(),
            cached_fixed_regions: Vec::new(),
            started_at: now,
            next_frame_at: now,
            last_scroll_at: None,
            fixed_dirty: true,
            gesture: None,
            tap_stops_fling: false,
            touch: app_window::touch::TouchTracker::new(),
            scroll_pos_at_press: 0.0,
            page_opened_at: now,
            edge_back: None,
            last_page_scroll_sent: 0.0,
            back_shots: Vec::new(),
            force_full_redraw: false,
            frame_gap_max_ms: 0.0,
            frame_gap_min_ms: f32::MAX,
            last_frame_begin: None,
            drawn_band: None,
            pull_refreshing: false,
            page_data: std::sync::Arc::new(json!({})),
            page_data_dirty: true,
            picker_sheet: None,
            fixed_rows: None,
            surface_size: None,
            gate_counts: (0, 0, 0, 0, 0),
            no_damage: std::env::var("MINI_NO_DAMAGE").is_ok(),
        };
        
        window.navigate_to(&first_page, HashMap::new())?;
        Ok(window)
    }
    
    /// 宿主是否要为这个页面画 tabBar（并为它预留高度）。
    ///
    /// `tabBar.custom = true` 的语义是「**由小程序自己画**」。此时宿主既不画原生
    /// tabBar 也不该预留高度 —— 页面会自带一条。uni-app 这类框架就是把 tabBar
    /// 编译进每个页面的，仓库里连 `custom-tab-bar/` 目录都没有。
    ///
    /// 从前只要「声明了 custom 且我们成功加载到 custom-tab-bar 组件」才认；
    /// 组件不存在时就悄悄退回原生 tabBar，于是页面自带的那条和原生那条**叠在一起**
    /// （tea-app 底部因此出现一排巨大的宋体字压在真正的图标标签上）。
    fn is_tabbar_page(&self, path: &str) -> bool {
        let Some(tb) = self.app_config.tab_bar.as_ref() else { return false };
        if tb.custom && self.custom_tabbar.is_none() {
            return false; // 小程序自己画，宿主完全不参与
        }
        tb.list.iter().any(|item| item.page_path == path)
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
            // 侧滑返回的收尾动画（推出去/滑回来）要连续出帧
            || self.edge_back.as_ref().map(|e| e.is_settling()).unwrap_or(false)
            || self.interaction.has_focused_input()
            || self.pull_refreshing // 指示器要持续转
            // 有自动播放的 swiper：即使页面没有 JS 定时器也要按刷新率醒着，
            // 否则到点该翻页时没人来推进它的状态
            || mini_render::renderer::components::has_autoplay_swiper()
            || self.app.has_active_timers()
            // 有请求在飞：必须继续出帧，否则响应回来了没人去取（页面永远停在加载态）
            || self.app.has_pending_network()
            // 有远程图片在下载：同理，下完要有一帧把它画出来
            || mini_render::renderer::components::image_net::has_pending()
            || self.toast.as_ref().map(|t| t.visible).unwrap_or(false)
            || self.loading.as_ref().map(|l| l.visible).unwrap_or(false)
            || self.modal.as_ref().map(|m| m.visible).unwrap_or(false)
            // picker 面板入场/退场动画期间要连续出帧
            || self.picker_sheet.as_ref().map(|s| s.animating()).unwrap_or(false)
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
        // 与交互窗体的 RedrawRequested 一致：每帧把逻辑层的输出冲出来。
        // 少了这一句，定时器 / 网络回调里的 console.log 全都攒在缓冲里看不到 ——
        // 排查时会误判成「回调没跑」。
        print_js_output(&self.app);
        if self.app.take_data_dirty() {
            self.needs_redraw = true;
            self.page_data_dirty = true;
        self.fixed_dirty = true;
        }
        let mut pull_req: Option<bool> = None;
        evt::process_ui_events(&mut self.app, &mut self.toast, &mut self.loading, &mut self.modal, &mut pull_req);
        self.apply_pull_down_request(pull_req);
        self.update_scroll();
        if self.scroll.take_pull_trigger() && self.page_enables_pull_down() {
            self.set_pull_refreshing(true);
        }
        // 与交互窗体一致：每帧取一次导航请求并执行。
        // 少了这一步，`--frames` 跑到的「倒计时结束自动进首页」只会在日志里出现，
        // 页面其实没换 —— 快照就测不到这类靠时间驱动的跳转。
        if self.pending_navigation.is_none() {
            self.pending_navigation = app_window::check_navigation(&mut self.app);
        }
        self.process_navigation();
        self.reap_picker_sheet();
        if mini_render::renderer::components::advance_due_swipers() {
            self.needs_redraw = true;
        }
        if self.app.take_network_dirty() {
            self.needs_redraw = true;
            self.page_data_dirty = true;
        }
        if mini_render::renderer::components::image_net::take_dirty() {
            self.needs_redraw = true;
            self.fixed_dirty = true;
            self.force_full_redraw = true;
        }
        // 长按由帧驱动（同 RedrawRequested）：无头链路也要能测到 longpress
        if self.touch.is_active() {
            let clock = self.touch_clock_ms();
            let outs = self.touch.tick(clock);
            let (mx, my) = self.mouse_pos;
            self.dispatch_touch_events(&outs, mx, my);
        }
        if !self.viewport_inside_drawn_band() {
            self.needs_redraw = true;
        }
        // 页面滚动不进 structural（理由见 RedrawRequested 里的同名判断）
        let sv_scroll = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
        let css_anim = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
        let structural = self.needs_redraw
            || mini_render::renderer::components::has_playing_video()
            || sv_scroll
            || self.interaction.has_focused_input()
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

    /// 最近是否发生过滚动（用于决定裁剪余量留多大）
    fn scroll_recent(&self) -> bool {
        self.last_scroll_at
            .map(|t| t.elapsed() < Duration::from_millis(400))
            .unwrap_or(false)
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

        // 页面 json 声明的自定义组件（`usingComponents`）：模板按标签登记，
        // 样式并入页面样式表（已带作用域前缀），JS 稍后作为模块执行并建实例。
        let mut component_templates = mini_render::parser::template::ComponentTemplates::new();
        let mut component_wxss = String::new();
        for c in &page_info.components {
            match WxmlParser::new(&c.wxml).parse() {
                Ok(nodes) => {
                    component_templates.insert(c.tag.clone(), nodes);
                    component_wxss.push('\n');
                    component_wxss.push_str(&c.wxss);
                }
                Err(e) => eprintln!("⚠️  组件 {} 的 WXML 解析失败: {}", c.tag, e),
            }
        }
        
        // app.wxss 在前、页面 WXSS 在后：同特异性时页面样式因书写顺序更靠后而胜出
        let merged_wxss = format!("{}\n{}\n{}", self.app_wxss, page_info.wxss, component_wxss);
        let mut wxss_parser = WxssParser::new(&merged_wxss);
        let stylesheet = wxss_parser.parse().map_err(|e| format!("WXSS error: {}", e))?;
        
        // 上一页的组件实例作废（组件是按页面声明的）
        self.app.eval("__resetPageComponents()").ok();
        // 先登记路由：页面实例的 `this.route` / `getCurrentPages()` 都依赖它，
        // 返回时也靠它判断逻辑层是否已经出过栈
        self.app.eval(&format!("__setPendingRoute({})", serde_json::to_string(path).unwrap_or_else(|_| "''".into()))).ok();
        // 页面 js 同样按模块作用域执行，且每次进入都重跑（`Page({...})` 要重新注册）。
        // path 即页面路由，决定页面里相对 require 的基准目录。
        self.app.load_module_script(path, &page_info.js)?;
        let query_json = serde_json::to_string(&query).unwrap_or("{}".to_string());
        self.app.eval(&format!("if(__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", query_json)).ok();
        self.app.eval("if(__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
        // onReady：微信在首次渲染完成后触发；编译端取数据快照时也会走这一步，
        // 窗体不调用会导致两端初始数据不同（例如在 onReady 里补数据的页面）。
        self.app.eval("if(__currentPage && __currentPage.onReady) __currentPage.onReady()").ok();
        // 自定义组件：先按「组件路径」执行它的 js（`Component()` 借此注册到该路径），
        // 再建实例。实例必须真的建出来 —— 框架型产物在 attached 里挂载自己的组件树
        // 并触发首次 setData，没有实例的话组件模板里全是空值（底部导航整条空白）。
        app_window::component_mount::mount_page_components(&mut self.app, &page_info.components, &wxml_nodes);
        print_js_output(&self.app);
        
        self.page_stack.push(PageInstance { path: path.to_string(), query, wxml_nodes, stylesheet, component_templates });
        // 事件对象的 timeStamp 以「页面打开」为零点（微信语义）
        self.page_opened_at = Instant::now();
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
        // 回到的这一页要重新渲染，它离开时留下的那张图用完即弃
        self.back_shots.pop();
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
            // 自定义组件的实例要重建：往前走的时候 `navigate_to` 调过
            // `__resetPageComponents()`，把**上一页**的组件实例也一起作废了。
            // 不重建的话返回后组件模板里全是空值 —— tea-app 从二级页返回首页，
            // 底部那条 tab（自定义组件）整条消失，正是这个原因。
            let comps = self.pages.get(&path).map(|i| i.components.clone()).unwrap_or_default();
            if !comps.is_empty() {
                let nodes = self.page_stack.last().map(|p| p.wxml_nodes.clone()).unwrap_or_default();
                self.app.eval("__resetPageComponents()").ok();
                app_window::component_mount::mount_page_components(&mut self.app, &comps, &nodes);
            }
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
        self.fixed_dirty = true;
        self.needs_redraw = true;
        Ok(())
    }
    
    fn switch_tab(&mut self, path: &str) -> Result<(), String> {
        self.unload_current_page();
        self.page_stack.clear();
        self.back_shots.clear();
        self.interaction.clear_page_state();
        // 切 tab 会销毁原页面栈（微信语义），逻辑层的栈也要一起清，否则越切越长
        self.app.eval("__resetPageStack()").ok();
        self.navigate_to(path.trim_start_matches('/'), HashMap::new())
    }

    /// 销毁页面之前派发 `onUnload`（`switchTab` / `reLaunch` / `redirectTo` 都会销毁页面）。
    ///
    /// 漏掉这一步的后果很具体：页面在 `onUnload` 里 `clearTimeout`/`clearInterval` 的定时器
    /// 会继续跑。tea-app 启动页就是这个写法（`onUnload(){ this.stopTimers() }`），
    /// 它有一个守护定时器兜底跳首页 —— 不派发 `onUnload` 的话，用户已经翻到别的页面了，
    /// 这个定时器还会把他 `switchTab` 拽回首页。`redirectTo` 早就做了这一步，
    /// `switchTab` / `reLaunch` 漏了。
    fn unload_current_page(&mut self) {
        self.app
            .eval("if(__currentPage && __currentPage.onUnload) __currentPage.onUnload()")
            .ok();
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
            let mut r = WxmlRenderer::new_with_scale(page.stylesheet.clone(), LOGICAL_WIDTH as f32, LOGICAL_HEIGHT as f32, self.scale_factor as f32);
            r.set_component_templates(page.component_templates.clone());
            self.renderer = Some(r);
            if let Some(ref ct) = self.custom_tabbar {
                self.tabbar_renderer = Some(WxmlRenderer::new_with_scale(ct.stylesheet.clone(), LOGICAL_WIDTH as f32, tabbar_height() as f32, self.scale_factor as f32));
            }
        }
    }

    fn render(&mut self) {
        self.render_with_damage(None);
    }

    /// 两个矩形的包围盒
    fn union_of(a: GeoRect, b: GeoRect) -> GeoRect {
        let x0 = a.x.min(b.x);
        let y0 = a.y.min(b.y);
        let x1 = (a.x + a.width).max(b.x + b.width);
        let y1 = (a.y + a.height).max(b.y + b.height);
        GeoRect::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// `damage` 为 `Some(rect)` 时只重绘该矩形（动画帧的局部重绘），
    /// 其余像素保留上一帧结果，并且不更新「已绘制条带」。
    fn render_with_damage(&mut self, damage: Option<GeoRect>) {
        // 调用方给的动画损伤区（可能会被下面并上 setData 的失效范围，先留一份原值）
        let anim_damage_in = damage;
        // 页面数据只在逻辑层 setData 之后才会变，所以只有脏了才做这趟
        // 「JS 侧 JSON.stringify 整份 data → Rust 侧反序列化」往返。
        // 从前每帧都做一次：动画帧里白付一次全量序列化。
        if self.app.take_data_dirty() {
            self.page_data_dirty = true;
            // 数据变了 fixed 覆盖层也要重画。这里是所有渲染路径的共同入口，
            // 只在事件循环里标记的话，快照/无窗口路径会漏掉（底部固定栏会整条丢失）。
            self.fixed_dirty = true;
        }
        if self.page_data_dirty {
            self.fixed_dirty = true;
            self.page_data = std::sync::Arc::new(
                self.app
                    .eval("__getRenderData()")
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
        // 余量是自适应的：滚动时留大一些（少触发重绘，滚动期间大部分帧只做上屏），
        // 静止时留小一些（`setData` 这种偶发整帧重绘只画可见区，帧尖峰更低）。
        let margin = if self.scroll.is_dragging || self.scroll.is_animating() || self.scroll_recent() {
            mini_render::renderer::VIEWPORT_CULL_MARGIN_PX
        } else {
            48.0
        };
        let band_y0 = (scroll_offset * sf - margin).floor() as i32;
        let band_y1 = (scroll_offset * sf + viewport_height * sf + margin).ceil() as i32;

        let t_render_begin = Instant::now();
        // 先把布局算好并问出「这次 setData 只影响哪一块」。必须在清屏之前 ——
        // 清多大是由它决定的。之后的绘制会命中布局缓存，不重复这趟开销。
        // viewport 必须与后面绘制那趟传的完全一致，否则会被判成「视口变了」
        // 而重建第二次布局 —— 既白付一趟开销，算出的失效范围也对不上。
        let plan = self
            .renderer
            .as_mut()
            .map(|r| r.plan_frame(&page.wxml_nodes, page_data, Some((scroll_offset, viewport_height))))
            .unwrap_or(FramePlan::Full);
        // 「有新图到位」这类数据之外的内容变化：本帧强制整帧，不能被增量范围收窄
        let force_full = std::mem::take(&mut self.force_full_redraw);
        // 数据变化的失效范围优先于调用方给的动画损伤区：
        // 这一帧既有 setData 又有动画时，两者的并集才是完整的重绘范围。
        let damage = if force_full { None } else { match plan {
            FramePlan::Unchanged => damage,
            // `MINI_NO_DAMAGE=1` 关掉增量失效，用来做「局部重绘 vs 整帧重绘逐像素一致」对照
            _ if self.no_damage => None,
            FramePlan::Full => None,
            FramePlan::Damage { rect, fixed_changed } => {
                if std::env::var("MINI_LAYOUT_LOG").is_ok() {
                    eprintln!(
                        "🩹 setData 增量重绘 {:.0}x{:.0} @({:.0},{:.0})  覆盖层{}",
                        rect.width, rect.height, rect.x, rect.y,
                        if fixed_changed { "要重画" } else { "复用" }
                    );
                }
                // 覆盖层没变就不用重画它（首页倒计时那种：弹窗内容一动不动，
                // 却要陪着整张覆盖层每秒重画一次，白付 4ms）
                if !fixed_changed {
                    self.fixed_dirty = false;
                }
                if rect.width <= 0.0 || rect.height <= 0.0 {
                    // 渲染结果完全没变（改的字段不参与渲染）
                    damage.or(Some(GeoRect::new(0.0, 0.0, 0.0, 0.0)))
                } else {
                    Some(match damage {
                        None => rect,
                        Some(d) => Self::union_of(d, rect),
                    })
                }
            }
        } };
        // 页面底色以 `page { background-color }` 为准（微信语义），拿不到才退回默认灰。
        // 写死 #F5F5F5 会让在 page 上定义暖色底的应用整体色调都不对。
        let page_bg = self
            .renderer
            .as_ref()
            .and_then(|r| r.page_style().background)
            .unwrap_or(Color::from_hex(0xF5F5F5));
        let mut content_height = 0.0f32;
        if let Some(canvas) = &mut self.canvas {
            match &damage {
                Some(rect) => canvas.clear_area(rect, page_bg),
                None => canvas.clear_band(band_y0, band_y1, page_bg),
            }
            if let Some(renderer) = &mut self.renderer {
                renderer.set_damage_clip(damage);
                content_height = renderer.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
            }
        }
        if let Some(r) = &mut self.renderer {
            let bounds = r.take_animated_bounds();
            // 只有「这一帧的裁剪范围覆盖了全部动画元素」时，收集到的包围盒才是完整的：
            //   - 整帧重绘：显然完整
            //   - 动画损伤区帧：裁剪范围本来就是由动画包围盒算出来的，完整
            //   - **纯 setData 局部帧**：裁剪范围只是数据变化的那一小块，
            //     落在外面的动画元素这一趟根本没被访问到。此时若拿它覆盖旧记录，
            //     下一个动画帧的损伤区就只剩这一小块 —— 外面的动画会就此冻住。
            let bounds_are_complete = damage.is_none() || anim_damage_in.is_some();
            if bounds_are_complete && (!bounds.is_empty() || damage.is_none()) {
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
                        canvas.clear_band(band_y0, band_y1, page_bg);
                        renderer.render_with_scroll_and_viewport(canvas, &page.wxml_nodes, &page_data, &mut self.interaction, scroll_offset, viewport_height);
                    }
                }
            }
        }
        
        let t_page_done = Instant::now();
        // 局部重绘帧里 fixed 覆盖层与 tabBar 不会变，直接沿用上一帧的画布 ——
        // 除非覆盖层自己带动画（那时 animation_damage_rect 会拒绝走局部路径）。
        if damage.is_none() && (self.fixed_dirty || self.fixed_layer_animates) {
            self.fixed_dirty = false;
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
                    // 记下覆盖层这一趟产生的事件绑定与遮挡区域：
                    // 事件绑定每帧都会清空重建，而覆盖层在「内容没变」的帧里跳过重绘，
                    // 不缓存的话那些帧里弹窗没有任何绑定，点击会直接穿到下层页面。
                    self.cached_fixed_bindings = r.fixed_event_bindings();
                    self.cached_fixed_regions = r.fixed_hit_regions().to_vec();
                }
                // 覆盖层内容变了才重算它占用的行区间（上屏据此只合成这几行）
                if let Some(fc) = &self.fixed_canvas {
                    self.fixed_rows = app_window::render::opaque_row_span(fc);
                }
            }
        } else if let Some(r) = &mut self.renderer {
            // 这一帧没重绘覆盖层：把上一次的绑定与遮挡区域补回来
            let (b, g) = (self.cached_fixed_bindings.clone(), self.cached_fixed_regions.clone());
            r.restore_fixed_bindings(&b, &g);
        }
        let t_fixed_done = Instant::now();
        
        if has_tabbar && damage.is_none() {
            if self.is_custom_tabbar() { self.render_custom_tabbar(&current_path); }
            else { self.render_native_tabbar(&current_path); }
        }
        // 记下这一帧真正画过的行区间（夹到画布内；画布之外由上屏填背景色）。
        // 局部重绘帧只碰了一个小矩形，不能声称整条带都是新的 ——
        // 否则滚动出旧条带时 viewport_inside_drawn_band 会误判「画过了」，
        // 上屏取到没画过的区域，也就是滑动时露白。
        if damage.is_none() {
            let canvas_h = self.canvas.as_ref().map(|c| c.height() as f32).unwrap_or(0.0);
            self.drawn_band = Some((
                (band_y0 as f32).max(0.0),
                (band_y1 as f32).min(canvas_h),
            ));
        }
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
        let picker_state = self.picker_sheet.clone();
        // 画布之外的填充色同样取 page 底色，避免露出突兀的灰边
        let present_bg = self
            .renderer
            .as_ref()
            .and_then(|r| r.page_style().background)
            .map(|c| ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32)
            .unwrap_or(0xF5F5F5);
        
        if let (Some(window), Some(surface)) = (&self.window, &mut self.surface) {
            let size = window.inner_size();
            if let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                // 只在尺寸真的变了才 resize：每帧都调等于让 softbuffer 反复确认
                // 平台侧后备缓冲的配置，是白付的开销。
                //
                // 注：这**不是**偶发 30~50ms 上屏停顿的原因（改前改后都有）。
                // 那个停顿发生在 `buffer_mut()`/`present()` 内部，即 macOS 合成器侧，
                // 与本引擎的光栅化无关（同一帧的「渲染」一直稳定在 2~3ms）。
                if self.surface_size != Some((size.width, size.height)) {
                    surface.resize(w, h).ok();
                    self.surface_size = Some((size.width, size.height));
                }
                if let Ok(mut buffer) = surface.buffer_mut() {
                    present_to_buffer(&mut buffer, size.width, size.height, canvas, self.fixed_canvas.as_ref(), self.tabbar_canvas.as_ref(),
                        (self.scroll.get_position() * self.scale_factor as f32) as i32, has_tabbar,
                        if has_tabbar { (tabbar_height() as f64 * self.scale_factor) as u32 } else { 0 },
                        self.fixed_rows, present_bg);
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
                    // picker 面板在所有覆盖层之上
                    if let Some(sheet) = &picker_state {
                        picker_sheet::render(&mut buffer, size.width, size.height, self.scale_factor as f32, sheet, self.text_renderer.as_deref());
                    }
                    // 左边缘侧滑返回：整帧右移，左边露出上一页。
                    // 放在最后（覆盖层之上）——被推走的是「整个页面」，页面自己的
                    // 弹窗、Toast 跟着一起走才对；不然弹窗会诡异地钉在原处。
                    if let Some(eb) = &self.edge_back {
                        let off = (eb.offset() * self.scale_factor as f32).round() as u32;
                        app_window::edge_back::compose(
                            &mut buffer, size.width, size.height, off,
                            self.back_shots.last(), eb.progress(),
                        );
                    }
                    buffer.present().ok();
                }
            }
        }
    }
    
    fn handle_click(&mut self, x: f32, y: f32) {
        if self.picker_sheet.as_ref().map(|s| s.visible).unwrap_or(false) { return; }
        if self.modal.as_ref().map(|m| m.visible).unwrap_or(false) { self.handle_modal_click(x, y); return; }
        if self.loading.as_ref().map(|l| l.visible).unwrap_or(false) { return; }
        // 点在 picker 上：弹出选择面板，不再走普通内容点击
        if self.open_picker_if_hit(x, y) { return; }
        
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
            let tap_ctx = (self.touch.identifier(), self.event_time_ms());
            if let Some(nav) = click::handle_content_click(x, y, &self.scroll, has_tabbar, &mut self.interaction,
                self.renderer.as_ref(), &mut self.app, self.scale_factor, self.text_renderer.as_deref(), self.window.as_ref(), &mut self.clipboard, tap_ctx) {
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

    /// 点击落在某个 `<picker>` 上时弹出底部选择面板。返回是否弹出了面板。
    fn open_picker_if_hit(&mut self, x: f32, y: f32) -> bool {
        let scroll = self.scroll.get_position();
        let on_fixed_layer = self.renderer.as_ref().map(|r| r.fixed_layer_hit(x, y)).unwrap_or(false);
        let binding = self.renderer.as_ref().and_then(|r| {
            // 覆盖层上的 picker 用视口坐标；正常流的用内容坐标（y + 滚动量）
            r.picker_hit(x, y, true)
                .or_else(|| if on_fixed_layer { None } else { r.picker_hit(x, y + scroll, false) })
                .cloned()
        });
        let Some(binding) = binding else {
            if std::env::var("MINI_PICKER_LOG").is_ok() {
                if let Some(r) = &self.renderer {
                    eprintln!("PICKER miss @({},{}) scroll={} regions={:?}", x, y, scroll,
                        r.picker_regions().iter().map(|p| (p.id.clone(), p.bounds)).collect::<Vec<_>>());
                }
            }
            return false;
        };
        println!("👆 picker -> 打开选择面板 mode={}", binding.mode);
        let sheet = picker_sheet::build(&binding);
        self.picker_sheet = Some(sheet);
        self.needs_redraw = true;
        if let Some(w) = &self.window { w.request_redraw(); }
        true
    }

    /// 面板可见时的按下：记录按压的头部按钮以给出反馈。返回是否消费了事件。
    fn handle_picker_sheet_press(&mut self, x: f32, y: f32) -> bool {
        let Some(sheet) = &mut self.picker_sheet else { return false };
        if !sheet.visible { return false; }
        let hit = picker_sheet::hit_test(sheet, x, y);
        sheet.pressed = match hit {
            picker_sheet::SheetHit::Cancel | picker_sheet::SheetHit::Confirm => Some(hit),
            _ => None,
        };
        self.needs_redraw = true;
        if let Some(w) = &self.window { w.request_redraw(); }
        true
    }

    /// 面板可见时的抬手：确定/取消/选项滚动/点遮罩关闭。返回是否消费了事件。
    fn handle_picker_sheet_release(&mut self, x: f32, y: f32) -> bool {
        let Some(sheet) = &mut self.picker_sheet else { return false };
        if !sheet.visible { return false; }
        sheet.pressed = None;
        // 退场动画进行中时，只吞事件不再响应
        if sheet.closing {
            return true;
        }
        let hit = picker_sheet::hit_test(sheet, x, y);
        match hit {
            picker_sheet::SheetHit::Confirm => {
                let value = sheet.change_value();
                let handler = sheet.handler.clone();
                let labels = sheet.picked_labels().join(" ");
                sheet.begin_close();
                if let Some(handler) = handler {
                    // `__callPageMethod` 的第二个参数就是 dataset，事件对象里 `detail` 直接指向它。
                    // 多包一层 `{detail:{...}}` 会让页面拿到 `e.detail.detail.value`，
                    // 于是 `e.detail.value` 是 undefined —— 选了「选项二」按确定却没反应就是这个。
                    let payload = serde_json::json!({ "value": value });
                    println!("👆 picker 确定 -> {} value={} ({})", handler, value, labels);
                    let code = format!("__callPageMethod('{}', {})", handler, payload);
                    self.app.eval(&code).ok();
                }
            }
            picker_sheet::SheetHit::Cancel | picker_sheet::SheetHit::Mask => {
                sheet.begin_close();
            }
            picker_sheet::SheetHit::Item { col, delta } => {
                sheet.move_selection(col, delta);
                if delta != 0 {
                    picker_sheet::relink_region(sheet, col);
                }
            }
        }
        self.needs_redraw = true;
        if let Some(w) = &self.window { w.request_redraw(); }
        true
    }

    /// 退场动画走完后丢弃面板
    fn reap_picker_sheet(&mut self) {
        if self.picker_sheet.as_ref().map(|s| s.finished_closing()).unwrap_or(false) {
            self.picker_sheet = None;
            self.needs_redraw = true;
        }
    }
    
    fn process_navigation(&mut self) {
        if let Some(nav) = self.pending_navigation.take() {
            match nav {
                NavigationRequest::NavigateTo { url } => {
                    let (p, q) = parse_url(&url);
                    // 入栈前给当前页留一张图，侧滑返回时它要出现在新页下面。
                    // 只在这一条分支做：switchTab/reLaunch 会清栈，redirectTo 是替换，
                    // 都没有「上一页」可回。
                    let shot = self.capture_viewport();
                    if self.navigate_to(&p, q).is_ok() {
                        if let Some(s) = shot {
                            self.back_shots.push(s);
                            // 页面栈上限 10（微信语义），快照跟着裁，免得越走越占内存
                            if self.back_shots.len() > 10 { self.back_shots.remove(0); }
                        }
                    }
                }
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
                    self.unload_current_page();
                    self.page_stack.clear();
                    self.back_shots.clear();
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
        
        let before = self.scroll.get_position();
        let (animating, event) = self.scroll.update_with_events(dt);
        if (self.scroll.get_position() - before).abs() > 0.01 {
            self.last_scroll_at = Some(now);
        }
        if let Some(e) = event { evt::handle_scroll_event(e, &mut self.app); self.needs_redraw = true; }
        // `onPageScroll`：位置变了就回调一次（微信语义）。放在这里而不是各条输入路径里 ——
        // 拖动、惯性、回弹、滚轮、脚本设置位置最终都会经过这一帧检查，只写一处不会漏。
        let pos = self.scroll.get_position();
        if (pos - self.last_page_scroll_sent).abs() > 0.5 {
            self.last_page_scroll_sent = pos;
            evt::dispatch_page_scroll(&mut self.app, pos);
        }
        
        let mut changed = animating;
        for c in self.interaction.scroll_controllers.values_mut() { if c.update(dt) { changed = true; } }
        if self.tick_edge_back(dt) { changed = true; }
        if changed { if let Some(w) = &self.window { w.request_redraw(); } }
    }

    /// 让所有滚动动画（惯性、边界回弹）走完，最多等 `budget_ms`。
    ///
    /// 手势快照必须等这一步：松手瞬间位置多半还在越界区（橡皮筋），
    /// 直接截图会截到一张回弹中途的图 —— 同一条命令两次跑出来的还不一样。
    fn settle_scroll_animations(&mut self, budget_ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(budget_ms);
        loop {
            let area_anim = self.interaction.scroll_controllers.values().any(|c| c.is_animating());
            // 侧滑返回的收尾动画也要等：它跑完才知道这一页是留下还是退栈
            let back_anim = self.edge_back.as_ref().map(|e| e.is_settling()).unwrap_or(false);
            if !self.scroll.is_animating() && !area_anim && !back_anim { break; }
            if Instant::now() >= deadline { break; }
            self.pump_one_frame();
            std::thread::sleep(Duration::from_millis(8));
        }
    }

    /// `--drag <每帧像素>x<帧数>`：走与交互窗体同一套鼠标事件模拟一次手指拖动，
    /// 逐帧计时。滑动手感只有连续拖动才测得出来（见 app_window::scroll_bench）。
    pub(crate) fn run_drag_bench(&mut self, spec: &app_window::scroll_bench::DragSpec, route: &str) {
        let (cx, mut cy) = (LOGICAL_WIDTH as f32 / 2.0, LOGICAL_HEIGHT as f32 * 0.75);
        self.sim_press(cx, cy);
        let mut frame_ms: Vec<f32> = Vec::with_capacity(spec.frames as usize);
        let mut worst = (0.0f32, (0.0f32, 0.0f32, 0.0f32));
        let mut input_total = 0.0f32;
        for _ in 0..spec.frames {
            cy -= spec.dy;
            let t0 = Instant::now();
            self.sim_move(cx, cy);
            input_total += t0.elapsed().as_secs_f32() * 1000.0;
            self.pump_one_frame();
            let ms = t0.elapsed().as_secs_f32() * 1000.0;
            if ms > worst.0 {
                worst = (ms, self.render_parts);
            }
            frame_ms.push(ms);
            // 按刷新率节流，模拟真实拖动的采样节奏
            let spent = t0.elapsed();
            if let Some(rest) = self.frame_interval.checked_sub(spent) {
                std::thread::sleep(rest);
            }
        }
        self.sim_release(cx, cy);
        self.settle_scroll_animations(2000);
        app_window::scroll_bench::report(route, &frame_ms, self.frame_interval);
        app_window::scroll_bench::report_worst_parts(worst.0, worst.1);
        println!(
            "   ↳ 其中输入处理（命中/手势/触摸事件派发）平均 {:.2}ms/帧",
            input_total / spec.frames.max(1) as f32
        );
        if let Some(s) = mini_render::renderer::draw_profile::summary(8) {
            println!("   ↳ {s}");
        }
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
                self.on_pointer_move(x, y);
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::MouseWheel { delta, phase, .. } => {
                // 指针停在 fixed 覆盖层（弹窗遮罩）上时锁住页面滚动，与微信一致；
                // 覆盖层内部自己的 scroll-view 仍然可滚（下面按元素命中处理）
                let over_fixed = self
                    .renderer
                    .as_ref()
                    .map(|r| r.fixed_layer_hit(self.mouse_pos.0, self.mouse_pos.1))
                    .unwrap_or(false);
                if evt::handle_mouse_wheel_gated(delta, self.mouse_pos, &mut self.interaction, &mut self.scroll, self.scale_factor, over_fixed) {
                    self.needs_redraw = true;
                    self.last_scroll_at = Some(Instant::now());
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
                let (x, y) = self.mouse_pos;
                if state == ElementState::Pressed {
                    self.on_pointer_press(x, y);
                } else {
                    self.on_pointer_release(x, y);
                }
                if let Some(w) = &self.window { w.request_redraw(); }
            }
            
            WindowEvent::RedrawRequested => {
                let frame_begin = Instant::now();
                if self.fps_log {
                    if let Some(prev) = self.last_frame_begin {
                        let gap = (frame_begin - prev).as_secs_f32() * 1000.0;
                        self.frame_gap_max_ms = self.frame_gap_max_ms.max(gap);
                        self.frame_gap_min_ms = self.frame_gap_min_ms.min(gap);
                    }
                    self.last_frame_begin = Some(frame_begin);
                }
                self.app.update().ok();
                print_js_output(&self.app);
                // 逻辑层这一帧改过数据就必须重绘。没有这一步的话，定时器/网络回调
                // 里的 setData 只在「刚好还有 CSS 动画在跑」时才顺带上屏，
                // 纯 JS 驱动的页面（秒杀倒计时、轮询刷新）会一直显示旧值。
                if self.app.take_data_dirty() {
                    self.needs_redraw = true;
                    self.page_data_dirty = true;
                    self.fixed_dirty = true;
                }
                // 网络响应回来了：即使这一帧逻辑层没有 setData（比如回调里只存了缓存），
                // 也要重绘一次 —— 图片 src 之类可能已经跟着变了
                if self.app.take_network_dirty() {
                    self.needs_redraw = true;
                    self.page_data_dirty = true;
                }
                // 远程图片下载完了：重绘一次让它出现（异步加载的必要配套 ——
                // 不置脏的话图片只会在下一次别的原因触发重绘时才显示）
                if mini_render::renderer::components::image_net::take_dirty() {
                    self.needs_redraw = true;
                    self.fixed_dirty = true;
                    self.force_full_redraw = true;
                }
                // 长按要由帧驱动：手指按住不动时没有任何输入事件进来，
                // 只在 move/up 里判时间的话那一下永远等不到
                if self.touch.is_active() {
                    let clock = self.touch_clock_ms();
                    let outs = self.touch.tick(clock);
                    let (mx, my) = self.mouse_pos;
                    self.dispatch_touch_events(&outs, mx, my);
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
                self.reap_picker_sheet();
                // swiper 的自动播放由宿主推进，而不是等它被画到才走时钟 ——
                // 滚出视口或被局部重绘剪掉的 swiper 会永远"到点"，把整页拖成每帧重绘
                if mini_render::renderer::components::advance_due_swipers() {
                    self.needs_redraw = true;
                }
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
                
                // 页面级滚动**不进** structural：整页画布用内容坐标，滚动只是让 present()
                // 按新偏移取不同切片（present 每帧都会跑）。只有滚出「已绘制条带」时，
                // 上面的 viewport_inside_drawn_band 才置 needs_redraw 触发一次重绘。
                // 从前把 `self.scroll.is_animating()||is_dragging` 也算进 structural，
                // 于是滚动的每一帧都整条带重画（6~7ms），刚好卡在 144Hz 的 6.9ms 预算边缘，
                // 时不时超一点就丢帧 —— 这正是「滑动像抖动、高刷没体现」的根因。
                // scroll-view 内滚动仍要重画：它的内容是按自身偏移画进整页画布的，没有独立切片。
                let sv_scroll = self.interaction.scroll_controllers.values().any(|c| c.is_animating() || c.is_dragging);
                let css_anim = self.renderer.as_ref().map(|r| r.has_active_animations()).unwrap_or(false);
                let t_logic = frame_begin.elapsed();
                // swiper 自动播放到点/正在滑动时也要重绘（它的状态在绘制期推进）
                let swiper_frame = mini_render::renderer::components::swiper_needs_frame();
                let structural = self.needs_redraw
                    || mini_render::renderer::components::has_playing_video()
                    || sv_scroll
                    || self.interaction.has_focused_input()
                    || swiper_frame;
                // 整帧重绘的归因统计（MINI_FPS=1 时每秒汇总）。
                // 「为什么这一帧又整屏重画了」如果只能靠猜，性能问题就没法收敛。
                if self.fps_log {
                    if self.needs_redraw { self.gate_counts.0 += 1; }
                    if swiper_frame { self.gate_counts.1 += 1; }
                    if sv_scroll { self.gate_counts.2 += 1; }
                    if css_anim { self.gate_counts.3 += 1; }
                    if !structural && css_anim && self.animation_damage_rect().is_none() {
                        self.gate_counts.4 += 1; // 动画帧被迫走整帧（损伤区被否）
                    }
                }
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
                    // 固定节拍：下一帧时点是「上一个时点 + 一个刷新周期」的累加，
                    // 不是「本帧开始 + 一个周期」。后者每帧都把 sleep 的过冲（macOS 上
                    // 约 0.5~2ms）算进新的起点，节拍会越走越偏且忽快忽慢 ——
                    // 平均帧率看着达标，实际帧间隔在抖，这正是「高刷没体现出来」的手感。
                    let now = Instant::now();
                    let mut target = self.next_frame_at + self.frame_interval;
                    // 落后超过一整帧（大重绘、系统抢占）就重新对齐，避免追帧追出一串挤压帧。
                    // 对齐到「现在」而不是「现在 + 一个周期」：一帧干了 10ms 活本来就已经
                    // 超预算了，再补睡 6.9ms 会把帧间隔顶到 17ms —— 手上感觉到的顿挫是
                    // **帧间隔**，不是干活时间，白睡的这一下等于把一次超时放大成两帧。
                    if target <= now {
                        target = now;
                    }
                    self.next_frame_at = target;
                    // 只睡到「还差一个自旋余量」，剩下的用让出时间片的忙等把边缘对准；
                    // sleep 的分辨率不足以稳定命中 6.9ms 的节拍。
                    const SPIN_MARGIN: Duration = Duration::from_micros(900);
                    if let Some(coarse) = target.checked_sub(SPIN_MARGIN) {
                        let now = Instant::now();
                        if coarse > now {
                            std::thread::sleep(coarse - now);
                        }
                    }
                    while Instant::now() < target {
                        std::hint::spin_loop();
                    }
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                } else {
                    self.next_frame_at = Instant::now();
                }
                if self.fps_log {
                    let ms = work_ms;
                    self.fps_frames += 1;
                    self.fps_worst_ms = self.fps_worst_ms.max(ms);
                    if self.fps_window_start.elapsed().as_secs_f32() >= 1.0 {
                        let (l, r, p) = self.fps_worst_parts;
                        let (rp, rf, rt) = self.fps_worst_render_parts;
                        println!(
                            "📊 {} 帧/秒，帧间隔 {:.1}~{:.1}ms，最慢一帧 {:.1}ms（逻辑 {:.1} 渲染 {:.1}[页面 {:.1} 覆盖层 {:.1} tabBar {:.1}] 上屏 {:.1}）{}",
                            self.fps_frames,
                            if self.frame_gap_min_ms == f32::MAX { 0.0 } else { self.frame_gap_min_ms },
                            self.frame_gap_max_ms,
                            self.fps_worst_ms,
                            l, r, rp, rf, rt, p,
                            if self.is_animating() { "（动画中）" } else { "" }
                        );
                        let g = self.gate_counts;
                        if g != (0, 0, 0, 0, 0) {
                            println!(
                                "   ↳ 整帧归因：需重绘 {} / swiper {} / scroll-view {} / 动画 {}（其中损伤区被否 {}）",
                                g.0, g.1, g.2, g.3, g.4
                            );
                        }
                        self.gate_counts = (0, 0, 0, 0, 0);
                        self.fps_window_start = Instant::now();
                        self.fps_frames = 0;
                        self.fps_worst_ms = 0.0;
                        self.fps_worst_parts = (0.0, 0.0, 0.0);
                        self.fps_worst_render_parts = (0.0, 0.0, 0.0);
                        self.frame_gap_max_ms = 0.0;
                        self.frame_gap_min_ms = f32::MAX;
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
    #[allow(clippy::too_many_arguments)]
    fn snapshot_all(&mut self, out_dir: &std::path::Path, scale: f64, time: Option<f32>, settle: Option<f32>, only: Option<&str>, scroll: f32, evals: &[String], frames: u32, actions: &[app_window::headless_script::Action]) -> Result<usize, String> {
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
                // 与 switchTab/reLaunch 一致：销毁上一页前派发 onUnload，
                // 否则上一页的定时器会继续跑，几秒后把路由顶掉
                // （启动页的守护定时器会 switchTab 回首页，快照就截错了页）
                self.unload_current_page();
                self.page_stack.clear();
                self.back_shots.clear();
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
                    // 跑**完整的一帧**（与交互窗体同一套闸门），不只是 update。
                    //
                    // 关键在于「真的出帧」：远程图片、`wx.request` 这些异步资源是
                    // **绘制期**才发起的（绘制到 `<image src="http…">` 才知道要下载它）。
                    // 从前 settle 只 update 不 render，于是等待这几秒里根本没人发起下载，
                    // 最后一帧才发起、当然来不及 —— 闪屏页的整屏背景图因此永远是占位图，
                    // 而真机上那几秒是一直在出帧的。
                    self.pump_one_frame();
                    evt::update_toast_timeout(&mut self.toast);
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
            //
            // 注入之前先出一帧：真机上 `setData` 永远发生在已经渲染过的页面上，
            // 于是它走的是「增量失效」路径。不先出这一帧的话，脚本注入等于
            // 首帧数据，永远只能测到整帧重绘那条路 —— 对照验证会变成空转。
            if !evals.is_empty() {
                self.render();
            }
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
            let anim_deadline = Instant::now() + Duration::from_millis(2000);
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
            // 手势/输入动作：按命令行**书写顺序**依次执行（见 app_window::headless_script）。
            // 每一步都走与交互窗体完全相同的入口，所以「点输入框 → 打字 → 点发送」
            // 这类多步交互在无头模式下也能如实复现。
            self.run_actions(actions, &route);
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
            // 截图前再走一帧完整闸门：异步资源（远程图片、网络回调）刚到位时
            // 需要一次**整帧**重绘才会出现 —— 只调 render() 的话，这一帧的重绘范围
            // 会被上一次 setData 的增量失效范围收窄（闪屏页倒计时每秒 setData，
            // 图片区域因此永远不在范围内，截出来一直是占位图）。
            self.pump_one_frame();
            self.render();

            let (pw, ph) = (
                (LOGICAL_WIDTH as f64 * scale) as u32,
                (LOGICAL_HEIGHT as f64 * scale) as u32,
            );
            let mut buffer = vec![0u32; (pw * ph) as usize];
            let present_bg = self
                .renderer
                .as_ref()
                .and_then(|r| r.page_style().background)
                .map(|c| ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32)
                .unwrap_or(0xF5F5F5);
            // 导航之后当前页可能已不是入口 route
            let route = self.page_stack.last().map(|p| p.path.clone()).unwrap_or(route);
            let has_tabbar = self.is_tabbar_page(&route);
            if let Some(canvas) = &self.canvas {
                present_to_buffer(
                    &mut buffer, pw, ph, canvas,
                    self.fixed_canvas.as_ref(), self.tabbar_canvas.as_ref(),
                    (self.scroll.get_position() * scale as f32) as i32, has_tabbar,
                    if has_tabbar { (tabbar_height() as f64 * scale) as u32 } else { 0 },
                    self.fixed_rows,
                    present_bg,
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
            if let Some(sheet) = &self.picker_sheet {
                picker_sheet::render(&mut buffer, pw, ph, scale as f32, sheet, self.text_renderer.as_deref());
            }
            // 侧滑返回进行中：与交互窗体同一段合成（无头也要能截到跟手的那一帧）
            if let Some(eb) = &self.edge_back {
                let off = (eb.offset() * scale as f32).round() as u32;
                app_window::edge_back::compose(
                    &mut buffer, pw, ph, off, self.back_shots.last(), eb.progress(),
                );
            }

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
    // 手势与输入按**书写顺序**入队（见 app_window::headless_script）
    let mut actions: Vec<app_window::headless_script::Action> = Vec::new();
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
            // `--press` / `--click` / `--touch` / `--swipe[-hold]` / `--drag` / `--type` /
            // `--key` / `--wait`：都是动作，谁写在前面谁先执行。
            // `--swipe-hold` 停在终点不抬手 —— 用来截「手势进行中」的那一帧
            // （侧滑返回的跟手位移、越界橡皮筋、按压态），松手之后就看不到了。
            "--press" | "--click" | "--touch" | "--swipe" | "--swipe-hold" | "--drag"
            | "--type" | "--key" | "--wait" | "--wheel" | "--wheel-lines" => {
                match app_window::headless_script::parse(&arg, it.next()) {
                    Some(a) => actions.push(a),
                    None => eprintln!("⚠️  {} 的参数无法解析，已忽略", arg),
                }
            }
            "--help" | "-h" => {
                println!("用法: mini-app-window <小程序目录> [--snapshot <输出目录>] [--scale 2]");
                println!("  --time <秒>    动画时钟位置（CSS @keyframes 求值到该时刻，不消耗真实时间）");
                println!("  --settle <秒>  真实等待该时长，让 setTimeout/setInterval、延时弹层、");
                println!("  --frames <N>   按刷新率跑 N 个与交互窗体同逻辑的帧再截图（验证动画/局部重绘）");
                println!("  --click <x,y>  在该逻辑坐标模拟一次点击，走真实命中/冒泡/导航链路");
                println!("  --press <x,y>  在该坐标按下并保持（验证 :active / hover-class 与过渡）");
                println!("                 轮播自动播放跑起来（要「和真机一样」的画面时用这个）");
                println!("  --route <页面路径>   --scroll <像素>");
                println!("  --drag <px>x<帧数>  模拟手指拖动并逐帧计时（滑动手感的可回归测量）");
                println!("  --touch <x,y[,按住ms]>       完整触摸序列：touchstart→(长按)→touchend/tap");
                println!("  --swipe <x1,y1,x2,y2[,步数]>  滑动手势：按下→逐步移动→抬起");
                println!("  --swipe-hold <同上>           同上但停在终点不抬手（截手势进行中的一帧）");
                println!("  --type <文本>   往当前焦点输入框逐字符输入（先 --click 到输入框上）");
                println!("  --key <按键>    enter/backspace/delete/escape/left/right/home/end/selectall");
                println!("  --wait <秒>     空转等待，期间照常出帧");
                println!("  --wheel <x,y,dx,dy[,步数]>    触控板双指滑动（dy>0 = 内容上走）");
                println!("  --wheel-lines <同上>          普通滚轮（行滚动，非精密）");
                println!("  上面这些动作按**书写顺序**执行，可重复：");
                println!("    --click 25,520 --type 1212121 --click 313,520");
                println!("  --eval <JS>    可重复；在 --settle 之后依次执行，每段跑完导航");
                return Ok(());
            }
            other => {
                // 示例小程序收在 sample/ 下，但命令行习惯写裸名字（`sample-app`），
                // 交给解析器统一处理裸名字 / `sample/xxx` / 任意路径三种写法
                let path = mini_render::app_dir::resolve(other);
                if path.is_dir() {
                    println!("📂 加载小程序: {}", path.display());
                    app_path = Some(path);
                } else {
                    eprintln!("❌ 找不到小程序: {}", other);
                    let apps = mini_render::app_dir::list_apps();
                    if !apps.is_empty() {
                        eprintln!("   可用的示例：{}", apps.join("、"));
                    }
                    return Err(format!("找不到小程序: {}", other).into());
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
        let count = window.snapshot_all(&out, scale, anim_time, settle, route.as_deref(), scroll, &evals, frames, &actions)?;
        println!("\n✅ 快照完成：{} 个页面 -> {}", count, out.display());
        return Ok(());
    }
    
    let event_loop = EventLoop::new()?;
    // 默认空闲休眠；动画期间由 about_to_wait 按刷新率切换到 WaitUntil。
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut window)?;
    Ok(())
}
