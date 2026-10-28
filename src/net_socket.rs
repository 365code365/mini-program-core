//! `wx.createTCPSocket` / `wx.createUDPSocket` 的原生实现：每个 socket 一个后台线程，
//! 事件进全局队列，宿主每帧取回（`MiniApp::process_io_events` → `__resolveNetSocket`）。
//!
//! 与 `websocket.rs` 同一个模式：读设 15ms 超时，超时就回头看一眼命令队列 —— 一个线程同时
//! 管读与写，不用 async 运行时。收到的数据进 `bin_stash`（事件里只带 `bin: true`），
//! 发送有直收 ArrayBuffer 的 `*_bytes` 版；base64 的入口保留给老外壳与测试。
//!
//! 与微信的对应（官方 api/network/tcp、api/network/udp）：
//! - TCP：`connect` 之后 `onConnect` / `onError`；`write` 排进发送队列；对端关闭或 `close()` 发 `onClose`；
//!   `connect` 的 `timeout` 单位是**秒**（默认 2）。
//! - UDP：`bind(port)` **同步**返回实际端口（0 = 系统分配），随后发 `onListening`；`send` 可以带
//!   `setBroadcast`；`connect` 之后 `write` 不用再带地址；`setTTL`。
//! - `onMessage` 的 `remoteInfo` / `localInfo` 字段名与微信一致（`address` `family` `port` `size`）。

#[cfg(test)]
use base64::Engine as _;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// 交回逻辑层的事件：(`type`, 参数 JSON)
pub type NetEvent = (&'static str, String);

enum Cmd {
    Write(Vec<u8>),
    SendTo(Vec<u8>, String, u16, bool),
    Connect(String, u16),
    Ttl(u32),
    Close,
}

type Events = Mutex<Vec<(u32, NetEvent)>>;

fn events() -> &'static Events {
    static E: OnceLock<Events> = OnceLock::new();
    E.get_or_init(|| Mutex::new(Vec::new()))
}

fn sockets() -> &'static Mutex<HashMap<u32, Sender<Cmd>>> {
    static S: OnceLock<Mutex<HashMap<u32, Sender<Cmd>>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn push(id: u32, ty: &'static str, json: serde_json::Value) {
    events().lock().unwrap().push((id, (ty, json.to_string())));
}

#[cfg(test)]
fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)] // 老的 base64 入口：JS 已改走 *_bytes（bin_stash.rs），只留给单测
fn unb64(s: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD.decode(s.as_bytes()).map_err(|e| e.to_string())
}

/// socket 是哪条线程建的（引擎各在自己的线程上跑逻辑层）。事件只送回建它的那条线程：
/// 同一进程里几个引擎（或测试并行跑）各取各的 —— 从前 `take_events` 一把全拿走，
/// 别的引擎 / 用例的回包就这么丢了（`net_socket::tests` 在并行回归里时不时收不到 connect）
fn owners() -> &'static Mutex<HashMap<u32, std::thread::ThreadId>> {
    static O: OnceLock<Mutex<HashMap<u32, std::thread::ThreadId>>> = OnceLock::new();
    O.get_or_init(|| Mutex::new(HashMap::new()))
}

fn own(id: u32) {
    owners().lock().unwrap().insert(id, std::thread::current().id());
}

/// 取走这条线程建的 socket 待送回 JS 的事件（别的线程建的留在队列里）
pub fn take_events() -> Vec<(u32, NetEvent)> {
    let me = std::thread::current().id();
    let owners = owners().lock().unwrap();
    let mut q = events().lock().unwrap();
    let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *q).into_iter().partition(|(id, _)| owners.get(id).map_or(true, |t| *t == me));
    *q = rest;
    mine
}

/// 有 socket 开着或有事件没送回：宿主据此续帧
pub fn has_pending() -> bool {
    !events().lock().unwrap().is_empty() || !sockets().lock().unwrap().is_empty()
}

fn family(a: &SocketAddr) -> &'static str {
    if a.is_ipv4() { "IPv4" } else { "IPv6" }
}

fn info(a: &SocketAddr, size: Option<usize>) -> serde_json::Value {
    let mut v = serde_json::json!({ "address": a.ip().to_string(), "family": family(a), "port": a.port() });
    if let Some(n) = size {
        v["size"] = serde_json::json!(n);
    }
    v
}

fn resolve(addr: &str, port: u16) -> Result<SocketAddr, String> {
    (addr, port).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or_else(|| format!("无法解析地址 {addr}"))
}

fn command(id: u32, cmd: Cmd, api: &str) -> Result<(), String> {
    let map = sockets().lock().unwrap();
    let tx = map.get(&id).ok_or_else(|| format!("{api}:fail socket is not connected"))?;
    tx.send(cmd).map_err(|_| format!("{api}:fail socket is not connected"))
}

// ───────────────────────── TCP ─────────────────────────

