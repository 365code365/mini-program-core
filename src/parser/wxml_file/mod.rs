//! 从磁盘读 WXML 文件，并在**读的这一刻**解析掉 `<include>` / `<import>`。
//!
//! ## 为什么在读文件时做，而不是在模板展开时做
//! `include src` / `import src` 是**相对写它的那个文件**解析的路径，而节点树里不带来源
//! 路径；整条管线（`PageInfo.wxml`、`ComponentSource.wxml`、H5 编译器）传来传去的又都是
//! WXML **源码字符串**。所以在知道文件路径的唯一时刻 —— 读文件 —— 把引用全部解析好，
//! 再序列化回 WXML 源码交出去：下游的 `WxmlParser::new(&wxml).parse()` 一行都不用改，
//! 页面、自定义组件、custom-tab-bar、H5 导出走的是同一个函数。
//!
//! ## 展开后的形状（`TemplateEngine` 在渲染时读）
//! - `<include src>` 原地保留，**子节点**换成被引入文件处理后的全部节点。渲染时透传
//!   （不产生真实节点），被引入文件里的 `<template>` 定义与 `<wxs>` 照常不渲染 ——
//!   于是「除模板和 WXS 外整段嵌入」成立。
//! - `<import src>` 的子节点是被引入文件**自己定义的** `<template name>`，外加它自己的
//!   `<import>`（只用来给那些模板的函数体提供作用域）。模板表只收 import 节点的**直接**
//!   `<template>` 子节点，所以 C import B、B import A 时 C 看不到 A（官方：不会间接引入）。
//! - 文件里任意层级的 `<template name>` 与 `<import>` 都上提到该文件顶层末尾（按文档序）：
//!   模板片段是文件级作用域、可以先用后定义，渲染时只需扫一层。
//!
//! 文件里一个 `<import`/`<include`/`<template` 都没有时**原样返回源码**（零开销，也保证
//! 现有小程序逐字节不变）；顶层文件自己解析失败时同样原样返回，让调用方照旧报错。

use std::path::Path;

/// 解析引用、上提定义
mod loader;
/// 节点树 → WXML 源码（展开结果交给下游重新解析）
mod serialize;
/// `<wxs src>` 与 wxs 里的 `require`：读进来打成一个包
mod wxs_bundle;

pub use serialize::to_source;
pub use wxs_bundle::requires as wxs_requires;

/// 读一个 WXML 文件并展开其中的 `<include>` / `<import>`。`app_root` 是小程序根目录
/// （`src="/x.wxml"` 相对它），`file` 是这个 WXML 文件的路径（相对 src 相对它的目录）。
///
/// 读不到文件时返回 IO 错误（调用方保留各自原来的处理）；被引用的文件缺失、成环、
/// 越出小程序目录只打警告并按「什么都不渲染」处理，**不会**让整页失败。
pub fn read_expanded(app_root: &Path, file: &Path) -> std::io::Result<String> {
    let src = std::fs::read_to_string(file)?;
    Ok(expand_source(app_root, file, &src))
}

/// 与 [`read_expanded`] 相同，但源码已经读进来了
pub fn expand_source(app_root: &Path, file: &Path, src: &str) -> String {
    if !needs_expansion(src) {
        return src.to_string();
    }
    let nodes = match crate::parser::WxmlParser::new(src).parse() {
        Ok(n) => n,
        // 顶层解析失败：原样交回，调用方的解析会给出原来那条错误
        Err(_) => return src.to_string(),
    };
    let processed = loader::Loader::new(app_root).process_file(file, nodes);
    to_source(&processed)
}

/// 文本里根本没有这几种标签时什么都不用做（`<wxs>` 要把 `src` / `require` 的文件读进来打包）
fn needs_expansion(src: &str) -> bool {
    src.contains("<import") || src.contains("<include") || src.contains("<template") || src.contains("<wxs")
}

#[cfg(test)]
mod tests;
