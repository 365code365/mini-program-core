//! SDK 宿主（`MiniEngine`，移动端走的那条路）与桌面窗体的**像素一致性**检查。
//!
//! 为什么需要它：移动端不能编进 winit/softbuffer，所以 SDK 用的是
//! `mini_render::host::MiniEngine`，而桌面用的是 `src/bin/window.rs`。两者共用
//! `host::` 下的页面加载、覆盖层、触摸状态机、像素合成，但**帧调度是各自的**。
//! 这个例子把每个页面用 SDK 那条路渲染成 PNG，交给 `tools/sdk-parity.sh` 与桌面
//! 基线逐像素对比 —— 一旦哪天两条路开始分叉，这里会先叫。
//!
//! ```bash
//! cargo run --release --example sdk_parity -- sample-app target/_sdk
//! bash tools/sdk-parity.sh          # 渲染 + 对比一条龙
//! ```

use mini_render::host::MiniEngine;

fn main() {
    let mut args = std::env::args().skip(1);
    let app = args.next().unwrap_or_else(|| "sample-app".to_string());
    let out = args.next().unwrap_or_else(|| "target/_sdk".to_string());
    let root = mini_render::app_dir::resolve(&app);
    let app_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("app.json")).expect("app.json"))
            .expect("app.json 不是合法 JSON");
    let routes: Vec<String> = app_json["pages"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    // 与桌面基线同样的参数：375x667 @2x
    let data = mini_render::data_dir::subdir("sdk-parity");
    let mut ok = 0usize;
    for route in &routes {
        // 每个页面单开一个实例，避免上一页的状态影响（与逐页快照的语义一致）
        let mut engine = match MiniEngine::new(
            &root.to_string_lossy(),
            Some(&data.to_string_lossy()),
            375,
            667,
            2.0,
        ) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("❌ 创建引擎失败: {e}");
                std::process::exit(1);
            }
        };
        // 与桌面基线的 `--time 2` 对齐：把 CSS 动画求值到同一时刻
        engine.set_animation_time(2.0);
        if let Err(e) = engine.launch(Some(route)) {
            eprintln!("⚠️  {route} 打开失败: {e}");
            continue;
        }
        // 与桌面 `--settle 0` 的快照对齐：**只跑一帧**。
        // 多跑几帧会让页面里的 `setTimeout` 有机会触发（sample-app 首页那个
        // 「新人专享礼包」延时浮层就是这样冒出来的，一张图 93% 都不一样），
        // 那不是渲染分叉，是两边等的时间不同。
        engine.pump(1);
        let (w, h) = engine.pixel_size();
        let dir = std::path::Path::new(&out).join(route);
        std::fs::create_dir_all(&dir).ok();
        let file = dir.join("rust.png");
        image::save_buffer(&file, engine.rgba(), w, h, image::ColorType::Rgba8)
            .expect("写 PNG 失败");
        println!("🖼  {route} -> {}", file.display());
        ok += 1;
    }
    println!("完成：{ok}/{} 页", routes.len());
}
