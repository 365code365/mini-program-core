//! `MiniApp` 与宿主外壳之间的那一段：逻辑层发给宿主的事件、宿主实现的 `wx.*`（hostCall）。
//! 从 app.rs 拆出（那边已过 500 行）。
//!
//! ## 文件怎么在小程序与宿主之间来回
//! 小程序只认虚拟路径（`wxfile://tmp/…`，见 `crate::wxfile`），宿主只认真实路径。两头各翻译一次，
//! 而且都在这里做 —— 每个宿主（Flutter 插件、原生 View、桌面窗体）就不用各自去猜沙盒目录长什么样：
//!
//! - **引擎 → 宿主**（[`MiniApp::take_host_events`]）：`hostCall` 事件补两个字段
//!   - `tempDir`：宿主可以直接写的临时目录。选图 / 拍照的结果写进这里，回包时引擎只换名字、不拷贝；
//!   - `files`：`args` 里出现的 `wxfile://` 路径 → 真实路径（`saveImageToPhotosAlbum({filePath})`、
//!     `openDocument` 要读的文件）。文件不存在的不列。
//! - **宿主 → 引擎**（[`MiniApp::resolve_host_call`]）：回包里 `file://` 开头的字符串是宿主交给小程序的文件，
//!   导入沙盒、换成 `wxfile://tmp/…`。同一个文件在回包里出现几次（`tempFilePaths` 与 `tempFiles[].path`）
//!   都换成同一个虚拟路径。导入失败（文件不存在）的原样留着并打一行日志。
//!
//! 为什么是 `file://` 前缀而不是「看着像绝对路径就导入」：回包里的字符串大多是普通数据（昵称、地址、
//! 扫码结果），只有宿主明说「这是个文件」的才动。也只在宿主回包这条路上做，小程序自己的 JS 碰不到
//! —— 否则它能借这个口子把沙盒外的任意文件拷进来读。
use super::app::MiniApp;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

impl MiniApp {
    /// 取走逻辑层交给宿主外壳的事件（导航栏、hostCall；JSON 串，按发生顺序）。
    /// `hostCall` 补上 `tempDir` / `files`（见模块注释）
    pub fn take_host_events(&self) -> Vec<String> {
        let mut evs = self.bridge.take_host_events();
        for raw in evs.iter_mut() {
            // host.js 发的是 JSON.stringify({type:'hostCall', …})：先按字面量筛，别的事件不解析
            if !raw.starts_with(r#"{"type":"hostCall""#) {
                continue;
            }
            if let Ok(mut v) = serde_json::from_str::<Value>(raw) {
                annotate_host_call(&mut v);
                *raw = v.to_string();
            }
        }
        evs
    }

    /// 把宿主实现的 API 装进 `wx`（`app.js` 执行之前调）
    pub fn install_host_apis(&self, names: &[String]) {
        if names.is_empty() {
            return;
        }
        let list = serde_json::to_string(names).unwrap_or_else(|_| "[]".into());
        if let Err(e) = self.eval(&format!("__installHostApis({list})")) {
            eprintln!("⚠️  安装宿主 API 失败: {e}");
        }
    }

    /// 宿主回 hostCall 的结果。`result_json` 是交给回调的 res（对象），解析不了的原样包成 `{data}`。
    /// 回包里的 `file://…` 先导入沙盒（见模块注释）。
    /// 返回 false 表示 id 不认识（已经回过、或页面早就换了）。
    pub fn resolve_host_call(&self, id: u32, ok: bool, result_json: &str) -> bool {
        let res = import_reply_files(result_json);
        let code = format!("__resolveHostCall({id}, {ok}, {})", Value::String(res));
        let r = self.eval(&code);
        self.pump_jobs();
        matches!(r.as_deref(), Ok("true"))
    }
}

/// `hostCall` 事件补 `tempDir` 与 `files`
fn annotate_host_call(v: &mut Value) {
    let mut files = Map::new();
    if let Some(args) = v.get("args") {
        collect_wxfiles(args, &mut files);
    }
    let Some(o) = v.as_object_mut() else { return };
    o.insert("tempDir".into(), Value::String(crate::wxfile::temp_dir().to_string_lossy().into_owned()));
    if !files.is_empty() {
        o.insert("files".into(), Value::Object(files));
    }
}

