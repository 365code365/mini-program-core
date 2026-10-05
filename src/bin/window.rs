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
    /// 各 scroll-view 在上一次画进页面画布时的滚动位置。
    /// 位置变了的那几个就是这一帧的损伤区（见 `scroll_area_damage_rect`）。
    drawn_area_positions: HashMap<String, f32>,
    /// 触控板手势与 macOS 惯性阶段的区分（见 `events::wheel::MomentumFilter`）
    wheel_filter: app_window::events::wheel::MomentumFilter,
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
    /// 上一帧画出来的光标处于「显示」还是「隐藏」的半个周期。
    /// 相位一翻转就要安排重绘，否则画面停在最后一次绘制那一帧 —— 光标不闪。
    /// 尤其是输入框在 `position:fixed` 覆盖层里时：那层画布只在标脏时才重画，
    /// 光标会被烤死在覆盖层画布上（tea-app 的搜索条就是这种）。
    caret_visible: bool,
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
            drawn_area_positions: HashMap::new(),
            wheel_filter: Default::default(),
            cached_fixed_bindings: Vec::new(),
            cached_fixed_regions: Vec::new(),
            started_at: now,
            next_frame_at: now,
            last_scroll_at: None,
            fixed_dirty: true, caret_visible: true,
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
