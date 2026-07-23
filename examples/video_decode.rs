//! 视频解码验证：加载 doc/videos/video.mp4，解码并在若干时间点抓取真实画面帧，
//! 保存为 PNG，用于验证 H.264 解码与逐帧播放推进是否正常（可离屏验证）。
//!
//! 运行：cargo run --example video_decode

use mini_render::renderer::components::{
    get_or_create_player, get_video_frame, get_video_progress,
};
use std::thread::sleep;
use std::time::Duration;

fn save_frame(src: &str, label: &str) {
    match get_video_frame(src) {
        Some((data, w, h)) => {
            // 统计非黑像素占比，判断是否为真实画面
            let mut non_black = 0usize;
            let total = (w * h) as usize;
            for px in data.chunks(4) {
                if px[0] as u16 + px[1] as u16 + px[2] as u16 > 30 {
                    non_black += 1;
                }
            }
            let pct = non_black as f32 / total.max(1) as f32 * 100.0;
            let path = format!("doc/videos/frame_{}.png", label);
            if let Some(buf) = image::RgbaImage::from_raw(w, h, data) {
                buf.save(&path).ok();
            }
            let (cur, dur) = get_video_progress(src).unwrap_or((0.0, 0.0));
            println!(
                "  {} -> {} ({}x{}) 非黑像素 {:.1}%  进度 {:.2}/{:.2}s",
                label, path, w, h, pct, cur, dur
            );
        }
        None => println!("  {} -> 无帧（解码失败或未加载）", label),
    }
}

fn main() {
    std::fs::create_dir_all("doc/videos").ok();
    let src = "doc/videos/video.mp4";
    println!("加载并解码视频: {}", src);

    let ok = get_or_create_player(src, true, false); // autoplay=true
    if !ok {
        eprintln!("❌ 视频加载失败");
        std::process::exit(1);
    }

    if let Some((_, dur)) = get_video_progress(src) {
        println!("✅ 时长 {:.2}s，开始按时间点抓帧验证解码：", dur);
    }

    // 播放开始后在不同时间点抓取画面，验证画面在推进（流畅播放）
    sleep(Duration::from_millis(300));
    save_frame(src, "t0");
    sleep(Duration::from_millis(3000));
    save_frame(src, "t3");
    sleep(Duration::from_millis(3000));
    save_frame(src, "t6");
    sleep(Duration::from_millis(3000));
    save_frame(src, "t9");

    println!("完成。可查看 doc/videos/frame_*.png 确认是真实且不同的画面。");
}
