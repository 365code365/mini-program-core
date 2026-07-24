//! 浏览器调试预览服务器（HTML 直渲染模式）
//!
//! 把小程序页面「编译」成 HTML + CSS 交给浏览器原生渲染：
//! - WXML(+data) → HTML，WXSS → CSS（rpx→px），见 `mini_render::transpile`
//! - 点击 → 服务端调用页面方法/setData/导航 → 返回新 HTML 片段 → 浏览器局部替换
//!
//! 相比逐帧渲染 PNG，HTML 渲染由浏览器原生完成，几乎零延迟、真实 DOM 可交互。
//!
//! 运行：cargo run --bin mini-devserver [小程序根目录] [端口]
//! 浏览器打开终端输出的 http://127.0.0.1:PORT

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::parser::wxml::WxmlNode;
use mini_render::compiler::html::{wxml_to_html, wxss_to_css, base_css};

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

const WIDTH: u32 = 375;

struct Preview {
    app: MiniApp,
    app_root: String,
    nodes: Vec<WxmlNode>,
    app_css: String,   // 全局 app.wxss（已转 css）
    page_css: String,  // 当前页 wxss（已转 css）
    stack: Vec<(String, HashMap<String, String>)>,
}

impl Preview {
    fn new(app_root: &str) -> Result<Self, String> {
        let mut app = MiniApp::new(WIDTH, 667)?;
        app.init()?;
        if let Ok(js) = std::fs::read_to_string(format!("{}/app.js", app_root)) {
            app.load_script(&js).ok();
            app.eval("if (typeof __app!=='undefined' && __app && __app.onLaunch) __app.onLaunch({})").ok();
        }
        let app_css = std::fs::read_to_string(format!("{}/app.wxss", app_root))
            .map(|s| wxss_to_css(&s)).unwrap_or_default();
        let start = std::fs::read_to_string(format!("{}/app.json", app_root)).ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("pages").and_then(|p| p.get(0)).and_then(|p| p.as_str().map(String::from)))
            .unwrap_or_else(|| "pages/index/index".to_string());
        let mut me = Self { app, app_root: app_root.to_string(), nodes: Vec::new(), app_css, page_css: String::new(), stack: Vec::new() };
        me.load_page(&start, &HashMap::new())?;
        me.stack.push((start, HashMap::new()));
        Ok(me)
    }

    fn load_page(&mut self, path: &str, query: &HashMap<String, String>) -> Result<(), String> {
        let prefix = format!("{}/{}", self.app_root, path);
        let js = std::fs::read_to_string(format!("{}.js", prefix)).unwrap_or_default();
        let wxml_src = std::fs::read_to_string(format!("{}.wxml", prefix))
            .map_err(|_| format!("未找到页面 WXML: {}.wxml", prefix))?;
        let wxss_src = std::fs::read_to_string(format!("{}.wxss", prefix)).unwrap_or_default();
        if !js.is_empty() {
            self.app.load_script(&js)?;
            let q = query_json(query);
            self.app.eval(&format!("if (__currentPage && __currentPage.onLoad) __currentPage.onLoad({})", q)).ok();
            self.app.eval("if (__currentPage && __currentPage.onShow) __currentPage.onShow()").ok();
            self.app.eval("if (__currentPage && __currentPage.onReady) __currentPage.onReady()").ok();
        }
        self.nodes = WxmlParser::new(&wxml_src).parse().map_err(|e| format!("WXML: {}", e))?;
        // 校验 wxss 可解析（出错则原样退化），再转 css
        let _ = WxssParser::new(&wxss_src).parse();
        self.page_css = wxss_to_css(&wxss_src);
        self.app.eval("__pendingNavigation = null;").ok();
        Ok(())
    }

    fn page_data(&self) -> serde_json::Value {
        self.app.eval("__getPageData()").ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    }

    fn body_html(&self) -> String {
        wxml_to_html(&self.nodes, &self.page_data())
    }

    fn css(&self) -> String {
        format!("{}\n{}", self.app_css, self.page_css)
    }

    /// 处理一次事件：调用页面方法 → 事件循环 → 导航。返回 (页面是否切换)
    fn handle_event(&mut self, handler: &str, event_json: &str) -> bool {
        let code = format!("__callPageMethod('{}', {})", handler, event_json);
        self.app.eval(&code).ok();
        self.app.update().ok();
        self.dispatch_nav()
    }

    fn dispatch_nav(&mut self) -> bool {
        let nav = self.app.eval("JSON.stringify(__pendingNavigation || null)").unwrap_or_else(|_| "null".into());
        if nav == "null" || nav.is_empty() { return false; }
        let v: serde_json::Value = match serde_json::from_str(&nav) { Ok(v) => v, Err(_) => return false };
        self.app.eval("__pendingNavigation = null;").ok();
        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let url = v.get("url").and_then(|u| u.as_str()).unwrap_or("");
        let (p, q) = parse_url(url);
        match typ {
            "navigateTo" => { if self.load_page(&p, &q).is_ok() { self.stack.push((p, q)); return true; } }
            "redirectTo" => { if self.load_page(&p, &q).is_ok() { self.stack.pop(); self.stack.push((p, q)); return true; } }
            "switchTab" | "reLaunch" => { if self.load_page(&p, &q).is_ok() { self.stack.clear(); self.stack.push((p, q)); return true; } }
            "navigateBack" => { self.back(); return true; }
            _ => {}
        }
        false
    }

    fn back(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            let (p, q) = self.stack.last().unwrap().clone();
            self.load_page(&p, &q).ok();
            true
        } else { false }
    }
}

