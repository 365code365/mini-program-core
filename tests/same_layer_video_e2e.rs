//! 同层渲染的 `<video>`（创建选项 `sameLayer: ["video"]`）：宿主只解码，帧交给引擎画进页面。
//!
//! 判据：
//! 1. 宿主拿得到页面上有哪些视频（`videos` 事件：键、解析好的本地路径、属性），而且**不再**当成
//!    要盖上去的控件报（`nativeViews` 里没有它）；
//! 2. 交来一帧，**只重画那个框**（`FrameKind::Damage`），画面按 `object-fit` 落在框里；
//!    每一帧都与整帧重画（`MINI_NO_DAMAGE` 的参照引擎）逐字节相同；
//! 3. 滚动时视频就是页面画布里的一块像素：只合成、不重画，画面跟着页面一起挪整整那么多像素
//!    —— 这正是盖上去的控件做不到的（两条上屏路，一拖就错开）；
//! 4. 点画面发 `videoCommand play`、`wx.createVideoContext().pause()` 发 `videoCommand pause`；
//!    宿主回传 `play` 派发给 `bindplay`，并把控制条 / 中间的播放键换成播放中的样子。
//!
//! 帧与状态存在进程级的表里（宿主在解码线程上交帧），两个引擎共用同一个键 —— 参照引擎正好
//! 拿到同一帧。本文件的用例串行跑。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use mini_render::renderer::components::host_feed;
use serde_json::Value;
use std::path::PathBuf;

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
const W: u32 = 375;
const H: u32 = 667;
const DPR: f32 = 2.0;
/// 视频框（逻辑像素，页面内容坐标）：上面 120 的卡片，左右各 20
const VX: f32 = 20.0;
const VY: f32 = 120.0;
const VW: f32 = 300.0;
const VH: f32 = 200.0;

fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("same-layer-video").join(name);
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="hd">卡片标题</view>
<video id="v" class="v" src="/assets/clip.mp4" bindplay="onPlay" bindpause="onPause" bindtimeupdate="onTime"></video>
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>"#;
    let wxss = "page { background-color: #ffffff; }
.hd { height: 120px; background-color: #f0f0f0; }
.v { margin-left: 20px; width: 300px; height: 200px; }
.row { height: 60px; border-bottom: 1px solid #eeeeee; }";
    let js = "Page({ data: { rows: Array.from({length: 30}, (_, i) => i + 1), ev: [] },
  onPlay(e) { this.data.ev.push('play'); }, onPause(e) { this.data.ev.push('pause'); },
  onTime(e) { this.data.ev.push('t' + e.detail.currentTime); } });";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", wxml),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", js),
        ("pages/p/p.json", "{}"),
        // 只是让 `url` 解析得到一个真实文件（引擎不读它）
        ("assets/clip.mp4", "not really a video"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, no_damage: bool) -> MiniEngine {
    let opts = EngineOptions {
        // 两张表都列了：同层渲染优先，video 不再当成要盖上去的控件
        native_components: vec!["video".into()],
        same_layer: vec!["video".into()],
        ..Default::default()
    };
    let env = HostEnv { no_damage, ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&root.join("_data").to_string_lossy()), W, H, DPR, env, opts)
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e
}

fn events(e: &mut MiniEngine) -> Vec<Value> {
    std::iter::from_fn(|| e.poll_event()).map(|s| serde_json::from_str(&s).unwrap()).collect()
}

/// 纯色帧（RGBA），4:3
fn solid(rgb: [u8; 3]) -> Vec<u8> {
    (0..8 * 6).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect()
}

/// 视口里 (x, y) 处的像素（逻辑像素）
fn px(e: &mut MiniEngine, x: f32, y: f32) -> [u8; 3] {
    let (w, _) = e.pixel_size();
    let i = (((y * DPR) as usize) * w as usize + (x * DPR) as usize) * 4;
    let p = e.rgba();
    [p[i], p[i + 1], p[i + 2]]
}

