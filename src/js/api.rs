//! 小程序 API 的注册：把逻辑层的 JS 前置代码（prelude）按域喂给 QuickJS。
//!
//! ## 为什么 JS 不再写在 Rust 字符串里
//! 这个文件曾经是 **1637 行，其中 1515 行是 `r#"..."#` 里的 JS**。代价很实：
//! 编辑器不给高亮、不给格式化、没有跳转，改一行 `wx.request` 要在 Rust 缩进里数括号；
//! 而且它把本项目「单文件不超过 500 行」的规则超了三倍多。
//!
//! 现在 JS 按域拆成 `src/js/prelude/*.js`，Rust 侧只剩一张注册表。`include_str!` 是
//! **编译期**展开，产物里仍然是同一串字节 —— 运行时零成本，也不用担心文件丢失
//! （编不过就是缺文件，而不是运行到一半才发现）。
//!
//! ## 顺序是有意义的，别随手调
//! 表的顺序就是执行顺序，每一条都有原因：
//! - `module` / `console` 必须最先：框架运行时（uni-app 的 vendor 包）第一行就 `require`，
//!   而任何一处出错都要有 `console` 才看得见；
//! - `wx` 对象要在所有 `wx.xxx = ...` 之前建出来；
//! - `app-page` 要早于 `lifecycle` / `page-components`：后两者要用它建好的页面栈与实例；
//! - `api-probe` 必须**最后**装：它用 `Proxy` 包住已经注册完的 `wx`，早了就会把
//!   后续注册的 API 也当成「未实现」。

use super::JsRuntime;
use std::sync::{Arc, Mutex};

/// 一段前置代码：出错时用 `name` 报出是哪一域，便于定位。
struct Prelude {
    name: &'static str,
    source: &'static str,
}

/// 宏只为省掉重复的字段名，顺序即执行顺序。
macro_rules! prelude {
    ($($name:literal => $file:literal,)*) => {
        &[$(Prelude { name: $name, source: include_str!(concat!("prelude/", $file)) },)*]
    };
}

