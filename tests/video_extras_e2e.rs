//! `<video>` 的封面、`bindprogress` / `bindwaiting`、点进度轨道跳转、`sendDanmu`（G-6）。
//!
//! 依据官方 component/video.html、api/media/video/VideoContext.html。夹具同 `video_e2e.rs`（tiny_1s.mp4）。
#![cfg(feature = "h264")]
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::time::{Duration, Instant};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn fixture() -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("video-extras-e2e");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<video id="v" src="/assets/x.mp4" poster="/assets/p.png" enable-danmu style="width:300px;height:200px"
  bindprogress="onProgress" bindwaiting="onWaiting" bindseekcomplete="onSeek"></video>"#;
    let js = "Page({ data: { ev: [] }, onProgress(e) { this.data.ev.push('progress:' + e.detail.buffered); }, onWaiting() { this.data.ev.push('waiting'); }, onSeek(e) { this.data.ev.push('seek:' + e.detail.position.toFixed(1)); } });";
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", "page{background:#ffffff}"),
        ("pages/index/index.js", js),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::copy(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/tiny_1s.mp4"), root.join("assets/x.mp4")).unwrap();
    image::RgbaImage::from_pixel(30, 20, image::Rgba([255, 0, 0, 255])).save(root.join("assets/p.png")).unwrap();
    Fixture(root)
}

#[test]
fn poster_progress_and_seek_on_track() {
    let fx = fixture();
    let data = fx.0.join("_data");
    let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).unwrap();
    e.launch(Some("pages/index/index")).unwrap();
    let mut t = 1000u64;
    let mut wait = |e: &mut MiniEngine, ms: u64| {
        let until = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < until {
            t += 16;
            e.pump(t);
            std::thread::sleep(Duration::from_millis(16));
        }
    };
    wait(&mut e, 300);
    let ev = |e: &MiniEngine| e.eval("__currentPage.data.ev.join(' ')").unwrap_or_default().trim_matches('"').to_string();
    assert!(ev(&e).contains("progress:100"), "整段预解码：加载完 buffered=100（{}）", ev(&e));
    // 还没播：封面（红）盖在画面上（画面中间偏上，避开控制条）
    let (w, _) = e.pixel_size();
    let px = |e: &mut MiniEngine, x: usize, y: usize| {
        let p = e.rgba();
        let i = (y * 2 * w as usize + x * 2) * 4;
        (p[i], p[i + 1], p[i + 2])
    };
    assert_eq!(px(&mut e, 150, 60), (255, 0, 0), "poster 盖在第一帧上");
    // 点控制条进度轨道的正中：跳到一半
    let (t0, tw) = mini_render::renderer::components::progress_track(300.0);
    let x = t0 + tw / 2.0;
    e.pointer_down(x, 200.0 - 18.0);
    wait(&mut e, 16);
    e.pointer_up(x, 200.0 - 18.0);
    wait(&mut e, 64);
    assert!(ev(&e).contains("seek:0.5"), "点进度轨道正中跳到 0.5s（{}）", ev(&e));
    // 跳过了（位置 > 0）：封面让开
    assert_ne!(px(&mut e, 150, 60), (255, 0, 0), "开始播过之后不再盖封面");
}
