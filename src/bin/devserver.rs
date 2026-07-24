//! 浏览器调试预览服务器
//!
//! 把小程序渲染成图片，通过内置 HTTP 服务在浏览器打开，可直接在画面上点击操作：
//! 点击 → 命中事件/交互控件 → 调用页面方法 → setData / 导航 / Toast → 重渲染。
//! 支持 tap、表单控件切换、navigateTo/switchTab/redirectTo/navigateBack 页面跳转、Toast 提示。
//!
//! 运行：
//!   cargo run --bin mini-devserver                 # 默认加载 sample-app（首页）
//!   cargo run --bin mini-devserver <小程序根目录> [端口]
//! 然后浏览器打开 http://127.0.0.1:9000
//!
//! 仅使用标准库网络（无额外依赖）。

use mini_render::runtime::MiniApp;
use mini_render::parser::{WxmlParser, WxssParser};
use mini_render::parser::wxml::WxmlNode;
use mini_render::renderer::WxmlRenderer;
use mini_render::ui::interaction::InteractionManager;
use mini_render::text::TextRenderer;
use mini_render::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Instant;

const W: u32 = 375;
const H: u32 = 667;
const SCALE: f32 = 2.0;

struct Preview {
    app: MiniApp,
    app_root: String,
    renderer: WxmlRenderer,
    nodes: Vec<WxmlNode>,
    interaction: InteractionManager,
    text: Option<TextRenderer>,
    /// 页面栈：(页面路径, query)，栈顶为当前页
    stack: Vec<(String, HashMap<String, String>)>,
    toast: Option<(String, Instant, u32)>,
    /// 帧缓存：仅在页面变化(dirty)时重渲染，空闲轮询直接返回缓存，避免卡顿
    dirty: bool,
    cache: Vec<u8>,
    /// 版本号：每次页面变化 +1，浏览器据此判断是否需要重新拉取帧
    ver: u64,
}

impl Preview {
    fn new(app_root: &str) -> Result<Self, String> {
        let mut app = MiniApp::new(W, H)?;
        app.init()?;
        // 加载 app.js（提供 App()/getApp() 等）
        if let Ok(js) = std::fs::read_to_string(format!("{}/app.js", app_root)) {
            app.load_script(&js).ok();
            app.eval("if (typeof __app !== 'undefined' && __app && __app.onLaunch) __app.onLaunch({})").ok();
        }
        let start = std::fs::read_to_string(format!("{}/app.json", app_root))
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("pages").and_then(|p| p.get(0)).and_then(|p| p.as_str().map(String::from)))
            .unwrap_or_else(|| "pages/index/index".to_string());

