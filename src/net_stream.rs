//! `wx.request` 的流式部分（game-doc/04 G4.4，05 GT-N1~N4）：二进制正文、`onHeadersReceived`、`enableChunked` 分块、
//! `downloadFile` 的 `onProgressUpdate`。小程序与小游戏共用（都是官方 API；从前这些是空函数 / 按 UTF-8 读坏二进制）。
//!
//! 结果分两路回到逻辑层，都按「发起它的实例」（owner，见 net.rs）分开：
//! - **最终结果**照旧走 net.rs 的完成队列 → `__resolveRequest`；二进制正文不进 JSON（5MB 的 base64 字面量要 eval 一遍），
//!   而是放进这里的字节仓，JS 在回调里调 `__net_take_bytes(id)` 直接拿 `ArrayBuffer`（js/binary.rs，不经 base64）；
//! - **过程事件**（收到响应头、一块正文、下载进度）进这里的事件队列，`MiniApp::update` 在处理完成队列**之前**取走
//!   （`__netEvent(id, kind, json)`）：同一个请求的 headersReceived 一定在 success 之前（GT-N3）。分块正文同样在字节仓里
//!   排队，`__net_take_chunk(id)` 按到达顺序取。
//! 实例销毁（`net::forget_owner`）时这里的队列与字节仓一并清掉。

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// 一块的大小：太小事件太多（5MB 就是上千次回调），太大分块就没意义
const CHUNK: usize = 64 * 1024;
/// 二进制正文上限（与 downloadFile 一致）
const BODY_LIMIT: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone)]
pub enum NetEvent {
    Headers { status: u16, headers: Vec<(String, String)> },
    /// 一块正文到了（字节在仓里，按顺序取）
    Chunk,
    Progress { written: u64, expected: u64 },
}

type Key = (u64, u32);

fn events() -> &'static Mutex<Vec<(u64, u32, NetEvent)>> {
    static E: OnceLock<Mutex<Vec<(u64, u32, NetEvent)>>> = OnceLock::new();
    E.get_or_init(|| Mutex::new(Vec::new()))
}

fn bodies() -> &'static Mutex<HashMap<Key, Vec<u8>>> {
    static B: OnceLock<Mutex<HashMap<Key, Vec<u8>>>> = OnceLock::new();
    B.get_or_init(|| Mutex::new(HashMap::new()))
}

fn chunks() -> &'static Mutex<HashMap<Key, VecDeque<Vec<u8>>>> {
    static C: OnceLock<Mutex<HashMap<Key, VecDeque<Vec<u8>>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn push_event(owner: u64, id: u32, e: NetEvent) {
    if let Ok(mut q) = events().lock() {
        q.push((owner, id, e));
    }
}

/// 取走这个实例的过程事件（按到达顺序）
pub fn take_events(owner: u64) -> Vec<(u32, NetEvent)> {
    let Ok(mut q) = events().lock() else { return Vec::new() };
    if q.is_empty() {
        return Vec::new();
    }
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *q).into_iter().partition(|(o, _, _)| *o == owner);
    *q = rest;
    mine.into_iter().map(|(_, id, e)| (id, e)).collect()
}

pub fn take_body(owner: u64, id: u32) -> Option<Vec<u8>> {
    bodies().lock().ok()?.remove(&(owner, id))
}

pub fn take_chunk(owner: u64, id: u32) -> Option<Vec<u8>> {
    let mut c = chunks().lock().ok()?;
    let q = c.get_mut(&(owner, id))?;
    let v = q.pop_front();
    if q.is_empty() {
        c.remove(&(owner, id));
    }
    v
}

/// 实例销毁：它的事件、正文、分块都不会再有人取
pub fn forget_owner(owner: u64) {
    if let Ok(mut q) = events().lock() {
        q.retain(|(o, _, _)| *o != owner);
    }
    if let Ok(mut b) = bodies().lock() {
        b.retain(|(o, _), _| *o != owner);
    }
    if let Ok(mut c) = chunks().lock() {
        c.retain(|(o, _), _| *o != owner);
    }
}

pub fn has_pending(owner: u64) -> bool {
    events().lock().map(|q| q.iter().any(|(o, _, _)| *o == owner)).unwrap_or(false)
}