fn parse_url(url: &str) -> (String, HashMap<String, String>) {
    let url = url.trim_start_matches('/');
    let mut parts = url.splitn(2, '?');
    let path = parts.next().unwrap_or("").to_string();
    let mut query = HashMap::new();
    if let Some(qs) = parts.next() {
        for kv in qs.split('&') {
            let mut it = kv.splitn(2, '=');
            if let (Some(k), Some(v)) = (it.next(), it.next()) { query.insert(k.to_string(), v.to_string()); }
        }
    }
    (path, query)
}

fn query_json(q: &HashMap<String, String>) -> String {
    let items: Vec<String> = q.iter()
        .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap(), serde_json::to_string(v).unwrap()))
        .collect();
    format!("{{{}}}", items.join(","))
}

fn full_page(preview: &Preview) -> String {
    format!(
        "<!doctype html><html lang=\"zh\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<title>Mini Render 预览</title>\
<style id=\"basecss\">{base}</style>\
<style>body{{margin:0;background:#20232a;display:flex;flex-direction:column;align-items:center;padding:20px;font-family:-apple-system,system-ui,sans-serif}}\
h1{{color:#9ad;font-size:15px}} .bar{{margin:12px;display:flex;gap:10px;align-items:center;color:#aaa;font-size:13px}}\
button.tb{{background:#07c160;color:#fff;border:0;padding:7px 15px;border-radius:8px;cursor:pointer}} button.tb.back{{background:#576b95}}\
#phone{{width:{w}px;height:812px;overflow:auto;background:#f5f6f8;box-shadow:0 12px 40px rgba(0,0,0,.5);border-radius:20px}}\
#app{{width:{w}px;min-height:100%;position:relative}}</style>\
<style id=\"pagecss\">{css}</style></head>\
<body><h1>Mini Render · HTML 直渲染预览（浏览器原生渲染，极速）</h1>\
<div id=\"phone\"><div id=\"app\">{body}</div></div>\
<div class=\"bar\"><button class=\"tb back\" id=\"backbtn\">← 返回</button><span id=\"msg\">点击页面即可操作（tap/输入/跳转/Toast），浏览器原生渲染</span></div>\
<script>{js}</script></body></html>",
        base = base_css(), w = WIDTH, css = preview.css(), body = preview.body_html(), js = RUNTIME_JS,
    )
}