        let empty_ss = WxssParser::new("").parse().unwrap_or_default();
        let mut me = Self {
            app,
            app_root: app_root.to_string(),
            renderer: WxmlRenderer::new_with_scale(empty_ss, W as f32, H as f32, SCALE),
            nodes: Vec::new(),
            interaction: InteractionManager::new(),
            text: TextRenderer::load_system_font().ok(),
            stack: Vec::new(),
            toast: None,
            dirty: true,
            cache: Vec::new(),
            ver: 0,
        };
        me.load_page(&start, &HashMap::new())?;
        me.stack.push((start, HashMap::new()));
        Ok(me)
    }

    /// 加载指定页面（读取 js/wxml/wxss，跑生命周期，重建渲染器）
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
        let ss = WxssParser::new(&wxss_src).parse().unwrap_or_default();
        self.renderer = WxmlRenderer::new_with_scale(ss, W as f32, H as f32, SCALE);
        self.interaction = InteractionManager::new();
        self.app.eval("__pendingNavigation = null;").ok();
        self.mark_dirty();
        Ok(())
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.ver = self.ver.wrapping_add(1);
    }

    fn page_data(&self) -> serde_json::Value {
        self.app.eval("__getPageData()").ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    }

    /// 返回当前帧 PNG：仅在页面变化(dirty)或存在动画时重渲染，否则返回缓存。
    fn frame(&mut self) -> Vec<u8> {
        // 动画状态（定时器/视频/Toast 显示中）需要持续出帧
        let animating = self.app.has_active_timers()
            || mini_render::renderer::components::has_playing_video()
            || self.toast.as_ref().map(|(_, t, d)| t.elapsed().as_millis() < *d as u128).unwrap_or(false);
        if animating {
            self.mark_dirty();
        }
        if self.dirty || self.cache.is_empty() {
            self.cache = self.render_png();
            self.dirty = false;
        }
        self.cache.clone()
    }

    fn render_png(&mut self) -> Vec<u8> {
        let t0 = Instant::now();
        let data = self.page_data();
        let td = t0.elapsed();
        let mut canvas = Canvas::new((W as f32 * SCALE) as u32, (H as f32 * SCALE) as u32);
        canvas.clear(Color::from_hex(0xF5F6F8));
        let tl = Instant::now();
        let _h = self.renderer.measure_content_height(&self.nodes, &data);
        let layout_t = tl.elapsed();
        let tr0 = Instant::now();
        self.renderer.render_with_interaction(&mut canvas, &self.nodes, &data, &mut self.interaction);
        let trend = tr0.elapsed();
        eprintln!("[perf] page_data={:?} layout(build)={:?} render(total)={:?}", td, layout_t, trend);
        // Toast 覆盖层
        if let Some((title, since, dur)) = &self.toast {
            if since.elapsed().as_millis() < *dur as u128 {
                self.draw_toast(&mut canvas, title);
            }
        }
        let te = Instant::now();
        let png = encode_png(&canvas);
        eprintln!("[perf] encode={:?} bytes={}", te.elapsed(), png.len());
        png
    }

    fn draw_toast(&self, canvas: &mut Canvas, title: &str) {
        let tr = match &self.text { Some(t) => t, None => return };
        let sf = SCALE;
        let fs = 15.0 * sf;
        let tw = tr.measure_text(title, fs);
        let pad = 20.0 * sf;
        let bw = (tw + pad * 2.0).max(120.0 * sf);
        let bh = 70.0 * sf;
        let cx = W as f32 * sf / 2.0;
        let cy = H as f32 * sf / 2.0;
        let x = cx - bw / 2.0;
        let y = cy - bh / 2.0;
        let mut path = Path::new();
        path.add_round_rect(x, y, bw, bh, 12.0 * sf);
        canvas.draw_path(&path, &Paint::new().with_color(Color::new(0, 0, 0, 200)).with_style(PaintStyle::Fill).with_anti_alias(true));
        let paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill);
        tr.draw_text(canvas, title, cx - tw / 2.0, cy + fs * 0.35, fs, &paint);
    }

    /// 处理一次点击（逻辑坐标）
    fn tap(&mut self, x: f32, y: f32) {
        // 1) 表单控件（checkbox/switch/radio/slider）就地切换
        if self.interaction.handle_click(x, y).is_some() {
            // 状态已更新，重渲染即可反映
        }
        // 2) tap 事件冒泡链 → 调用页面方法
        let chain = self.renderer.hit_test_bubble(x, y, "tap");
        for b in &chain {
            let dj = serde_json::to_string(&b.data).unwrap_or_else(|_| "{}".into());
            let code = format!(
                "__callPageMethod('{}', {{ type:'tap', currentTarget:{{ dataset:{} }}, target:{{ dataset:{} }}, detail:{{}} }})",
                b.handler, dj, dj
            );
            self.app.eval(&code).ok();
        }
        // 3) 泵事件循环（setData/Promise/定时器）
        self.app.update().ok();
        // 4) Toast（读取并清空 __toastConfig）
        if let Ok(t) = self.app.eval("(function(){var t=(typeof __toastConfig!=='undefined')?__toastConfig:null; __toastConfig=null; return t?JSON.stringify(t):'';})()") {
            if !t.is_empty() && t != "null" {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                    if let Some(title) = v.get("title").and_then(|s| s.as_str()) {
                        if !title.is_empty() { self.toast = Some((title.to_string(), Instant::now(), 1500)); }
                    }
                }
            }
        }
        // 5) 导航
        self.dispatch_nav();
        // 状态可能已变化（setData/Toast），标记需要重渲染
        self.mark_dirty();
    }

    fn dispatch_nav(&mut self) {
        let nav = self.app.eval("JSON.stringify(__pendingNavigation || null)").unwrap_or_else(|_| "null".into());
        if nav == "null" || nav.is_empty() { return; }
        let v: serde_json::Value = match serde_json::from_str(&nav) { Ok(v) => v, Err(_) => return };
        self.app.eval("__pendingNavigation = null;").ok();
        let typ = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let url = v.get("url").and_then(|u| u.as_str()).unwrap_or("");
        match typ {
            "navigateTo" => {
                let (p, q) = parse_url(url);
                if self.load_page(&p, &q).is_ok() { self.stack.push((p, q)); }
            }
            "redirectTo" => {
                let (p, q) = parse_url(url);
                if self.load_page(&p, &q).is_ok() { self.stack.pop(); self.stack.push((p, q)); }
            }
            "switchTab" | "reLaunch" => {
                let (p, q) = parse_url(url);
                if self.load_page(&p, &q).is_ok() { self.stack.clear(); self.stack.push((p, q)); }
            }
            "navigateBack" => { self.back(); }
            _ => {}
        }
    }

    fn back(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
            let (p, q) = self.stack.last().unwrap().clone();
            self.load_page(&p, &q).ok();
        }
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
            if let (Some(k), Some(val)) = (it.next(), it.next()) {
                query.insert(k.to_string(), val.to_string());
            }
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

