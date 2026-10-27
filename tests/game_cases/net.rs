//! GT-N1~N4（game-doc/05 §3）：本地 HTTP 服务给一份固定种子的 5MB 二进制，
//! 游戏侧按 arraybuffer / 分块 / 下载三种方式拿，在 JS 里算 FNV-1a 校验和，与 Rust 这边算的比。
use super::*;

fn payload() -> Vec<u8> {
    let mut x = 0x1234_5678u32;
    (0..5 * 1024 * 1024)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 24) as u8
        })
        .collect()
}

fn fnv(b: &[u8]) -> u32 {
    b.iter().fold(0x811c9dc5u32, |h, &c| (h ^ c as u32).wrapping_mul(16777619))
}

/// GET /bin（Content-Length + X-Test 头）；其它 404
fn serve(body: Vec<u8>) -> u16 {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let body = std::sync::Arc::new(body);
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            let body = body.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                let ok = String::from_utf8_lossy(&buf[..n]).starts_with("GET /bin ");
                if ok {
                    let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nX-Test: yes\r\nConnection: close\r\n\r\n", body.len());
                    // 分几次写，让分块与进度真的分几次到
                    for c in body.chunks(256 * 1024) {
                        let _ = s.write_all(c);
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                } else {
                    let _ = write!(s, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                }
            });
        }
    });
    port
}

const JS: &str = r#"
var T = require('_lib/t.js');
function fnv(buf) { var a = new Uint8Array(buf), h = 0x811c9dc5; for (var i = 0; i < a.length; i++) { h ^= a[i]; h = Math.imul(h, 16777619) >>> 0; } return h >>> 0; }
var base = 'http://127.0.0.1:' + T.q.port;
var order = [];
// N1 + N3：arraybuffer，headersReceived 在 success 之前
var t1 = wx.request({ url: base + '/bin', responseType: 'arraybuffer',
  success: function (r) { order.push('success'); T.ev('n1', { isAB: r.data instanceof ArrayBuffer, len: r.data.byteLength, h: fnv(r.data), status: r.statusCode, order: order }); } });
t1.onHeadersReceived(function (r) { order.push('headers'); T.ev('n3', { x: r.header['x-test'], status: r.statusCode }); });
// N2：分块拼接
var parts = [], total = 0;
var t2 = wx.request({ url: base + '/bin', responseType: 'arraybuffer', enableChunked: true,
  success: function (r) {
    var all = new Uint8Array(total), o = 0;
    parts.forEach(function (p) { all.set(new Uint8Array(p), o); o += p.byteLength; });
    T.ev('n2', { chunks: parts.length, len: total, h: fnv(all.buffer), dataLen: r.data.byteLength });
  } });
t2.onChunkReceived(function (r) { parts.push(r.data); total += r.data.byteLength; });
// N4：下载进度单调、最后 100
var prog = [];
var t3 = wx.downloadFile({ url: base + '/bin', success: function (r) {
  var buf = wx.getFileSystemManager().readFileSync(r.tempFilePath);
  T.ev('n4', { prog: prog, h: fnv(buf), status: r.statusCode });
} });
t3.onProgressUpdate(function (p) { prog.push(p.progress); });
T.ready('net');
"#;

#[test]
fn n1_n4_二进制_分块_响应头_进度() {
    let _g = serial();
    let body = payload();
    let want = fnv(&body) as u64;
    let q: &'static str = Box::leak(format!("port={}", serve(body)).into_boxed_str());
    let mut h = GameHarness::open_inline("net", JS, Opts { query: q, ..Opts::default() });
    for k in ["T:EV n1", "T:EV n2", "T:EV n4"] {
        h.until(k, 2000);
    }
    h.assert_no_fail();
    let n1 = h.events("n1").pop().unwrap();
    assert_eq!((n1["isAB"].as_bool(), n1["len"].as_u64(), n1["status"].as_u64()), (Some(true), Some(5 * 1024 * 1024), Some(200)));
    assert_eq!(n1["h"].as_u64(), Some(want), "arraybuffer 内容与源不同");
    assert_eq!(n1["order"], serde_json::json!(["headers", "success"]), "headersReceived 要在 success 之前");
    let n3 = h.events("n3").pop().unwrap();
    assert_eq!((n3["x"].as_str(), n3["status"].as_u64()), (Some("yes"), Some(200)));
    let n2 = h.events("n2").pop().unwrap();
    assert!(n2["chunks"].as_u64().unwrap() >= 2, "块数 ≥ 2：{n2}");
    assert_eq!((n2["len"].as_u64(), n2["h"].as_u64()), (Some(5 * 1024 * 1024), Some(want)), "分块拼接后与源不同");
    assert_eq!(n2["dataLen"].as_u64(), Some(5 * 1024 * 1024), "分块模式下 success 的 data 仍是全量");
    let n4 = h.events("n4").pop().unwrap();
    let prog: Vec<u64> = n4["prog"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    assert!(!prog.is_empty() && prog.windows(2).all(|w| w[0] <= w[1]), "进度单调不减：{prog:?}");
    assert_eq!(*prog.last().unwrap(), 100, "最后一次是 100");
    assert_eq!(n4["h"].as_u64(), Some(want), "下载落盘的内容与源不同");
}

/// WebSocket 回声 + TCP 回声（原样回）
fn echo_servers() -> (u16, u16) {
    let ws = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let ports = (ws.local_addr().unwrap().port(), tcp.local_addr().unwrap().port());
    std::thread::spawn(move || {
        for s in ws.incoming().flatten() {
            std::thread::spawn(move || {
                let Ok(mut ws) = tungstenite::accept(s) else { return };
                while let Ok(m) = ws.read() {
                    if (m.is_binary() || m.is_text()) && ws.send(m).is_err() {
                        break;
                    }
                }
            });
        }
    });
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        for mut s in tcp.incoming().flatten() {
            std::thread::spawn(move || {
                let mut buf = [0u8; 65536];
                while let Ok(n) = s.read(&mut buf) {
                    if n == 0 || s.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            });
        }
    });
    ports
}

