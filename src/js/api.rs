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
    // 所有 `wx.xxx = ...` 的落点，必须先存在
    "wx-object" => "wx-object.js",
    // 定时器：只登记，真正的到点由宿主每帧驱动（逻辑层不占线程）
    "timer" => "timer.js",
    // storage 全套：写操作**写穿到磁盘**，所以「首次拉取存起来、之后走缓存」的
    // 业务分支才会真的走到第二条（从前只有进程内 HashMap，每次启动都是空的）
    "storage" => "storage.js",
    // Toast / Loading / Modal / ActionSheet / 下拉刷新 / createAnimation
    "ui" => "ui.js",
    // `wx.createCanvasContext`：把 2D 指令录下来交给原生光栅化
    "canvas" => "canvas.js",
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
    // `Component()` / `Behavior()` 与组件实例工厂（properties / observers / triggerEvent）
    "component" => "component.js",
    // **最后**：给 `wx` 装未实现 API 的探针。关键是仍然返回 `undefined` ——
    // 框架普遍用 `typeof wx.xxx === 'function'` 做能力探测，返回假函数会让它们
    // 走进不存在的分支。
    "api-probe" => "api-probe.js",
];

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
        for p in PRELUDE {
            let rt = self.runtime.lock().unwrap();
            rt.eval(p.source).map_err(|e| format!("{}: {e}", p.name))?;
        }
        println!("    prelude: {} 段 JS 已注入", PRELUDE.len());
        Ok(())
    }

    /// 前置代码的域名列表（诊断与测试用：确认注册表没被漏掉一段）
    pub fn prelude_names() -> Vec<&'static str> {
        PRELUDE.iter().map(|p| p.name).collect()
    }
}
