//! `wx.connectSocket` 的原生实现：每条连接一个后台线程，事件进全局队列，宿主每帧取回。
//!
//! 与 `net.rs` 同一个模式（逻辑层不阻塞、回调由 `MiniApp::update` 每帧送回 JS），
//! 只是连接是长的：线程里循环「读一帧 / 发掉排队的消息」。读设了 15ms 超时，
//! 超时就回头看一眼发送队列 —— 不用为每条连接再起一个写线程，也不用 async 运行时。
//!
//! 协议层用 `tungstenite`（握手、分帧、掩码、ping/pong 都在里面），TLS 自己接 rustls：
//! 根证书与 ureq 用的是同一份 `webpki-roots`，`wss://` 与 `wx.request` 的 https 信任同一批 CA。
//!
//! 二进制帧不经 base64：收到的字节进 `bin_stash`，事件里只带 `"bin": true`，JS 按 id 取 ArrayBuffer；
//! 发送走 `send_bytes`（`__native_socket_send_bin` 直接收 ArrayBuffer）。base64 的 `send` 仍保留（老的 JS 外壳、测试）。

use base64::Engine as _;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tungstenite::client::IntoClientRequest;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;
use tungstenite::{Error as WsError, Message};

/// 交回逻辑层的事件（`__resolveSocket(id, type, json)`）
#[derive(Debug, Clone, PartialEq)]
pub enum SocketEvent {
    Open { header: Vec<(String, String)>, protocol: String },
    Text(String),
    /// 二进制帧（字节在 `to_js` 时放进 bin_stash）
    Binary(Vec<u8>),
    Error(String),
    Close { code: u16, reason: String },
}

impl SocketEvent {
    /// (`type`, 参数 JSON)：JS 侧按 type 分发到 onOpen / onMessage / onError / onClose。
    /// 二进制帧的字节在这里交给 bin_stash（JS 收到 `bin: true` 再取），所以按值消费
    pub fn into_js(self, id: u32) -> (&'static str, String) {
        if let SocketEvent::Binary(b) = self {
            crate::bin_stash::stash(crate::bin_stash::WS, id, b);
            return ("message", serde_json::json!({ "bin": true }).to_string());
        }
        self.to_js()
    }

    pub fn to_js(&self) -> (&'static str, String) {
        use serde_json::json;
        match self {
            SocketEvent::Open { header, protocol } => {
                let h: serde_json::Map<String, serde_json::Value> =
                    header.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
                ("open", json!({ "header": h, "protocol": protocol }).to_string())
            }
            SocketEvent::Text(s) => ("message", json!({ "data": s }).to_string()),
            SocketEvent::Binary(b) => ("message", json!({ "bytes": b.len() }).to_string()),
            SocketEvent::Error(e) => ("error", json!({ "errMsg": e }).to_string()),
            SocketEvent::Close { code, reason } => ("close", json!({ "code": code, "reason": reason }).to_string()),
        }
    }
}

enum Cmd {
    Text(String),
    Binary(Vec<u8>),
    Close(u16, String),
}

type Events = Mutex<Vec<(u32, SocketEvent)>>;

fn events() -> &'static Events {
    static E: OnceLock<Events> = OnceLock::new();
    E.get_or_init(|| Mutex::new(Vec::new()))
}

fn sockets() -> &'static Mutex<HashMap<u32, Sender<Cmd>>> {
    static S: OnceLock<Mutex<HashMap<u32, Sender<Cmd>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn push(id: u32, ev: SocketEvent) {
    events().lock().unwrap().push((id, ev));
}

/// 取走全部待送回 JS 的事件
pub fn take_events() -> Vec<(u32, SocketEvent)> {
    std::mem::take(&mut *events().lock().unwrap())
}

/// 有连接开着、或有事件没送回：宿主据此续帧（与「有请求在飞」同一个判据）
pub fn has_pending() -> bool {
    !events().lock().unwrap().is_empty() || !sockets().lock().unwrap().is_empty()
}

/// 微信同时最多 5 条 WebSocket（官方 api/network/websocket 说明）
pub const MAX_SOCKETS: usize = 5;

