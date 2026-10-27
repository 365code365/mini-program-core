//! `wx.request` / `wx.downloadFile` / `wx.uploadFile` 的原生实现：后台线程发起 HTTP，主线程按帧取回结果。
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

/// 已完成、等待喂回 JS 的响应，带上**发起它的那个引擎实例**（[`new_owner`] 发的号）。
///
/// 为什么要分实例：JS 侧的请求 id 每个运行时都从 1 数起，而队列是进程级的。
/// 同一进程里有两个 `MiniApp`（测试并行跑、宿主同时预热两个小程序）时，谁先 `update()`
/// 谁就把别人的响应全取走 —— 自己那份落空，对方拿到一个 id 撞车的回调。
/// `network_tests::unsupported_scheme_goes_to_fail` 在全量并行里偶发失败就是这么来的。
static DONE: OnceLock<Mutex<Vec<(u64, NetResponse)>>> = OnceLock::new();
/// 实例号发号器（每个 `JsBridge` 取一个，从 1 开始）
static NEXT_OWNER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
/// 是否有请求在飞（宿主据此决定要不要继续出帧等回调）
static IN_FLIGHT: OnceLock<Mutex<u32>> = OnceLock::new();
/// 请求日志开关（`MINI_NET_LOG=1`）
static LOG: OnceLock<bool> = OnceLock::new();

fn done() -> &'static Mutex<Vec<(u64, NetResponse)>> {
    DONE.get_or_init(|| Mutex::new(Vec::new()))
}

/// 给一个引擎实例发一个网络归属号（每个 `JsBridge` 一个）
pub fn new_owner() -> u64 {
    let owner = NEXT_OWNER.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut live) = live().lock() {
        live.insert(owner);
    }
    owner
}

fn in_flight() -> &'static Mutex<u32> {
    IN_FLIGHT.get_or_init(|| Mutex::new(0))
}

fn log_enabled() -> bool {
    *LOG.get_or_init(|| std::env::var("MINI_NET_LOG").is_ok())
}

/// 单个请求还没回来就一直算「在飞」，宿主用它决定是否续帧。
/// 已完成的只看 `owner` 自己的：已销毁实例迟到的响应没人取，算进来会让宿主永远续帧
/// （「在飞」计数仍是进程级，最多拖到那条请求超时）
pub fn has_pending(owner: u64) -> bool {
    *in_flight().lock().unwrap() > 0 || done().lock().unwrap().iter().any(|(o, _)| *o == owner) || crate::net_stream::has_pending(owner)
}

/// 实例销毁：丢掉它还没取的响应，之后迟到的也不再入队（[`push_done`] 只收活着的实例）
pub fn forget_owner(owner: u64) {
    if let Ok(mut live) = live().lock() {
        live.remove(&owner);
    }
    if let Ok(mut q) = done().lock() {
        q.retain(|(o, _)| *o != owner);
    }
    crate::net_stream::forget_owner(owner);
}

/// 还活着的实例号
fn live() -> &'static Mutex<std::collections::HashSet<u64>> {
    static LIVE: OnceLock<Mutex<std::collections::HashSet<u64>>> = OnceLock::new();
    LIVE.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

/// 取走 `owner` 这个实例已完成的响应（读后清掉；别的实例的留在队列里）
pub fn take_completed(owner: u64) -> Vec<NetResponse> {
    let mut q = done().lock().unwrap();
    if q.iter().all(|(o, _)| *o != owner) {
        return Vec::new();
    }
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *q).into_iter().partition(|(o, _)| *o == owner);
    *q = rest;
    mine.into_iter().map(|(_, r)| r).collect()
}

/// 发起一次请求。立即返回，结果稍后进 [`take_completed`]（按 `owner` 取）。
///
/// `headers_json` 是 `{"名":"值"}` 形式；`timeout_ms` 为 0 时用默认 60s（与微信一致）。
pub fn submit(
    owner: u64,
    id: u32,
    method: &str,
    url: &str,
    headers_json: &str,
    body: &str,
    timeout_ms: u64,
) {
    submit_ex(owner, id, method, url, headers_json, body, timeout_ms, Default::default())
}