fn collect_wxfiles(v: &Value, out: &mut Map<String, Value>) {
    match v {
        Value::String(s) if s.starts_with(crate::wxfile::SCHEME) && !out.contains_key(s) => {
            if let Some(p) = crate::wxfile::to_path(s).filter(|p| p.is_file()) {
                out.insert(s.clone(), Value::String(p.to_string_lossy().into_owned()));
            }
        }
        Value::Array(a) => a.iter().for_each(|x| collect_wxfiles(x, out)),
        Value::Object(o) => o.values().for_each(|x| collect_wxfiles(x, out)),
        _ => {}
    }
}

/// 回包里的 `file://…` → `wxfile://tmp/…`。不是 JSON、或一个 `file://` 都没有时原样返回
fn import_reply_files(result_json: &str) -> String {
    // 先按字面量筛只能筛 `file:`：宿主的 JSON 库常把 `/` 转义成 `\/`（Android 的 org.json、iOS 的
    // JSONSerialization 默认都这样），回包原文里是 `file:\/\/\/data\/…`。从前筛 `file://`，原生 Android 视图
    // 选的图原样以 `file:///…` 交给了小程序，`<image>` 显示不出来
    if !result_json.contains("file:") {
        return result_json.to_string();
    }
    let Ok(mut v) = serde_json::from_str::<Value>(result_json) else { return result_json.to_string() };
    let mut seen = HashMap::new();
    import_in(&mut v, &mut seen);
    v.to_string()
}

fn import_in(v: &mut Value, seen: &mut HashMap<String, String>) {
    match v {
        Value::String(s) if s.starts_with("file://") => {
            if let Some(uri) = seen.get(s.as_str()) {
                *s = uri.clone();
                return;
            }
            let imported = file_uri_path(s).ok_or_else(|| "文件不存在".to_string()).and_then(|p| {
                crate::wxfile::import(&p).map_err(|e| e.to_string())
            });
            match imported {
                Ok(uri) => {
                    seen.insert(s.clone(), uri.clone());
                    *s = uri;
                }
                // 走 diag：Android 上 stderr 进不了 logcat，失败了要看得见
                Err(e) => crate::compositor::diag::emit(&format!("⚠️ 宿主回包里的 {s} 导入失败：{e}（原样交给小程序）")),
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| import_in(x, seen)),
        Value::Object(o) => o.values_mut().for_each(|x| import_in(x, seen)),
        _ => {}
    }
}

/// `file://` 后面的真实路径。宿主多半直接拼 `"file://" + 绝对路径`（Kotlin / Dart），也可能给的是
/// 百分号转义过的标准 URI（Swift 的 `URL.absoluteString`）：先按原样找，找不到且带 `%` 再解码。
/// Windows 的 `file:///C:/x` 去掉盘符前那个 `/`
fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let fix = |s: &str| -> PathBuf {
        let b = s.as_bytes();
        if b.len() > 2 && b[0] == b'/' && b[2] == b':' && b[1].is_ascii_alphabetic() {
            PathBuf::from(&s[1..])
        } else {
            PathBuf::from(s)
        }
    };
    let raw = fix(rest);
    if raw.is_file() {
        return Some(raw);
    }
    if rest.contains('%') {
        let decoded = fix(&percent_decode(rest));
        if decoded.is_file() {
            return Some(decoded);
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decoding_and_windows_drive() {
        assert_eq!(percent_decode("/a%20b/%E5%9B%BE.jpg"), "/a b/图.jpg");
        assert_eq!(percent_decode("/100%"), "/100%");
        assert_eq!(percent_decode("/x%zz"), "/x%zz");
        // 不存在的文件：不导入
        assert!(file_uri_path("file:///definitely/not/here.jpg").is_none());
        assert!(file_uri_path("/no/scheme.jpg").is_none());
    }

    #[test]
    fn reply_without_file_uris_is_untouched() {
        let s = r#"{"code":"x","list":[1,2]}"#;
        assert_eq!(import_reply_files(s), s);
        assert_eq!(import_reply_files("not json file://x"), "not json file://x");
    }

    #[test]
    fn existing_file_uri_is_found_raw_and_percent_encoded() {
        let dir = std::env::temp_dir().join("mini-render-app-host-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a b.txt");
        std::fs::write(&p, "x").unwrap();
        let raw = format!("file://{}", p.display());
        let enc = format!("file://{}", p.display().to_string().replace(' ', "%20"));
        assert_eq!(file_uri_path(&raw).as_deref(), Some(p.as_path()));
        assert_eq!(file_uri_path(&enc).as_deref(), Some(p.as_path()));
        std::fs::remove_dir_all(&dir).ok();
    }
}
