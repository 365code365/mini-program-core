//! 浏览器调试预览服务器
//!
//! 把小程序页面渲染成图片，通过内置 HTTP 服务在浏览器打开，
//! 可直接在页面上点击操作 UI（点击 -> 命中事件 -> 调用页面方法 -> setData -> 重渲染）。
//!
//! 运行：
//!   cargo run --bin mini-devserver                      # 默认加载 sample-app/pages/index
//!   cargo run --bin mini-devserver <页面目录> [端口]     # 指定页面目录与端口
//! 然后浏览器打开终端输出的 http://127.0.0.1:9000
//!
//! 仅使用标准库网络（无额外依赖）。

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::parser::wxml::WxmlNode;
use mini_render::renderer::WxmlRenderer;
use mini_render::ui::interaction::InteractionManager;
use mini_render::{Canvas, Color};
use serde_json::json;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

const W: u32 = 375;
const H: u32 = 667;
const SCALE: f32 = 2.0;

struct Preview {
    app: MiniApp,
    renderer: WxmlRenderer,
    nodes: Vec<WxmlNode>,
    interaction: InteractionManager,
    bg: u32,
}

impl Preview {
    fn load(dir: &str) -> Result<Self, String> {
        let js = read_page(dir, "js").unwrap_or_default();
        let wxml_src = read_page(dir, "wxml").ok_or_else(|| format!("目录下未找到 .wxml: {}", dir))?;
        let wxss_src = read_page(dir, "wxss").unwrap_or_default();

        let mut app = MiniApp::new(W, H)?;
        app.init()?;
        if !js.is_empty() {
            app.load_script(&js)?;
            app.eval("if (__currentPage && __currentPage.onLoad) __currentPage.onLoad({})").ok();
            app.eval("if (__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
            app.eval("if (__currentPage && __currentPage.onReady) __currentPage.onReady()").ok();
        }

        let nodes = WxmlParser::new(&wxml_src).parse().map_err(|e| format!("WXML: {}", e))?;
        let ss = WxssParser::new(&wxss_src).parse().unwrap_or_default();
        let renderer = WxmlRenderer::new_with_scale(ss, W as f32, H as f32, SCALE);

        Ok(Self { app, renderer, nodes, interaction: InteractionManager::new(), bg: 0xF5F6F8 })
    }

    fn page_data(&self) -> serde_json::Value {
        self.app.eval("__getPageData()").ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| json!({}))
    }

    /// 渲染当前页面为 PNG 字节
    fn render_png(&mut self) -> Vec<u8> {
        let data = self.page_data();
        let mut canvas = Canvas::new((W as f32 * SCALE) as u32, (H as f32 * SCALE) as u32);
        canvas.clear(Color::from_hex(self.bg));
        self.renderer.render_with_interaction(&mut canvas, &self.nodes, &data, &mut self.interaction);
        encode_png(&canvas)
    }

    /// 处理一次点击（逻辑坐标），派发到 JS 事件处理函数
    fn tap(&mut self, x: f32, y: f32) {
        let chain = self.renderer.hit_test_bubble(x, y, "tap");
        for b in &chain {
            let data_json = serde_json::to_string(&b.data).unwrap_or_else(|_| "{}".into());
            let code = format!(
                "__callPageMethod('{}', {{ type:'tap', currentTarget:{{ dataset:{} }}, target:{{ dataset:{} }}, detail:{{}} }})",
                b.handler, data_json, data_json
            );
            self.app.eval(&code).ok();
        }
        // 泵一次事件循环，处理 setData/Promise/定时器等异步
        self.app.update().ok();
    }
}

/// 找到目录下第一个匹配扩展名的文件名
fn read_page(dir: &str, ext: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some(ext) {
            return std::fs::read_to_string(&p).ok();
        }
    }
    None
}

fn encode_png(canvas: &Canvas) -> Vec<u8> {
    let buf = image::RgbaImage::from_raw(canvas.width(), canvas.height(), canvas.to_rgba());
    let mut out = std::io::Cursor::new(Vec::new());
    if let Some(b) = buf {
        b.write_to(&mut out, image::ImageFormat::Png).ok();
    }
    out.into_inner()
}

