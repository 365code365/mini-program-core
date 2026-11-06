//! 页面里的 `__native_*` 过桥之后落到这里（名字去掉 `__native_` 前缀，参数与 QuickJS 模式的原生签名一一对应）。
//! 实现调的是同一份 Rust：wxfs、wxfile、net、storage_file、clipboard、wxfs::image —— 两种模式行为一致。
//!
//! - [`WebGame::call`]：同步，返回字符串（`fs` 读写、取二进制回包、临时文件……）；
//! - [`WebGame::post`]：异步，不返回（发请求、storage 写、宿主事件、日志）。
//!
//! 不认识的名字：同步返回空串、异步丢掉并记一行日志（页面那边的过桥层只发它定义过的名字）。

use super::WebGame;
use serde_json::{json, Value};

/// 消息体 `{"n": 名字, "a": [参数…]}` → (名字, 参数)。参数一律转成字符串（原生签名都是字符串）
pub fn parse_message(text: &str) -> Option<(String, Vec<String>)> {
    let v: Value = serde_json::from_str(text).ok()?;
    let n = v.get("n")?.as_str()?.to_string();
    let a = v
        .get("a")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .map(|x| match x {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    other => other.to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Some((n, a))
}

impl WebGame {
    /// 同步调用（`text` 是消息体 JSON）
    pub fn call_message(&mut self, text: &str) -> String {
        match parse_message(text) {
            Some((n, a)) => self.call(&n, &a),
            None => String::new(),
        }
    }

    /// 异步调用（`text` 是消息体 JSON）
    pub fn post_message(&mut self, text: &str) {
        if let Some((n, a)) = parse_message(text) {
            self.post(&n, &a);
        }
    }

    pub fn call(&mut self, name: &str, a: &[String]) -> String {
        let arg = |i: usize| a.get(i).map(|s| s.as_str()).unwrap_or("");
        match name {
            // FileSystemManager（二进制按 base64：fs.js 在没有 __native_fs_raw 时就这么走）
            "fs" => crate::wxfs::call(arg(0), arg(1)),
            // createBufferURL / 画布 toTempFilePath：base64 → wxfile://tmp/…（扩展名按内容认）
            "temp_write" => {
                let bytes = crate::wxfs::codec::unb64(arg(0)).unwrap_or_default();
                let (uri, real) = crate::wxfs::wxfile::new_temp(crate::js::sniff_ext(&bytes));
                if let Some(dir) = real.parent() {
                    std::fs::create_dir_all(dir).ok();
                }
                if std::fs::write(&real, &bytes).is_ok() { uri } else { String::new() }
            }
            // responseType:'arraybuffer' 的正文 / enableChunked 的分块（base64；页面解回 ArrayBuffer）
            "net_take_bytes" => {
                let id = arg(0).parse().unwrap_or(0);
                crate::net::stream::take_body(self.net_owner, id).map(|b| crate::wxfs::codec::b64(&b)).unwrap_or_default()
            }
            "net_take_chunk" => {
                let id = arg(0).parse().unwrap_or(0);
                crate::net::stream::take_chunk(self.net_owner, id).map(|b| crate::wxfs::codec::b64(&b)).unwrap_or_default()
            }
            "clipboard_get" => crate::sys::clipboard::get(),
            "image_info" => crate::wxfs::image::info(arg(0)),
            "compress_image" => crate::wxfs::image::compress(
                arg(0),
                arg(1).parse::<f64>().unwrap_or(80.0) as u8,
                arg(2).parse().unwrap_or(0),
                arg(3).parse().unwrap_or(0),
            ),
            "video_info" => crate::wxfs::video_info::info_json(arg(0)),
            // wx.loadFont：字族名（与 QuickJS 模式同一个取法：name 表 nameID 1），读不出给空串（页面回 null）
            "font_family" => super::super::fonts::family_of(arg(0), &self.root).unwrap_or_default(),
            "local_ip" => crate::net::socket::local_ip().unwrap_or_default(),
            // wxfile:// 的真实存在性（图片 src 给了不存在的临时文件时，页面按 QuickJS 模式同样的文案报 onerror）
            "file_exists" => crate::wxfs::wxfile::to_path(arg(0)).is_some_and(|p| p.is_file()).to_string(),
            _ => {
                self.log(&format!("⚠ 高性能模式：没有同步原生 {name}"));
                String::new()
            }
        }
    }

    pub fn post(&mut self, name: &str, a: &[String]) {
        let arg = |i: usize| a.get(i).map(|s| s.as_str()).unwrap_or("");
        let num = |i: usize| arg(i).parse::<u64>().unwrap_or(0);
        match name {
            "storage_set" => {
                self.storage.insert(arg(0).to_string(), arg(1).to_string());
                self.storage_dirty = true;
            }
            "storage_remove" => {
                self.storage.remove(arg(0));
                self.storage_dirty = true;
            }
            "storage_clear" => {
                self.storage.clear();
                self.storage_dirty = true;
            }
            // (id, method, url, headersJson, body, timeoutMs, flags)
            "request" => {
                let flags = num(6) as u32;
                let opts = crate::net::stream::StreamOpts { binary: flags & 1 != 0, chunked: flags & 2 != 0 };
                crate::net::submit_ex(self.net_owner, num(0) as u32, arg(1), arg(2), arg(3), arg(4), num(5), opts);
            }
            // (id, url, headersJson, filePath, timeoutMs)
            "download" => crate::net::submit_download(self.net_owner, num(0) as u32, arg(1), arg(2), arg(3), num(4)),
            // (id, url, filePath, name, headersJson, formJson, timeoutMs)
            "upload" => crate::net::submit_upload(self.net_owner, num(0) as u32, arg(1), arg(2), arg(3), arg(4), arg(5), num(6)),
            "clipboard_set" => crate::sys::clipboard::set(arg(0)),
            "temp_remove" => {
                let uri = arg(0);
                if uri.starts_with("wxfile://tmp/") {
                    if let Some(p) = crate::wxfs::wxfile::to_path(uri) {
                        std::fs::remove_file(p).ok();
                    }
                }
            }
            // 宿主事件（host.js 的 __emitHostEvent 等）：hostCall 补 tempDir / files（与 QuickJS 模式同一处理）
            "host_event" => {
                let raw = arg(0);
                let out = match serde_json::from_str::<Value>(raw) {
                    Ok(mut v) if v["type"] == "hostCall" => {
                        crate::runtime::annotate_host_call(&mut v);
                        v.to_string()
                    }
                    Ok(v) => v.to_string(),
                    Err(_) => return,
                };
                self.events.push_back(out);
            }
            // __native_host_request(kind, json)：页面自己处理不了的（预览图片、设背景色……）原样交给宿主
            "host_request" => {
                let data: Value = serde_json::from_str(arg(1)).unwrap_or(Value::Null);
                self.events.push_back(json!({ "type": "hostRequest", "kind": arg(0), "data": data }).to_string());
            }
            // 日志（console.* 与 __native_print，页面攒一批发一次）：一行一条
            "log" => {
                for line in arg(0).split('\n') {
                    self.events.push_back(json!({ "type": "log", "text": line }).to_string());
                }
            }
            // 页面捕获到的异常（window.onerror / 未处理的 Promise 拒绝）：与 QuickJS 模式同形的 error 事件
            "error" => {
                self.events.push_back(json!({ "type": "error", "kind": arg(0), "message": arg(1), "stack": arg(2) }).to_string());
            }
            _ => self.log(&format!("⚠ 高性能模式：没有异步原生 {name}")),
        }
    }

    /// 后台线程完成的请求 → `__resolveRequest`；过程事件（headers / chunk / progress）→ `__netEvent`
    /// （与 MiniApp::process_network_responses / process_net_events 同形，app.rs、app_io.rs）
    pub(super) fn collect_network(&mut self) {
        let done = crate::net::take_completed(self.net_owner);
        for (id, e) in crate::net::stream::take_events(self.net_owner) {
            let (kind, payload) = match e {
                crate::net::stream::NetEvent::Headers { status, headers } => {
                    let h: serde_json::Map<String, Value> = headers.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
                    ("headers", json!({ "statusCode": status, "header": h }))
                }
                crate::net::stream::NetEvent::Chunk => ("chunk", Value::Null),
                crate::net::stream::NetEvent::Progress { written, expected } => ("progress", json!({ "written": written, "expected": expected })),
            };
            self.js.push_back(format!("__netEvent({id}, {}, {});", json!(kind), json!(payload.to_string())));
        }
        for r in done {
            let headers: serde_json::Map<String, Value> = r.headers.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
            self.js.push_back(format!(
                "__wgCall(__resolveRequest, [{}, {}, {}, {}, {}]);",
                r.id,
                r.status,
                Value::String(Value::Object(headers).to_string()),
                Value::String(r.body),
                r.error.map(Value::String).unwrap_or(Value::Null),
            ));
        }
    }
}
