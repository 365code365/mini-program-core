//! setData 整棵重建的计时：每轮改一个参与渲染的字段（逼着整棵重建），量 pump 时间。
//!
//! ```bash
//! cargo run --release --example layout_bench -- [小程序目录] [页面] [字段] [--n 300]
//! # 默认：Flutter 示例的「节点与滚动」页（pages/query/query），改 scrollTop
//! ```
//! 只看 p50 / 最慢：同一台机器、同一时段前后交替跑才有可比性。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine};
use std::time::Instant;

fn main() {
    let mut pos = Vec::new();
    let mut n = 300usize;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--n" => n = args.next().and_then(|v| v.parse().ok()).unwrap_or(n),
            _ => pos.push(a),
        }
    }
    let app = pos.first().cloned().unwrap_or_else(|| "sdk/flutter/example/assets/demo".into());
    let route = pos.get(1).cloned().unwrap_or_else(|| "pages/query/query".into());
    let field = pos.get(2).cloned().unwrap_or_else(|| "scrollTop".into());
    let root = mini_render::app_dir::resolve(&app);
    let data = mini_render::data_dir::subdir("layout-bench");
    std::fs::remove_dir_all(&data).ok();
    let opts = EngineOptions { navigation_bar: true, route_animation: true, menu_button: true, ..Default::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 360, 785, 3.0, HostEnv::from_process(), opts)
        .expect("创建引擎");
    e.launch(Some(&route)).expect("打开页面");
    let mut t = 1u64;
    for _ in 0..20 {
        e.pump_ex(t);
        while e.poll_event().is_some() {}
        t += 11;
    }
    let mut ms = Vec::with_capacity(n);
    for i in 0..n {
        e.eval(&format!("getCurrentPages()[getCurrentPages().length-1].setData({{ {field}: {} }})", 1000 + i)).ok();
        let a = Instant::now();
        e.pump_ex(t);
        ms.push(a.elapsed().as_secs_f32() * 1000.0);
        while e.poll_event().is_some() {}
        t += 11;
    }
    ms.sort_by(f32::total_cmp);
    let p = |q: f32| ms[((q * ms.len() as f32) as usize).min(ms.len() - 1)];
    println!("{route} 改 {field} ×{n}：pump p50 {:.3}ms p90 {:.3}ms 最慢 {:.3}ms", p(0.5), p(0.9), ms[ms.len() - 1]);
}
