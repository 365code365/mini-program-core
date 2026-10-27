//! 小游戏运行实例（game-doc/02 §2，决策 D-1 方案 B：与 `MiniEngine` 平级）。
//!
//! G1 这一步只有「运行模式骨架」：读 `game.json`、装小游戏的前置代码、把包内 `.js` 注册成模块、
//! 以模块作用域执行 `game.js`、每次 `pump` 推进逻辑层并把日志 / 错误交给宿主。
//! 主画布、rAF、上屏（G2）、触摸（G3）在后面几期接进来；这里先把它们要挂的位置留好。
//!
//! 为什么复用 `MiniApp`：它就是「QuickJS + 原生桥 + 每帧推进定时器 / 网络 / IO」那一层，与页面模型无关；
//! 页面相关的东西都在 `MiniEngine` 里，小游戏不碰。

pub mod audio;
mod audio_events;
pub mod canvas;
mod clock;
pub mod config;
mod fonts;
pub mod image;
mod lifecycle;
mod package;
mod window;
mod image_events;
mod input;
pub mod pc_input;
pub mod touch;
mod frame;
pub use frame::FRAME_WAKE_AT;
mod loader;
mod logs;
pub mod raf_pacer;

use crate::app_profile::{detect, AppProfile};
use crate::js::PreludeProfile;
use crate::runtime::MiniApp;
use config::GameConfig;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// 创建选项（`mr_app_create_ex` 的 JSON 在游戏实例上的子集；G1 起逐步补齐）
#[derive(Clone, Debug, Default)]
pub struct GameOptions {
    /// 小游戏 appId：数据命名空间与 Referer 都用它（G1.10 / 11 §7 N-1）。缺省回落包目录名
    pub app_id: Option<String>,
    /// 合并进 `wx.getSystemInfoSync()` 的字段（与小程序的 `systemInfo` 选项同义）
    pub system_info: serde_json::Map<String, Value>,
    /// 启动参数（`wx.getLaunchOptionsSync()`）
    pub launch_options: Option<Value>,
    /// 关掉每帧日志限流（`MINI_GAME_LOG_ALL=1` 同义，调试用）
    pub log_all: bool,
    /// 单次进入 JS 的时间上限（毫秒，默认 5000；`MINI_GAME_WATCHDOG_MS` 覆盖），超了中断并报 `watchdog`
    pub watchdog_ms: Option<u64>,
    /// JS 堆上限（MB，默认 1024；0 = 不限）。超了 JS 侧抛 out of memory，进程不崩
    pub js_memory_limit_mb: Option<u64>,
    /// 故障注入（测试用，与 `MINI_GAME_FAULT` 同义）：`panic@pump` / `panic@eval`，验证 FFI 的 panic 防护
    pub fault: Option<String>,
    /// 固定时钟（无头测试）：宿主给的时刻就是游戏时间，帧间 performance.now() 停住，结果逐帧可复现（clock.rs）
    pub fixed_clock: bool,
    /// 从独立分包启动（分包名或 root）：执行它的入口而不是 game.js，主包要 `loadSubpackage({name:'__GAME__'})`（G6.1，GT-P5）
    pub entry_subpackage: Option<String>,
}

/// 游戏实例报给 `wx.getSystemInfoSync().SDKVersion` 的基础库版本（决策 D-9 未定：取证后按已实现的 API 集合选定，
/// 之后随实现进度升；`wx.canIUse` 必须与它一致）。先沿用小程序默认值，免得报高了让游戏调用我们还没有的 API
pub const GAME_SDK_VERSION: &str = "3.0.0";

/// 编译目标对应的 `platform`（game-doc/09 §2.3）：游戏引擎普遍按它分支，`devtools` 会走调试逻辑。
/// 只有不在这几个目标上（Linux 等开发 / CI 机器）才报 `devtools`
pub fn target_platform() -> &'static str {
    if cfg!(target_os = "android") { "android" }
    else if cfg!(target_os = "ios") { "ios" }
    else if cfg!(target_os = "macos") { "mac" }
    else if cfg!(target_os = "windows") { "windows" }
    else { "devtools" }
}

/// 看门狗默认值（game-doc/02 §3.3）
pub const DEFAULT_WATCHDOG_MS: u64 = 5000;
pub const DEFAULT_JS_MEMORY_MB: u64 = 1024;