/// GT-N5：WebSocket 1MB 二进制帧、TCP 64KB 往返内容相同，且不经 base64（JS 侧的 __b64encode / __b64decode 一次都没被调）
#[test]
fn n5_长连接二进制直通() {
    let _g = serial();
    let (wsp, tcpp) = echo_servers();
    let js = format!(r#"
var T = require('_lib/t.js');
function fnv(buf) {{ var a = new Uint8Array(buf), h = 0x811c9dc5; for (var i = 0; i < a.length; i++) {{ h ^= a[i]; h = Math.imul(h, 16777619) >>> 0; }} return h >>> 0; }}
var b64calls = 0, e0 = __b64encode, d0 = __b64decode;
globalThis.__b64encode = function (x) {{ b64calls++; return e0(x); }};
globalThis.__b64decode = function (x) {{ b64calls++; return d0(x); }};
var big = new Uint8Array(1024 * 1024); for (var i = 0; i < big.length; i++) big[i] = (i * 31 + 7) & 255;
var want = fnv(big.buffer);
var ws = wx.connectSocket({{ url: 'ws://127.0.0.1:{wsp}/' }});
ws.onOpen(function () {{ ws.send({{ data: big.buffer }}); }});
ws.onMessage(function (r) {{ T.ev('ws', {{ isAB: r.data instanceof ArrayBuffer, len: r.data.byteLength, ok: fnv(r.data) === want, b64: b64calls }}); ws.close({{}}); }});
var small = new Uint8Array(65536); for (var j = 0; j < small.length; j++) small[j] = (j * 7) & 255;
var got = [], gotLen = 0;
var tcp = wx.createTCPSocket();
tcp.onConnect(function () {{ tcp.write(small); }});
tcp.onMessage(function (r) {{
  got.push(r.message); gotLen += r.message.byteLength;
  if (gotLen >= small.length) {{
    var all = new Uint8Array(gotLen), o = 0; got.forEach(function (g) {{ all.set(new Uint8Array(g), o); o += g.byteLength; }});
    T.ev('tcp', {{ len: gotLen, ok: fnv(all.buffer) === fnv(small.buffer), b64: b64calls }}); tcp.close();
  }}
}});
tcp.connect({{ address: '127.0.0.1', port: {tcpp} }});
T.ready('n5');
"#);
    let js: &'static str = Box::leak(js.into_boxed_str());
    let mut h = GameHarness::open_inline("n5", js, Opts::default());
    h.until("T:EV ws", 2000);
    h.until("T:EV tcp", 2000);
    h.assert_no_fail();
    let ws = h.events("ws").pop().unwrap();
    assert_eq!((ws["isAB"].as_bool(), ws["len"].as_u64(), ws["ok"].as_bool()), (Some(true), Some(1024 * 1024), Some(true)), "{ws}");
    assert_eq!(ws["b64"], 0, "WebSocket 二进制不该经过 base64");
    let tcp = h.events("tcp").pop().unwrap();
    assert_eq!((tcp["len"].as_u64(), tcp["ok"].as_bool()), (Some(65536), Some(true)), "{tcp}");
    assert_eq!(tcp["b64"], 0, "TCP 二进制不该经过 base64");
}

/// GT-N6：没给 timeout 时按 game.json 的 networkTimeout.request（300ms）超时，文案 `request:fail timeout`；
/// 显式给 timeout 的照自己的；服务端 800ms 才回
#[test]
fn n6_networktimeout() {
    let _g = serial();
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            std::thread::spawn(move || {
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                std::thread::sleep(std::time::Duration::from_millis(800));
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
            });
        }
    });
    let js = format!(r#"
var T = require('_lib/t.js');
var t0 = Date.now();
wx.request({{ url: 'http://127.0.0.1:{port}/a', success: function () {{ T.ev('def', {{ ok: true }}); }}, fail: function (e) {{ T.ev('def', {{ ok: false, msg: e.errMsg, ms: Date.now() - t0 }}); }} }});
wx.request({{ url: 'http://127.0.0.1:{port}/b', timeout: 3000, success: function (r) {{ T.ev('own', {{ ok: true, data: r.data }}); }}, fail: function (e) {{ T.ev('own', {{ ok: false, msg: e.errMsg }}); }} }});
T.ev('nt', globalThis.__networkTimeout);
"#);
    let js: &'static str = Box::leak(js.into_boxed_str());
    let mut h = GameHarness::open_inline_with("n6", r#"{"networkTimeout":{"request":300}}"#, js, Opts::default());
    h.until("T:EV def", 2000);
    h.until("T:EV own", 2000);
    assert_eq!(h.events("nt")[0]["request"], 300);
    let d = h.events("def").pop().unwrap();
    assert_eq!((d["ok"].as_bool(), d["msg"].as_str()), (Some(false), Some("request:fail timeout")), "{d}");
    assert!(d["ms"].as_u64().unwrap() < 780, "按 300ms 超时，不是等到服务端回：{d}");
    let o = h.events("own").pop().unwrap();
    assert_eq!((o["ok"].as_bool(), o["data"].as_str()), (Some(true), Some("ok")), "显式 timeout 优先：{o}");
}
