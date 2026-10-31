//! 高性能模式（游戏 JS 跑在 WebView 里，game/web）的桌面检查：同一个小游戏包在 headless Chrome 里跑，打印日志、可截图。
//!   cargo run --release --example web_game_check -- <游戏目录> [秒数=8] [--query a=1&b=2] [--shot out.png] [--open]
//! `--open`：不跑 headless，只起服务、打印地址，手工用 Chrome 打开调试（开发者工具里看页面）。
//! 日志里出现 `T:DONE` 就提前结束。宿主事件（hostCall、exit …）原样打印。
use mini_render::game::web::{cdp::Chrome, devserver::DevServer, WebGame};
use mini_render::game::GameOptions;
use std::sync::{Arc, Mutex};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = a.get(1).expect("游戏目录").clone();
    let secs: f64 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(8.0);
    let flag = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned();
    let query: serde_json::Map<String, serde_json::Value> = flag("--query").unwrap_or_default().split('&')
        .filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), serde_json::json!(v))).collect();
    let opts = GameOptions { app_id: Some("wxwebcheck".into()), launch_options: Some(serde_json::json!({ "query": query, "scene": 1001 })), ..Default::default() };
    let data = std::env::temp_dir().join("web-game-check-data");
    let (w, h) = (375, 667);
    let game = WebGame::new(&dir, Some(data.to_str().unwrap()), w, h, 2.0, &opts).expect("建实例");
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let l2 = log.clone();
    let srv = DevServer::start(game, move |e| {
        let v: serde_json::Value = serde_json::from_str(e).unwrap_or_default();
        let line = if v["type"] == "log" { v["text"].as_str().unwrap_or("").to_string() } else { format!("[事件] {e}") };
        println!("{line}");
        l2.lock().unwrap().push(line);
    }).expect("起服务");
    if a.iter().any(|x| x == "--open") {
        println!("打开 {}（Ctrl-C 结束）", srv.url());
        loop { std::thread::sleep(std::time::Duration::from_secs(3600)); }
    }
    let mut c = Chrome::launch(w, h, 2.0).expect("Chrome");
    c.navigate(&srv.url()).expect("打开页面");
    let t0 = std::time::Instant::now();
    while t0.elapsed().as_secs_f64() < secs {
        if log.lock().unwrap().iter().any(|l| l.contains("T:DONE")) { break; }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
    if let Some(p) = flag("--shot") {
        std::fs::write(&p, c.screenshot().expect("截图")).expect("写截图");
        println!("截图 {p}");
    }
}