impl GameOptions {
    /// 解析创建选项 JSON（空串 / null = 全默认）
    pub fn from_json(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.is_empty() || text == "null" {
            return Ok(Self::default());
        }
        let v: Value = serde_json::from_str(text).map_err(|e| format!("创建选项不是合法 JSON：{e}"))?;
        Ok(Self {
            app_id: v.get("appId").and_then(|x| x.as_str()).map(str::to_string),
            system_info: v.get("systemInfo").and_then(|x| x.as_object()).cloned().unwrap_or_default(),
            launch_options: v.get("launchOptions").cloned(),
            log_all: v.get("logAll").and_then(|x| x.as_bool()).unwrap_or(false),
            watchdog_ms: v.get("watchdogMs").and_then(|x| x.as_u64()),
            js_memory_limit_mb: v.get("jsMemoryLimitMB").and_then(|x| x.as_u64()),
            fault: v.get("fault").and_then(|x| x.as_str()).map(str::to_string),
            fixed_clock: v.get("fixedClock").and_then(|x| x.as_bool()).unwrap_or(false),
            entry_subpackage: v.get("entrySubpackage").and_then(|x| x.as_str()).map(str::to_string),
        })
    }
}

pub struct GameEngine {
    app: MiniApp,
    config: GameConfig,
    root: PathBuf,
    width: u32,
    height: u32,
    dpr: f32,
    events: VecDeque<String>,
    logs: logs::LogPump,
    guards: crate::js::Guards,
    fault: Option<String>,
    canvases: std::rc::Rc<std::cell::RefCell<canvas::GameCanvases>>,
    pacer: std::rc::Rc<std::cell::RefCell<raf_pacer::RafPacer>>,
    clock: std::rc::Rc<std::cell::RefCell<clock::GameClock>>,
    /// 合成好的输出帧（RGBA，主画布分辨率）与它是不是最新的
    frame: frame::FrameOut,
    backgrounded: bool,
    touch: touch::TouchInput,
    pc: pc_input::PcInput,
    images: std::rc::Rc<std::cell::RefCell<image::ImageStore>>,
    audio: std::rc::Rc<std::cell::RefCell<audio::GameAudio>>,
    /// 当前方向（portrait / landscape / landscapeReverse，window.rs）
    orientation: String,
    /// 宿主给的系统信息覆盖项（resize 时重算系统信息要用）
    system_info_opts: serde_json::Map<String, Value>,
}

