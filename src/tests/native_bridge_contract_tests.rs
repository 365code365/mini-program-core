//! JS ↔ Rust 桥的**契约测试**：两层的函数名必须对得上。
//!
//! ## 为什么是「契约」而不是「合并重复」
//! 加一个 `wx.*` API 天然要动三层，这三层各自的内容不同、无法合并：
//!
//! | 层 | 位置 | 干什么 |
//! |---|---|---|
//! | ① JS 前置代码 | `src/js/prelude/*.js` | 实现 `wx.xxx`，参数校验、回调登记，最后调 `__native_xxx` |
//! | ② 原生入口 | `src/js/bridge.rs` | `register_function("__native_xxx", …)`：把请求塞进事件队列或直接干活 |
//! | ③ 宿主消费 | `src/runtime/app.rs` → `host::*` | 每帧取事件队列，真正执行（弹 Toast、发请求、起定时器…） |
//!
//! 真正的风险不是「代码重复」，而是**层与层之间对不上**：JS 调了一个没人注册的
//! `__native_xxx`，QuickJS 只会抛 `TypeError: not a function`（甚至因为
//! `typeof` 能力探测而静默走进兜底分支），报错里既没有名字也没有栈。
//! 所以这里把「名字对得上」变成可执行的判据 —— 不需要维护任何清单：
//! 引用方从 prelude 的 `.js` 里扫，注册方从 `bridge.rs` 的源码里扫，两边都是真相本身。

use std::collections::HashSet;

/// prelude 目录里的所有 `.js`（**运行时读目录**，不维护清单）。
///
/// 上一版把 15 个文件名写死在 `include_str!` 里 —— 于是加一个 prelude 文件时，
/// 这里不改就会把它引用的原生函数误判成「死的」（我加固定时钟那次就撞上了）。
/// 手维护的清单本身就是这套测试要消灭的东西，所以改成读目录。
fn prelude_files() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/js/prelude");
    let mut out = Vec::new();
    for e in std::fs::read_dir(&dir).expect("读 prelude 目录").flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) == Some("js") {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            out.push((name, std::fs::read_to_string(&p).expect("读 .js")));
        }
    }
    out.sort();
    assert!(out.len() >= 10, "prelude 至少该有十来个域，实际 {}", out.len());
    out
}

/// prelude 里所有 `__native_*` 引用（JS 侧要求原生提供的东西）
fn referenced_in_js() -> HashSet<String> {
    let mut out = HashSet::new();
    for (_, src) in prelude_files() {
        collect_native_names(&src, &mut out);
    }
    out
}

/// `bridge.rs` 里 `register_function("__native_x", …)` 注册的名字
fn registered_in_rust() -> HashSet<String> {
    let src = include_str!("../js/bridge.rs");
    let mut out = HashSet::new();
    let mut rest = src;
    while let Some(i) = rest.find("register_function(\"") {
        rest = &rest[i + "register_function(\"".len()..];
        if let Some(end) = rest.find('"') {
            out.insert(rest[..end].to_string());
            rest = &rest[end..];
        }
    }
    out
}

/// 扫出形如 `__native_foo` 的标识符
fn collect_native_names(src: &str, out: &mut HashSet<String>) {
    let bytes = src.as_bytes();
    let mut i = 0;
    while let Some(rel) = src[i..].find("__native_") {
        let start = i + rel;
        let mut end = start + "__native_".len();
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
        out.insert(src[start..end].to_string());
        i = end;
    }
}

/// JS 自己实现的 `__native_*`（不需要 Rust 注册）。
/// `__native_print` 是纯 JS 的输出缓冲，宿主每帧把缓冲取走打印。
const JS_PROVIDED: &[&str] = &["__native_print"];

#[test]
fn js_引用的每个原生函数都注册过() {
    let referenced = referenced_in_js();
    let registered = registered_in_rust();
    let missing: Vec<_> = referenced
        .iter()
        .filter(|n| !registered.contains(*n) && !JS_PROVIDED.contains(&n.as_str()))
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "prelude 里调了这些原生函数，但 bridge.rs 没注册：{missing:?}\n\
         —— 运行时只会抛 `TypeError: not a function`，既没有名字也没有栈"
    );
}

#[test]
fn 注册的原生函数没有一个是死的() {
    let referenced = referenced_in_js();
    let registered = registered_in_rust();
    let dead: Vec<_> = registered
        .iter()
        .filter(|n| !referenced.contains(*n))
        .cloned()
        .collect();
    assert!(
        dead.is_empty(),
        "bridge.rs 注册了这些原生函数，但没有任何 JS 调它：{dead:?}\n\
         —— 要么是改名字时漏了 JS 侧，要么是该删的残留"
    );
}

#[test]
fn 引导之后每个原生函数在运行时真的可调用() {
    // 上面两条是「源码级」的对账，这条是**运行时**的：
    // 名字对得上，也可能因为注册顺序、错误被吞掉而实际没挂上去。
    let mut app = crate::runtime::MiniApp::new(375, 667).expect("创建运行时");
    app.init().expect("初始化");
    let mut names: Vec<_> = referenced_in_js().into_iter().collect();
    names.sort();
    let mut bad = Vec::new();
    for n in &names {
        let kind = app.eval(&format!("typeof {n}")).unwrap_or_default();
        if !kind.contains("function") {
            bad.push(format!("{n} = {}", kind.trim()));
        }
    }
    assert!(
        bad.is_empty(),
        "这些名字在运行时不是函数：{bad:?}（共检查 {} 个）",
        names.len()
    );
}

#[test]
fn prelude_目录里没有孤儿文件() {
    // 放进目录但忘了写进 `api.rs` 的注册表 = 这段 JS 永远不会被执行，
    // 而症状只是「某个 wx API 不存在」，很难往「文件没注册」上想。
    let table = include_str!("../js/api.rs");
    let missing: Vec<_> = prelude_files()
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| !table.contains(&format!("\"{name}\"")))
        .collect();
    assert!(
        missing.is_empty(),
        "这些 prelude 文件没写进 api.rs 的 PRELUDE 表，永远不会被执行：{missing:?}"
    );
}