fn red(p: [u8; 3]) -> bool {
    p[0] > 240 && p[1] < 20 && p[2] < 20
}

#[test]
fn 交帧只重画那个框_与整帧逐字节相同_滚动时跟着页面走() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("frames");
    let mut a = engine(&root, false);
    let mut b = engine(&root, true);
    let mut t = 1000u64;
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str| -> FrameKind {
        t += 16;
        a.pump_ex(t);
        b.pump_ex(t);
        assert!(a.rgba() == b.rgba(), "{what}：局部重画与整帧重画不一致");
        a.last_frame_kind()
    };
    step(&mut a, &mut b, "首帧");
    let evs = events(&mut a);
    let videos = evs.iter().find(|v| v["type"] == "videos").expect("要报页面上的视频");
    let v = &videos["views"][0];
    assert_eq!(v["key"], "v");
    assert_eq!(v["attrs"]["src"], "/assets/clip.mp4");
    assert!(v["url"].as_str().unwrap().ends_with("assets/clip.mp4") && v["url"].as_str().unwrap().starts_with('/'), "包内路径解析成绝对路径：{v}");
    assert!(
        !evs.iter().any(|e| e["type"] == "nativeViews" && !e["views"].as_array().unwrap().is_empty()),
        "同层渲染的视频不该再当成盖上去的控件报"
    );
    events(&mut b);
    // 还没有帧：黑底 + 中间的播放键
    assert_eq!(px(&mut a, VX + 30.0, VY + 30.0), [0, 0, 0]);

    // 交一帧：只重画那个框
    assert!(host_feed::push_rgba("v", 8, 6, 32, &solid([255, 0, 0])));
    let k = step(&mut a, &mut b, "第一帧画面");
    assert_eq!(k, FrameKind::Damage, "新帧只重画视频框，不能整帧");
    // 4:3 的帧 aspectFit 进 300×200：高顶满、左右各留 16.7 的黑边
    assert!(red(px(&mut a, VX + 40.0, VY + 30.0)), "画面画进框里：{:?}", px(&mut a, VX + 40.0, VY + 30.0));
    assert_eq!(px(&mut a, VX + 5.0, VY + 30.0), [0, 0, 0], "aspectFit 的黑边");
    assert_eq!(px(&mut a, VX + 40.0, VY - 10.0), [0xf0, 0xf0, 0xf0], "框外一个像素都没动");
    // 没帧变化：什么都不画
    assert_eq!(step(&mut a, &mut b, "静止"), FrameKind::Idle);

    // 滚动：只合成；视频跟着页面挪整整 100 —— 与卡片在同一张图里
    let before: Vec<[u8; 3]> = (0..200).map(|i| px(&mut a, VX + 40.0, i as f32 * 2.0 + 100.0)).collect();
    a.scroll_to(50.0);
    b.scroll_to(50.0);
    // （裁剪窗边上缺的几行会顺手补上：补条带，不是重画视频）
    let k = step(&mut a, &mut b, "滚动");
    assert!(matches!(k, FrameKind::Compose | FrameKind::Band), "滚动只合成 / 补条带，不重画视频：{k:?}");
    let after: Vec<[u8; 3]> = (0..175).map(|i| px(&mut a, VX + 40.0, i as f32 * 2.0 + 50.0)).collect();
    assert_eq!(&after[..], &before[..175], "视频（和它上面的卡片）一起往上挪了 50");

    // 第二帧（绿）：还是只重画那个框
    assert!(host_feed::push_rgba("v", 8, 6, 32, &solid([0, 255, 0])));
    assert_eq!(step(&mut a, &mut b, "第二帧画面"), FrameKind::Damage);
    assert_eq!(px(&mut a, VX + 40.0, VY - 50.0 + 30.0), [0, 255, 0]);

    // 宿主回传 play：控制条 / 中间播放键换成播放中，只重画那个框
    let center = (VX + VW / 2.0, VY - 50.0 + VH / 2.0);
    let paused = px(&mut a, center.0, center.1);
    assert_ne!(paused, [0, 255, 0], "暂停着：画面正中盖着播放键");
    assert!(a.dispatch_native_view_event("v", "play", serde_json::json!({})), "派发给 bindplay");
    b.dispatch_native_view_event("v", "play", serde_json::json!({}));
    assert_eq!(step(&mut a, &mut b, "开始播放"), FrameKind::Damage);
    assert_eq!(px(&mut a, center.0, center.1), [0, 255, 0], "播放中：中间的播放键收起");
    assert_eq!(a.eval("JSON.stringify(getCurrentPages()[0].data.ev)").unwrap(), r#"["play"]"#);
    host_feed::retain(&[]);
}