fn encode_png(canvas: &Canvas) -> Vec<u8> {
    // 使用最快压缩 + 无滤波，大幅降低编码耗时（点击时才编码，追求低延迟）
    use image::codecs::png::{PngEncoder, CompressionType, FilterType};
    use image::{ColorType, ImageEncoder};
    let rgba = canvas.to_rgba();
    let mut out: Vec<u8> = Vec::new();
    let enc = PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::NoFilter);
    if enc.write_image(&rgba, canvas.width(), canvas.height(), ColorType::Rgba8).is_err() {
        return Vec::new();
    }
    out
}

const PAGE_HTML: &str = r#"<!doctype html><html lang="zh"><head><meta charset="utf-8">
<title>Mini Render 调试预览</title>
<style>
  body{margin:0;background:#20232a;color:#eee;font-family:-apple-system,system-ui,sans-serif;display:flex;flex-direction:column;align-items:center;padding:24px}
  h1{font-size:16px;font-weight:600;color:#9ad}
  .phone{width:375px;height:667px;box-shadow:0 12px 40px rgba(0,0,0,.5);border-radius:20px;overflow:hidden;background:#000}
  canvas{display:block;width:375px;height:667px;cursor:pointer}
  .bar{margin:14px 0;display:flex;gap:10px;align-items:center;font-size:13px;color:#aaa}
  button{background:#07c160;color:#fff;border:0;padding:8px 16px;border-radius:8px;cursor:pointer;font-size:13px}
  button.back{background:#576b95}
  .hint{font-size:12px;color:#888;margin-top:8px;max-width:440px;text-align:center}
</style></head><body>
<h1>Mini Render · 浏览器调试预览 <span style="color:#07c160">(canvas 载体)</span></h1>
<div class="phone"><canvas id="c" width="750" height="1334"></canvas></div>
<div class="bar">
  <button class="back" onclick="nav('/back')">← 返回</button>
  <button onclick="refresh()">刷新</button>
  <label><input type="checkbox" id="auto" checked> 自动刷新</label>
  <span id="msg"></span>
</div>
<div class="hint">画面以 &lt;canvas&gt; 承载（原生分辨率、更清晰）。点击即可操作：bindtap / 表单控件 / 页面跳转(navigateTo/switchTab) / Toast，均会实时重渲染。</div>
<script>
const cv=document.getElementById('c'), ctx=cv.getContext('2d'), msg=document.getElementById('msg');
let lastVer=-1, busy=false;
async function drawFrame(){
  const r=await fetch('/frame.png?t='+Date.now());
  const blob=await r.blob();
  const bmp=await createImageBitmap(blob);
  ctx.drawImage(bmp,0,0,cv.width,cv.height);
  if(bmp.close) bmp.close();
}
// 只查询轻量版本号，变化时才拉取整帧，空闲几乎零开销
async function poll(){
  if(busy) return;
  try{
    const v=await (await fetch('/ver?t='+Date.now())).text();
    if(v!==lastVer){ lastVer=v; await drawFrame(); }
  }catch(e){}
}
async function nav(u){ busy=true; try{ await fetch(u); lastVer=-1; await poll(); }finally{ busy=false; } }
cv.addEventListener('click', async (e)=>{
  const rect=cv.getBoundingClientRect();
  const x=Math.round((e.clientX-rect.left)*(375/rect.width));
  const y=Math.round((e.clientY-rect.top)*(667/rect.height));
  msg.textContent='tap ('+x+','+y+')';
  if(busy) return; busy=true;
  try{ await fetch('/tap?x='+x+'&y='+y); lastVer=-1; await poll(); }finally{ busy=false; }
});
poll();
setInterval(()=>{ if(document.getElementById('auto').checked) poll(); }, 300);
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
    let path = req.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/");

    let out = if path.starts_with("/ver") {
        // 轻量版本号轮询：不变则浏览器不拉取帧
        http_response("200 OK", "text/plain", preview.ver.to_string().as_bytes())
    } else if path.starts_with("/frame") {
        let png = preview.frame();
        http_response("200 OK", "image/png", &png)
    } else if path.starts_with("/tap") {
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
    } else if path.starts_with("/back") {
        preview.back();
        http_response("200 OK", "text/plain", b"ok")
    } else {
        http_response("200 OK", "text/html; charset=utf-8", PAGE_HTML.as_bytes())
    };
    let _ = stream.write_all(&out);
    let _ = stream.flush();
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let app_root = args.get(1).cloned().unwrap_or_else(|| "sample-app".to_string());
    let port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(9000);

    println!("📂 加载小程序: {}", app_root);
    let mut preview = Preview::new(&app_root)?;

    // 端口被占用（例如残留的旧实例）时自动向后尝试，避免新代码无法启动而误连旧服务
    let mut listener = None;
    let mut bound_port = port;
    for p in port..port.saturating_add(20) {
        match TcpListener::bind(("127.0.0.1", p)) {
            Ok(l) => { listener = Some(l); bound_port = p; break; }
            Err(_) => println!("⚠️  端口 {} 被占用（可能是残留的旧实例），尝试 {} ...", p, p + 1),
        }
    }
    let listener = listener.ok_or_else(|| format!("{}~{} 端口均被占用", port, port + 19))?;
    println!("\n🌐 调试预览已启动，请在浏览器打开：\n   http://127.0.0.1:{}\n", bound_port);
    if bound_port != port {
        println!("（注意：{} 被占用，已改用 {}。如需释放旧端口：lsof -ti :{} | xargs kill -9）", port, bound_port, port);
    }
    println!("（点击画面操作 UI，支持页面跳转/表单/Toast；Ctrl+C 退出）");

    for stream in listener.incoming() {
        if let Ok(mut s) = stream { handle(&mut s, &mut preview); }
    }
    Ok(())
}
