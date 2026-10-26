//! 桌面窗体的小游戏入口（game-doc/04 G1.4）。
//!
//! G1 只有无头：按 `--hz` 的节拍 pump `--frames` 帧，把游戏的日志 / 错误逐行打到 stdout，然后退出。
//! 主画布与上屏在 G2 接进来（那时 `--snapshot` 才有图可存）；交互窗口在 G2/G3。
//!
//! 退出码：0 正常；1 建实例失败（包不合法）；3 运行中出现过错误事件（脚本异常、看门狗、OOM），
//! 让脚本能区分「跑完了」与「跑完了但游戏报过错」。

use mini_render::game::{GameEngine, GameOptions};
use std::path::Path;

pub struct GameRun {
    pub frames: u32,
    pub hz: u32,
    pub query: Option<String>,
    pub evals: Vec<String>,
}

pub fn run(dir: &Path, opts: GameRun) -> i32 {
    let query: serde_json::Map<String, serde_json::Value> = opts
        .query
        .as_deref()
        .unwrap_or("")
        .split('&')
        .filter_map(|kv| kv.split_once('=').map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string()))))
        .collect();
    let go = GameOptions { launch_options: Some(serde_json::json!({ "query": query, "scene": 1001 })), ..Default::default() };
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/game-data");
    let mut engine = match GameEngine::new(&dir.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0, go) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("❌ {e}");
            return 1;
        }
    };
    let (w, h, dpr) = engine.logical_size();
    println!("🎮 小游戏：{}（{}，{w}x{h}@{dpr}）", dir.display(), engine.config().device_orientation.as_str());
    let mut had_error = drain(&mut engine);
    let hz = opts.hz.max(1) as u64;
    let start = std::time::Instant::now();
    for n in 1..=opts.frames as u64 {
        // 定时器在 G2 之前按墙上时钟到点：无头也按节拍真等，结果才与交互窗体一致
        let due = std::time::Duration::from_micros(n * 1_000_000 / hz);
        if let Some(wait) = due.checked_sub(start.elapsed()) {
            std::thread::sleep(wait);
        }
        engine.pump_ex(n * 1000 / hz);
        had_error |= drain(&mut engine);
    }
    for js in &opts.evals {
        match engine.eval(js) {
            Ok(v) => println!("[eval] {v}"),
            Err(e) => {
                println!("[eval] ❌ {e}");
                had_error = true;
            }
        }
        had_error |= drain(&mut engine);
    }
    if had_error { 3 } else { 0 }
}

/// 打印事件，返回期间有没有错误事件
fn drain(engine: &mut GameEngine) -> bool {
    let mut err = false;
    while let Some(e) = engine.poll_event() {
        let v: serde_json::Value = serde_json::from_str(&e).unwrap_or_default();
        match v["type"].as_str() {
            Some("log") => println!("{}", v["text"].as_str().unwrap_or("")),
            Some("error") => {
                err = true;
                eprintln!("❌ [{}] {}", v["kind"].as_str().unwrap_or(""), v["message"].as_str().unwrap_or(""));
            }
            _ => println!("{e}"),
        }
    }
    err
}