impl GameEngine {
    /// 建实例并执行 `game.js`。
    ///
    /// `game.js` 顶层抛错**不算**创建失败（R5）：实例照常可以 pump，错误交给 `wx.onError` 与宿主的 `error` 事件 ——
    /// 微信里也是这样，游戏只是停在出错的那一步。只有包本身不合法（不是小游戏、game.json 坏了）才返回 Err。
    pub fn new(app_dir: &str, data_dir: Option<&str>, width: u32, height: u32, dpr: f32, opts: GameOptions) -> Result<Self, String> {
        let root = PathBuf::from(app_dir);
        match detect(&root).map_err(|e| e.to_string())? {
            AppProfile::MiniGame => {}
            AppProfile::MiniProgram => return Err(format!("{app_dir} 是小程序（有 app.json），不能按小游戏打开")),
        }
        let text = std::fs::read_to_string(root.join("game.json")).map_err(|e| format!("读不出 game.json：{e}"))?;
        let config = GameConfig::parse(&text)?;
        let source = std::fs::read_to_string(root.join("game.js")).map_err(|e| format!("读不出 game.js：{e}"))?;
        if let Some(d) = data_dir {
            crate::data_dir::set_data_dir(d);
        }
        // 数据按 appId 隔离（game-doc/09 §2.2）：必须在建运行时之前设，storage 在桥初始化时就读回
        let app_id = opts.app_id.clone().unwrap_or_else(|| dir_name(&root));
        crate::data_dir::set_app_scope(Some(&app_id));
        // 包内资源（图片、音频、require 的基准）都按包根解析
        crate::host::page_loader::set_app_path(root.clone());
        let (width, height) = (width.max(1), height.max(1));
        let dpr = if dpr > 0.1 { dpr } else { 1.0 };

        let mut app = MiniApp::new(width, height)?;
        // 小游戏自己的原生函数要在前置代码之前注册（game-canvas.js 加载时就取 __game_2d）
        let canvases = std::rc::Rc::new(std::cell::RefCell::new(canvas::GameCanvases::new((width, height))));
        let pacer = std::rc::Rc::new(std::cell::RefCell::new(raf_pacer::RafPacer::default()));
        let clock = std::rc::Rc::new(std::cell::RefCell::new(clock::GameClock::new(opts.fixed_clock)));
        let images = std::rc::Rc::new(std::cell::RefCell::new(image::ImageStore::new(root.clone())));
        // 无头（固定时钟）不开声卡：混音器按帧时钟离线渲染，逐样本可复现
        let audio = std::rc::Rc::new(std::cell::RefCell::new(audio::GameAudio::new(root.clone(), opts.fixed_clock)));
        {
            let rt = app.js_runtime();
            let rt = rt.lock().unwrap();
            canvas::register(&rt, canvases.clone(), images.clone())?;
            image::register(&rt, images.clone())?;
            fonts::register(&rt, root.clone())?;
            audio::register(&rt, audio.clone())?;
            audio::register_webaudio(&rt, audio.clone())?;
            frame::register(&rt, pacer.clone(), clock.clone())?;
        }
        app.init_profile(PreludeProfile::MiniGame)?;
        let config_landscape = config.device_orientation.is_landscape();
        let eo = crate::host::EngineOptions { system_info: opts.system_info.clone(), ..Default::default() };
        let mut info = crate::host::engine_options::system_info(width, height, dpr, &eo);
        info["deviceOrientation"] = json!(config.device_orientation.as_str());
        // 宿主不能把 platform 覆盖成 devtools，也不能改 SDKVersion（与 canIUse 必须一致）
        let host_platform = info.get("platform").and_then(|v| v.as_str()).map(str::to_string);
        info["platform"] = json!(target_platform());
        info["SDKVersion"] = json!(GAME_SDK_VERSION);
        crate::host::engine_options::install_system_info(&app, &info);
        crate::host::engine_options::install_launch_context(&app, &app_id, opts.launch_options.as_ref());
        // game.json 的 networkTimeout：request / connectSocket / uploadFile / downloadFile 没给 timeout 时的默认（GT-N6）
        let nt = json!({ "request": config.timeout_request, "connectSocket": config.timeout_connect_socket,
            "uploadFile": config.timeout_upload_file, "downloadFile": config.timeout_download_file });
        let _ = app.eval(&format!("globalThis.__networkTimeout = {nt};"));

        let watchdog_ms = std::env::var("MINI_GAME_WATCHDOG_MS").ok().and_then(|v| v.parse().ok())
            .or(opts.watchdog_ms).unwrap_or(DEFAULT_WATCHDOG_MS);
        let mem = opts.js_memory_limit_mb.unwrap_or(DEFAULT_JS_MEMORY_MB) as usize * 1024 * 1024;
        let guards = app.js_runtime().lock().unwrap().install_guards(watchdog_ms, mem);
        let host_platform_note = host_platform.filter(|p| p != target_platform());
        let mut engine = Self { app, config, root, width, height, dpr, events: VecDeque::new(), logs: logs::LogPump::new(opts.log_all), guards,
            fault: opts.fault.clone().or_else(|| std::env::var("MINI_GAME_FAULT").ok()),
            canvases, pacer, clock, frame: frame::FrameOut::default(), backgrounded: false,
            touch: Default::default(), pc: Default::default(), images, audio,
            orientation: if config_landscape { "landscape".into() } else { "portrait".into() },
            system_info_opts: opts.system_info.clone() };
        engine.announce_orientation();
        let log = std::env::var("MINI_GAME_LOG").is_ok();
        if opts.app_id.is_none() {
            engine.host_log(&format!("⚠️ 创建选项没给 appId，数据命名空间回落成包目录名 {app_id:?}：换了解包目录名存档就会丢"));
        }
        if let Some(p) = host_platform_note {
            engine.host_log(&format!("⚠️ 宿主给的 systemInfo.platform={p:?} 被忽略：小游戏实例按编译目标报 {:?}", target_platform()));
        }
        for w in engine.config.warnings.clone() {
            engine.host_log(&format!("⚠️ {w}"));
        }
        if log && !engine.config.unknown_fields.is_empty() {
            engine.host_log(&format!("🎮 game.json 未知字段：{:?}", engine.config.unknown_fields));
        }
        let (n, errors) = loader::register_modules(&engine.app, &engine.root);
        for e in errors {
            engine.report_error("module", &format!("模块 {} 注册失败：{}", e.key, e.message));
        }
        if log {
            engine.host_log(&format!("🎮 小游戏：{}（{}，{n} 个模块）", engine.root.display(), engine.config.device_orientation.as_str()));
        }
        // game.js 顶层登记的定时器从游戏时间轴的此刻起算（固定时钟下是 0），不是墙上时钟
        let at = engine.clock.borrow().current_instant();
        engine.app.set_timer_clock(Some(at));
        let entry = engine.install_packages(opts.entry_subpackage.as_deref());
        engine.guards.watchdog.arm();
        let r = match &entry {
            // 独立分包：经 require 执行它的入口（模块缓存里记一笔，之后 loadSubpackage 同一个分包不会再执行一遍）
            Some(key) => engine.app.eval(&format!("__require('', {key:?})")).map(|_| ()),
            None => loader::run_module(&engine.app, "game", &source),
        };
        engine.guards.watchdog.disarm();
        if let Err(e) = r {
            engine.report_error(error_kind(&e, "script"), &e);
        }
        engine.after_js();
        Ok(engine)
    }