/// `TCPSocket.connect`：立即返回，连上与否以 `connect` / `error` 事件告知
pub fn tcp_connect(id: u32, address: &str, port: u16, timeout_ms: u64) -> Result<(), String> {
    if port == 0 {
        return Err("connect:fail invalid port".into());
    }
    own(id);
    let (tx, rx) = channel();
    {
        let mut map = sockets().lock().unwrap();
        if map.contains_key(&id) {
            return Err("connect:fail socket is already connected".into());
        }
        map.insert(id, tx);
    }
    let address = address.to_string();
    let timeout = Duration::from_millis(timeout_ms.max(1));
    std::thread::spawn(move || {
        tcp_run(id, &address, port, timeout, rx);
        sockets().lock().unwrap().remove(&id);
    });
    Ok(())
}

fn tcp_run(id: u32, address: &str, port: u16, timeout: Duration, rx: Receiver<Cmd>) {
    let stream = resolve(address, port).and_then(|a| TcpStream::connect_timeout(&a, timeout).map_err(|e| e.to_string()));
    let mut stream = match stream {
        Ok(s) => s,
        Err(e) => {
            push(id, "error", serde_json::json!({ "errMsg": format!("connect:fail {e}") }));
            push(id, "close", serde_json::json!({}));
            return;
        }
    };
    stream.set_read_timeout(Some(Duration::from_millis(15))).ok();
    stream.set_nodelay(true).ok();
    let (remote, local) = (stream.peer_addr().ok(), stream.local_addr().ok());
    push(id, "connect", serde_json::json!({}));
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        loop {
            match rx.try_recv() {
                Ok(Cmd::Write(data)) => {
                    if let Err(e) = stream.write_all(&data) {
                        push(id, "error", serde_json::json!({ "errMsg": format!("write:fail {e}") }));
                    }
                }
                Ok(Cmd::Close) | Err(TryRecvError::Disconnected) => {
                    stream.shutdown(std::net::Shutdown::Both).ok();
                    push(id, "close", serde_json::json!({}));
                    return;
                }
                Ok(_) => {}
                Err(TryRecvError::Empty) => break,
            }
        }
        match stream.read(&mut buf) {
            Ok(0) => {
                push(id, "close", serde_json::json!({}));
                return;
            }
            Ok(n) => {
                crate::bin_stash::stash(crate::bin_stash::NET, id, buf[..n].to_vec());
                let mut ev = serde_json::json!({ "bin": true });
                if let Some(r) = &remote {
                    ev["remoteInfo"] = info(r, Some(n));
                }
                if let Some(l) = &local {
                    ev["localInfo"] = info(l, None);
                }
                push(id, "message", ev);
            }
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => {}
            Err(e) => {
                push(id, "error", serde_json::json!({ "errMsg": e.to_string() }));
                push(id, "close", serde_json::json!({}));
                return;
            }
        }
    }
}

#[cfg(test)] // 老的 base64 入口：JS 已改走 *_bytes（bin_stash.rs），只留给单测
pub fn tcp_write(id: u32, data_b64: &str) -> Result<(), String> {
    tcp_write_bytes(id, unb64(data_b64)?)
}

pub fn tcp_write_bytes(id: u32, bytes: Vec<u8>) -> Result<(), String> {
    command(id, Cmd::Write(bytes), "write")
}

pub fn close(id: u32) -> Result<(), String> {
    command(id, Cmd::Close, "close")
}

// ───────────────────────── UDP ─────────────────────────

/// `UDPSocket.bind`：同步绑定并返回实际端口；之后后台线程收包
pub fn udp_bind(id: u32, port: u16, ipv6: bool) -> Result<u16, String> {
    if sockets().lock().unwrap().contains_key(&id) {
        return Err("bind:fail socket is already bound".into());
    }
    own(id);
    let sock = UdpSocket::bind(if ipv6 { format!("[::]:{port}") } else { format!("0.0.0.0:{port}") }).map_err(|e| format!("bind:fail {e}"))?;
    let local = sock.local_addr().map_err(|e| format!("bind:fail {e}"))?;
    sock.set_read_timeout(Some(Duration::from_millis(15))).ok();
    let (tx, rx) = channel();
    sockets().lock().unwrap().insert(id, tx);
    push(id, "listening", serde_json::json!({}));
    std::thread::spawn(move || {
        udp_run(id, sock, local, rx);
        sockets().lock().unwrap().remove(&id);
    });
    Ok(local.port())
}

