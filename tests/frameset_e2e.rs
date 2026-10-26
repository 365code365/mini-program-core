//! 分栏模式（微信 frameset：阔折叠展开 / PC 窗口拉宽，src/host/engine_frameset.rs）。
//!
//! 判据：
//! - 不开分栏、或开了但窗口窄：与从前逐字节相同，一个 `frameset` 事件都没有；
//! - 拉宽到 ≥600：引擎按一栏（一半宽）排，`windowWidth` 是一栏宽；单页时活页面在左、右栏是占位；
//! - 进下一页：活页面挪到右栏，左栏是上一页 —— 与「一开始就按一栏宽打开上一页」逐字节相同；
//! - 左栏滚轮只动左栏；窄回去退回单栏，页面栈不丢。
//!
//! 夹具写在 `CARGO_TARGET_TMPDIR` 下，串行跑（小程序根目录是进程级的，见 embed_host.rs）。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine, PixelFormat};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());
const H: u32 = 640;

/// p：顶上一条红、下面 2000 高的绿色长页；q：白页
fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("frameset").join(name);
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/p/p","pages/q/q"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", r#"<view class="bar">左栏</view><view class="long">长页</view><view class="end">底</view>"#),
        ("pages/p/p.wxss", "page{background-color:#00ff00;} .bar{height:80px;background:#ff0000;} .long{height:2000px;} .end{height:100px;background:#0000ff;}"),
        ("pages/p/p.js", "Page({data:{n:1}});"),
        ("pages/p/p.json", "{}"),
        ("pages/q/q.wxml", "<view>第二页</view>"),
        ("pages/q/q.wxss", "page{background-color:#ffffff;}"),
        ("pages/q/q.js", "Page({});"),
        ("pages/q/q.json", "{}"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, w: u32, opts: &str) -> MiniEngine {
    let data = root.join("_data");
    let opts = EngineOptions::from_json(opts).unwrap();
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), w, H, 1.0, HostEnv::default(), opts)
        .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e.pump(1);
    e
}

fn frameset_events(e: &mut MiniEngine) -> Vec<Value> {
    let mut out = Vec::new();
    while let Some(ev) = e.poll_event() {
        let v: Value = serde_json::from_str(&ev).unwrap();
        if v["type"] == "frameset" {
            out.push(v);
        }
    }
    out
}

fn side(e: &mut MiniEngine) -> Vec<u8> {
    let (w, h) = e.side_pixel_size();
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let n = e.side_pixels_into(&mut buf, (w * 4) as usize, PixelFormat::Rgba8);
    assert_eq!(n, buf.len(), "另一栏取得到图");
    buf
}

fn px(buf: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * w + x) * 4) as usize;
    [buf[i], buf[i + 1], buf[i + 2]]
}

fn window_width(e: &MiniEngine) -> f64 {
    e.eval("wx.getSystemInfoSync().windowWidth").unwrap().trim().parse().unwrap()
}

#[test]
fn 窄窗口与不开分栏时和从前逐字节相同() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("narrow");
    let mut old = engine(&root, 390, "{}");
    let base = old.rgba().to_vec();
    let mut auto = engine(&root, 390, r#"{"frameset":"auto"}"#);
    assert_eq!(auto.rgba(), &base[..], "开了 auto、窗口窄：画面逐字节相同");
    assert!(frameset_events(&mut auto).is_empty(), "没分栏过就不发事件");
    // 不开分栏（app.json 也没写 frameset）：再宽也是单栏
    old.resize(900, H, 1.0);
    old.pump(20);
    assert_eq!(old.pixel_size(), (900, H));
    assert_eq!(old.side_pixel_size(), (0, 0));
    assert!(frameset_events(&mut old).is_empty());
}

