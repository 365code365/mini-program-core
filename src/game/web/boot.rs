//! 高性能模式的启动页：`/__wg.html` → `/__wg/boot.js`（数据 + 过桥层）→ `/__wg/seg/*.js`（基础库与 DOM 层，一段一个）
//! → 每个包内 `.js` 一个 `<script>`（`/__wg/m/<键>.js`，注册成 CommonJS 模块）→ `/__wg/entry.js`（执行 game.js）。
//!
//! 为什么每个模块一个 `<script>` 而不是拼成一个大文件：一个模块有语法错误时只坏它自己（`window.onerror`
//! 带着文件名报上来），其余照常注册 —— 与 QuickJS 模式逐个 `eval` 的行为一致；异常栈里的文件名、行号也是对的
//! （包装头与源码第一行在同一行，见 loader.rs 的同一条约定）。
//!
//! 模块的包装：`function(module, exports, require) { return (function(GameGlobal, window, self, globalThis) {<源码>\n}).call(this, …) }`。
//! 多套一层函数是为了把 `GameGlobal` / `window` / `self` / `globalThis` 换成 `__wgG`（wg-env.js 的代理）：浏览器里 `window.location`
//! 赋值会跳走页面、`window.document` 是只读的，而 weapp-adapter 这类适配层正好要往全局上挂这几个名字。
//! 不用 `var GameGlobal = …;` 写在源码前面：那样源码开头的 `"use strict"` 就不再是指令序言，严格模式悄悄丢了。

use super::WebGame;
use serde_json::{json, Value};

/// 装进页面的基础库（`js/prelude/`，顺序即执行顺序；与 QuickJS 模式的 GAME_CORE + GAME_EXTRA 同序，只挑与平台无关的）。
/// 不装的：console / timer / fake-clock（用浏览器自己的）、audio 的原生入口与 webaudio（DOM 层另给）、
/// game-raf / image / canvas / webgl / wasm / worker（DOM 层另给）、opendata（高性能模式不支持，宿主回落 QuickJS）
pub const SEGMENTS: &[&str] = &[
    "module", "wx-object", "codec", "storage", "ui", "device", "network", "host", "system", "socket", "fs", "audio", "app-info",
    "host-capabilities", "base-ext", "media-ext",
    "game-global", "game-compat", "game-surface", "game-input", "game-lifecycle", "game-window", "game-keyboard",
    "game-proxies", "game-pc",
];

/// 高性能+ 多装的普通模式那几层（画布 / 图片 / WebGL 的 JS 语义），它们的原生入口由 wg-remote.js 录下来交回 App
pub const PLUS_SEGMENTS: &[&str] = &["game-image", "game-canvas", "game-webgl-consts", "game-webgl", "game-webgl2"];

/// DOM 层（`js/web_game/`），在基础库之后装：覆盖掉基础库里依赖原生画布 / 混音器的那几个 wx.*
const WEB_LAYER: &[(&str, &str)] = &[
    ("wg-remote", include_str!("../../js/web_game/wg-remote.js")),
    ("wg-canvas", include_str!("../../js/web_game/wg-canvas.js")),
    ("wg-input", include_str!("../../js/web_game/wg-input.js")),
    ("wg-frame", include_str!("../../js/web_game/wg-frame.js")),
    ("wg-audio", include_str!("../../js/web_game/wg-audio.js")),
    ("wg-overlay", include_str!("../../js/web_game/wg-overlay.js")),
    ("wg-extra", include_str!("../../js/web_game/wg-extra.js")),
];

/// 过桥层：最先装（基础库加载时就要看得见 `__native_*`）
const ENV: &str = include_str!("../../js/web_game/wg-env.js");

const HEAD: &str = r#"<!doctype html>
<html><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,minimum-scale=1,user-scalable=no,viewport-fit=cover">
<style>
html,body{margin:0;padding:0;width:100%;height:100%;overflow:hidden;background:#000;touch-action:none;
-webkit-user-select:none;user-select:none;-webkit-touch-callout:none;-webkit-tap-highlight-color:transparent;-webkit-text-size-adjust:none}
canvas{display:block}
</style></head><body>
"#;