#[test]
fn 点画面与_videocontext_的命令交给宿主执行() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("commands");
    let mut e = engine(&root, false);
    e.pump_ex(1000);
    events(&mut e);
    let commands = |e: &mut MiniEngine, t: u64| -> Vec<Value> {
        e.pump_ex(t);
        events(e).into_iter().filter(|v| v["type"] == "videoCommand").collect()
    };
    // 暂停着点画面：起播
    e.pointer_down(VX + 150.0, VY + 80.0);
    e.pump_ex(1016);
    e.pointer_up(VX + 150.0, VY + 80.0);
    let c = commands(&mut e, 1032);
    assert_eq!(c.len(), 1, "{c:?}");
    assert_eq!((c[0]["key"].as_str(), c[0]["command"].as_str()), (Some("v"), Some("play")));
    // 宿主回了 play（起播了）：再点控制条左侧的播放键是暂停
    e.dispatch_native_view_event("v", "loadedmetadata", serde_json::json!({"duration": 10, "width": 8, "height": 6}));
    e.dispatch_native_view_event("v", "play", serde_json::json!({}));
    e.pointer_down(VX + 12.0, VY + VH - 18.0);
    e.pump_ex(1048);
    e.pointer_up(VX + 12.0, VY + VH - 18.0);
    let c = commands(&mut e, 1064);
    assert_eq!(c.first().and_then(|c| c["command"].as_str()), Some("pause"), "{c:?}");
    // 点进度轨道正中：跳到一半
    let (t0, tw) = mini_render::renderer::components::progress_track(VW);
    let x = VX + t0 + tw / 2.0;
    e.pointer_down(x, VY + VH - 18.0);
    e.pump_ex(1080);
    e.pointer_up(x, VY + VH - 18.0);
    let c = commands(&mut e, 1096);
    assert_eq!(c.first().and_then(|c| c["command"].as_str()), Some("seek"), "{c:?}");
    assert!((c[0]["position"].as_f64().unwrap() - 5.0).abs() < 0.1, "{c:?}");
    // VideoContext
    e.eval("wx.createVideoContext('v').pause()").unwrap();
    let c = commands(&mut e, 1112);
    assert_eq!(c.first().and_then(|c| c["command"].as_str()), Some("pause"), "{c:?}");
    e.eval("wx.createVideoContext('v').seek(3)").unwrap();
    let c = commands(&mut e, 1128);
    assert_eq!((c[0]["command"].as_str(), c[0]["position"].as_f64()), (Some("seek"), Some(3.0)));
    host_feed::retain(&[]);
}

