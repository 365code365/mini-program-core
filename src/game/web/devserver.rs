//! 高性能模式的桌面调试服务：用 HTTP 扮演 iOS 宿主（`MiniWebGameView.swift`）的那一层传话，让同一份页面在桌面 Chrome 里跑。
//! 测试（tests/game_cases/web.rs、examples/web_game_check.rs）与手工调试（Chrome 开发者工具看页面）都用它。
//!
//! | 请求 | iOS 宿主里对应的 |
//! |---|---|
//! | `GET <路径>` | WKURLSchemeHandler → [`WebGame::resource`] |
//! | `POST /__wg/sync`（消息体） | WKUIDelegate 的 prompt → [`WebGame::call_message`] |
//! | `POST /__wg/post`（一行一条） | WKScriptMessageHandler → [`WebGame::post_message`] |
//! | `GET /__wg/poll` | 宿主定时 `evaluateJavaScript(take_js)` |
//!
//! 宿主事件（[`WebGame::take_event`]）交给调用方给的 `on_event`：日志、hostCall、退出……在测试里就是被断言的东西。
//! 只监听 127.0.0.1。

use super::WebGame;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

pub type Shared = Arc<Mutex<WebGame>>;

pub struct DevServer {
    pub port: u16,
    pub game: Shared,
}

/// 页面的异步消息按批编号（`/__wg/post?seq=N`，fetch 发、可能乱序到）：按编号顺序落地；
/// 同步调用（`/__wg/sync?after=N`）等第 N 批落地之后才处理（与 iOS 上「异步的先到」一致）
#[derive(Default)]
struct Order {
    applied: u64,
    early: std::collections::BTreeMap<u64, String>,
}

static ORDER: Mutex<Option<Order>> = Mutex::new(None);
static ORDER_CV: std::sync::Condvar = std::sync::Condvar::new();

fn query_num(path: &str, key: &str) -> Option<u64> {
    path.split_once('?')?.1.split('&').find_map(|kv| kv.strip_prefix(key)?.strip_prefix('=')?.parse().ok())
}

impl DevServer {
    /// 起服务（后台线程）。`on_event` 在服务线程上调，每条宿主事件一次（JSON 串）
    pub fn start(game: WebGame, on_event: impl Fn(&str) + Send + Sync + 'static) -> std::io::Result<Self> {
        // 一个进程里同一时刻只有一个调试服务（测试串行）：编号从头数
        *ORDER.lock().unwrap_or_else(|e| e.into_inner()) = Some(Order::default());
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let game: Shared = Arc::new(Mutex::new(game));
        let g = game.clone();
        let on_event = Arc::new(on_event);
        std::thread::Builder::new().name("wg-devserver".into()).spawn(move || {
            for conn in listener.incoming().flatten() {
                let (g, ev) = (g.clone(), on_event.clone());
                std::thread::spawn(move || {
                    let _ = handle(conn, &g, &*ev);
                });
            }
        })?;
        Ok(Self { port, game })
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/__wg.html", self.port)
    }
}

fn handle(mut s: TcpStream, g: &Shared, on_event: &dyn Fn(&str)) -> std::io::Result<()> {
    s.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
    let mut r = BufReader::new(s.try_clone()?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or("").to_string(), parts.next().unwrap_or("/").to_string());
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h == "\r\n" || h == "\n" {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                len = v.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    let body = String::from_utf8_lossy(&body).into_owned();
    let route = path.split('?').next().unwrap_or("").to_string();
    if method == "POST" && route == "/__wg/post" {
        // 按编号排队：前面的批还没到就先存着
        let n = query_num(&path, "seq").unwrap_or(0);
        let mut o = ORDER.lock().unwrap_or_else(|e| e.into_inner());
        let o = o.get_or_insert_with(Default::default);
        o.early.insert(n, body.clone());
        while let Some(b) = o.early.remove(&(o.applied + 1)) {
            let mut game = g.lock().unwrap_or_else(|e| e.into_inner());
            for l in b.lines().filter(|l| !l.is_empty()) {
                game.post_message(l);
            }
            o.applied += 1;
        }
        ORDER_CV.notify_all();
    }
    if method == "POST" && route == "/__wg/sync" {
        let after = query_num(&path, "after").unwrap_or(0);
        let mut o = ORDER.lock().unwrap_or_else(|e| e.into_inner());
        let t0 = std::time::Instant::now();
        while o.get_or_insert_with(Default::default).applied < after && t0.elapsed().as_secs() < 5 {
            o = ORDER_CV.wait_timeout(o, std::time::Duration::from_millis(50)).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
    let (status, mime, bytes) = {
        let mut game = g.lock().unwrap_or_else(|e| e.into_inner());
        let out = match (method.as_str(), route.as_str()) {
            ("POST", "/__wg/sync") => (200, "text/plain; charset=utf-8", game.call_message(&body).into_bytes()),
            ("POST", "/__wg/post") => (200, "text/plain", Vec::new()),
            ("GET", "/__wg/poll") => (200, "application/javascript; charset=utf-8", game.take_js().unwrap_or_default().into_bytes()),
            ("GET", _) => match crate::game::web::unlocked_resource(&path).unwrap_or_else(|| game.resource(&path)) {
                Some(res) => (200, res.mime, res.bytes),
                None => (404, "text/plain", b"not found".to_vec()),
            },
            _ => (405, "text/plain", Vec::new()),
        };
        while let Some(e) = game.take_event() {
            on_event(&e);
        }
        out
    };
    let head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        if status == 200 { "OK" } else { "ERR" },
        bytes.len()
    );
    s.write_all(head.as_bytes())?;
    s.write_all(&bytes)?;
    s.flush()
}