/// `/__wg.html`：每段一个 `<script>`（按顺序同步执行）。不拼成一个文件、也不包 try：
/// 每段都是独立的全局代码，与 QuickJS 模式一段一 `eval` 的作用域完全相同（块里的函数声明、顶层 let 不会变味），
/// 一段抛错只停它自己，`window.onerror` 带着段名报上来
pub fn boot_html(plus: bool) -> String {
    let mut out = String::from(HEAD);
    if plus {
        // 画面在下面的原生图层上（App 经 ANGLE 上屏），页面只画覆盖层：背景透明
        out = out.replace("background:#000;", "background:transparent;");
    }
    out.push_str("<script src=\"/__wg/boot.js\"></script>\n");
    let mut names: Vec<&str> = SEGMENTS.to_vec();
    for (n, _) in WEB_LAYER {
        match *n {
            // wg-remote 要在普通模式那几层之前装（它们加载时就取原生入口）；DOM 画布只有高性能模式要
            "wg-remote" if plus => {
                names.push(n);
                names.extend_from_slice(PLUS_SEGMENTS);
            }
            "wg-remote" => {}
            "wg-canvas" if plus => {}
            _ => names.push(n),
        }
    }
    for name in names {
        out.push_str(&format!("<script src=\"/__wg/seg/{name}.js\"></script>\n"));
    }
    out.push_str("<script>__wgStart();</script>\n</body></html>\n");
    out
}

/// `/__wg/seg/<名字>.js`：一段基础库或 DOM 层
pub fn segment_js(name: &str) -> Option<String> {
    if let Some((_, src)) = WEB_LAYER.iter().find(|(n, _)| *n == name) {
        return Some(format!("{src}\n//# sourceURL=/__wg/{name}.js"));
    }
    if !SEGMENTS.contains(&name) && !PLUS_SEGMENTS.contains(&name) {
        return None;
    }
    Some(format!("{}\n//# sourceURL=/__wg/prelude/{name}.js", segment_source(name)))
}

fn segment_source(name: &str) -> &'static str {
    crate::js::prelude_source(name).unwrap_or_else(|| crate::js::game_prelude_source(name))
}

impl WebGame {
    /// `/__wg/boot.js`：启动数据 + 过桥层（最先执行：基础库加载时就要看得见 `__native_*`）
    pub fn boot_js(&self) -> String {
        format!("var __wgBoot = {};\n{ENV}\n//# sourceURL=/__wg/wg-env.js", self.boot_data())
    }

    /// 页面启动时要的全部数据（一次给齐，免得基础库加载时同步过桥）
    fn boot_data(&self) -> Value {
        let (subs, _) = super::super::package::describe(&self.root, &self.config);
        let main = super::super::package::main_bytes(&self.root, &self.config);
        let entry_sub = self
            .entry_subpackage
            .as_deref()
            .and_then(|n| super::super::package::entry_subpackage(&self.config, n).ok())
            .map(|(name, key)| json!({ "name": name, "key": key }));
        let nt = json!({ "request": self.config.timeout_request, "connectSocket": self.config.timeout_connect_socket,
            "uploadFile": self.config.timeout_upload_file, "downloadFile": self.config.timeout_download_file });
        json!({
            "info": self.info,
            "launch": self.launch,
            "appId": self.app_id,
            "storage": self.storage,
            "hostApis": self.host_apis,
            "networkTimeout": nt,
            "subpackages": subs,
            "mainBytes": main,
            "entrySub": entry_sub,
            "modules": self.module_keys(),
            "orientation": self.config.device_orientation.as_str(),
            "workers": self.config.workers,
            "plus": self.plus,
            "glOps": gl_ops(),
        })
    }

    /// 包内全部 `.js` 的模块键（排序，与 loader.rs 同一个顺序）
    pub fn module_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        collect(&self.root, &self.root, &mut keys);
        keys.sort();
        keys
    }

    /// `/__wg/m/<键>.js`：注册一个模块（键里不许有 `..`，只认包里真有的 `.js`）
    pub fn module_js(&self, key: &str) -> Option<String> {
        if key.split('/').any(|p| p == ".." || p.is_empty()) {
            return None;
        }
        let src = std::fs::read_to_string(self.root.join(format!("{key}.js"))).ok()?;
        Some(wrap("__defineModule", key, &src))
    }

    /// `/__wg/entry.js`：执行 game.js（或独立分包的入口）
    pub fn entry_js(&self) -> String {
        let entry = self
            .entry_subpackage
            .as_deref()
            .and_then(|n| super::super::package::entry_subpackage(&self.config, n).ok());
        match entry {
            Some((_, key)) => format!("__wgRunEntry(function() {{ __require('', {}); }});", json!(key)),
            None => {
                let src = std::fs::read_to_string(self.root.join("game.js")).unwrap_or_default();
                format!("__wgRunEntry(function() {{ {}\n}});\n", wrap("__runAsModule", "game", &src))
            }
        }
    }
}