/// 同 [`submit`]，带流式选项（`responseType:'arraybuffer'` / `enableChunked`，见 net_stream.rs）
#[allow(clippy::too_many_arguments)]
pub fn submit_ex(
    owner: u64,
    id: u32,
    method: &str,
    url: &str,
    headers_json: &str,
    body: &str,
    timeout_ms: u64,
    opts: crate::net_stream::StreamOpts,
) {
    let method = method.to_uppercase();
    let url = url.to_string();
    let body = body.to_string();
    let headers = parse_headers(headers_json);
    let timeout = Duration::from_millis(if timeout_ms == 0 { 60_000 } else { timeout_ms });

    // 只接受 http/https：小程序里 `wx.request` 也只支持这两种协议，
    // 让别的 scheme 早失败比让 ureq 抛一个含内部细节的错好。
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        push_done(owner, NetResponse {
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
        let resp = perform(owner, id, &method, &url, &headers, &body, timeout, opts);
        let ms = started.elapsed().as_millis();
        if log_enabled() {
            match &resp.error {
                Some(e) => eprintln!("🌐 ✗ [{}] {} ({}ms)", id, e, ms),
                None => eprintln!("🌐 ← [{}] {} {} bytes ({}ms)", id, resp.status, resp.body.len(), ms),
            }
        }
        push_done(owner, NetResponse { id, ..resp });
        let mut n = in_flight().lock().unwrap();
        *n = n.saturating_sub(1);
    });
}

fn push_done(owner: u64, r: NetResponse) {
    // 发起它的实例已经销毁：没人会取，入队只会一直占着内存
    if !live().lock().map(|l| l.contains(&owner)).unwrap_or(true) {
        return;
    }
    done().lock().unwrap().push((owner, r));
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
#[allow(clippy::too_many_arguments)]
fn perform(
    owner: u64,
    id: u32,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: &str,
    timeout: Duration,
    opts: crate::net_stream::StreamOpts,
) -> NetResponse {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        // 跟随重定向：微信的 request 默认也跟
        .max_redirects(5)
        // **4xx/5xx 不当错误**：微信里它们照样走 success 回调，只是 statusCode 不是 2xx。
        // ureq 3 默认把状态码当 Err（2.x 是 `Error::Status(code, resp)` 带着响应），
        // 关掉这个开关才能拿到响应体 —— 否则接口返回 400 + 错误详情时，
        // 页面只能看到一句 request:fail，看不到服务端说了什么。
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);

    let mut req = ureq::http::Request::builder()
        .method(method)
        .uri(url);
    let mut has_content_type = false;
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("content-type") {
            has_content_type = true;
        }
        req = req.header(k, v);
    }
    // 微信的默认 content-type 是 application/json
    if !has_content_type && !body.is_empty() {
        req = req.header("content-type", "application/json");
    }
    let built = if body.is_empty() {
        req.body(String::new())
    } else {
        req.body(body.to_string())
    };
    let request = match built {
        Ok(r) => r,
        Err(e) => {
            return NetResponse {
                id: 0,
                status: 0,
                headers: Vec::new(),
                body: String::new(),
                error: Some(format!("request:fail {}", e)),
            }
        }
    };

    match agent.run(request) {
        Ok(resp) => collect(owner, id, resp, opts),
        Err(e) => NetResponse {
            id: 0,
            status: 0,
            headers: Vec::new(),
            body: String::new(),
            error: Some(format!("request:fail {}", e)),
        },
    }
}

// ───────────── downloadFile / uploadFile（文件进出，结果同样进 DONE 队列） ─────────────

/// 单个文件的上限：下载 / 上传都不超过 200MB（微信 downloadFile 的上限也是 200MB）
const FILE_LIMIT: u64 = 200 * 1024 * 1024;

fn file_fail(owner: u64, id: u32, msg: String) {
    push_done(owner, NetResponse { id, status: 0, headers: Vec::new(), body: String::new(), error: Some(msg) });
}