const PAGE_HTML: &str = r#"<!doctype html><html lang="zh"><head><meta charset="utf-8">
<title>Mini Render 调试预览</title>
<style>
  body{margin:0;background:#20232a;color:#eee;font-family:-apple-system,system-ui,sans-serif;display:flex;flex-direction:column;align-items:center;padding:24px}
  h1{font-size:16px;font-weight:600;color:#9ad}
  .phone{width:375px;box-shadow:0 12px 40px rgba(0,0,0,.5);border-radius:20px;overflow:hidden;background:#000}
  img{display:block;width:375px;cursor:pointer}
  .bar{margin:14px 0;display:flex;gap:10px;align-items:center;font-size:13px;color:#aaa}
  button{background:#07c160;color:#fff;border:0;padding:8px 16px;border-radius:8px;cursor:pointer;font-size:13px}
  .hint{font-size:12px;color:#888;margin-top:8px}
</style></head><body>
<h1>Mini Render · 浏览器调试预览</h1>
<div class="phone"><img id="f" src="/frame.png"></div>
<div class="bar">
  <button onclick="refresh()">刷新</button>
  <label><input type="checkbox" id="auto" checked> 自动刷新(动画/定时器)</label>
  <span id="msg"></span>
</div>
<div class="hint">在上面的画面里点击即可操作 UI：点击会命中事件并调用页面方法，再自动重渲染。</div>
<script>
const img=document.getElementById('f'), msg=document.getElementById('msg');
function refresh(){ img.src='/frame.png?t='+Date.now(); }
img.addEventListener('click', async (e)=>{
  const x=Math.round(e.offsetX), y=Math.round(e.offsetY);
  msg.textContent='tap ('+x+','+y+')';
  await fetch('/tap?x='+x+'&y='+y);
  refresh();
});
setInterval(()=>{ if(document.getElementById('auto').checked) refresh(); }, 500);
</script></body></html>"#;

fn http_response(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut resp = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        status, content_type, body.len()
    ).into_bytes();
    resp.extend_from_slice(body);
    resp
}

fn handle(stream: &mut TcpStream, preview: &mut Preview) {
    let mut buf = [0u8; 2048];
    let n = match stream.read(&mut buf) { Ok(n) => n, Err(_) => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req.lines().next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");

    let out = if path.starts_with("/frame") {
        let png = preview.render_png();
        http_response("200 OK", "image/png", &png)
    } else if path.starts_with("/tap") {
        // 解析 ?x=..&y=..
        let (mut x, mut y) = (0.0f32, 0.0f32);
        if let Some(q) = path.split('?').nth(1) {
            for kv in q.split('&') {
                let mut it = kv.split('=');
                match (it.next(), it.next()) {
                    (Some("x"), Some(v)) => x = v.parse().unwrap_or(0.0),
                    (Some("y"), Some(v)) => y = v.parse().unwrap_or(0.0),
                    _ => {}
                }
            }
        }
        preview.tap(x, y);
        http_response("200 OK", "text/plain", b"ok")
    } else {
        http_response("200 OK", "text/html; charset=utf-8", PAGE_HTML.as_bytes())
    };
    let _ = stream.write_all(&out);
    let _ = stream.flush();
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).cloned().unwrap_or_else(|| "sample-app/pages/index".to_string());
    let port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(9000);

    println!("📂 加载页面目录: {}", dir);
    let mut preview = Preview::load(&dir)?;

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).map_err(|e| format!("无法监听 {}: {}", addr, e))?;
    println!("\n🌐 调试预览已启动，请在浏览器打开：");
    println!("   http://{}\n", addr);
    println!("（在画面上点击即可操作 UI；Ctrl+C 退出）");

    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => handle(&mut s, &mut preview),
            Err(_) => {}
        }
    }
    Ok(())
}