    /// 取一条宿主事件（JSON 串：`{"type":"log","text"}` / `{"type":"error","kind","message"}`），没有了返回 None
    pub fn poll_event(&mut self) -> Option<String> {
        self.events.pop_front()
    }

    /// 执行一段 JS（诊断与测试；`mr_app_eval` 同义）
    pub fn eval(&mut self, js: &str) -> Result<String, String> {
        self.inject("panic@eval");
        self.guards.watchdog.arm();
        let r = self.app.eval(js);
        self.guards.watchdog.disarm();
        self.after_js();
        r
    }

    fn inject(&self, at: &str) {
        if self.fault.as_deref() == Some(at) {
            panic!("MINI_GAME_FAULT={at}（故障注入）");
        }
    }

    pub fn config(&self) -> &GameConfig {
        &self.config
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn logical_size(&self) -> (u32, u32, f32) {
        (self.width, self.height, self.dpr)
    }

    /// 每段 JS 跑完之后：仍然没人处理的 Promise 拒绝交给 wx.onUnhandledRejection，再收日志
    fn after_js(&mut self) {
        {
            let rt = self.app.js_runtime();
            let rt = rt.lock().unwrap();
            rt.flush_rejections(&self.guards);
            if rt.call_num("__gameGcTake", &[]).unwrap_or(0.0) != 0.0 {
                rt.collect_garbage();
            }
        }
        self.drain_logs();
        // JS 交给宿主的事件（exit / restart、数据上报、hostCall）：从前小游戏没取，宿主一条也收不到
        for raw in self.app.take_host_events() {
            self.events.push_back(raw);
        }
    }

    /// `wx.triggerGC` / 宿主内存告警：跑一遍 GC
    pub fn collect_garbage(&self) {
        self.app.js_runtime().lock().unwrap().collect_garbage();
    }

    fn drain_logs(&mut self) {
        for line in self.logs.drain(&self.app) {
            self.events.push_back(json!({ "type": "log", "text": line }).to_string());
        }
    }

    fn host_log(&mut self, text: &str) {
        self.events.push_back(json!({ "type": "log", "text": text }).to_string());
    }

    /// 原生侧捕获到的错误：打日志、交给 `wx.onError`（游戏里注册了的话）、发宿主 `error` 事件
    fn report_error(&mut self, kind: &str, message: &str) {
        let (msg, stack) = split_exception(message);
        let call = format!("typeof __gameDispatchError === 'function' ? __gameDispatchError({}, {}) : 0", json!(msg), json!(stack));
        let _ = self.app.eval(&call);
        self.drain_logs();
        self.events.push_back(json!({ "type": "error", "kind": kind, "message": msg, "stack": stack }).to_string());
        self.host_log(&format!("[error] {message}"));
    }
}

/// 看门狗中断的异常是 `InternalError: interrupted`：单独标成 watchdog，宿主据此区分「卡死」与普通异常
fn error_kind(e: &str, default: &'static str) -> &'static str {
    if e.contains("interrupted") { "watchdog" } else if e.contains("out of memory") { "oom" } else { default }
}

impl Drop for GameEngine {
    /// 数据命名空间是进程级的（data_dir::set_app_scope）：实例没了就清掉，否则之后在同一进程里打开的
    /// 小程序（或并行跑的测试）会把 storage 写进这个游戏的目录（踩过：lib 测试并行时 storage 用例读到了游戏的路径）
    fn drop(&mut self) {
        crate::data_dir::set_app_scope(None);
    }
}

fn dir_name(root: &Path) -> String {
    root.file_name().and_then(|s| s.to_str()).unwrap_or("game").to_string()
}

/// `JsRuntime::eval` 的错误串是 `JS Exception: <Name>: <msg>\n    <栈首两行>`：拆成 message 与 stack
fn split_exception(e: &str) -> (String, String) {
    let e = e.strip_prefix("JS Exception: ").unwrap_or(e);
    let mut it = e.splitn(2, '\n');
    let first = it.next().unwrap_or("").to_string();
    let stack = it.next().unwrap_or("").trim().to_string();
    // 官方 onError 的 message 不带类型名前缀以外的东西；保留 `TypeError: …` 整句（微信真机的格式待取证）
    (first, stack)
}
