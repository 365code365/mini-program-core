//! 包结构（game-doc/04 G6.1）：game.json 的 subpackages 整理成 JS 侧（game-lifecycle.js）要用的样子，
//! 以及「从独立分包启动」时该执行哪个入口。
//!
//! 分包 root 两种写法（官方）：目录（`stage1/`，入口 `stage1/game.js`）或单个文件（`one.js`，入口就是它）。
//! 入口文件不存在在微信开发者工具里编译就拦下了；这里不拦启动，记一条警告，loadSubpackage 时照常失败。

use super::config::{GameConfig, Subpackage};
use serde_json::{json, Value};
use std::path::Path;

/// 一个分包的入口模块键（相对包根、去 `.js`）与「属于它的模块」前缀（单文件分包没有前缀）
pub fn entry_of(s: &Subpackage) -> (String, Option<String>) {
    let root = s.root.trim_start_matches("./").trim_start_matches('/');
    if root.ends_with(".js") {
        (root.trim_end_matches(".js").to_string(), None)
    } else {
        let dir = root.trim_end_matches('/');
        (format!("{dir}/game"), Some(format!("{dir}/")))
    }
}

fn size_of(p: &Path) -> u64 {
    if p.is_file() {
        return p.metadata().map(|m| m.len()).unwrap_or(0);
    }
    std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| size_of(&e.path())).sum()).unwrap_or(0)
}

/// `__gameSubpackages`：[{name, root, independent, entry, prefix, bytes}]；第二项是缺入口文件的警告
pub fn describe(root: &Path, cfg: &GameConfig) -> (Value, Vec<String>) {
    let mut warn = Vec::new();
    let list: Vec<Value> = cfg
        .subpackages
        .iter()
        .map(|s| {
            let (entry, prefix) = entry_of(s);
            let file = root.join(format!("{entry}.js"));
            if !file.is_file() {
                warn.push(format!("分包 {:?} 的入口 {entry}.js 不存在", s.name));
            }
            let dir = root.join(s.root.trim_start_matches('/').trim_end_matches('/'));
            json!({ "name": s.name, "root": s.root.trim_start_matches('/'), "independent": s.independent, "entry": entry,
                "prefix": prefix, "bytes": size_of(&dir) })
        })
        .collect();
    (Value::Array(list), warn)
}

/// 主包字节数：包根下不属于任何分包的文件
pub fn main_bytes(root: &Path, cfg: &GameConfig) -> u64 {
    let subs: Vec<std::path::PathBuf> = cfg.subpackages.iter().map(|s| root.join(s.root.trim_start_matches('/').trim_end_matches('/'))).collect();
    fn walk(p: &Path, subs: &[std::path::PathBuf]) -> u64 {
        if subs.iter().any(|s| s == p) {
            return 0;
        }
        if p.is_file() {
            return p.metadata().map(|m| m.len()).unwrap_or(0);
        }
        std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| walk(&e.path(), subs)).sum()).unwrap_or(0)
    }
    walk(root, &subs)
}

/// 从独立分包启动：返回 (分包名, 入口模块键)。名字找不到、或不是独立分包，返回 Err（调用方报警告、照常从主包启动）
pub fn entry_subpackage<'a>(cfg: &'a GameConfig, name: &str) -> Result<(&'a str, String), String> {
    let s = cfg.subpackages.iter().find(|s| s.name == name || s.root.trim_matches('/') == name.trim_matches('/'))
        .ok_or_else(|| format!("创建选项 entrySubpackage={name:?} 在 game.json 的 subpackages 里找不到"))?;
    if !s.independent {
        return Err(format!("分包 {name:?} 不是独立分包（independent: true），不能直接从它启动"));
    }
    Ok((&s.name, entry_of(s).0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 目录与单文件分包的入口() {
        let d = Subpackage { name: "a".into(), root: "stage1/".into(), independent: false };
        assert_eq!(entry_of(&d), ("stage1/game".into(), Some("stage1/".into())));
        let f = Subpackage { name: "b".into(), root: "/one.js".into(), independent: false };
        assert_eq!(entry_of(&f), ("one".into(), None));
    }
}
