//! 移动端 SDK（`MiniEngine`）页面滚动的逐帧计时：拖动 + 松手惯性，每帧 `pump_ex` + 写像素。
//!
//! 为什么单独一个：`backend_bench` / `--drag` 量的是**桌面窗体**那条帧调度，SDK 的出帧闸门
//! 是另一份（`host/engine_frame.rs`），两者分叉过 —— 桌面上滑得顺，Flutter 宿主里照样卡。
//! 这里按宿主的真实节奏驱动引擎：每个 vsync 先送一个 move，再 `pump_ex`，有新画面就把整帧
//! 写进一块 RGBA 缓冲（代替 `ANativeWindow_lock` 拿到的那块），两段分别计时。
//!
//! ```bash
//! cargo run --release --example sdk_scroll_bench -- [小程序目录] [页面] [--hz 60] [--rounds 3]
//! # 默认：Flutter 示例的内置小程序（长列表 + 无限旋转的 CSS 动画 + 每秒 setData + 原生 video），
//! # 411×786 @2.625（模拟器 1080×2400 / 420dpi 的视口）
//! ```
//!
//! 判据看**最慢一帧**与超预算帧数，不看平均（steering 10）。机器负载高时 p99 / 最慢会抖
//! 2~5 倍，p50 与「整帧重画次数」是稳的 —— 改动前后要在同一时段交替跑（`--rounds`）。

use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::{EngineOptions, MiniEngine, PixelFormat};
use std::time::Instant;

/// 缺省视口（`--size 360x785 --dpr 3` 改成一加 PLY110 那种 1080×2354 / 480dpi）
const W: u32 = 411;
const H: u32 = 786;
const DPR: f32 = 2.625;

#[derive(Clone, Copy)]
struct Viewport {
    w: u32,
    h: u32,
    dpr: f32,
    /// `--start <pt>`：先慢拖这么多再开始量（复杂元素在页面下半截时用，例如 WXSS 页的 transform 区）
    start: f32,
}

struct Frame {
    pump_ms: f32,
    blit_ms: f32,
    kind: &'static str,
}