const PRELUDE: &[Prelude] = prelude![
    // CommonJS 模块系统 + `global`/`self`/`window` 别名。
    // 官方小程序逻辑层以 CommonJS 组织多文件依赖；打包器与框架运行时还会摸这些全局
    // 别名做环境探测，缺 `global` 时 Vue 运行时在第一次特征检测就 ReferenceError。
    "module" => "module.js",
    // console 与**错误对象的可读化**：`Error` 的 message/stack 是不可枚举属性，
    // 直接 JSON.stringify 只会得到 `{}` —— 屏幕上只有 `JS Exception: {}`，
    // 跑第三方编译产物时等于没有报错信息。
    "console" => "console.js",
    // 可注入的固定时钟（`MINI_FAKE_NOW`）：必须在任何页面逻辑之前替换掉 `Date`，
    // 否则页面在 onLoad 里算出来的「今天」还是真实日期
    "fake-clock" => "fake-clock.js",
    // 所有 `wx.xxx = ...` 的落点，必须先存在
    "wx-object" => "wx-object.js",
    // base64 / UTF-8 编解码（QuickJS 没有 btoa / TextEncoder）：二进制过桥都靠它，socket / fs 之前
    "codec" => "codec.js",
    // 定时器：只登记，真正的到点由宿主每帧驱动（逻辑层不占线程）
    "timer" => "timer.js",
    // storage 全套：写操作**写穿到磁盘**，所以「首次拉取存起来、之后走缓存」的
    // 业务分支才会真的走到第二条（从前只有进程内 HashMap，每次启动都是空的）
    "storage" => "storage.js",
    // Toast / Loading / Modal / ActionSheet / 下拉刷新 / createAnimation
    "ui" => "ui.js",
    // `wx.createCanvasContext`：把 2D 指令录下来交给原生光栅化
    "canvas" => "canvas.js",
    // `wx.createVideoContext`：命令经 `__native_host_request('video')` 交给宿主（host::video）
    "media" => "media.js",
    // SelectorQuery / IntersectionObserver / nextTick：查询与相交计算由宿主按布局树做
    // （host::selector_query / host::observers），结果异步回调
    "query" => "query.js",
    // `<canvas type="2d">` 的 Canvas 节点与属性式 2D 上下文（经 SelectorQuery 的 node() 拿到）
    "canvas2d" => "canvas2d.js",
    // App / Page 注册、`setData` 的数据路径（`a.b.c` / `list[0].x`）、页面栈。
    // 这里也桥了 `wx.createPage/createComponent/createApp` —— uni-app / Taro 走的是
    // 那三个，只认 `Page()` 的话页面永远注册不上，屏幕上只剩一张静态骨架。
    "app-page" => "app-page.js",
    // `wx.onError` / `onUnhandledRejection` / `onPageNotFound` / `onThemeChange`：
    // 与 `App({onError})` **并存**（微信里两边都收到）。必须真的实现 ——
    // 框架的包装层能通过 `typeof uni.onError === 'function'` 的探测，
    // 底下缺了就是 `cannot read property 'apply' of undefined`，整个 app 脚本中断。
    "app-hooks" => "app-hooks.js",
    // 三级生命周期分发 + `__dispatchEvent`（事件对象由 Rust 侧构造后送进来）
    "lifecycle" => "lifecycle.js",
    // 页面 json 里 `usingComponents` 声明的组件：挂载、属性传递、事件归属。
    // 事件要送给「声明它的那个组件实例」，不是碰巧同名的页面 —— uni-app 给页面和
    // 每个组件各自生成 `e0_0` 这种短名字，撞名是常态。
    "page-components" => "page-components.js",
    // 五个路由 API：只把请求写进 `__pendingNavigation`，由宿主取走执行
    "route" => "route.js",
    // 设备与系统信息：`safeArea` / `safeAreaInsets` / `statusBarHeight` 这些是布局
    // 算式的输入，少一个字段就是一个 `cannot read property 'bottom' of undefined`
    "device" => "device.js",
    // `wx.request`（含 `RequestTask.abort()`）/ `downloadFile` / `loadFontFace`：
    // 回调式、非阻塞，响应由宿主每帧取回后调 `__resolveRequest`
    "network" => "network.js",
    // 导航栏 API 与宿主实现的 `wx.*`（hostCall）：意图经 `__native_host_event` 交给宿主外壳。
    // 要早于 api-probe：探针包住 wx 之后再加的属性照样可见，但放这里语义更清楚
    "host" => "host.js",
    // 选图 / 选视频 / 选文件 / 存相册 / 打开文档：宿主点名之后的参数归一、移动端「拍照 / 从相册选」
    // 的操作菜单、回包按官方形状补齐（host.js 的 __hostCallHooks；没点名的不经过它）
    "host-media" => "host-media.js",
    // 剪贴板、振动、操作菜单、图片预览，与「要宿主才能做」的能力（登录、选图、定位…）的默认实现：
    // 宿主在 hostApis 里点名的由 __installHostApis 换成真的，没点名的立刻 fail（而不是 undefined）
    "system" => "system.js",
    // 原生 tabBar 的运行时 API（角标、红点、改项、改样式、显隐），由 host::tabbar_api 落地
    "tabbar" => "tabbar.js",
    // WebSocket（SocketTask 与旧式全局 API），连接在原生线程（src/websocket.rs）
    "socket" => "socket.js",
    // FileSystemManager 与旧式 saveFile / getFileInfo：路径权限与文件操作在 src/wxfs
    "fs" => "fs.js",
    // InnerAudioContext / 背景音频 / 旧式 AudioContext / RecorderManager（录音交给宿主）
    "audio" => "audio.js",
    // 网络状态、胶囊、canIUse、更新、账号、分享菜单、屏幕、背景色、电量、键盘、日志、图片信息
    "app-info" => "app-info.js",
    // ── 对照微信补齐的那批（2026-10，见 .agents/tasks/wx-gap-2026-10-23/gap.md）──
    // 要宿主才能做的能力：默认立刻 fail（宿主在 hostApis 点名就交给宿主），监听型 wx.onXxx 的统一登记与宿主推事件
    "host-capabilities" => "host-capabilities.js",
    // 基础 / 系统 / 随机数 / 隐私 / 广告 / AI：引擎里能真做的就真做（__asyncOk / __asyncFail 也在这里）
    "base-ext" => "base-ext.js",
    // 由宿主承载的对象：MapContext / CameraContext / LivePlayer / LivePusher / NFC / MediaRecorder / VideoDecoder …
    "host-objects" => "host-objects.js",
    // wx.createTCPSocket / createUDPSocket / getLocalIPAddress（src/net_socket.rs）
    "net-socket" => "net-socket.js",
    // 路由：校验、success 时机、EventChannel、路由事件、返回询问、rewriteRoute、exit / restart
    "route-ext" => "route-ext.js",
    // 页面 / 组件实例补齐的方法（options、getOpenerEventChannel、router、getPageId …）
    "page-ext" => "page-ext.js",
    // 旧版语音 / 背景音频 / 录音、previewMedia、getVideoInfo、createOffscreenCanvas
    "media-ext" => "media-ext.js",
    // wx.createWorker、this.createMediaQueryObserver
    "worker-mq" => "worker-mq.js",
    // this.animate / clearAnimation：按选择器写节点样式（data 的 __nodeStyles）与共用的帧循环
    "animate" => "animate.js",
    // wx.worklet（共享值 / 派生值 / timing / spring / decay / 组合 / Easing）、applyAnimatedStyle、wx.router
    "worklet" => "worklet.js",
    // wx.createWebAudioContext（节点图、AudioParam 自动化、decodeAudioData）与 createMediaAudioPlayer
    "webaudio" => "webaudio.js",
    // `Component()` / `Behavior()` 与组件实例工厂（properties / observers / triggerEvent）
    "component" => "component.js",
    // 组件实例树：每处使用一个实例，宿主每帧对账（建实例 / 改属性触发 observer / detached）
    "component-tree" => "component-tree.js",
    // wxs 模块：模板表达式 `{{m.f(x)}}` 由渲染层经 `parser::expr::wxs` 的宿主调到这里
    "wxs" => "wxs.js",
    // **最后**：给 `wx` 装未实现 API 的探针。关键是仍然返回 `undefined` ——
    // 框架普遍用 `typeof wx.xxx === 'function'` 做能力探测，返回假函数会让它们
    // 走进不存在的分支。
    "api-probe" => "api-probe.js",
];