const RUNTIME_JS: &str = r#"
const appEl=document.getElementById('app'), cssEl=document.getElementById('pagecss'), msg=document.getElementById('msg');
function ds(el){const o={};for(const a of el.attributes){if(a.name.startsWith('data-ds-'))o[a.name.slice(8)]=a.value;}return o;}
function evt(type,el,extra){return Object.assign({type:type,timeStamp:Date.now(),detail:(extra&&extra.detail)||{},currentTarget:{dataset:ds(el)},target:{dataset:ds(el)}},extra||{});}
async function fire(handler,event,swap){
  try{
    const r=await fetch('/event',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({handler:handler,event:event})});
    const j=await r.json();
    if(swap!==false){ if(j.css){cssEl.textContent=j.css;} appEl.innerHTML=j.html; }
  }catch(e){}
}
appEl.addEventListener('click',e=>{const el=e.target.closest('[data-tap]');if(!el)return;msg.textContent='tap → '+el.dataset.tap;fire(el.dataset.tap,evt('tap',el));});
appEl.addEventListener('change',e=>{const el=e.target.closest('[data-change]');if(!el)return;fire(el.dataset.change,evt('change',el,{value:el.value,detail:{value:el.value}}));});
appEl.addEventListener('input',e=>{const el=e.target.closest('[data-input]');if(!el)return;fire(el.dataset.input,evt('input',el,{value:el.value,detail:{value:el.value}}),false);});
appEl.addEventListener('keydown',e=>{if(e.key!=='Enter')return;const el=e.target.closest('[data-confirm]');if(!el)return;fire(el.dataset.confirm,evt('confirm',el,{value:el.value,detail:{value:el.value}}));});
document.getElementById('backbtn').onclick=async()=>{const r=await fetch('/back',{method:'POST'});const j=await r.json();if(j.css)cssEl.textContent=j.css;appEl.innerHTML=j.html;};
"#;

fn http(status: &str, ctype: &str, body: &[u8]) -> Vec<u8> {
    let mut r = format!("HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", status, ctype, body.len()).into_bytes();
    r.extend_from_slice(body);
    r
}

fn read_request(stream: &mut TcpStream) -> (String, String) {
    // 读取直到 header 结束，再按 Content-Length 读 body
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let mut header_end = None;
    while header_end.is_none() {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(p) = find_subslice(&buf, b"\r\n\r\n") { header_end = Some(p); }
                if buf.len() > 1 << 20 { break; }
            }
            Err(_) => break,
        }
    }
    let head_end = match header_end { Some(p) => p, None => return (String::new(), String::new()) };
    let header = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let content_len = header.lines()
        .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1)).and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_len {
        match stream.read(&mut tmp) { Ok(0) => break, Ok(n) => body.extend_from_slice(&tmp[..n]), Err(_) => break }
    }
    let line = header.lines().next().unwrap_or("").to_string();
    (line, String::from_utf8_lossy(&body).to_string())
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn handle(stream: &mut TcpStream, preview: &mut Preview) {
    let (line, body) = read_request(stream);
    let path = line.split_whitespace().nth(1).unwrap_or("/");
    let out = if path.starts_with("/event") {
        // body: {handler, event}
        let v: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::json!({}));
        let handler = v.get("handler").and_then(|h| h.as_str()).unwrap_or("").to_string();
        let event_json = v.get("event").map(|e| e.to_string()).unwrap_or_else(|| "{}".into());
        let nav = if handler.is_empty() { false } else { preview.handle_event(&handler, &event_json) };
        let resp = serde_json::json!({ "html": preview.body_html(), "css": if nav { preview.css() } else { serde_json::Value::Null.to_string() } });
        http("200 OK", "application/json", resp.to_string().as_bytes())
    } else if path.starts_with("/back") {
        preview.back();
        let resp = serde_json::json!({ "html": preview.body_html(), "css": preview.css() });
        http("200 OK", "application/json", resp.to_string().as_bytes())
    } else {
        http("200 OK", "text/html; charset=utf-8", full_page(preview).as_bytes())
    };
    let _ = stream.write_all(&out);
    let _ = stream.flush();
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let app_root = args.get(1).cloned().unwrap_or_else(|| "sample-app".to_string());
    let start_port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(9000);

    println!("📂 加载小程序: {}", app_root);
    let mut preview = Preview::new(&app_root)?;

    let mut listener = None;
    let mut bound = start_port;
    for p in start_port..start_port.saturating_add(20) {
        match TcpListener::bind(("127.0.0.1", p)) {
            Ok(l) => { listener = Some(l); bound = p; break; }
            Err(_) => println!("⚠️  端口 {} 被占用，尝试 {} ...", p, p + 1),
        }
    }
    let listener = listener.ok_or("找不到可用端口")?;
    println!("\n🌐 HTML 直渲染预览已启动：\n   http://127.0.0.1:{}\n", bound);
    if bound != start_port {
        println!("（{} 被占用，已改用 {}；释放旧端口：lsof -ti :{} | xargs kill -9）", start_port, bound, start_port);
    }
    println!("（浏览器原生渲染，点击即时响应；Ctrl+C 退出）");

    for stream in listener.incoming() {
        if let Ok(mut s) = stream { handle(&mut s, &mut preview); }
    }
    Ok(())
}