/// 全屏（横屏）：宿主收到 `videoFullscreen` 与按全屏尺寸（宽高对调）读回的 `box`；交来的帧只重新合成、
/// 页面不重画；画面转过来画（画面里的点落在屏幕上对应的位置）；返回键退出全屏、`box` 换回页面里的框
#[test]
fn 全屏_横屏_宿主按对调的尺寸交帧_只重新合成() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("fullscreen");
    let mut e = engine(&root, false);
    let mut t = 1000u64;
    let mut pump = |e: &mut MiniEngine| -> (FrameKind, Vec<Value>) {
        t += 16;
        e.pump_ex(t);
        (e.last_frame_kind(), events(e))
    };
    pump(&mut e);
    // 4:3 的画面：不给方向就横屏（90）
    e.dispatch_native_view_event("v", "loadedmetadata", serde_json::json!({"duration": 10, "width": 8, "height": 6}));
    pump(&mut e);
    e.eval("wx.createVideoContext('v').requestFullScreen()").unwrap();
    let (_, evs) = pump(&mut e);
    let fs = evs.iter().find(|v| v["type"] == "videoFullscreen").expect("宿主要收起状态栏");
    assert_eq!((fs["fullScreen"].as_bool(), fs["direction"].as_i64(), fs["key"].as_str()), (Some(true), Some(90), Some("v")));
    let videos = evs.iter().find(|v| v["type"] == "videos").expect("全屏换了读回尺寸，要重报 videos");
    let v = &videos["views"][0];
    assert_eq!(v["box"], serde_json::json!([(H as f32 * DPR), (W as f32 * DPR)]), "横屏：宽高对调");
    assert_eq!(v["attrs"]["object-fit"], "contain");
    // 宿主按 box 与 contain 读回：8×6 → 1000×750（高顶满）
    let (fw, fh) = (1000u32, 750u32);
    let frame: Vec<u8> = (0..fw * fh).flat_map(|_| [255u8, 0, 0, 255]).collect();
    assert!(host_feed::push_rgba("v", fw, fh, fw as usize * 4, &frame));
    let (k, _) = pump(&mut e);
    assert_eq!(k, FrameKind::Compose, "全屏的新帧只重新合成，页面一个像素不画");
    // 画面 (200, 100)（避开顶栏、控制条与中间的播放键）在横屏时落在屏幕 (W - 100, 200)
    assert!(red(px(&mut e, W as f32 - 100.0, 200.0)), "{:?}", px(&mut e, W as f32 - 100.0, 200.0));
    // 画面左右的黑边：屏幕最上面那一截
    assert_eq!(px(&mut e, 100.0, 20.0), [0, 0, 0]);
    // 返回键：退出全屏（不离开页面），宿主放出状态栏，读回尺寸换回页面里的框
    assert!(e.back());
    let (_, evs) = pump(&mut e);
    let fs = evs.iter().find(|v| v["type"] == "videoFullscreen").expect("宿主要放出状态栏");
    assert_eq!(fs["fullScreen"].as_bool(), Some(false));
    let videos = evs.iter().find(|v| v["type"] == "videos").expect("要重报 videos");
    assert_eq!(videos["views"][0]["box"], serde_json::json!([VW * DPR, VH * DPR]));
    assert!(videos["views"][0]["attrs"].get("object-fit").is_none());
    host_feed::retain(&[]);
}

#[test]
fn 宿主交来的_yuv_帧照样画出来() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("yuv");
    let mut e = engine(&root, false);
    e.pump_ex(1000);
    events(&mut e);
    // BT.601 的红（Y=81 U=90 V=240），4:3，U / V 交错（Android 常见）
    let (w, h) = (8usize, 6usize);
    let y = vec![81u8; w * h];
    let uv: Vec<u8> = (0..(w / 2) * (h / 2)).flat_map(|_| [90u8, 240]).collect();
    let f = host_feed::Yuv420 { width: 8, height: 6, y: &y, y_stride: w, u: &uv, v: &uv[1..], uv_stride: w, uv_pixel_stride: 2 };
    assert!(host_feed::push_yuv("v", &f));
    assert_ne!(e.pump_ex(1016) & FRAME_NEW, 0);
    assert!(red(px(&mut e, VX + 40.0, VY + 30.0)), "{:?}", px(&mut e, VX + 40.0, VY + 30.0));
    host_feed::retain(&[]);
}
