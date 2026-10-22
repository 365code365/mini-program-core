//! 移动端 SDK 的「转场与弹层」逐帧计时：navigateTo / navigateBack 的滑入滑出、
//! ActionSheet / Modal 的弹出收起。与 `sdk_scroll_bench` 同一种量法（每个 vsync 一次 `pump_ex`，
//! 有新画面就把整帧写进一块 RGBA 缓冲，代替 `ANativeWindow_lock` 那块），选项与 Flutter 宿主一致
//! （引擎画导航栏 + 转场动画 + 胶囊）。
//!
//! ```bash
//! cargo run --release --example sdk_motion_bench -- [小程序目录] [--hz 120] [--size 360x785] [--dpr 3]
//! ```
//!
//! 判据看**最慢一帧**与超预算帧数（steering 10），不看平均。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::{EngineOptions, MiniEngine, PixelFormat};
use std::time::Instant;

struct Bench {
    e: MiniEngine,
    window: Vec<u8>,
    stride: usize,
    t: u64,
    step: u64,
}

impl Bench {
    /// 推一帧，返回 (耗时 ms, 出了新画面没有, 帧种类)
    fn frame(&mut self) -> (f32, bool, &'static str) {
        self.t += self.step;
        let a = Instant::now();
        let flags = self.e.pump_ex(self.t);
        let new = flags & FRAME_NEW != 0;
        if new {
            self.e.write_pixels(&mut self.window, self.stride, PixelFormat::Rgba8);
        }
        while self.e.poll_event().is_some() {}
        let ms = a.elapsed().as_secs_f32() * 1000.0;
        (ms, new, if new { self.e.last_frame_kind().label() } else { "空转" })
    }

    /// 一直推到不再要帧（最多 `max` 帧），把出了画面的那些记下来
    fn run(&mut self, label: &str, max: usize, budget: f32) {
        let mut times = Vec::new();
        let mut kinds: Vec<(&str, usize)> = Vec::new();
        for _ in 0..max {
            let (ms, new, kind) = self.frame();
            if new {
                times.push(ms);
                match kinds.iter_mut().find(|(k, _)| *k == kind) {
                    Some((_, n)) => *n += 1,
                    None => kinds.push((kind, 1)),
                }
            }
            if !new && !self.e.wants_next_frame() {
                break;
            }
        }
        let first = times.first().copied().unwrap_or(0.0);
        let rest: Vec<f32> = times.iter().skip(1).copied().collect();
        let mut sorted = rest.clone();
        sorted.sort_by(f32::total_cmp);
        let over = rest.iter().filter(|t| **t > budget).count();
        let p50 = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0);
        println!(
            "  {label:<10} {:>3} 帧  首帧 {first:>6.2}ms  之后 p50 {p50:>5.2}ms 最慢 {:>6.2}ms  超 {budget:.1}ms 预算 {over:>2} 帧  | {:?}",
            times.len(),
            sorted.last().copied().unwrap_or(0.0),
            kinds,
        );
    }

    fn tap(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.frame();
        self.e.pointer_up(x, y);
    }
}

fn main() {
    let mut pos = Vec::new();
    let (mut hz, mut w, mut h, mut dpr) = (120.0f32, 360u32, 785u32, 3.0f32);
    let mut rounds = 2u32;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--hz" => hz = args.next().and_then(|v| v.parse().ok()).unwrap_or(hz),
            "--dpr" => dpr = args.next().and_then(|v| v.parse().ok()).unwrap_or(dpr),
            // 多跑几轮：给 `sample` / Instruments 采样留时间
            "--rounds" => rounds = args.next().and_then(|v| v.parse().ok()).unwrap_or(rounds),
            "--size" => {
                if let Some((a, b)) = args.next().as_deref().and_then(|s| s.split_once('x')) {
                    w = a.parse().unwrap_or(w);
                    h = b.parse().unwrap_or(h);
                }
            }
            _ => pos.push(a),
        }
    }
    let app = pos.first().cloned().unwrap_or_else(|| "sdk/flutter/example/assets/demo".into());
    let root = mini_render::app_dir::resolve(&app);
    let data = mini_render::data_dir::subdir("sdk-motion-bench");
    std::fs::remove_dir_all(&data).ok();
    mini_render::renderer::components::reset_shared_component_state();
    let opts = EngineOptions {
        navigation_bar: true,
        route_animation: true,
        menu_button: true,
        native_components: vec!["video".into()],
        ..Default::default()
    };
    let e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), w, h, dpr, HostEnv::from_process(), opts)
        .expect("创建引擎");
    let (pw, ph) = e.pixel_size();
    let budget = 1000.0 / hz;
    let mut b = Bench { e, window: vec![0u8; pw as usize * 4 * ph as usize], stride: pw as usize * 4, t: 1, step: (1000.0 / hz).round() as u64 };
    b.e.launch(None).expect("打开首页");
    for _ in 0..20 {
        b.frame();
    }
    println!("SDK 转场与弹层：{}  {w}×{h} @{dpr}（{pw}×{ph} 像素）  {hz}Hz 预算 {budget:.1}ms", root.display());
    for round in 1..=rounds {
        println!("第 {round} 轮");
        for page in ["pages/wxss/wxss", "pages/perf/perf"] {
            b.e.navigate(&format!("/{page}")).expect("navigateTo");
            b.run(&format!("进 {}", page.rsplit('/').next().unwrap_or(page)), 120, budget);
            for _ in 0..10 {
                b.frame();
            }
            b.e.back();
            b.run("返回", 120, budget);
        }
        b.e.eval("wx.showActionSheet({itemList:['拍照','从相册选择','取消关注']})").ok();
        b.run("ActionSheet", 120, budget);
        // 点遮罩（屏幕顶上）收起
        b.tap(w as f32 / 2.0, 120.0);
        b.run("收 Sheet", 120, budget);
        b.e.eval("wx.showModal({title:'提示',content:'确定要删除吗？'})").ok();
        b.run("Modal", 120, budget);
        b.tap(w as f32 * 0.25, h as f32 / 2.0 + 60.0);
        b.run("收 Modal", 120, budget);
    }
}
