//! 驱动一个 headless Chrome（Chrome DevTools Protocol）：桌面上测高性能模式的页面 —— 打开、执行 JS、发触摸、截图。
//! 只给测试与调试工具用（tests/game_cases/web.rs、examples/web_game_check.rs）。
//!
//! 为什么不用现成的 crate：只要五个命令（navigate / evaluate / 触摸 / 截图 / 设备尺寸），tungstenite 已经在依赖里，
//! 不为测试再引一棵依赖树。找不到 Chrome 时 [`Chrome::launch`] 返回 Err，调用方跳过（并打一行说明）。

use serde_json::{json, Value};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tungstenite::{Message, WebSocket};

pub struct Chrome {
    proc: Proc,
    ws: WebSocket<TcpStream>,
    next: u64,
    profile: PathBuf,
}

/// Chrome 的可执行文件：`MINI_CHROME` 优先，其次 macOS 的默认位置与 PATH 里常见的名字
pub fn chrome_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("MINI_CHROME") {
        return Some(PathBuf::from(p));
    }
    let mac = PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
    if mac.is_file() {
        return Some(mac);
    }
    ["google-chrome", "chromium", "chromium-browser"].iter().find_map(|n| {
        let out = Command::new("which").arg(n).output().ok()?;
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!p.is_empty()).then(|| PathBuf::from(p))
    })
}

/// 起起来的 Chrome，外面套一层看门狗 shell：shell 读自己的标准输入，我们这边一退（正常放掉、panic、被 SIGKILL）
/// 管道就断，shell 杀掉 Chrome 再退。从前直接起 Chrome、只在 `Drop` 里 kill：起到一半出错（20 秒没起来）的那份没人管，
/// 测试进程被超时杀掉的那份也没人管 —— 2026-11-06 机器上攒了 275 个孤儿 Chrome，后面的测试全被它们拖慢、超时
struct Proc {
    shell: Child,
    stdin: Option<std::process::ChildStdin>,
}

