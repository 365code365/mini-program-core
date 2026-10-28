//! 小游戏无头长稳（game-doc/04 A9.2 的桌面版）：固定时钟按 60fps 跑 N 个游戏分钟（默认 30 = 108000 帧），
//! 每 5 个游戏分钟采 RSS 与 QuickJS 堆，前后各 5 分钟统计每帧 pump 耗时。真机的那一份见 tools/android-game-soak.sh。
//!   cargo run --release --example game_soak -- <游戏目录> [分钟] [query]
use mini_render::game::{GameEngine, GameOptions};

fn rss_kb() -> u64 {
    let pid = std::process::id().to_string();
    let out = std::process::Command::new("ps").args(["-o", "rss=", "-p", &pid]).output().ok();
    out.and_then(|o| String::from_utf8(o.stdout).ok()).and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = a.get(1).expect("游戏目录");
    let min: u64 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let query: serde_json::Map<String, serde_json::Value> = a.get(3).map(|q| q.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), serde_json::json!(v))).collect()).unwrap_or_default();
    let opts = GameOptions { app_id: Some("wxsoak".into()), fixed_clock: true, launch_options: Some(serde_json::json!({ "query": query })), ..Default::default() };
    let data = std::env::temp_dir().join("game-soak-data");
    let mut e = GameEngine::new(dir, Some(data.to_str().unwrap()), 375, 667, 2.0, opts).expect("建实例");
    let frames = min * 60 * 60;
    let (mut head, mut tail) = (Vec::new(), Vec::new());
    let mut errors = 0;
    println!("t_min,rss_kb,js_malloc_kb,js_used_kb");
    for n in 1..=frames {
        let t0 = std::time::Instant::now();
        e.pump_ex(n * 1_000_000 / 60);
        let _ = e.frame_rgba();
        let us = t0.elapsed().as_micros() as u64;
        if n <= 5 * 3600 { head.push(us) } else if n > frames.saturating_sub(5 * 3600) { tail.push(us) }
        while let Some(ev) = e.poll_event() {
            if ev.contains("\"type\":\"error\"") {
                errors += 1;
                if errors < 5 { eprintln!("{ev}"); }
            }
        }
        if n % (5 * 3600) == 0 || n == 1 {
            let (m, u) = e.js_heap();
            println!("{},{},{},{}", n / 3600, rss_kb(), m / 1024, u / 1024);
        }
    }
    let stat = |v: &mut Vec<u64>| { v.sort(); (v[v.len() / 2] as f64 / 1000.0, v[v.len() * 99 / 100] as f64 / 1000.0, *v.last().unwrap() as f64 / 1000.0) };
    println!("head pump ms median/p99/max {:?}", stat(&mut head));
    println!("tail pump ms median/p99/max {:?}", stat(&mut tail));
    println!("errors {errors}");
}
