//! `wx.request` / `wx.downloadFile` 的原生实现：后台线程发起 HTTP，主线程按帧取回结果。
//!
//! 设计要点是**不能阻塞 JS 线程**。小程序的网络 API 全是回调式的，逻辑层发起请求后
//! 立刻返回，等宿主把响应喂回来。所以这里：
//!
//! 1. JS 侧 `wx.request` 生成一个自增 id，把回调存进表里，调 `__native_request`；
//! 2. Rust 侧派一个线程去跑 ureq，完成后把结果塞进全局队列；
//! 3. 宿主每帧（`MiniApp::update`）取走已完成的响应，回调 JS 的 `__resolveRequest`。
//!
//! 这样 `wx.request(...).then(...)`、`Promise` 链、`async/await` 都能正常推进 ——
//! 它们靠的是微任务队列，而宿主每帧都会 pump 一次。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// 一次请求的结果。`error` 非空表示失败（对应 `fail` 回调）。
#[derive(Debug, Clone)]
pub struct NetResponse {
    pub id: u32,
    pub status: u16,
    /// 响应头（已小写化的 name -> value）
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub error: Option<String>,
}

/// 已完成、等待喂回 JS 的响应
static DONE: OnceLock<Mutex<Vec<NetResponse>>> = OnceLock::new();
/// 是否有请求在飞（宿主据此决定要不要继续出帧等回调）
static IN_FLIGHT: OnceLock<Mutex<u32>> = OnceLock::new();
/// 请求日志开关（`MINI_NET_LOG=1`）
static LOG: OnceLock<bool> = OnceLock::new();

fn done() -> &'static Mutex<Vec<NetResponse>> {
    DONE.get_or_init(|| Mutex::new(Vec::new()))
}

fn in_flight() -> &'static Mutex<u32> {
    IN_FLIGHT.get_or_init(|| Mutex::new(0))
}

fn log_enabled() -> bool {
    *LOG.get_or_init(|| std::env::var("MINI_NET_LOG").is_ok())
}

/// 单个请求还没回来就一直算「在飞」，宿主用它决定是否续帧
pub fn has_pending() -> bool {
    *in_flight().lock().unwrap() > 0 || !done().lock().unwrap().is_empty()
}

/// 取走全部已完成的响应（读后清空）
pub fn take_completed() -> Vec<NetResponse> {
    std::mem::take(&mut *done().lock().unwrap())
}

/// 发起一次请求。立即返回，结果稍后进 [`take_completed`]。
///
/// `headers_json` 是 `{"名":"值"}` 形式；`timeout_ms` 为 0 时用默认 60s（与微信一致）。
pub fn submit(
    id: u32,
    method: &str,
    url: &str,
    headers_json: &str,
    body: &str,
    timeout_ms: u64,
) {
    let method = method.to_uppercase();
    let url = url.to_string();
    let body = body.to_string();
    let headers = parse_headers(headers_json);
    let timeout = Duration::from_millis(if timeout_ms == 0 { 60_000 } else { timeout_ms });

    // 只接受 http/https：小程序里 `wx.request` 也只支持这两种协议，
    // 让别的 scheme 早失败比让 ureq 抛一个含内部细节的错好。
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        push_done(NetResponse {
            id,
            status: 0,
            headers: Vec::new(),
            body: String::new(),
            error: Some(format!("request:fail 不支持的协议: {}", url)),
        });
        return;
    }

    *in_flight().lock().unwrap() += 1;
    if log_enabled() {
        eprintln!("🌐 → [{}] {} {}", id, method, url);
    }

    // 每个请求一个线程。小程序对并发请求本来就有上限（微信是 10 个），
    // 页面级用量远达不到需要线程池的规模，换来的是实现简单、没有共享状态。
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let resp = perform(&method, &url, &headers, &body, timeout);
        let ms = started.elapsed().as_millis();
        if log_enabled() {
            match &resp.error {
                Some(e) => eprintln!("🌐 ✗ [{}] {} ({}ms)", id, e, ms),
                None => eprintln!("🌐 ← [{}] {} {} bytes ({}ms)", id, resp.status, resp.body.len(), ms),
            }
        }
        push_done(NetResponse { id, ..resp });
        let mut n = in_flight().lock().unwrap();
        *n = n.saturating_sub(1);
    });
}