/// 发起连接。立即返回：成功与否以 `open` / `error` 事件告知。
/// `headers_json`：`{"名":"值"}`；`protocols_json`：`["子协议", …]`；`timeout_ms` 为 0 时 60s
pub fn connect(id: u32, url: &str, headers_json: &str, protocols_json: &str, timeout_ms: u64) -> Result<(), String> {
    if !(url.starts_with("ws://") || url.starts_with("wss://")) {
        return Err(format!("connectSocket:fail invalid url \"{url}\""));
    }
    let (tx, rx) = channel();
    {
        let mut map = sockets().lock().unwrap();
        if map.len() >= MAX_SOCKETS {
            return Err(format!("connectSocket:fail exceed max task count {MAX_SOCKETS}"));
        }
        map.insert(id, tx);
    }
    let url = url.to_string();
    let headers = parse_map(headers_json);
    let protocols: Vec<String> = serde_json::from_str(protocols_json).unwrap_or_default();
    let timeout = Duration::from_millis(if timeout_ms == 0 { 60_000 } else { timeout_ms });
    std::thread::spawn(move || {
        run(id, &url, &headers, &protocols, timeout, rx);
        sockets().lock().unwrap().remove(&id);
    });
    Ok(())
}

/// `SocketTask.send`：文本或 base64 二进制。连接不存在（已关 / 没连上）时给错误串
pub fn send(id: u32, text: Option<String>, binary_b64: Option<String>) -> Result<(), String> {
    let cmd = match (text, binary_b64) {
        (_, Some(b)) => Cmd::Binary(
            base64::engine::general_purpose::STANDARD.decode(b.as_bytes()).map_err(|e| format!("sendSocketMessage:fail {e}"))?,
        ),
        (Some(t), None) => Cmd::Text(t),
        (None, None) => Cmd::Text(String::new()),
    };
    let map = sockets().lock().unwrap();
    let tx = map.get(&id).ok_or_else(|| "sendSocketMessage:fail WebSocket is not connected".to_string())?;
    tx.send(cmd).map_err(|_| "sendSocketMessage:fail WebSocket is not connected".to_string())
}

/// `SocketTask.send` 的二进制版（字节直接从 ArrayBuffer 来）
pub fn send_bytes(id: u32, bytes: Vec<u8>) -> Result<(), String> {
    let map = sockets().lock().unwrap();
    let tx = map.get(&id).ok_or_else(|| "sendSocketMessage:fail WebSocket is not connected".to_string())?;
    tx.send(Cmd::Binary(bytes)).map_err(|_| "sendSocketMessage:fail WebSocket is not connected".to_string())
}

/// `SocketTask.close`：发关闭帧，等对端回关闭帧后以 `close` 事件结束
pub fn close(id: u32, code: u16, reason: &str) -> Result<(), String> {
    let map = sockets().lock().unwrap();
    let tx = map.get(&id).ok_or_else(|| "closeSocket:fail WebSocket is not connected".to_string())?;
    tx.send(Cmd::Close(code, reason.to_string())).map_err(|_| "closeSocket:fail WebSocket is not connected".to_string())
}

fn parse_map(json: &str) -> Vec<(String, String)> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    map.into_iter()
        .filter_map(|(k, v)| match v {
            serde_json::Value::Null => None,
            serde_json::Value::String(s) => Some((k, s)),
            other => Some((k, other.to_string())),
        })
        .collect()
}