/// 在后台线程里跑 `job`，「在飞」计数与日志同 [`submit`]
fn spawn_job(owner: u64, id: u32, what: String, job: impl FnOnce() -> NetResponse + Send + 'static) {
    *in_flight().lock().unwrap() += 1;
    if log_enabled() {
        eprintln!("🌐 → [{}] {}", id, what);
    }
    std::thread::spawn(move || {
        let resp = job();
        if log_enabled() {
            match &resp.error {
                Some(e) => eprintln!("🌐 ✗ [{}] {}", id, e),
                None => eprintln!("🌐 ← [{}] {} {}", id, resp.status, resp.body),
            }
        }
        push_done(owner, NetResponse { id, ..resp });
        let mut n = in_flight().lock().unwrap();
        *n = n.saturating_sub(1);
    });
}

/// 发一次请求、把正文按**字节**读回来（下载要原样落盘，不能当 UTF-8 文本读）。
/// `audio_ctx` 拉远程音频也用它（在它自己的解码线程里调，阻塞无妨）
pub(crate) fn fetch_bytes(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Vec<u8>,
    timeout: Duration,
) -> Result<(u16, Vec<(String, String)>, Vec<u8>), String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(5)
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut req = ureq::http::Request::builder().method(method).uri(url);
    for (k, v) in headers {
        req = req.header(k, v);
    }
    let request = req.body(body).map_err(|e| e.to_string())?;
    let mut resp = agent.run(request).map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let hdrs: Vec<(String, String)> = resp
        .headers()
        .iter()
        .filter_map(|(n, v)| v.to_str().ok().map(|v| (n.as_str().to_lowercase(), v.to_string())))
        .collect();
    let bytes = resp.body_mut().with_config().limit(FILE_LIMIT).read_to_vec().map_err(|e| e.to_string())?;
    Ok((status, hdrs, bytes))
}

/// `wx.downloadFile`：GET 之后落到 `wxfile://`（给了 `filePath` 就存那里，否则建一个临时文件）。
/// 回给逻辑层的正文是 `{"tempFilePath": …}` 或 `{"filePath": …}`；4xx/5xx 照样落盘、照样 success（微信语义）
pub fn submit_download(owner: u64, id: u32, url: &str, headers_json: &str, file_path: &str, timeout_ms: u64) {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return file_fail(owner, id, format!("downloadFile:fail 不支持的协议: {url}"));
    }
    let dest = if file_path.is_empty() {
        let (uri, path) = crate::wxfile::new_temp(&crate::wxfile::ext_of_url(url));
        (uri, path, "tempFilePath")
    } else {
        match crate::wxfile::to_path(file_path) {
            Some(p) => (file_path.to_string(), p, "filePath"),
            None => return file_fail(owner, id, format!("downloadFile:fail invalid filePath: {file_path}")),
        }
    };
    let (url, headers) = (url.to_string(), parse_headers(headers_json));
    let timeout = Duration::from_millis(if timeout_ms == 0 { 60_000 } else { timeout_ms });
    // 边收边写盘（大文件不整份进内存），每块报一次进度（DownloadTask.onProgressUpdate，GT-N4）
    spawn_job(owner, id, format!("GET {url} → {}", dest.0), move || match crate::net_stream::run("GET", &url, &headers, Vec::new(), timeout) {
        Ok(mut resp) => {
            let status = resp.status().as_u16();
            let hdrs: Vec<(String, String)> = resp
                .headers()
                .iter()
                .filter_map(|(n, v)| v.to_str().ok().map(|v| (n.as_str().to_lowercase(), v.to_string())))
                .collect();
            crate::net_stream::push_event(owner, id, crate::net_stream::NetEvent::Headers { status, headers: hdrs.clone() });
            let (uri, path, key) = dest;
            match crate::net_stream::download_to(owner, id, &mut resp, &path) {
                Ok(_) => {
                    let mut m = serde_json::Map::new();
                    m.insert(key.to_string(), serde_json::Value::String(uri));
                    NetResponse { id, status, headers: hdrs, body: serde_json::Value::Object(m).to_string(), error: None }
                }
                Err(e) => NetResponse { id, status: 0, headers: Vec::new(), body: String::new(), error: Some(format!("downloadFile:fail {e}")) },
            }
        }
        Err(e) => NetResponse { id, status: 0, headers: Vec::new(), body: String::new(), error: Some(format!("downloadFile:fail {e}")) },
    });
}

