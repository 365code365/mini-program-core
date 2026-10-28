//! 桌面窗体的小游戏入口（game-doc/04 G1.4）。
//!
//! 只有无头（`--touch` / `--swipe` / `--multitouch` 驱动 wx.onTouch*，见 game_script.rs）：按 `--hz` 的节拍 pump `--frames` 帧，把游戏的日志 / 错误逐行打到 stdout，然后退出。
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
    /// 跑完之后存最后一帧（缩放到逻辑尺寸 × dpr，与小程序快照同一尺寸）
    pub snapshot: Option<std::path::PathBuf>,
    /// 按帧排好的指针事件（game_script.rs）
    pub input: Vec<super::game_script::Ev>,
}

pub fn run(dir: &Path, opts: GameRun) -> i32 {
    let query: serde_json::Map<String, serde_json::Value> = opts
        .query
        .as_deref()
        .unwrap_or("")
        .split('&')
        .filter_map(|kv| kv.split_once('=').map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string()))))
        .collect();
    // 无头一律固定时钟：同一个 --hz 跑两次逐帧相同（game/clock.rs）
    let go = GameOptions {
        launch_options: Some(serde_json::json!({ "query": query, "scene": 1001 })),
        fixed_clock: true,
        ..Default::default()
    };
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/game-data");
    // 宿主按 game.json 锁方向（G6.3）：横屏包给横屏窗口
    let (w, h) = if landscape(&dir) { (667, 375) } else { (375, 667) };
    let mut engine = match GameEngine::new(&dir.to_string_lossy(), Some(&data.to_string_lossy()), w, h, 2.0, go) {
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
    let mut pump_us: Vec<u64> = Vec::with_capacity(opts.frames as usize);
    // 脚本比 --frames 长就跑到最后一个事件之后一帧，抬手那一帧的回调也要跑到
    let frames = (opts.frames as u64).max(opts.input.last().map(|e| e.frame + 1).unwrap_or(0));
    let mut input = opts.input.iter().peekable();
    for n in 1..=frames {
        // 固定时钟：不用真等，帧时刻就是 n / hz
        let t = n * 1_000_000 / hz;
        while let Some(e) = input.next_if(|e| e.frame <= n) {
            engine.touch(e.phase, e.id, e.x, e.y, 0.0, t);
        }
        let t0 = std::time::Instant::now();
        engine.pump_ex(t);
        pump_us.push(t0.elapsed().as_micros() as u64);
        had_error |= drain(&mut engine);
    }
    if pump_us.len() >= 10 {
        let mut s = pump_us.clone();
        s.sort_unstable();
        let worst: Vec<String> = { let mut v: Vec<(u64, usize)> = pump_us.iter().copied().zip(1..).collect(); v.sort_unstable_by(|a, b| b.0.cmp(&a.0)); v.iter().take(5).map(|(us, n)| format!("#{n} {:.2}ms", *us as f64 / 1000.0)).collect() };
        println!("⏱ 最慢 5 帧：{}", worst.join("、"));
        println!("⏱ pump {} 帧：中位 {:.3}ms p99 {:.3}ms 最大 {:.3}ms", s.len(), s[s.len() / 2] as f64 / 1000.0, s[s.len() * 99 / 100] as f64 / 1000.0, s[s.len() - 1] as f64 / 1000.0);
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
    if let Some(dir) = opts.snapshot.as_ref() {
        let (dw, dh) = ((w as f32 * dpr) as u32, (h as f32 * dpr) as u32);
        let px = engine.frame_rgba_scaled(dw, dh);
        let path = dir.join(format!("frame_{frames:04}.png"));
        std::fs::create_dir_all(dir).ok();
        match image::RgbaImage::from_raw(dw, dh, px).map(|img| img.save(&path)) {
            Some(Ok(())) => println!("📸 {}", path.display()),
            _ => {
                eprintln!("❌ 存快照失败：{}", path.display());
                had_error = true;
            }
        }
        // 主画布自己的分辨率（不缩放）：与 Chrome 参考逐像素比时用这一张（tools/game-compare.py）
        let (nw, nh, raw) = engine.frame_rgba();
        let native = dir.join(format!("frame_{frames:04}.native.png"));
        if let Some(Ok(())) = image::RgbaImage::from_raw(nw, nh, raw.to_vec()).map(|img| img.save(&native)) {
            println!("📸 {}", native.display());
        }
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

/// game.json 的 deviceOrientation 是不是横屏（读不出按竖屏）
pub fn landscape(dir: &std::path::Path) -> bool {
    std::fs::read_to_string(dir.join("game.json")).ok()
        .and_then(|t| mini_render::game::config::GameConfig::parse(&t).ok())
        .is_some_and(|c| c.device_orientation.is_landscape())
}