/// 明文或 TLS 的 TCP 流（tungstenite 只要 Read + Write）
enum Stream {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Stream {
    fn tcp(&self) -> &TcpStream {
        match self {
            Stream::Plain(s) => s,
            Stream::Tls(s) => &s.sock,
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain(s) => s.read(buf),
            Stream::Tls(s) => s.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Plain(s) => s.write(buf),
            Stream::Tls(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Plain(s) => s.flush(),
            Stream::Tls(s) => s.flush(),
        }
    }
}

fn tls_config() -> Arc<rustls::ClientConfig> {
    static CFG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    CFG.get_or_init(|| {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let cfg = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("rustls 默认协议版本")
            .with_root_certificates(roots)
            .with_no_client_auth();
        Arc::new(cfg)
    })
    .clone()
}

fn open_stream(url: &str, timeout: Duration) -> Result<Stream, String> {
    let req = url.into_client_request().map_err(|e| e.to_string())?;
    let uri = req.uri();
    let host = uri.host().ok_or("url 缺主机名")?.trim_matches(['[', ']']).to_string();
    let tls = uri.scheme_str() == Some("wss");
    let port = uri.port_u16().unwrap_or(if tls { 443 } else { 80 });
    let addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| format!("DNS 解析失败: {e}"))?
        .next()
        .ok_or("DNS 无结果")?;
    let tcp = TcpStream::connect_timeout(&addr, timeout).map_err(|e| e.to_string())?;
    tcp.set_nodelay(true).ok();
    // 握手阶段用整段超时；握手完再换成短超时（见 run）
    tcp.set_read_timeout(Some(timeout)).ok();
    tcp.set_write_timeout(Some(timeout)).ok();
    if !tls {
        return Ok(Stream::Plain(tcp));
    }
    let name = rustls::pki_types::ServerName::try_from(host).map_err(|e| e.to_string())?;
    let conn = rustls::ClientConnection::new(tls_config(), name).map_err(|e| e.to_string())?;
    Ok(Stream::Tls(Box::new(rustls::StreamOwned::new(conn, tcp))))
}

fn is_timeout(e: &WsError) -> bool {
    matches!(e, WsError::Io(io) if matches!(io.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
}

fn run(id: u32, url: &str, headers: &[(String, String)], protocols: &[String], timeout: Duration, rx: Receiver<Cmd>) {
    let fail = |msg: String| {
        push(id, SocketEvent::Error(msg));
        push(id, SocketEvent::Close { code: 1006, reason: "abnormal closure".into() });
    };
    let stream = match open_stream(url, timeout) {
        Ok(s) => s,
        Err(e) => return fail(format!("connect fail: {e}")),
    };
    let mut req = match url.into_client_request() {
        Ok(r) => r,
        Err(e) => return fail(e.to_string()),
    };
    for (k, v) in headers {
        if let (Ok(name), Ok(val)) = (
            tungstenite::http::HeaderName::from_bytes(k.as_bytes()),
            tungstenite::http::HeaderValue::from_str(v),
        ) {
            req.headers_mut().insert(name, val);
        }
    }
    if !protocols.is_empty() {
        if let Ok(v) = tungstenite::http::HeaderValue::from_str(&protocols.join(", ")) {
            req.headers_mut().insert("Sec-WebSocket-Protocol", v);
        }
    }
    let (mut ws, resp) = match tungstenite::client::client(req, stream) {
        Ok(x) => x,
        Err(e) => return fail(format!("handshake fail: {e}")),
    };
    let header: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let protocol = resp
        .headers()
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    push(id, SocketEvent::Open { header, protocol });
    ws.get_ref().tcp().set_read_timeout(Some(Duration::from_millis(15))).ok();

    loop {
        // 先把排队的发出去
        loop {
            match rx.try_recv() {
                Ok(Cmd::Text(t)) => {
                    if let Err(e) = ws.send(Message::text(t)) {
                        return fail(e.to_string());
                    }
                }
                Ok(Cmd::Binary(b)) => {
                    if let Err(e) = ws.send(Message::binary(b)) {
                        return fail(e.to_string());
                    }
                }
                Ok(Cmd::Close(code, reason)) => {
                    let frame = CloseFrame { code: CloseCode::from(code), reason: reason.into() };
                    if ws.close(Some(frame)).is_err() {
                        push(id, SocketEvent::Close { code, reason: String::new() });
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                // JS 那边的发送端没了（引擎销毁）：直接收线
                Err(TryRecvError::Disconnected) => {
                    ws.close(None).ok();
                    return;
                }
            }
        }
        match ws.read() {
            Ok(Message::Text(t)) => push(id, SocketEvent::Text(t.to_string())),
            Ok(Message::Binary(b)) => push(id, SocketEvent::Binary(b.to_vec())),
            Ok(Message::Close(frame)) => {
                let (code, reason) = frame.map(|f| (u16::from(f.code), f.reason.to_string())).unwrap_or((1005, String::new()));
                // 对端先关：tungstenite 会自动回关闭帧，再读一次把它冲出去
                ws.flush().ok();
                push(id, SocketEvent::Close { code, reason });
                return;
            }
            Ok(_) => {} // ping / pong / 原始帧：tungstenite 自己处理
            Err(e) if is_timeout(&e) => {}
            Err(WsError::ConnectionClosed) => {
                push(id, SocketEvent::Close { code: 1000, reason: String::new() });
                return;
            }
            Err(e) => return fail(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_ws_urls() {
        assert!(connect(9001, "http://a.com", "{}", "[]", 0).is_err());
        assert!(send(9002, Some("x".into()), None).is_err());
        assert!(close(9003, 1000, "").is_err());
    }

    #[test]
    fn event_json_shapes() {
        let (t, j) = SocketEvent::Close { code: 1000, reason: "bye".into() }.to_js();
        assert_eq!(t, "close");
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["code"], 1000);
        let (t, j) = SocketEvent::Binary(vec![0, 1]).into_js(9004);
        assert_eq!(t, "message");
        assert!(j.contains("\"bin\""));
        assert_eq!(crate::bin_stash::take(crate::bin_stash::WS, 9004), Some(vec![0, 1]), "字节进了 bin_stash，不在 JSON 里");
    }
}