impl Drop for Proc {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(3) {
            if matches!(self.shell.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.shell.kill();
        let _ = self.shell.wait();
    }
}

const WATCHDOG: &str = r#""$@" & c=$!; cat >/dev/null; kill $c 2>/dev/null; sleep 1; kill -9 $c 2>/dev/null; wait $c 2>/dev/null; exit 0"#;

impl Chrome {
    /// 起一个 headless Chrome，视口 `w × h`（CSS 像素）、`dpr` 倍，打开触摸模拟（移动端页面）
    pub fn launch(w: u32, h: u32, dpr: f32) -> Result<Self, String> {
        let exe = chrome_path().ok_or("找不到 Chrome（设 MINI_CHROME=<路径>）")?;
        let profile = std::env::temp_dir().join(format!("wg-chrome-{}-{}", std::process::id(), rand_tag()));
        std::fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
        let mut shell = Command::new("/bin/sh")
            .args(["-c", WATCHDOG, "sh"])
            .arg(exe)
            .args([
                "--headless=new", "--remote-debugging-port=0", "--no-first-run", "--no-default-browser-check",
                "--autoplay-policy=no-user-gesture-required", "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
                "--hide-scrollbars", "--mute-audio",
                // 软件合成：机器忙（别的会话在编译）时 GPU 合成经 SwiftShader 只剩 3~5fps，rAF 节拍不准、计时类断言全挂；
                // 软件合成稳在 60（WebGL 照样走 SwiftShader，2D 画布走软件光栅）
                "--disable-gpu-compositing",
            ])
            .args(std::env::var("MINI_CHROME_FLAGS").map(|f| f.split_whitespace().map(str::to_string).collect::<Vec<_>>()).unwrap_or_default())
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(format!("--window-size={w},{h}"))
            .arg("about:blank")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("起 Chrome 失败：{e}"))?;
        // 从这里起任何一条出错的路都会放掉 proc，看门狗跟着杀掉 Chrome
        let proc = Proc { stdin: shell.stdin.take(), shell };
        // DevToolsActivePort：第一行端口
        let port_file = profile.join("DevToolsActivePort");
        let t0 = Instant::now();
        let port = loop {
            if let Some(p) = std::fs::read_to_string(&port_file).ok().and_then(|s| s.lines().next().and_then(|l| l.trim().parse::<u16>().ok())) {
                break p;
            }
            if t0.elapsed() > Duration::from_secs(20) {
                return Err("Chrome 20 秒没起来".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let list = http_get(port, "/json/list")?;
        let pages: Value = serde_json::from_str(&list).map_err(|e| e.to_string())?;
        let url = pages
            .as_array()
            .and_then(|a| a.iter().find(|p| p["type"] == "page"))
            .and_then(|p| p["webSocketDebuggerUrl"].as_str())
            .ok_or("Chrome 没有页面")?
            .to_string();
        let stream = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
        let (ws, _) = tungstenite::client(url.as_str(), stream).map_err(|e| e.to_string())?;
        let mut c = Self { proc, ws, next: 0, profile };
        c.send("Emulation.setDeviceMetricsOverride", json!({ "width": w, "height": h, "deviceScaleFactor": dpr, "mobile": true }))?;
        c.send("Emulation.setTouchEmulationEnabled", json!({ "enabled": true, "maxTouchPoints": 10 }))?;
        Ok(c)
    }

    /// 发一条命令、等它的回包（中间的事件丢掉）
    pub fn send(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        self.ws.send(Message::text(json!({ "id": id, "method": method, "params": params }).to_string())).map_err(|e| e.to_string())?;
        let t0 = Instant::now();
        loop {
            let m = self.ws.read().map_err(|e| e.to_string())?;
            if let Message::Text(t) = m {
                let v: Value = serde_json::from_str(t.as_str()).unwrap_or(Value::Null);
                if v["id"].as_u64() == Some(id) {
                    if let Some(e) = v.get("error") {
                        return Err(format!("{method}: {e}"));
                    }
                    return Ok(v["result"].clone());
                }
            }
            if t0.elapsed() > Duration::from_secs(60) {
                return Err(format!("{method} 60 秒没回"));
            }
        }
    }

    pub fn navigate(&mut self, url: &str) -> Result<(), String> {
        self.send("Page.navigate", json!({ "url": url })).map(|_| ())
    }

    /// 在页面里求值（返回值按 JSON 带回）
    pub fn eval(&mut self, js: &str) -> Result<Value, String> {
        let r = self.send("Runtime.evaluate", json!({ "expression": js, "returnByValue": true, "awaitPromise": true }))?;
        if let Some(e) = r.get("exceptionDetails") {
            return Err(format!("页面里抛错：{}", e["exception"]["description"].as_str().unwrap_or(&e.to_string())));
        }
        Ok(r["result"]["value"].clone())
    }

    /// 触摸：`kind` = touchStart / touchMove / touchEnd / touchCancel；`points` = [(x, y, id)]（CSS 像素）
    pub fn touch(&mut self, kind: &str, points: &[(f32, f32, u32)]) -> Result<(), String> {
        let pts: Vec<Value> = points.iter().map(|(x, y, id)| json!({ "x": x, "y": y, "id": id })).collect();
        self.send("Input.dispatchTouchEvent", json!({ "type": kind, "touchPoints": pts })).map(|_| ())
    }

    /// 点一下（按下、抬起）
    pub fn tap(&mut self, x: f32, y: f32) -> Result<(), String> {
        self.touch("touchStart", &[(x, y, 1)])?;
        self.touch("touchEnd", &[])
    }

    /// 截图（PNG 字节）
    pub fn screenshot(&mut self) -> Result<Vec<u8>, String> {
        let r = self.send("Page.captureScreenshot", json!({ "format": "png" }))?;
        crate::wxfs::codec::unb64(r["data"].as_str().unwrap_or("")).ok_or_else(|| "截图解不开".into())
    }
}

impl Drop for Chrome {
    fn drop(&mut self) {
        // 先停 Chrome（Proc 的 Drop 在这个函数返回之后才跑，profile 目录要等 Chrome 退了再删）
        drop(self.proc.stdin.take());
        let _ = self.proc.shell.wait();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

fn http_get(port: u16, path: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    let mut s = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(Duration::from_secs(10))).ok();
    // 1.1 + Content-Length（Chrome 的 DevTools HTTP 对 1.0 不回包体；1.1 的连接它不主动关，按长度读完即止）
    write!(s, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = s.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&buf);
        if let Some((head, body)) = text.split_once("\r\n\r\n") {
            let len = head.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok()));
            if len.is_some_and(|l| body.len() >= l) {
                return Ok(body.to_string());
            }
        }
    }
    let out = String::from_utf8_lossy(&buf).into_owned();
    Ok(out.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default())
}

fn rand_tag() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0)
}
