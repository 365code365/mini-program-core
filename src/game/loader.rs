//! 小游戏包的 JS 加载：把包内 `.js` 注册成 CommonJS 模块，再以模块作用域执行 `game.js`。
//!
//! 与小程序那条路（`MiniApp::register_all_modules` / `JsRuntime::define_module`）的区别只有一处：
//! **包装函数头与源码第一行写在同一行**。小程序那条路是 `function(module, exports, require) {\n<源码>`，
//! 异常栈里的行号全部比源文件多 1 —— 开发者照着栈去找代码永远差一行（game-doc/09 §3-3、06 §3.7-41）。
//! 小程序侧不改（改了会动小程序的报错文本，属于另一件事）。

use crate::runtime::MiniApp;
use std::path::Path;

/// 包装后的源码：`<head>{<源码>\n}<tail>`。结尾换行是必须的：源码最后一行是 `// 注释` 时，
/// 不换行的话右花括号会被注释吃掉
fn wrap(head: &str, key: &str, source: &str) -> String {
    format!("{head}({key:?}, function(module, exports, require) {{{source}\n}});")
}

/// 一个注册失败的模块（语法错误之类）：不中断其余模块，交给调用方上报
pub struct ModuleError {
    pub key: String,
    pub message: String,
}

/// 递归注册包内全部 `.js`（键 = 相对包根、去掉 `.js`）。返回注册个数、命中字节码缓存的个数与失败的模块。
/// 大文件走字节码缓存（js/bytecode_cache.rs）：包装后的整段源码是缓存键，包更新了自然换键
pub fn register_modules(app: &MiniApp, root: &Path) -> (usize, usize, Vec<ModuleError>) {
    let mut files = Vec::new();
    collect(root, root, &mut files);
    // 排序：注册顺序稳定，出错时的报告顺序也稳定（read_dir 的顺序随文件系统变）
    files.sort();
    let (mut ok, mut cached) = (0, 0);
    let mut errors = Vec::new();
    for (key, path) in files {
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                errors.push(ModuleError { key, message: format!("读不出：{e}") });
                continue;
            }
        };
        let (r, hit) = app.eval_cached(&wrap("__defineModule", &key, &source));
        match r {
            Ok(_) => {
                ok += 1;
                cached += hit as usize;
            }
            Err(message) => errors.push(ModuleError { key, message }),
        }
    }
    (ok, cached, errors)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, std::path::PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(root, &p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("js") {
            if let Ok(rel) = p.strip_prefix(root) {
                let key = rel.to_string_lossy().replace('\\', "/");
                out.push((key.trim_end_matches(".js").to_string(), p));
            }
        }
    }
}

/// 以模块作用域执行一段源码（`game.js`、分包入口）。不缓存：与 `__runAsModule` 同语义
pub fn run_module(app: &MiniApp, key: &str, source: &str) -> Result<(), String> {
    app.eval_cached(&wrap("__runAsModule", key, source)).0.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn 包装头与源码同一行_结尾换行防注释吞括号() {
        let w = wrap("__runAsModule", "game", "var a = 1; // 尾注释");
        assert_eq!(w.lines().count(), 2, "源码第一行必须和包装头在同一行：{w}");
        assert!(w.starts_with("__runAsModule(\"game\", function(module, exports, require) {var a = 1;"));
        assert!(w.ends_with("\n});"));
    }
}
