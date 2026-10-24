//! 字体在真实页面上占多少内存：用移动端 SDK 那条路（`MiniEngine`）把示例小程序的页面挨个打开、滚到底，
//! 每页之后打一行 `text::mem::report()`（展开了多少字形、几何 / 表 / 位图缓存各多少）和进程 占用。
//!
//! ```bash
//! cargo run --release --example font_mem_pages -- [小程序目录] [--dpr 3]
//! # 默认：Flutter 示例的内置小程序（一加真机上量的就是它），360×785 @3
//! ```
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine};

/// 物理占用（macOS 的 phys_footprint：脏页 + 压缩页，不含干净的文件页；Android 上 PSS 的近似）。
/// 别处退回 占用
fn rss_mb() -> f32 {
    #[cfg(target_os = "macos")]
    unsafe {
        let mut info: libc::rusage_info_v2 = std::mem::zeroed();
        if libc::proc_pid_rusage(std::process::id() as i32, libc::RUSAGE_INFO_V2, &mut info as *mut _ as *mut _) == 0 {
            return info.ri_phys_footprint as f32 / 1048576.0;
        }
    }
    rss_mb_ps()
}

fn rss_mb_ps() -> f32 {
    let pid = std::process::id().to_string();
    let out = std::process::Command::new("ps").args(["-o", "rss=", "-p", &pid]).output().ok();
    out.and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f32>().ok()).unwrap_or(0.0) / 1024.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut app = "sdk/flutter/example/assets/demo".to_string();
    let mut dpr = 3.0f32;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--dpr" => dpr = args.next().and_then(|v| v.parse().ok()).unwrap_or(dpr),
            _ => app = a,
        }
    }
    let pages: Vec<String> = std::fs::read_to_string(format!("{app}/app.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["pages"].as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.as_str().map(str::to_string))
        .collect();
    let data = std::env::temp_dir().join("font_mem_pages");
    println!("启动前 占用 {:.1}MB", rss_mb());
    let mut e = MiniEngine::new_with_options(&app, Some(&data.to_string_lossy()), 360, 785, dpr, HostEnv::default(), EngineOptions::default())
        .expect("创建引擎");
    e.launch(None).expect("打开首页");
    let mut t = 1u64;
    let mut pump = |e: &mut MiniEngine, n: usize| {
        for _ in 0..n {
            t += 16;
            e.pump_ex(t);
            while e.poll_event().is_some() {}
        }
    };
    pump(&mut e, 3);
    println!("首页之后 占用 {:.1}MB  {}", rss_mb(), mini_render::text::mem::report());
    for p in &pages {
        if e.navigate(&format!("/{p}")).is_err() {
            continue;
        }
        pump(&mut e, 3);
        for k in 1..=6 {
            e.scroll_to(k as f32 * 600.0);
            pump(&mut e, 2);
        }
        println!("{p:<28} 占用 {:6.1}MB  {}", rss_mb(), mini_render::text::mem::report());
        if std::env::var("MEM_DETAIL").is_ok() {
            println!("    {}", e.memory_report_line());
        }
        let _ = e.back();
        pump(&mut e, 2);
    }
    println!("走完之后 占用 {:.1}MB", rss_mb());
    println!("{}", e.memory_report_line());
    if let Ok(secs) = std::env::var("HOLD").map(|v| v.parse::<u64>().unwrap_or(30)) {
        std::thread::sleep(std::time::Duration::from_secs(secs));
    }
}
