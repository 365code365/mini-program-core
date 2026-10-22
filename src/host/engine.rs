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
//!
//! ## 文件划分（都是 `impl MiniEngine`）
//! - 本文件：结构体、构造、页面路由与生命周期；
//! - `engine_frame.rs`：出帧（`pump` / `render` / 合成）；
//! - `engine_damage.rs`：这一帧重画多少（只合成 / 局部 / 补条带 / 整帧）与整帧归因；
//! - `engine_ui.rs`：Toast / Loading / Modal 状态与覆盖层查询；
//! - `engine_input.rs`：指针 / 键盘输入与逻辑层发起的路由；
//! - `engine_backend.rs`：后端选择、surface 挂卸、带标志位的 `pump_ex`；
//! - `engine_stats.rs`：`MINI_FPS` 的每秒统计；
//! - `engine_options.rs`：创建选项（系统信息、安全区、宿主 API、原生组件）；
//! - `engine_events.rs`：引擎 → 宿主的事件队列（页面、导航栏、输入框、hostCall、日志）；
//! - `engine_pixels.rs`：像素出口（RGBA / BGRA、带行跨度直写平台缓冲）；
//! - `engine_resize.rs`：尺寸 / 像素比 / 安全区变化。

use super::engine_backend::select_backend;
use super::engine_events::HostEvents;
use super::engine_options::{self, EngineOptions};
use super::engine_stats::FrameStats;
use crate::compositor::{
    diag, BackendPref, BackendStatus, DeviceInfo, HostEnv, HostKind, LayerCanvas, PageCanvas,
    SurfaceKind,
};
use crate::host::{
    load_all_pages, page_loader, tabbar_height,
    ui_overlay::{LoadingState, ModalState, ToastState},
    AppConfig, PageInstance, CONTENT_HEIGHT,
};
use crate::renderer::WxmlRenderer;
use crate::runtime::MiniApp;
use crate::text::TextRenderer;
use crate::ui::interaction::InteractionManager;
use crate::ui::ScrollController;
use crate::Canvas;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

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
    /// 三块画布的写入都要经过 `write*()`：声明写了哪块，给以后按脏区上传的合成后端用
    pub(crate) canvas: PageCanvas,
    pub(crate) fixed_canvas: LayerCanvas,
    pub(crate) tabbar_canvas: LayerCanvas,

    pub(crate) interaction: InteractionManager,
    pub(crate) scroll: ScrollController,
    pub(crate) touch: crate::host::touch::TouchTracker,
    /// 拖动手势仲裁（方向锁定 / 嵌套传递 / catchtouchmove），与桌面窗体同一份实现
    pub(crate) gesture: Option<crate::host::gesture::DragGesture>,
    /// 页面拖动按事件时间重采样（engine_resample.rs）
    pub(super) resample: super::engine_resample::DragResampler,
    /// `<picker>` 弹出的底部面板（微信里是原生浮层）
    pub(crate) picker_sheet: Option<crate::host::picker_sheet::PickerSheetState>,
    /// 左边缘侧滑返回进行中（与桌面窗体同一份 `host::edge_back`，见 engine_edge_back.rs）
    pub(crate) edge_back: Option<crate::host::edge_back::EdgeBack>,
    /// 页面栈上每一页被盖住那一刻的视口图（侧滑返回时露出来的就是它），与 page_stack 底下几层对齐
    pub(crate) back_shots: Vec<crate::host::edge_back::PageShot>,
    /// 侧滑收尾动画上一次推进的时刻
    pub(crate) edge_back_tick: Option<std::time::Instant>,
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
    pub(super) drawn_band: Option<(f32, f32)>,
    pub(crate) caret_visible: bool,
    /// 固定的动画时钟（仅测试用，见 set_animation_time）
    pub(crate) anim_time: Option<f32>,
    /// fixed 覆盖层实际画过的行区间（设备像素）
    pub(super) fixed_rows: Option<(u32, u32)>,
    /// 合成缓冲（0xRRGGBB）与交给宿主的 RGBA
    pub(super) compose: Vec<u32>,
    pub(super) rgba: Vec<u8>,
    /// `rgba` 落后于 `compose`：只在有人要 RGBA 时才转换（`pixels_into` 直接读 compose）
    pub(super) rgba_stale: bool,
    /// 引擎自己的单调时钟原点（宿主从不给时间时用）
    pub(super) clock_origin: Instant,
    /// 宿主最近一次给的时间戳与收到它的时刻。触摸、长按、事件 timeStamp 全部用
    /// 这一个时钟 —— 从前按下用墙上时钟、长按计时用宿主时钟，两者差几个数量级，
    /// 宿主一传 `now_ms` 长按就永远触发不了。
    pub(super) host_clock: Option<(u64, Instant)>,
    pub(super) last_pump_ms: u64,
    /// 宿主帧时间 → `Instant` 的锚点（`frame_clock`：动画按 vsync 取样，不按画的那一刻）
    pub(super) frame_anchor: crate::frame_clock::Anchor,
    /// 当前页面有没有 `onPageScroll`（按 (栈深, 打开时刻, 路由) 记住，见 scroll_view.rs）
    pub(super) page_scroll_handler: Option<((usize, u64, String), bool)>,
    pub(super) page_opened_ms: u64,
    /// 创建选项（安全区、宿主 API、原生组件；见 engine_options.rs）
    pub(crate) opts: EngineOptions,
    /// 右上角胶囊的按压状态（engine_capsule.rs）
    pub(crate) capsule: super::engine_capsule::CapsuleState,
    /// 引擎画的默认导航栏（`navigationBar` 选项，engine_navbar.rs）
    pub(crate) nav: super::engine_navbar::NavChrome,
    /// 页面转场（`routeAnimation` 选项，route_anim.rs）
    pub(crate) route: super::route_anim::RouteState,
    /// 被盖住的页与切走的 tab 页留着的视图（engine_keep.rs）
    pub(crate) keep: super::page_keep::Keep,
    /// app.json 的 `window`（原样保留，页面事件里与页面 json 合并后交给宿主画导航栏）
    pub(crate) app_window: serde_json::Value,
    /// 引擎 → 宿主的事件队列（见 engine_events.rs）
    pub(crate) events: HostEvents,
    /// 这一帧已由 GPU 画到 surface 上（`pump_ex` 带 PRESENTED，宿主不取像素）
    pub(super) frame_presented: bool,
    /// surface 暂时取不到，下一次 pump 补一帧（`pump_ex` 带 PENDING）
    pub(super) present_pending: bool,
    /// SDK 的 GPU 合成器（宿主挂了原生 surface、门槛与自检都过了才有，见 engine_gpu.rs）
    #[cfg(feature = "gpu")]
    pub(crate) gpu: Option<Box<crate::compositor::gpu::GpuCompositor>>,
    /// surface 被收走时留下的合成器（只放了 surface，设备 / 管线还在），下次挂载直接复用（engine_gpu.rs）
    #[cfg(feature = "gpu")]
    pub(crate) gpu_parked: Option<Box<crate::compositor::gpu::GpuCompositor>>,
    /// 本实例 GPU 运行中失败过（粘性回退的原因）
    #[cfg(feature = "gpu")]
    pub(crate) gpu_failed: Option<String>,
    /// 宿主挂上来的原生 surface（种类 + 指针地址）。存地址而不是裸指针：裸指针会让
    /// `MiniEngine` 失去 `Send`；这里也从不解引用它，只在 `set_backend` / `set_device_info`
    /// 改了选择输入时拿它重建合成器。指针活到 detach 由宿主保证。
    #[cfg(feature = "gpu")]
    pub(crate) native_surface: Option<(crate::compositor::SurfaceKind, usize)>,
    /// 上一帧由 GPU 上屏、没做 CPU 合成：有人取像素时按这组参数（有无 tabBar、页面底色）补一次
    #[cfg(feature = "gpu")]
    pub(crate) cpu_frame_pending: Option<(bool, crate::Color)>,

    // ── 后端选择（见 engine_backend.rs / engine_gpu.rs）。这些字段决定用哪个合成器、
    //    `mr_app_backend*` 报什么、`MINI_FPS` 的标签写什么 ──
    /// 创建时读一次的环境变量（`MINI_BACKEND` / `MINI_GPU_*` / `MINI_FPS`，R14）
    pub(super) env: HostEnv,
    /// 宿主给的偏好（`mr_app_set_backend`）；没给是 `None`
    pub(super) host_pref: Option<BackendPref>,
    pub(super) backend: BackendStatus,
    /// 宿主报的机型信息（机型分级用）
    pub(crate) device_info: DeviceInfo,
    /// 宿主挂上来的 surface 种类（只记种类，指针不收）
    pub(super) surface: Option<SurfaceKind>,
    /// 实际后端被**引擎自己**改了，下一次 `pump_ex` 要带 `FRAME_BACKEND_CHANGED`
    pub(super) backend_changed: bool,
    /// `MINI_FPS` 的每秒统计
    pub(super) stats: FrameStats,
    /// 局部重画的跨帧状态与本帧的活儿归类（见 engine_damage.rs）
    pub(crate) dmg: super::engine_damage::DamageState,
}