fn pct(sorted: &[f32], q: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (q * sorted.len() as f32).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

fn report(label: &str, frames: &[Frame], budget_ms: f32) {
    let mut total: Vec<f32> = frames.iter().map(|f| f.pump_ms + f.blit_ms).collect();
    let mut pump: Vec<f32> = frames.iter().map(|f| f.pump_ms).collect();
    total.sort_by(f32::total_cmp);
    pump.sort_by(f32::total_cmp);
    let over = total.iter().filter(|t| **t > budget_ms).count();
    // 每种帧各自的次数与耗时：「补条带」「整帧」的尖峰要单独看，混在一起会被大量便宜帧摊平
    let mut kinds: Vec<(&str, Vec<f32>)> = Vec::new();
    for f in frames {
        let t = f.pump_ms + f.blit_ms;
        match kinds.iter_mut().find(|(k, _)| *k == f.kind) {
            Some((_, v)) => v.push(t),
            None => kinds.push((f.kind, vec![t])),
        }
    }
    let kinds: Vec<String> = kinds
        .iter_mut()
        .map(|(k, v)| {
            v.sort_by(f32::total_cmp);
            if *k == "空转" {
                format!("{k} {}", v.len())
            } else {
                format!("{k} {}（p50 {:.1} 最慢 {:.1}）", v.len(), pct(v, 0.5), v.last().copied().unwrap_or(0.0))
            }
        })
        .collect();
    println!(
        "  {label:<4} {:>3} 帧  每帧(pump+写像素) p50 {:>5.2}ms  p99 {:>5.2}ms  最慢 {:>6.2}ms  \
         超 {budget_ms:.1}ms 预算 {over:>3} 帧  | pump p50 {:.2}ms  | {}",
        frames.len(),
        pct(&total, 0.5),
        pct(&total, 0.99),
        total.last().copied().unwrap_or(0.0),
        pct(&pump, 0.5),
        kinds.join(" / "),
    );
}

fn run_once(app: &str, route: &str, hz: f32, vp: Viewport) -> (Vec<Frame>, Vec<Frame>, Vec<Frame>, Vec<Frame>) {
    let data = mini_render::data_dir::subdir("sdk-scroll-bench");
    std::fs::remove_dir_all(&data).ok();
    mini_render::renderer::components::reset_shared_component_state();
    // 与 Flutter 宿主同一组选项（引擎画导航栏 + 转场 + 胶囊，安全区按真机）：从前只开了原生组件，
    // 导航栏那条路径（坐标换算、导航栏重画）基准里根本没走到
    // `BENCH_NO_NAV=1` / `BENCH_NO_CAPSULE=1`：单独关掉一项，看合成成本是谁带来的
    let opts = EngineOptions {
        native_components: vec!["video".into()],
        navigation_bar: std::env::var_os("BENCH_NO_NAV").is_none(),
        route_animation: true,
        menu_button: std::env::var_os("BENCH_NO_CAPSULE").is_none(),
        safe_area: Some(mini_render::host::SafeAreaInsets { top: 40.0, right: 0.0, bottom: 0.0, left: 0.0 }),
        ..Default::default()
    };
    let mut e = MiniEngine::new_with_options(
        app,
        Some(&data.to_string_lossy()),
        vp.w,
        vp.h,
        vp.dpr,
        HostEnv::from_process(),
        opts,
    )
    .expect("创建引擎");
    e.launch(Some(route)).expect("打开页面");
    let (pw, ph) = e.pixel_size();
    let stride = pw as usize * 4;
    let mut window = vec![0u8; stride * ph as usize];
    let step_ms = (1000.0 / hz).round() as u64;
    let mut t: u64 = 1;
    // 先让首屏、图片、定时器起来（与宿主「打开页面后等一会儿再滑」一致）
    for _ in 0..10 {
        e.pump_ex(t);
        while e.poll_event().is_some() {}
        t += step_ms;
    }
    let mut frame = |e: &mut MiniEngine, t: u64| -> Frame {
        let a = Instant::now();
        let flags = e.pump_ex(t);
        let b = Instant::now();
        if flags & FRAME_NEW != 0 {
            e.write_pixels(&mut window, stride, PixelFormat::Rgba8);
        }
        let blit_ms = b.elapsed().as_secs_f32() * 1000.0;
        // 宿主每帧都把事件取走（nativeViews 等），不取的话队列越积越长
        while e.poll_event().is_some() {}
        Frame {
            pump_ms: (b - a).as_secs_f32() * 1000.0,
            blit_ms,
            kind: if flags & FRAME_NEW != 0 { e.last_frame_kind().label() } else { "空转" },
        }
    };
    if vp.start > 0.0 {
        let mut left = vp.start;
        while left > 0.0 {
            let d = left.min(600.0);
            e.pointer_down(200.0, 700.0);
            for i in 1..=(d / 10.0) as i32 {
                t += step_ms;
                e.pointer_move(200.0, 700.0 - 10.0 * i as f32);
                frame(&mut e, t);
            }
            for _ in 0..3 {
                t += step_ms;
                frame(&mut e, t);
            }
            e.pointer_up(200.0, 700.0 - d);
            left -= d;
        }
        for _ in 0..300 {
            t += step_ms;
            if frame(&mut e, t).kind == "空转" && !e.wants_next_frame() {
                break;
            }
        }
    }
    // 下拉：停在顶上往下拉 300pt 再松手回弹（橡皮筋越界 + 回弹，真机上「往下滑到顶」天天发生）
    let mut pull = Vec::new();
    e.pointer_down(200.0, 200.0);
    for i in 1..=30 {
        t += step_ms;
        e.pointer_move(200.0, 200.0 + 10.0 * i as f32);
        pull.push(frame(&mut e, t));
    }
    e.pointer_up(200.0, 500.0);
    for _ in 0..120 {
        t += step_ms;
        let f = frame(&mut e, t);
        let idle = f.kind == "空转";
        pull.push(f);
        if idle && !e.wants_next_frame() {
            break;
        }
    }
    // 拖动：手指从 y=650 匀速上推 500pt（每帧 10pt），与真机「慢慢往上滑」同一种手势
    let mut drag = Vec::new();
    e.pointer_down(200.0, 650.0);
    for i in 1..=50 {
        t += step_ms;
        e.pointer_move(200.0, 650.0 - 10.0 * i as f32);
        drag.push(frame(&mut e, t));
    }
    // 松手前最后几帧快一些，带出惯性
    for i in 1..=4 {
        t += step_ms;
        e.pointer_move(200.0, 150.0 - 30.0 * i as f32);
        drag.push(frame(&mut e, t));
    }
    e.pointer_up(200.0, 30.0);
    let mut fling = Vec::new();
    for _ in 0..240 {
        t += step_ms;
        let f = frame(&mut e, t);
        let idle = f.kind == "空转";
        fling.push(f);
        if idle && !e.wants_next_frame() {
            break;
        }
    }
    // 拖回顶上：手指每帧往下 10pt，页面往回滚、条带从上面补（首页头图那截渐变 + 半透明文字最贵）
    let mut back = Vec::new();
    e.pointer_down(200.0, 100.0);
    for i in 1..=70 {
        t += step_ms;
        e.pointer_move(200.0, 100.0 + 10.0 * i as f32);
        back.push(frame(&mut e, t));
    }
    e.pointer_up(200.0, 800.0);
    for _ in 0..60 {
        t += step_ms;
        let f = frame(&mut e, t);
        let idle = f.kind == "空转";
        back.push(f);
        if idle && !e.wants_next_frame() {
            break;
        }
    }
    (pull, drag, fling, back)
}

fn main() {
    let mut pos = Vec::new();
    let mut hz = 60.0f32;
    let mut rounds = 1usize;
    let mut vp = Viewport { w: W, h: H, dpr: DPR, start: 0.0 };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--hz" => hz = args.next().and_then(|v| v.parse().ok()).unwrap_or(hz),
            "--rounds" => rounds = args.next().and_then(|v| v.parse().ok()).unwrap_or(rounds),
            "--start" => vp.start = args.next().and_then(|v| v.parse().ok()).unwrap_or(0.0),
            "--dpr" => vp.dpr = args.next().and_then(|v| v.parse().ok()).unwrap_or(vp.dpr),
            "--size" => {
                if let Some((w, h)) = args.next().as_deref().and_then(|s| s.split_once('x')) {
                    vp.w = w.parse().unwrap_or(vp.w);
                    vp.h = h.parse().unwrap_or(vp.h);
                }
            }
            _ => pos.push(a),
        }
    }

    let app = pos.first().cloned().unwrap_or_else(|| "sdk/flutter/example/assets/demo".into());
    // 内置示例的首页改成了导航中心（tabBar 页），原来的「长列表 + 动画 + 定时器 + video」
    // 场景原样搬到了 pages/perf/perf
    let route = pos.get(1).cloned().unwrap_or_else(|| "pages/perf/perf".into());
    let root = mini_render::app_dir::resolve(&app);
    let budget = 1000.0 / hz;
    println!("SDK 滚动基准：{} {route}  {}×{} @{}（{}×{} 像素）  {hz}Hz 预算 {budget:.1}ms",
        root.display(), vp.w, vp.h, vp.dpr, (vp.w as f32 * vp.dpr) as u32, (vp.h as f32 * vp.dpr) as u32);
    for r in 1..=rounds {
        let (pull, drag, fling, back) = run_once(&root.to_string_lossy(), &route, hz, vp);
        println!("第 {r} 轮");
        report("下拉", &pull, budget);
        report("拖动", &drag, budget);
        report("惯性", &fling, budget);
        report("拖回顶", &back, budget);
    }
}