/// 流式请求的选项（JS 的 `responseType` / `enableChunked`）
#[derive(Clone, Copy, Debug, Default)]
pub struct StreamOpts {
    pub binary: bool,
    pub chunked: bool,
}

/// 读正文：分块时每 64KB 一个 Chunk 事件；二进制时整份进字节仓、返回空串；否则按 UTF-8（替换非法字节）返回
pub(crate) fn read_body(owner: u64, id: u32, resp: &mut ureq::http::Response<ureq::Body>, opts: StreamOpts) -> Result<String, String> {
    use std::io::Read;
    let mut reader = resp.body_mut().with_config().limit(BODY_LIMIT).reader();
    let mut all = Vec::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        // 一块要读满（或到尾）才发：read 一次可能只给几百字节
        let mut filled = 0;
        while filled < CHUNK {
            match reader.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) => return Err(format!("request:fail {e}")),
            }
        }
        if filled == 0 {
            break;
        }
        if opts.chunked {
            if let Ok(mut c) = chunks().lock() {
                c.entry((owner, id)).or_default().push_back(buf[..filled].to_vec());
            }
            push_event(owner, id, NetEvent::Chunk);
        }
        // 分块模式下最终的 data：二进制照样给全量（官方 success 里仍有 data），文本也给全量
        all.extend_from_slice(&buf[..filled]);
        if filled < CHUNK {
            break;
        }
    }
    if opts.binary {
        if let Ok(mut b) = bodies().lock() {
            b.insert((owner, id), all);
        }
        Ok(String::new())
    } else {
        Ok(String::from_utf8_lossy(&all).into_owned())
    }
}

/// 下载落盘，每写一块报一次进度（总长未知时 expected = 0，JS 侧 progress 按 0 报）
pub(crate) fn download_to(owner: u64, id: u32, resp: &mut ureq::http::Response<ureq::Body>, path: &std::path::Path) -> Result<u64, String> {
    use std::io::{Read, Write};
    let expected = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(0u64);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut reader = resp.body_mut().with_config().limit(BODY_LIMIT).reader();
    let mut buf = vec![0u8; CHUNK];
    let mut written = 0u64;
    let mut last = std::time::Instant::now() - Duration::from_secs(1);
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        written += n as u64;
        // 进度最多每 50ms 一次（一帧也就消费一次），最后一次在结尾补
        if last.elapsed() >= Duration::from_millis(50) {
            push_event(owner, id, NetEvent::Progress { written, expected });
            last = std::time::Instant::now();
        }
    }
    push_event(owner, id, NetEvent::Progress { written, expected: if expected == 0 { written } else { expected } });
    Ok(written)
}

/// 发一个请求拿到响应（正文还没读）：4xx / 5xx 不当错误（微信语义），跟随重定向
pub(crate) fn run(method: &str, url: &str, headers: &[(String, String)], body: Vec<u8>, timeout: Duration) -> Result<ureq::http::Response<ureq::Body>, String> {
    let config = ureq::Agent::config_builder().timeout_global(Some(timeout)).max_redirects(5).http_status_as_error(false).build();
    let agent = ureq::Agent::new_with_config(config);
    let mut req = ureq::http::Request::builder().method(method).uri(url);
    for (k, v) in headers {
        req = req.header(k, v);
    }
    agent.run(req.body(body).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// `__net_take_bytes(id)` / `__net_take_chunk(id)`：拿 ArrayBuffer（没有返回 null）
pub fn register(rt: &crate::js::JsRuntime, owner: u64) -> Result<(), String> {
    use rquickjs::Value;
    rt.register_raw("__net_take_bytes", move |ctx, args| {
        let id = args.0.first().and_then(|v| v.as_number()).unwrap_or(0.0) as u32;
        match take_body(owner, id) {
            Some(b) => crate::js::binary::array_buffer(&ctx, b),
            None => Ok(Value::new_null(ctx)),
        }
    })?;
    rt.register_raw("__net_take_chunk", move |ctx, args| {
        let id = args.0.first().and_then(|v| v.as_number()).unwrap_or(0.0) as u32;
        match take_chunk(owner, id) {
            Some(b) => crate::js::binary::array_buffer(&ctx, b),
            None => Ok(Value::new_null(ctx)),
        }
    })?;
    Ok(())
}