impl MiniEngine {
    /// 创建实例。`app_dir` 是解包后的小程序目录，`data_dir` 是宿主沙盒里可写的目录
    /// （storage 与图片磁盘缓存落在这里；不传的话缓存会**静默失效**）。
    ///
    /// 这里是 SDK 的宿主入口：后端相关的环境变量在这里读一次（R14），之后只传值。
    pub fn new(
        app_dir: &str,
        data_dir: Option<&str>,
        width: u32,
        height: u32,
        dpr: f32,
    ) -> Result<Self, String> {
        Self::new_with_env(app_dir, data_dir, width, height, dpr, HostEnv::from_process())
    }

    /// 同 [`new`](Self::new)，但环境变量由调用方给（测试用这个，不碰进程环境）
    pub fn new_with_env(
        app_dir: &str,
        data_dir: Option<&str>,
        width: u32,
        height: u32,
        dpr: f32,
        env: HostEnv,
    ) -> Result<Self, String> {
        Self::new_with_options(app_dir, data_dir, width, height, dpr, env, EngineOptions::default())
    }

    /// 带创建选项（`mr_app_create_ex`）：系统信息与宿主 API 必须在 `app.js` 执行**之前**装好，
    /// 因为 `onLaunch` 里读系统信息、用 `typeof` 探测能力是常态。
    pub fn new_with_options(
        app_dir: &str,
        data_dir: Option<&str>,
        width: u32,
        height: u32,
        dpr: f32,
        env: HostEnv,
        opts: EngineOptions,
    ) -> Result<Self, String> {
        for w in env.warnings(HostKind::Sdk) {
            diag::emit(&w);
        }
        let device_info = DeviceInfo::default();
        let backend = select_backend(&env, None, &device_info, None);
        diag::emit(&diag::selected(&backend));
        if let Some(d) = data_dir {
            crate::data_dir::set_data_dir(d);
        }
        page_loader::set_app_path(std::path::PathBuf::from(app_dir));
        let (width, height) = (width.max(1), height.max(1));
        let dpr = if dpr > 0.1 { dpr } else { 1.0 };

        let mut app = MiniApp::new(width, height)?;
        app.init()?;
        engine_options::install_system_info(&app, &engine_options::system_info(width, height, dpr, &opts));
        app.install_host_apis(&opts.host_apis);
        // 之后解析的样式表（自定义 tabBar、页面）看到宿主给的安全区
        Self::apply_env_context(&opts, width, height);
        // 先把包内所有 .js 注册成 CommonJS 模块：uni-app / TS 产物第一行就 require
        // 一个几百 KB 的 vendor 包，不预注册直接失败。
        let root = page_loader::get_app_path();
        app.register_all_modules(&root).ok();
        app.load_module_script("app", &page_loader::load_app_js())?;

        let app_json = page_loader::load_app_json();
        let config: AppConfig =
            serde_json::from_str(&app_json).map_err(|e| format!("app.json 解析失败: {e}"))?;
        let app_window = serde_json::from_str::<serde_json::Value>(&app_json)
            .ok()
            .and_then(|v| v.get("window").cloned())
            .unwrap_or(serde_json::Value::Null);
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
        let tabbar_px = ((tabbar_logical_height(custom_tabbar.is_some(), &opts) * dpr) as u32).max(1);
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
            canvas: PageCanvas::new(Canvas::new(px(width), px(CONTENT_HEIGHT.max(height)))),
            fixed_canvas: LayerCanvas::new(Canvas::new(px(width), px(height))),
            tabbar_canvas: LayerCanvas::new(Canvas::new(px(width), tabbar_px)),
            interaction: InteractionManager::new(),
            scroll: ScrollController::new(vp, vp),
            touch: crate::host::touch::TouchTracker::new(),
            gesture: None,
            resample: Default::default(),
            picker_sheet: None,
            edge_back: None,
            back_shots: Vec::new(),
            edge_back_tick: None,
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
            rgba_stale: false,
            clock_origin: Instant::now(),
            host_clock: None,
            last_pump_ms: 0,
            frame_anchor: Default::default(),
            page_scroll_handler: None,
            page_opened_ms: 0,
            opts,
            capsule: Default::default(),
            nav: Default::default(),
            route: Default::default(),
            keep: Default::default(),
            app_window,
            events: HostEvents::default(),
            frame_presented: false,
            present_pending: false,
            #[cfg(feature = "gpu")]
            gpu: None,
            #[cfg(feature = "gpu")]
            gpu_parked: None,
            #[cfg(feature = "gpu")]
            gpu_failed: None,
            #[cfg(feature = "gpu")]
            native_surface: None,
            #[cfg(feature = "gpu")]
            cpu_frame_pending: None,
            stats: FrameStats::new(env.fps),
            dmg: super::engine_damage::DamageState {
                no_damage: env.no_damage,
                log: env.damage_log,
                ..Default::default()
            },
            env,
            host_pref: None,
            backend,
            device_info,
            surface: None,
            backend_changed: false,
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

    /// 打开一个页面（不传就开首页）。可以带 query（`pages/x/x?id=1`）：深链冷启动直接落到
    /// 带参数的页面，参数进 `onLoad(query)`。从前 query 被当成路径的一部分，「页面不存在」。
    pub fn launch(&mut self, route: Option<&str>) -> Result<(), String> {
        let route = route.map(str::to_string).unwrap_or_else(|| self.home_route());
        let (path, query) = crate::host::navigation::parse_url(&route);
        self.navigate_to(&path, query)
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

    /// 在逻辑层执行一段 JS（宿主的深链参数注入、无头测试的断言都要用）
    pub fn eval(&self, code: &str) -> Result<String, String> {
        self.app.eval(code)
    }

    pub fn page_depth(&self) -> usize {
        self.page_stack.len()
    }

    pub(crate) fn is_tabbar_page(&self, path: &str) -> bool {
        // `wx.hideTabBar()` 之后按「不是 tabBar 页」排版与命中（host::tabbar_api）
        !crate::host::tabbar_api::hidden()
            && self.config
            .tab_bar
            .as_ref()
            .map(|tb| tb.list.iter().any(|i| i.page_path == path))
            .unwrap_or(false)
    }

    /// 页面视口高：画面高减引擎画的导航栏（engine_navbar.rs）、再减 tabBar
    pub(crate) fn viewport_height(&self) -> f32 {
        let tab = if self.is_tabbar_page(&self.current_route()) { self.tabbar_h() } else { 0.0 };
        (self.page_window_height() - tab).max(1.0)
    }
    /// tabBar 占的逻辑高（含底部安全区，见 [`tabbar_logical_height`]）
    pub(crate) fn tabbar_h(&self) -> f32 {
        tabbar_logical_height(self.custom_tabbar.is_some(), &self.opts)
    }
}

/// tabBar 占的逻辑高。原生（app.json 配置式）tabBar 底下再垫一条底部安全区 —— 微信在有
/// 底部横条 / 虚拟导航键的机型上就是这样：图标与文字仍在上面那 50px 里，底下那条是 tabBar 底色。
/// 不垫的话宿主把画面铺到屏幕底边时，tabBar 整条压在系统导航键下面（荣耀三键导航实测：
/// 导航键占底部 38px，50px 的 tabBar 几乎全被盖住）。
/// 自定义 tabBar 的高度是按组件 WXSS 实测的，安全区由组件自己写 `env(safe-area-inset-bottom)`，不再加。
fn tabbar_logical_height(custom: bool, opts: &EngineOptions) -> f32 {
    let safe_bottom = if custom { 0.0 } else { opts.safe_area.map(|s| s.bottom.max(0.0)).unwrap_or(0.0) };
    tabbar_height() as f32 + safe_bottom
}