#[test]
fn 拉宽分两栏_进下一页活页面到右栏_左栏与按一栏宽打开的上一页相同() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("wide");
    let mut e = engine(&root, 390, r#"{"frameset":"auto"}"#);
    frameset_events(&mut e);
    assert!(e.resize(900, H, 1.0));
    let mut t = 20;
    let flags = e.pump_ex(t);
    assert_ne!(flags & 64, 0, "FRAME_SIDE");
    assert_eq!(e.pixel_size(), (450, H), "引擎按一栏宽排");
    assert_eq!(window_width(&e), 450.0, "windowWidth 是一栏宽");
    let ev = frameset_events(&mut e);
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert_eq!((ev[0]["on"].as_bool(), ev[0]["livePane"].as_str(), ev[0]["paneWidth"].as_u64()), (Some(true), Some("left"), Some(450)));
    // 单页：右栏是占位底色
    let s = side(&mut e);
    assert_eq!(e.side_pixel_size(), (450, H));
    assert_eq!(px(&s, 450, 225, 320), [0xF7, 0xF7, 0xF7]);
    let p_frame = e.rgba().to_vec();
    assert_eq!(px(&p_frame, 450, 440, 10), [0xFF, 0, 0], "红条铺满一栏宽");

    e.navigate("/pages/q/q").unwrap();
    t += 16;
    assert_ne!(e.pump_ex(t) & 64, 0, "换页：另一栏换图");
    let ev = frameset_events(&mut e);
    assert_eq!(ev.last().unwrap()["livePane"], "right");
    assert_eq!(side(&mut e), p_frame, "左栏就是进下一页之前的 p");
    assert_eq!(px(e.rgba(), 450, 225, 320), [0xFF, 0xFF, 0xFF], "右栏是 q");

    // 左栏滚轮：只重画左栏，右栏（活页面）不变
    let q_frame = e.rgba().to_vec();
    e.side_wheel(300.0, true);
    t += 16;
    assert_ne!(e.pump_ex(t) & 64, 0);
    let s = side(&mut e);
    assert_ne!(s, p_frame, "左栏滚了");
    assert_eq!(px(&s, 450, 225, 10), [0, 0xFF, 0], "红条滚出去了");
    assert_eq!(e.rgba(), &q_frame[..], "右栏不动");

    // 窄回去：单栏，栈还在，返回后 p 停在左栏滚到的位置
    assert!(e.resize(390, H, 1.0));
    t += 16;
    e.pump(t);
    assert_eq!(e.pixel_size(), (390, H));
    assert_eq!(e.side_pixel_size(), (0, 0));
    assert_eq!(frameset_events(&mut e).last().unwrap()["on"], false);
    assert_eq!(e.page_depth(), 2);
    assert!(e.back());
    t += 16;
    e.pump(t);
    assert!((e.scroll_position() - 300.0).abs() < 1.0, "p 停在左栏滚到的 300：{}", e.scroll_position());
}

#[test]
fn 先进下一页再拉宽_左栏按新宽重画上一页() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture("repaint");
    // 参照：一开始就 900 宽打开 p（一栏 450 宽）
    let mut reference = engine(&root, 900, r#"{"frameset":"auto"}"#);
    assert_eq!(reference.pixel_size(), (450, H), "一打开就够宽：从一开始就是一栏宽");
    let want = reference.rgba().to_vec();
    drop(reference);

    let mut e = engine(&root, 390, r#"{"frameset":"auto"}"#);
    e.navigate("/pages/q/q").unwrap();
    e.pump(20);
    assert!(e.resize(900, H, 1.0));
    e.pump(40);
    assert_eq!(e.side_pixel_size(), (450, H));
    assert_eq!(side(&mut e), want, "左栏 = 按一栏宽打开的 p（逐字节）");
    assert_eq!(px(e.rgba(), 450, 225, 320), [0xFF, 0xFF, 0xFF], "活页面仍是 q");
    // 再返回：p 回到左栏、右栏占位
    assert!(e.back());
    e.pump(60);
    assert_eq!(e.rgba(), &want[..], "回到 p：与参照相同");
    assert_eq!(px(&side(&mut e), 450, 225, 320), [0xF7, 0xF7, 0xF7]);
}