fn udp_run(id: u32, sock: UdpSocket, local: SocketAddr, rx: Receiver<Cmd>) {
    let mut buf = vec![0u8; 64 * 1024];
    let mut connected = false;
    loop {
        loop {
            match rx.try_recv() {
                Ok(Cmd::SendTo(data, addr, port, broadcast)) => {
                    if broadcast {
                        sock.set_broadcast(true).ok();
                    }
                    let r = resolve(&addr, port).and_then(|a| sock.send_to(&data, a).map_err(|e| e.to_string()));
                    if let Err(e) = r {
                        push(id, "error", serde_json::json!({ "errMsg": format!("send:fail {e}") }));
                    }
                }
                Ok(Cmd::Connect(addr, port)) => match resolve(&addr, port).and_then(|a| sock.connect(a).map_err(|e| e.to_string())) {
                    Ok(()) => connected = true,
                    Err(e) => push(id, "error", serde_json::json!({ "errMsg": format!("connect:fail {e}") })),
                },
                Ok(Cmd::Write(data)) => {
                    let r = if connected { sock.send(&data).map(|_| ()).map_err(|e| e.to_string()) } else { Err("socket is not connected".into()) };
                    if let Err(e) = r {
                        push(id, "error", serde_json::json!({ "errMsg": format!("write:fail {e}") }));
                    }
                }
                Ok(Cmd::Ttl(ttl)) => {
                    sock.set_ttl(ttl).ok();
                }
                Ok(Cmd::Close) | Err(TryRecvError::Disconnected) => {
                    push(id, "close", serde_json::json!({}));
                    return;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        match sock.recv_from(&mut buf) {
            Ok((n, from)) => push(
                id,
                "message",
                {
                    crate::bin_stash::stash(crate::bin_stash::NET, id, buf[..n].to_vec());
                    serde_json::json!({ "bin": true, "remoteInfo": info(&from, Some(n)), "localInfo": info(&local, None) })
                },
            ),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => {}
            // Windows 上对端端口不可达会让下一次 recv 报 ConnectionReset：报错但不关
            Err(e) => push(id, "error", serde_json::json!({ "errMsg": e.to_string() })),
        }
    }
}

#[cfg(test)] // 老的 base64 入口：JS 已改走 *_bytes（bin_stash.rs），只留给单测
pub fn udp_send(id: u32, address: &str, port: u16, data_b64: &str, broadcast: bool) -> Result<(), String> {
    udp_send_bytes(id, address, port, unb64(data_b64)?, broadcast)
}

pub fn udp_send_bytes(id: u32, address: &str, port: u16, bytes: Vec<u8>, broadcast: bool) -> Result<(), String> {
    command(id, Cmd::SendTo(bytes, address.to_string(), port, broadcast), "send")
}

pub fn udp_connect(id: u32, address: &str, port: u16) -> Result<(), String> {
    command(id, Cmd::Connect(address.to_string(), port), "connect")
}

#[cfg(test)] // 老的 base64 入口：JS 已改走 *_bytes（bin_stash.rs），只留给单测
pub fn udp_write(id: u32, data_b64: &str) -> Result<(), String> {
    udp_write_bytes(id, unb64(data_b64)?)
}

pub fn udp_write_bytes(id: u32, bytes: Vec<u8>) -> Result<(), String> {
    command(id, Cmd::Write(bytes), "write")
}

pub fn udp_set_ttl(id: u32, ttl: u32) -> Result<(), String> {
    command(id, Cmd::Ttl(ttl), "setTTL")
}

/// `wx.getLocalIPAddress`：向一个公网地址「连接」UDP（不发包）取本机出口地址
pub fn local_ip() -> Result<String, String> {
    let s = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    s.connect("8.8.8.8:80").map_err(|e| e.to_string())?;
    Ok(s.local_addr().map_err(|e| e.to_string())?.ip().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// 只摘走自己要的那一条（测试并行跑，别的用例的事件得留在队列里）
    fn wait_for(id: u32, ty: &str, ms: u64) -> Option<String> {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end {
            {
                let mut q = events().lock().unwrap();
                if let Some(pos) = q.iter().position(|(i, (t, _))| *i == id && *t == ty) {
                    return Some(q.remove(pos).1 .1);
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn udp_echo_through_loopback() {
        let a = udp_bind(9001, 0, false).expect("bind a");
        let b = udp_bind(9002, 0, false).expect("bind b");
        assert!(a > 0 && b > 0 && a != b);
        udp_send(9001, "127.0.0.1", b, &b64(b"ping"), false).unwrap();
        let msg = wait_for(9002, "message", 2000).expect("b 收到");
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["bin"], true);
        assert_eq!(crate::bin_stash::take(crate::bin_stash::NET, 9002).unwrap(), b"ping");
        assert_eq!(v["remoteInfo"]["port"], a);
        assert_eq!(v["remoteInfo"]["size"], 4);
        close(9001).unwrap();
        close(9002).unwrap();
    }

    #[test]
    fn tcp_connect_write_and_peer_close() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut b = [0u8; 5];
            s.read_exact(&mut b).unwrap();
            s.write_all(b"world").unwrap();
            // 函数结束时关闭：客户端应收到 close
        });
        tcp_connect(9101, "127.0.0.1", port, 2000).unwrap();
        assert!(wait_for(9101, "connect", 2000).is_some(), "要先 connect");
        tcp_write(9101, &b64(b"hello")).unwrap();
        let msg = wait_for(9101, "message", 2000).expect("收到回包");
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["bin"], true);
        assert_eq!(crate::bin_stash::take(crate::bin_stash::NET, 9101).unwrap(), b"world");
        server.join().unwrap();
        assert!(wait_for(9101, "close", 2000).is_some(), "对端关了要报 close");
    }

    #[test]
    fn tcp_refused_reports_error_then_close() {
        // 绑一个端口再放掉，大概率没人听
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        tcp_connect(9201, "127.0.0.1", port, 1000).unwrap();
        let e = wait_for(9201, "error", 3000).expect("连不上要报 error");
        assert!(e.contains("connect:fail"), "{e}");
    }
}