/// 应用种类决定装哪些前置代码（game-doc/02 §3.1）。
///
/// 小程序装**整张表、原顺序**（与引入 profile 之前逐字节相同的执行序列）；
/// 小游戏只装 [`GAME_CORE`] 里的通用段，同样按表的顺序，再接小游戏自己的那几段（G1 起）。
/// 不把表拆成两张：顺序本身有意义（见文件头），拆开以后两张表的相对顺序就没人守了。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreludeProfile {
    MiniProgram,
    MiniGame,
}

/// 小游戏也要的通用段（名字须在 `PRELUDE` 里，测试守着）。页面栈、组件、路由、tabBar、
/// WXML 查询这类只对小程序有意义的段不在这里。`api-probe` 仍然最后装（表里它就在最后）。
pub const GAME_CORE: &[&str] = &[
    "module", "console", "fake-clock", "wx-object", "codec", "timer", "storage", "device", "network",
    "host", "system", "socket", "fs", "audio", "app-info", "host-capabilities", "base-ext", "net-socket",
    "media-ext", "webaudio", "api-probe",
];

fn in_profile(name: &str, profile: PreludeProfile) -> bool {
    match profile {
        PreludeProfile::MiniProgram => true,
        PreludeProfile::MiniGame => GAME_CORE.contains(&name),
    }
}