fn push_done(r: NetResponse) {
    done().lock().unwrap().push(r);
}

fn parse_headers(json: &str) -> Vec<(String, String)> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    map.into_iter()
        .filter_map(|(k, v)| {
            let val = match v {
                serde_json::Value::String(s) => s,
                serde_json::Value::Null => return None,
                other => other.to_string(),
            };
            Some((k, val))
        })
        .collect()
}

/// 真正发请求。返回的 `id` 字段是占位，由调用方填。
fn perform(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: &str,
    timeout: Duration,
) -> NetResponse {
    let agent = ureq::AgentBuilder::new()
        .timeout(timeout)
        // 跟随重定向：微信的 request 默认也跟
        .redirects(5)
        .build();
    let mut req = agent.request(method, url);
    let mut has_content_type = false;
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("content-type") {
            has_content_type = true;
        }
        req = req.set(k, v);
    }
    // 微信的默认 content-type 是 application/json
    if !has_content_type && !body.is_empty() {
        req = req.set("content-type", "application/json");
    }

    let result = if body.is_empty() {
        req.call()
    } else {
        req.send_string(body)
    };

    match result {
        Ok(resp) => collect(resp, None),
        // HTTP 错误码（4xx/5xx）在微信里**仍然走 success**，只是 statusCode 不是 2xx
        Err(ureq::Error::Status(_, resp)) => collect(resp, None),
        Err(e) => NetResponse {
            id: 0,
            status: 0,
            headers: Vec::new(),
            body: String::new(),
            error: Some(format!("request:fail {}", e)),
        },
    }
}

fn collect(resp: ureq::Response, error: Option<String>) -> NetResponse {
    let status = resp.status();
    let headers: Vec<(String, String)> = resp
        .headers_names()
        .into_iter()
        .filter_map(|n| resp.header(&n).map(|v| (n.to_lowercase(), v.to_string())))
        .collect();
    // 读不出正文时给空串而不是报错：微信在这种情况下也会回 success + 空 data
    let body = resp.into_string().unwrap_or_default();
    NetResponse { id: 0, status, headers, body, error }
}

/// 有没有请求在飞（供宿主的动画/续帧判断）
pub static NET_DIRTY: AtomicBool = AtomicBool::new(false);

/// 标记「有响应回来了，需要重绘」
pub fn mark_dirty() {
    NET_DIRTY.store(true, Ordering::Relaxed);
}

/// 取走重绘标记
pub fn take_dirty() -> bool {
    NET_DIRTY.swap(false, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_scheme_immediately() {
        submit(9001, "GET", "ftp://example.com/x", "{}", "", 0);
        let done = take_completed();
        let r = done.iter().find(|r| r.id == 9001).expect("应立即产出一条失败结果");
        assert!(r.error.as_deref().unwrap_or("").contains("不支持的协议"));
        assert_eq!(r.status, 0);
    }

    #[test]
    fn parses_header_map() {
        let h = parse_headers(r#"{"Content-Type":"application/json","X-N":1,"skip":null}"#);
        assert!(h.iter().any(|(k, v)| k == "Content-Type" && v == "application/json"));
        // 非字符串值也要带过去（微信允许数字头）
        assert!(h.iter().any(|(k, v)| k == "X-N" && v == "1"));
        // null 头丢弃
        assert!(!h.iter().any(|(k, _)| k == "skip"));
    }

    #[test]
    fn empty_header_json_is_tolerated() {
        assert!(parse_headers("").is_empty());
        assert!(parse_headers("not json").is_empty());
    }
}