/// `wx.uploadFile`：`multipart/form-data` POST，`formData` 的每一项是一个字段，文件字段名是 `name`。
/// `filePath` 可以是 `wxfile://` 或包内路径（与 `<image src>` 同一套解析）
#[allow(clippy::too_many_arguments)]
pub fn submit_upload(owner: u64, id: u32, url: &str, file_path: &str, name: &str, headers_json: &str, form_json: &str, timeout_ms: u64) {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return file_fail(owner, id, format!("uploadFile:fail 不支持的协议: {url}"));
    }
    let Some(path) = crate::assets::resolve(file_path) else {
        return file_fail(owner, id, format!("uploadFile:fail file not found: {file_path}"));
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) if (b.len() as u64) <= FILE_LIMIT => b,
        Ok(_) => return file_fail(owner, id, "uploadFile:fail file too large".into()),
        Err(e) => return file_fail(owner, id, format!("uploadFile:fail {e}")),
    };
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("file").to_string();
    let form = parse_headers(form_json);
    let (body, boundary) = multipart(&form, name, &filename, &bytes);
    let mut headers: Vec<(String, String)> = parse_headers(headers_json)
        .into_iter()
        .filter(|(k, _)| !k.eq_ignore_ascii_case("content-type"))
        .collect();
    headers.push(("content-type".into(), format!("multipart/form-data; boundary={boundary}")));
    let url = url.to_string();
    let timeout = Duration::from_millis(if timeout_ms == 0 { 60_000 } else { timeout_ms });
    spawn_job(owner, id, format!("POST {url} ← {file_path}"), move || match fetch_bytes("POST", &url, &headers, body, timeout) {
        Ok((status, hdrs, resp)) => NetResponse { id, status, headers: hdrs, body: String::from_utf8_lossy(&resp).into_owned(), error: None },
        Err(e) => NetResponse { id, status: 0, headers: Vec::new(), body: String::new(), error: Some(format!("uploadFile:fail {e}")) },
    });
}

/// 拼 multipart 正文：普通字段在前，文件在最后
pub(crate) fn multipart(form: &[(String, String)], file_field: &str, filename: &str, file: &[u8]) -> (Vec<u8>, String) {
    let boundary = format!("----MiniRenderBoundary{:016x}", {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (file.len(), filename, std::time::SystemTime::now()).hash(&mut h);
        h.finish()
    });
    let mut out = Vec::with_capacity(file.len() + 512);
    for (k, v) in form {
        out.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n").as_bytes());
    }
    out.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{file_field}\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n")
            .as_bytes(),
    );
    out.extend_from_slice(file);
    out.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (out, boundary)
}

fn collect(owner: u64, id: u32, mut resp: ureq::http::Response<ureq::Body>, opts: crate::net_stream::StreamOpts) -> NetResponse {
    let status = resp.status().as_u16();
    let headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .filter_map(|(n, v)| v.to_str().ok().map(|v| (n.as_str().to_lowercase(), v.to_string())))
        .collect();
    // 响应头先到（RequestTask.onHeadersReceived，GT-N3）：在正文读完之前就排进过程事件
    crate::net_stream::push_event(owner, id, crate::net_stream::NetEvent::Headers { status, headers: headers.clone() });
    // 读不出正文时给空串而不是报错：微信在这种情况下也会回 success + 空 data。
    // 文本按 UTF-8 读、替换非法字节（而不是整个正文丢成空串）；arraybuffer 原样进字节仓（net_stream.rs）
    let body = crate::net_stream::read_body(owner, id, &mut resp, opts).unwrap_or_default();
    NetResponse { id: 0, status, headers, body, error: None }
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
        // 自己领一个归属号：并行跑的 MiniApp 测试 update() 时拿不走这条
        let owner = new_owner();
        submit(owner, 9001, "GET", "ftp://example.com/x", "{}", "", 0);
        let done = take_completed(owner);
        forget_owner(owner);
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