impl WebGame {
    /// `/__wg/worker.js?entry=<路径>`：Worker 线程的运行环境（与 QuickJS 模式 game/worker/runtime.js 同一套全局：
    /// `worker.onMessage / postMessage`、console、require 按包根解析）+ game.json `workers` 目录里的模块，最后执行入口。
    /// console 经 postMessage 交给主线程（wg-extra.js 认 `__wgLog`），日志与主线程的一起进宿主
    pub fn worker_js(&self, entry: &str) -> Option<String> {
        let dir = self.config.workers.as_deref()?.trim_matches('/').to_string();
        let key = entry.trim_start_matches('/').trim_end_matches(".js").to_string();
        if key.split('/').any(|p| p == ".." || p.is_empty()) {
            return None;
        }
        let mut out = String::from(segment_source("module"));
        out.push_str(WORKER_ENV);
        let mut keys = Vec::new();
        collect(&self.root, &self.root.join(&dir), &mut keys);
        keys.sort();
        for k in keys {
            if let Ok(src) = std::fs::read_to_string(self.root.join(format!("{k}.js"))) {
                out.push_str(&format!("try {{ __defineModule({k:?}, function(module, exports, require) {{{src}\n}}); }} catch (e) {{ __wgWorkerErr(e); }}\n"));
            }
        }
        out.push_str(&format!("try {{ __require('', {key:?}); }} catch (e) {{ __wgWorkerErr(e); }}\n"));
        Some(out)
    }
}

/// Worker 线程的全局（官方 Worker 里能用的：worker、console、定时器、require）
const WORKER_ENV: &str = r#"
(function() {
    var g = self;
    g.GameGlobal = g;
    function fmt(args) { return Array.prototype.map.call(args, function(a) { if (typeof a === 'string') return a; if (a instanceof Error) return (a.name || 'Error') + ': ' + a.message + (a.stack ? '\n' + a.stack : ''); try { return JSON.stringify(a); } catch (e) { return String(a); } }).join(' '); }
    var real = g.console;
    g.console = {};
    ['log', 'info', 'warn', 'error', 'debug'].forEach(function(l) { g.console[l] = function() { postMessage({ __wgLog: (l === 'error' ? '[ERROR] ' : l === 'warn' ? '[WARN] ' : '') + '[Worker] ' + fmt(arguments) }); }; });
    function report(e) { postMessage({ __wgError: { message: e && e.message !== undefined ? String(e.message) : String(e), stack: e && e.stack ? String(e.stack) : '' } }); }
    g.__wgWorkerErr = report;
    g.addEventListener('error', function(e) { e.preventDefault(); report(e.error || { message: e.message }); });
    var listeners = [];
    g.worker = {
        onMessage: function(cb) { if (typeof cb === 'function') listeners.push(cb); },
        postMessage: function(msg, transfer) { postMessage(msg, transfer || []); },
        getCameraFrameData: function() { return undefined; },
        testOnProcessKilled: function() { close(); }
    };
    g.onmessage = function(e) { listeners.slice().forEach(function(cb) { try { cb(e.data); } catch (err) { report(err); } }); };
})();
"#;

fn gl_ops() -> Vec<&'static str> {
    #[cfg(feature = "webgl")]
    {
        crate::game::webgl::op_names().to_vec()
    }
    #[cfg(not(feature = "webgl"))]
    {
        Vec::new()
    }
}

/// 包装头与源码第一行同一行（行号与源文件一致）；结尾换行防 `// 注释` 吃掉右括号
fn wrap(head: &str, key: &str, src: &str) -> String {
    format!(
        "{head}({key:?}, function(module, exports, require) {{ return (function(GameGlobal, window, self, globalThis) {{{src}\n}}).call(this, __wgG, __wgG, __wgG, __wgG); }});\n//# sourceURL=/{key}.js"
    )
}

fn collect(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(root, &p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("js") {
            if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/").trim_end_matches(".js").to_string());
            }
        }
    }
}
