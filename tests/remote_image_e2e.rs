//! G-1：远程图片在 scroll-view 的离屏缓存画好**之后**才到，从前一直显示占位图，滑一下才出图。
//!
//! 根因：图片到位只让宿主重画，数据没变所以布局报 Unchanged、scroll-view 的离屏缓存也一直「干净」，
//! 重画时贴回去的还是画着占位图的旧缓存。现在图片到位（以及 canvas 画过）时缓存一并作废。
//! 本地起一个慢 0.4s 才回图的 HTTP 服务，保证图片晚于缓存到达。走 `MiniEngine`（SDK 链路）。
use mini_render::host::MiniEngine;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// 每个请求都慢 400ms 回同一张纯色 PNG
fn slow_png_server(png: Vec<u8>) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let png = png.clone();
            std::thread::spawn(move || {
                let mut s = s;
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf);
                std::thread::sleep(Duration::from_millis(400));
                let head = format!("HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", png.len());
                let _ = s.write_all(head.as_bytes());
                let _ = s.write_all(&png);
            });
        }
    });
    url
}

#[test]
fn late_remote_image_inside_scroll_view_replaces_placeholder() {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(60, 40, image::Rgba([200, 30, 30, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let base = slow_png_server(png);
    // 每次跑一个新 URL：图片层有磁盘缓存，命中缓存就测不到「晚到」
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();

    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("remote-image-e2e");
    std::fs::remove_dir_all(&root).ok();
    let fx = Fixture(root.clone());
    let js = format!(
        "Page({{ data: {{ url: '' }}, onLoad() {{ setTimeout(() => this.setData({{ url: '{base}/a.png?{stamp}' }}), 100); }} }});"
    );
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#.to_string()),
        ("app.js", "App({});".to_string()),
        ("app.wxss", String::new()),
        ("pages/index/index.json", "{}".to_string()),
        (
            "pages/index/index.wxml",
            r#"<scroll-view scroll-y style="height:667px"><view style="height:1200px"><image src="{{url}}" style="width:200px;height:100px" wx:if="{{url}}"></image></view></scroll-view>"#
                .to_string(),
        ),
        ("pages/index/index.wxss", "page{background:#ffffff}".to_string()),
        ("pages/index/index.js", js),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let data = root.join("_data");
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/index/index")).expect("打开页面");

    let mut t = 1000u64;
    let center = |e: &mut MiniEngine| {
        let (w, _) = e.pixel_size();
        let p = e.rgba();
        let i = ((50 * 2) * w as usize + 100 * 2) * 4;
        (p[i], p[i + 1], p[i + 2])
    };
    // 先画出 setData 之后、图片没到的那一帧（离屏缓存里是占位图）
    let start = Instant::now();
    let mut saw_placeholder = false;
    let mut got = (0, 0, 0);
    while start.elapsed() < Duration::from_secs(5) {
        t += 16;
        e.pump(t);
        got = center(&mut e);
        if got == (200, 30, 30) {
            break;
        }
        if got != (255, 255, 255) {
            saw_placeholder = true;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    assert!(saw_placeholder, "图片晚到：应先画过一帧占位（否则没测到「晚到」这条路）");
    assert_eq!(got, (200, 30, 30), "图片到位后 scroll-view 里应换成图片，不能一直是占位图");
    drop(fx);
}