/// 只装进小游戏的段（顺序即执行顺序），插在通用段之后、`api-probe` 之前。
/// 不进 `PRELUDE`：小程序的那张表一个字节都不能变（G0.4 的等价证明守着）
const GAME_EXTRA: &[Prelude] = prelude![
    // GameGlobal、console 缓冲泄漏的止血、wx.onError / onUnhandledRejection、回调守卫 __gameCall
    "game-global" => "game-global.js",
    // wx.encode / wx.decode、原生 base64（二进制桥，js/bridge_bin.rs）
    "game-compat" => "game-compat.js",
    // 官方 API 表面的宿主代理 / 监听型补齐与小游戏版 canIUse（game-doc/10；延期项见 取证/api-surface-deferred.json）
    "game-surface" => "game-surface.js",
    // requestAnimationFrame / cancelAnimationFrame / setPreferredFramesPerSecond / performance.now（帧时钟）
    "game-raf" => "game-raf.js",
    // wx.createImage（解码在原生后台线程，game/image）；要在 game-canvas 之前：drawImage 认 __GameImage
    "game-image" => "game-image.js",
    // wx.createCanvas、Canvas、CanvasRenderingContext2D（立即模式，原生在 game/canvas/natives.rs）
    "game-canvas" => "game-canvas.js",
    // wx.onTouch* / onMouse* / onWheel 的监听表与派发（原生在 game/touch.rs、game/pc_input.rs）
    "game-input" => "game-input.js",
    // InnerAudioContext 接小游戏自己的混音器与真设备（game/audio）：只换掉原生入口，外壳仍是 audio.js
    "game-audio" => "game-audio.js",
    // wx.onShow / onHide、exit / restartMiniProgram、loadSubpackage（G6.1 / G6.2；分包表由原生在 game.js 之前设）
    "game-lifecycle" => "game-lifecycle.js",
    // wx.onWindowResize（小游戏形状）/ onDeviceOrientationChange / setDeviceOrientation（G6.3，原生在 game/window.rs）
    "game-window" => "game-window.js",
    // wx.createWebAudioContext 换成真混音图（game/audio/graph + web）；没编进去时（__game_webaudio 不存在）保留 webaudio.js 那份
    "game-webaudio" => "game-webaudio.js",
];

/// 一个应用种类实际执行的段（按顺序）
fn sequence(profile: PreludeProfile) -> Vec<&'static Prelude> {
    let base = PRELUDE.iter().filter(|p| in_profile(p.name, profile));
    match profile {
        PreludeProfile::MiniProgram => base.collect(),
        PreludeProfile::MiniGame => {
            let (probe, rest): (Vec<_>, Vec<_>) = base.partition(|p| p.name == "api-probe");
            rest.into_iter().chain(GAME_EXTRA.iter()).chain(probe).collect()
        }
    }
}

/// 小程序 API
pub struct MiniAppApi {
    runtime: Arc<Mutex<JsRuntime>>,
}

impl MiniAppApi {
    pub fn new(runtime: Arc<Mutex<JsRuntime>>) -> Self {
        Self { runtime }
    }

    /// 按注册表顺序注入全部前置代码。
    ///
    /// 每段单独一次 `eval` 并单独持锁：出错时能报出域名，也避免长时间独占运行时。
    /// 分段不改变语义 —— `var` / `function` 声明都落在全局，后一段照样看得见前一段。
    pub fn init(&self) -> Result<(), String> {
        self.init_profile(PreludeProfile::MiniProgram)
    }

    /// 按应用种类注入前置代码（见 [`PreludeProfile`]）
    pub fn init_profile(&self, profile: PreludeProfile) -> Result<(), String> {
        let mut n = 0;
        for p in sequence(profile) {
            let rt = self.runtime.lock().unwrap();
            rt.eval(p.source).map_err(|e| format!("{}: {e}", p.name))?;
            n += 1;
        }
        println!("    prelude: {} 段 JS 已注入", n);
        Ok(())
    }

    /// 前置代码的域名列表（诊断与测试用：确认注册表没被漏掉一段）
    pub fn prelude_names() -> Vec<&'static str> {
        Self::prelude_names_for(PreludeProfile::MiniProgram)
    }

    /// 某个应用种类实际装的段（按执行顺序）
    pub fn prelude_names_for(profile: PreludeProfile) -> Vec<&'static str> {
        sequence(profile).into_iter().map(|p| p.name).collect()
    }
}
