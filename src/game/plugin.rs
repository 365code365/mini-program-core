//! 引擎插件（game-doc/11 B-7、04 G6.7，决策 D-12，GT-PL）：`game.json.plugins` + `requirePlugin(name)`。
//!
//! 插件包从哪来：
//! 1. 创建选项 `pluginDir`：`<pluginDir>/<provider>/<version>/`（宿主或商店服务负责下载、缓存）；
//! 2. 没有时用游戏包里 `plugins[name].path` 指向的本地副本（Cocos「引擎分离」允许本地兜底）；
//! 3. 都没有：`requirePlugin` 抛错（文案待取证），错误进 wx.onError（游戏顶层没接住时）。
//!
//! 插件的文件注册在自己的模块命名空间里（键 `__plugin__/<name>/<相对插件根>`），插件里的相对 require 只在插件内解析，
//! 不会和游戏包的模块撞名。入口：插件根的 `plugin.json` 的 `main`（缺省 `index.js`）；本地兜底的 path 也可以直接是一个 js 文件。
//! `requirePlugin('name/sub/path')` 取插件里的指定模块。`contexts`（主域 / 开放数据域）只解析不限制：开放数据域还没做（G9.7）。

use super::config::GameConfig;
use crate::runtime::MiniApp;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const NS: &str = "__plugin__";

/// 一个插件找到的位置与入口
struct Found {
    name: String,
    dir: PathBuf,
    /// 入口模块（相对插件根、去 .js）
    main: String,
    from: &'static str,
}

fn locate(root: &Path, plugin_dir: Option<&Path>, p: &super::config::Plugin) -> Option<Found> {
    let mut cands: Vec<(PathBuf, &'static str)> = Vec::new();
    if let Some(d) = plugin_dir {
        cands.push((d.join(&p.provider).join(&p.version), "pluginDir"));
    }
    if let Some(local) = &p.path {
        cands.push((root.join(local.trim_start_matches("./").trim_start_matches('/')), "包内 path"));
    }
    for (c, from) in cands {
        if c.is_file() && c.extension().and_then(|e| e.to_str()) == Some("js") {
            let main = c.file_stem()?.to_string_lossy().to_string();
            return Some(Found { name: p.name.clone(), dir: c.parent()?.to_path_buf(), main, from });
        }
        if c.is_dir() {
            let main = std::fs::read_to_string(c.join("plugin.json")).ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|v| v.get("main").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_else(|| "index.js".into());
            let main = main.trim_start_matches("./").trim_end_matches(".js").to_string();
            if c.join(format!("{main}.js")).is_file() {
                return Some(Found { name: p.name.clone(), dir: c, main, from });
            }
        }
    }
    None
}

/// 解析全部插件、把找到的插件文件注册成模块。返回 `__gamePlugins`（名字 → 入口键 / null）与日志
pub fn install(app: &MiniApp, root: &Path, plugin_dir: Option<&Path>, cfg: &GameConfig) -> (Value, Vec<String>) {
    let mut table = serde_json::Map::new();
    let mut logs = Vec::new();
    for p in &cfg.plugins {
        let Some(f) = locate(root, plugin_dir, p) else {
            logs.push(format!("⚠ 插件 {:?}（{} {}）没找到：pluginDir 与包内 path 都没有", p.name, p.provider, p.version));
            table.insert(p.name.clone(), Value::Null);
            continue;
        };
        let mut files = Vec::new();
        super::worker::collect_js(&f.dir, &f.dir, &mut files);
        files.sort();
        let mut n = 0;
        for (rel, path) in files {
            let Ok(src) = std::fs::read_to_string(&path) else { continue };
            let key = format!("{NS}/{}/{rel}", f.name);
            match app.eval(&format!("__defineModule({key:?}, function(module, exports, require) {{{src}\n}});")) {
                Ok(_) => n += 1,
                Err(e) => logs.push(format!("⚠ 插件 {} 的 {rel}.js 注册失败：{e}", f.name)),
            }
        }
        logs.push(format!("🧩 插件 {:?} 来自{}（{}，{n} 个模块）", f.name, f.from, f.dir.display()));
        table.insert(f.name.clone(), json!(format!("{NS}/{}/{}", f.name, f.main)));
    }
    (Value::Object(table), logs)
}
