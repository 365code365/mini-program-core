//! 浏览器端**响应式**运行时（导出 HTML 工程用）
//!
//! 与静态首屏不同，运行时内置一个 WXML 解释器：读取页面内嵌的 `window.__WXML__` AST，
//! 结合页面 `data` 渲染真实 DOM；`setData` 会按模板**重新渲染**，从而让所有事件、
//! 列表更新、条件渲染、`model:` 双向数据绑定都能真正工作。
//!
//! 关键点：
//! - `{{表达式}}` 直接用 `new Function + with(scope)` 求值（WXML 表达式本就是 JS 语法）
//! - `wx:for` / `wx:if` / `wx:elif` / `wx:else` / `block` 支持
//! - 事件委托（bind/catch → data-*），`model:value` → `data-model` 双向绑定
//! - `wx.*` 常用 API、旧版 canvas 2D 适配
//!
//! JS 本体放在 `runtime/*.js`（**不是** Rust 模块，只是被 `include_str!` 拼起来的文本）。
//! 以前它是一整条 900 行的 `r####"…"####` 原始字符串：编辑器不给高亮、不给括号配平，
//! 里头出个笔误要跑一遍导出才发现。拆成 `.js` 后可以直接用 JS 工具检查。
//!
//! 顺序不能动 —— 整段是一个 IIFE，拼接结果必须与原文逐字节相同。

/// 运行时 JS 源码（按职责分片，拼接顺序即执行顺序）
pub const RUNTIME_JS: &str = concat!(
    // IIFE 头、框架 API（App/Page/getApp）、表达式求值与插值
    include_str!("runtime/01_bootstrap.js"),
    // 标签映射与 WXML → VNode 树（wx:for / wx:if / slot / 内置图标）
    include_str!("runtime/02_vnode.js"),
    // VNode → 真实 DOM 的创建与 diff/patch（保留焦点与输入态）
    include_str!("runtime/03_dom_patch.js"),
    // wx.createAnimation 的播放：actions 映射成 CSS transition
    include_str!("runtime/04_animation.js"),
    // setData / 挂载 / 更新，以及事件委托
    include_str!("runtime/05_setdata.js"),
    // picker 底部选择面板（各 mode 的列与提交值）
    include_str!("runtime/06_picker.js"),
    // 组件初始化（swiper 等，幂等、不重构 DOM）
    include_str!("runtime/07_components.js"),
    // 页面跳转与 wx.* API、下拉刷新、createAnimation 构造
    include_str!("runtime/08_wx_api.js"),
    // hover-class 按压反馈与启动
    include_str!("runtime/09_hover_boot.js"),
);
