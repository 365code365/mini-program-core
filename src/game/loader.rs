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

/// 源码里认得出的 `xx.函数名` 别名先装上（js/api_alias.rs）。单独一次 eval、不拼进包装：
/// 字节码缓存的键（= 包装后的源码，precompile_package 要对得上）不变
pub(crate) fn guess_alias(app: &MiniApp, source: &str) {
    if let Some(s) = crate::js::api_alias::guess_script([source]) {
        if let Err(e) = app.eval(&s) {
            eprintln!("⚠ [API 别名] 按函数名认别名失败：{e}");
        }
    }
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
        guess_alias(app, &source);
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

/// 预编译一个小游戏包（js/bytecode_cache.rs 的 precompile）：与 [`register_modules`] / [`run_module`] 包装成一模一样的源码，
/// 缓存键才对得上。`data_dir` = 之后建实例时给的数据目录。任意线程调；返回 (要缓存的段数, 新编的段数, 毫秒)
pub fn precompile_package(root: &Path, data_dir: &Path) -> (usize, usize, u128) {
    let t = std::time::Instant::now();
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort();
    let mut srcs: Vec<String> = files
        .iter()
        .filter_map(|(key, path)| std::fs::read_to_string(path).ok().map(|src| wrap("__defineModule", key, &src)))
        .collect();
    // game.js 还会以模块作用域再执行一次（run_module）：那一段也编上
    if let Ok(src) = std::fs::read_to_string(root.join("game.js")) {
        srcs.push(wrap("__runAsModule", "game", &src));
    }
    let (big, done) = crate::js::bytecode_cache::precompile(&srcs, &data_dir.join("jscache"));
    (big, done, t.elapsed().as_millis())
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
    guess_alias(app, source);
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

    /// 销毁实例后，注册进 JS 的原生闭包捏着的共享状态（GameAudio、图片、画布）全部释放，包里有走字节码缓存的大脚本也一样。
    /// 从前缓存那条路每段脚本漏一份上下文引用（js/bytecode_cache.rs 的 run）：真游戏（脚本都 ≥16KB）的 GameAudio
    /// 一个都不析构，Android 上 AAudio 流一直开着，退出游戏背景音乐还在响。单文件的小夹具走不到缓存，测不出来
    #[test]
    fn 销毁实例_闭包捏着的状态全部释放_含走字节码缓存的大脚本() {
        // 建实例会改进程级的数据命名空间与包根（同 ffi_game/tests.rs 的 scope_lock）
        let _scope = crate::sys::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _root = crate::sys::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("mr-game-drop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("js")).unwrap();
        std::fs::write(dir.join("game.json"), r#"{"deviceOrientation":"portrait"}"#).unwrap();
        // 内容固定：缓存文件每次是同一个（第一次跑编译落盘，之后读回；两条路都经 run）
        let big = format!("/* drop-check {} */ module.exports = {{ n: 1 }};", "x".repeat(crate::js::bytecode_cache::MIN_BYTES));
        std::fs::write(dir.join("js/big.js"), big).unwrap();
        std::fs::write(dir.join("game.js"), "var b = require('./js/big'); var a = wx.createInnerAudioContext(); a.src = 'none.mp3'; b.n;").unwrap();
        for round in 1..=2 {
            let opts = crate::game::GameOptions { fixed_clock: true, app_id: Some("wxdropcheck".into()), ..Default::default() };
            // 数据目录给 None：不改进程级的数据目录（别的用例并行在用），缓存落在默认的 target/jscache
            let mut e = crate::game::GameEngine::new(dir.to_str().unwrap(), None, 320, 480, 2.0, opts).unwrap();
            e.pump_ex(16_667);
            let held = (std::rc::Rc::downgrade(&e.audio), std::rc::Rc::downgrade(&e.images), std::rc::Rc::downgrade(&e.canvases));
            assert!(held.0.strong_count() > 1, "前提：JS 里的原生闭包捏着 GameAudio");
            drop(e);
            let left = (held.0.strong_count(), held.1.strong_count(), held.2.strong_count());
            assert_eq!(left, (0, 0, 0), "第 {round} 次：实例销毁后 (GameAudio, 图片, 画布) 还有引用活着 —— JS 上下文漏了");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
