//! 同层渲染的**通用**组件（创建选项 `sameLayer: ["map"]`）：宿主把控件（地图）画进离屏缓冲，帧交给引擎画进页面。
//!
//! 判据（与 `same_layer_video_e2e.rs` 同一组，换成没有播放器的组件）：
//! 1. 宿主拿得到页面上有哪些同层组件（`sameLayerViews`：键、标签、属性、设备像素 `box`），而且**不再**
//!    当成要盖上去的控件报（`nativeViews` 里没有它）—— 那条路滑动时控件跟页面错开、一顿一顿；
//! 2. 交来一帧，**只重画那个框**（`FrameKind::Damage`），画面拉满框；每一帧与整帧重画逐字节相同；
//! 3. 滚动时地图就是页面画布里的一块像素：只合成 / 补条带、不重画，跟着页面挪整整那么多像素，宿主不用交新帧；
//! 4. 点在框里发 `nativeViewTap`（视图本地坐标），宿主回传的 `markertap` 派发给节点的 `bindmarkertap`；
//!    框外的点不发。
//!
//! 帧存在进程级的表里，两个引擎共用同一个键 —— 参照引擎正好拿到同一帧。本文件的用例串行跑。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use mini_render::renderer::components::host_view;
use serde_json::Value;
use std::path::PathBuf;

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
const W: u32 = 375;
const H: u32 = 667;
const DPR: f32 = 2.0;
/// 地图框（逻辑像素，页面内容坐标）：上面 120 的卡片，左右各 20
const MX: f32 = 20.0;
const MY: f32 = 120.0;
const MW: f32 = 335.0;
const MH: f32 = 200.0;

fn fixture(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("same-layer-view").join(name);
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="hd">卡片标题</view>
<map id="map1" class="map" latitude="22.54" longitude="114.05" bindmarkertap="onMarker"></map>
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>"#;
    let wxss = "page { background-color: #ffffff; }
.hd { height: 120px; background-color: #f0f0f0; }
.map { margin-left: 20px; width: 335px; height: 200px; background-color: #e8eef5; }
.row { height: 60px; border-bottom: 1px solid #eeeeee; }";
    let js = "Page({ data: { rows: Array.from({length: 30}, (_, i) => i + 1), ev: [] },
  onMarker(e) { this.data.ev.push('marker' + e.detail.markerId); } });";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", wxml),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", js),
        ("pages/p/p.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, no_damage: bool) -> MiniEngine {
    let opts = EngineOptions {
        // 两张表都列了：同层渲染优先，map 不再当成要盖上去的控件
        native_components: vec!["map".into()],
        same_layer: vec!["map".into()],
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

/// 宿主画的一帧：上半红、下半蓝（看得出拉满、上下没颠倒），尺寸 = box
fn frame(w: usize, h: usize) -> Vec<u8> {
    (0..w * h).flat_map(|i| if i / w < h / 2 { [255, 0, 0, 255] } else { [0, 0, 255, 255] }).collect()
}

/// 视口里 (x, y) 处的像素（逻辑像素）
fn px(e: &mut MiniEngine, x: f32, y: f32) -> [u8; 3] {
    let (w, _) = e.pixel_size();
    let i = (((y * DPR) as usize) * w as usize + (x * DPR) as usize) * 4;
    let p = e.rgba();
    [p[i], p[i + 1], p[i + 2]]
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
    let views = evs.iter().find(|v| v["type"] == "sameLayerViews").expect("要报页面上的同层组件");
    let v = &views["views"][0];
    assert_eq!((v["key"].as_str(), v["tag"].as_str()), (Some("map1"), Some("map")));
    assert_eq!(v["attrs"]["latitude"], "22.54");
    assert_eq!(v["box"], serde_json::json!([MW * DPR, MH * DPR]), "宿主按设备像素画：{v}");
    assert!(
        !evs.iter().any(|e| e["type"] == "nativeViews" && !e["views"].as_array().unwrap().is_empty()),
        "同层渲染的地图不该再当成盖上去的控件报"
    );
    events(&mut b);
    // 还没有帧：框的背景照常画
    assert_eq!(px(&mut a, MX + 30.0, MY + 30.0), [0xe8, 0xee, 0xf5]);

    // 交一帧（按 box 画的）：只重画那个框，拉满
    let (bw, bh) = ((MW * DPR) as usize, (MH * DPR) as usize);
    assert!(host_view::push_rgba("map1", bw as u32, bh as u32, bw * 4, &frame(bw, bh)));
    let k = step(&mut a, &mut b, "第一帧画面");
    assert_eq!(k, FrameKind::Damage, "新帧只重画地图框，不能整帧");
    assert_eq!(px(&mut a, MX + 1.0, MY + 1.0), [255, 0, 0], "左上角是帧的左上角（拉满）");
    assert_eq!(px(&mut a, MX + MW - 1.0, MY + MH - 1.0), [0, 0, 255], "右下角是帧的右下角");
    assert_eq!(px(&mut a, MX + 40.0, MY - 10.0), [0xf0, 0xf0, 0xf0], "框外一个像素都没动");
    assert_eq!(step(&mut a, &mut b, "静止"), FrameKind::Idle, "没新帧什么都不画");

    // 滚动：只合成 / 补条带；地图跟着页面挪整整 50，宿主一帧都不用交
    let before: Vec<[u8; 3]> = (0..250).map(|i| px(&mut a, MX + 40.0, i as f32 * 2.0)).collect();
    a.scroll_to(50.0);
    b.scroll_to(50.0);
    let k = step(&mut a, &mut b, "滚动");
    assert!(matches!(k, FrameKind::Compose | FrameKind::Band), "滚动只合成 / 补条带，不重画地图：{k:?}");
    let after: Vec<[u8; 3]> = (0..225).map(|i| px(&mut a, MX + 40.0, i as f32 * 2.0)).collect();
    assert_eq!(&after[..], &before[25..250], "地图（和它上面的卡片）一起往上挪了 50");

    // 内容变了（宿主重画了一帧）：还是只重画那个框
    let green: Vec<u8> = (0..bw * bh).flat_map(|_| [0u8, 255, 0, 255]).collect();
    assert!(host_view::push_rgba("map1", bw as u32, bh as u32, bw * 4, &green));
    assert_eq!(step(&mut a, &mut b, "第二帧画面"), FrameKind::Damage);
    assert_eq!(px(&mut a, MX + 40.0, MY - 50.0 + 30.0), [0, 255, 0]);
    host_view::retain(&[]);
}

#[test]
fn 点在框里交给宿主_回传事件派发给节点的绑定() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("tap");
    let mut e = engine(&root, false);
    e.pump_ex(1000);
    events(&mut e);
    // 点在框里：视图本地坐标
    e.pointer_down(MX + 100.0, MY + 60.0);
    e.pump_ex(1016);
    e.pointer_up(MX + 100.0, MY + 60.0);
    e.pump_ex(1032);
    let evs = events(&mut e);
    let tap = evs.iter().find(|v| v["type"] == "nativeViewTap").expect("点在框里要发 nativeViewTap");
    assert_eq!(tap["key"], "map1");
    assert!((tap["x"].as_f64().unwrap() - 100.0).abs() < 0.5 && (tap["y"].as_f64().unwrap() - 60.0).abs() < 0.5, "{tap}");
    // 点在框外：不发
    e.pointer_down(MX + 100.0, MY + MH + 30.0);
    e.pump_ex(1048);
    e.pointer_up(MX + 100.0, MY + MH + 30.0);
    e.pump_ex(1064);
    assert!(!events(&mut e).iter().any(|v| v["type"] == "nativeViewTap"), "框外不发");
    // 宿主判出点到了标记：回传 markertap → bindmarkertap
    assert!(e.dispatch_native_view_event("map1", "markertap", serde_json::json!({"markerId": 1})), "派发给 bindmarkertap");
    assert_eq!(e.eval("JSON.stringify(getCurrentPages()[0].data.ev)").unwrap(), r#"["marker1"]"#);
    host_view::retain(&[]);
}

/// 宿主点名 `sameLayerGestures: ["map"]`（真地图要自己平移 / 捏合缩放）：
/// 1. 按在地图上的这一串触摸（含第二根手指）整串交给宿主：`nativeViewTouch` 的 start / move / end，框内逻辑坐标；
/// 2. 页面一个像素不滚、不派 `nativeViewTap`（点击由宿主从这串触摸里自己判）；
/// 3. 按在地图外照旧滚页面；地图写了 `enable-scroll="false" enable-zoom="false"` 的不接手势；
/// 4. `markers` 这类数组属性交给宿主的是**标准 JSON**（文字里带撇号也不坏）。
#[test]
fn 地图自己的手势_整串触摸交给宿主_页面不滚_数组属性是标准json() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("gesture");
    std::fs::write(
        root.join("pages/p/p.wxml"),
        r#"<view class="hd">卡片标题</view>
<map id="map1" class="map" latitude="22.54" longitude="114.05" markers="{{markers}}"></map>
<map id="map2" class="map" enable-scroll="false" enable-zoom="false"></map>
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>"#,
    )
    .unwrap();
    std::fs::write(
        root.join("pages/p/p.js"),
        r#"Page({ data: { rows: Array.from({length: 30}, (_, i) => i + 1),
  markers: [{ id: 7, latitude: 22.54, longitude: 114.05, callout: { content: "Tom's 咖啡" } }] } });"#,
    )
    .unwrap();
    let opts = EngineOptions {
        same_layer: vec!["map".into()],
        same_layer_gestures: vec!["map".into()],
        ..Default::default()
    };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&root.join("_data").to_string_lossy()), W, H, DPR, HostEnv::default(), opts)
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e.pump_ex(1000);
    let evs = events(&mut e);
    let views = evs.iter().find(|v| v["type"] == "sameLayerViews").expect("sameLayerViews");
    let m = views["views"].as_array().unwrap().iter().find(|v| v["key"] == "map1").unwrap();
    let markers: Value = serde_json::from_str(m["attrs"]["markers"].as_str().unwrap()).expect("markers 是标准 JSON");
    assert_eq!(markers[0]["callout"]["content"], "Tom's 咖啡");
    assert_eq!(markers[0]["id"], 7);

    // 拖地图：整串交给宿主，页面不滚
    let (x0, y0) = (MX + 100.0, MY + 150.0);
    e.pointer_down(x0, y0);
    e.pump_ex(1016);
    for i in 1..=6 {
        e.pointer_move(x0 + 5.0 * i as f32, y0 - 20.0 * i as f32);
        e.pump_ex(1016 + 16 * i);
    }
    e.pointer2(0, x0 + 80.0, y0 - 40.0);
    e.pointer2(1, x0 + 90.0, y0 - 50.0);
    e.pointer2(2, x0 + 90.0, y0 - 50.0);
    e.pointer_up(x0 + 30.0, y0 - 120.0);
    e.pump_ex(1200);
    let touches: Vec<Value> = events(&mut e).into_iter().filter(|v| v["type"] == "nativeViewTouch").collect();
    let phases: Vec<String> = touches.iter().map(|v| format!("{}{}", v["phase"].as_str().unwrap(), v["pointer"])).collect();
    assert_eq!(phases.first().map(String::as_str), Some("start0"), "{phases:?}");
    assert_eq!(phases.last().map(String::as_str), Some("end0"), "{phases:?}");
    assert_eq!(phases.iter().filter(|p| *p == "move0").count(), 6, "{phases:?}");
    assert!(phases.contains(&"start1".to_string()) && phases.contains(&"move1".to_string()) && phases.contains(&"end1".to_string()), "第二根手指也交给地图：{phases:?}");
    assert!(touches.iter().all(|v| v["key"] == "map1"));
    let s = &touches[0];
    assert!((s["x"].as_f64().unwrap() - 100.0).abs() < 0.5 && (s["y"].as_f64().unwrap() - 150.0).abs() < 0.5, "框内坐标：{s}");
    let last_move = touches.iter().filter(|v| v["phase"] == "move" && v["pointer"] == 0).last().unwrap();
    assert!((last_move["y"].as_f64().unwrap() - 30.0).abs() < 0.5, "原点在按下时定，页面没滚：{last_move}");
    assert!(e.scroll_position().abs() < 0.01, "拖地图页面不滚：{}", e.scroll_position());

    // 不接手势的地图上拖：照旧滚页面，不发 nativeViewTouch
    let (gx, gy) = (MX + 100.0, MY + MH + 100.0); // map2：不接手势的那张
    e.pointer_down(gx, gy);
    e.pump_ex(1500);
    for i in 1..=8 {
        e.pointer_move(gx, gy - 25.0 * i as f32);
        e.pump_ex(1500 + 16 * i);
    }
    e.pointer_up(gx, gy - 200.0);
    e.pump_ex(1700);
    assert!(!events(&mut e).iter().any(|v| v["type"] == "nativeViewTouch"), "enable-scroll / enable-zoom 都关了的地图不接手势");
    assert!(e.scroll_position() > 50.0, "按在不接手势的地图上拖：页面照滚 {}", e.scroll_position());
    host_view::retain(&[]);
}

/// WP-F1 map：自定义气泡（`slot="callout"` 里带 marker-id 的 cover-view）与 `setting`：
/// 1. 气泡离屏出图交给宿主：`sameLayerViews` 里这张地图带 `callouts [{markerId, path, width, height}]`，
///    path 是 PNG（宽高 = 逻辑宽高 × dpr），内容不变时路径不变；事件带 `wxfileRoot`；
/// 2. slot 不占流、不画在页面上（地图框里那块还是地图的底色，不是气泡的红底）；
/// 3. `setting {enableScroll:false, enableZoom:false}` 盖过属性：这张地图不接手势，拖动照滚页面。
#[test]
fn f1_自定义气泡出图交给宿主_slot_不画在页面上_setting_盖过属性() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("callout");
    std::fs::write(
        root.join("pages/p/p.wxml"),
        r#"<view class="hd">卡片标题</view>
<map id="map1" class="map" latitude="22.54" longitude="114.05" setting="{{setting}}">
  <cover-view slot="callout">
    <cover-view marker-id="1" class="bubble">气泡一</cover-view>
    <cover-view marker-id="2" class="bubble wide">第二个气泡更宽</cover-view>
  </cover-view>
</map>
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>"#,
    )
    .unwrap();
    std::fs::write(
        root.join("pages/p/p.wxss"),
        "page { background-color: #ffffff; }
.hd { height: 120px; background-color: #f0f0f0; }
.map { margin-left: 20px; width: 335px; height: 200px; background-color: #e8eef5; }
.bubble { width: 80px; height: 30px; background-color: #ff0000; color: #ffffff; }
.wide { width: 120px; }
.row { height: 60px; }",
    )
    .unwrap();
    std::fs::write(
        root.join("pages/p/p.js"),
        "Page({ data: { rows: Array.from({length: 30}, (_, i) => i + 1), setting: { enableScroll: false, enableZoom: false } } });",
    )
    .unwrap();
    let opts = EngineOptions {
        same_layer: vec!["map".into()],
        same_layer_gestures: vec!["map".into()],
        ..Default::default()
    };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&root.join("_data").to_string_lossy()), W, H, DPR, HostEnv::default(), opts)
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e.pump_ex(1000);
    let evs = events(&mut e);
    let ev = evs.iter().find(|v| v["type"] == "sameLayerViews").expect("sameLayerViews");
    assert!(ev["wxfileRoot"].as_str().is_some_and(|s| s.contains("wxfile")), "{ev}");
    let m = &ev["views"][0];
    let cs = m["callouts"].as_array().unwrap_or_else(|| panic!("要带 callouts：{m}"));
    assert_eq!(cs.len(), 2, "{m}");
    assert_eq!(cs[0]["markerId"], 1, "markerId 是 number");
    assert_eq!((cs[0]["width"].as_f64(), cs[0]["height"].as_f64()), (Some(80.0), Some(30.0)), "{cs:?}");
    assert_eq!(cs[1]["markerId"], 2);
    assert_eq!(cs[1]["width"].as_f64(), Some(120.0), "每个气泡按自己的宽：{cs:?}");
    let png = image::open(cs[0]["path"].as_str().unwrap()).expect("PNG").to_rgba8();
    assert_eq!((png.width(), png.height()), (160, 60), "物理像素 = 逻辑 × dpr");
    let p = png.get_pixel(4, 50);
    assert_eq!((p[0], p[1], p[2], p[3]), (255, 0, 0, 255), "气泡的红底画进了图");
    // slot 不画在页面上：地图框左上那块仍是地图底色
    assert_eq!(px(&mut e, MX + 10.0, MY + 10.0), [0xe8, 0xee, 0xf5], "slot=callout 不该画在页面上");

    // setting 关了拖动与缩放：不接手势，页面照滚
    let (gx, gy) = (MX + 100.0, MY + 150.0);
    e.pointer_down(gx, gy);
    e.pump_ex(1500);
    for i in 1..=8 {
        e.pointer_move(gx, gy - 25.0 * i as f32);
        e.pump_ex(1500 + 16 * i);
    }
    e.pointer_up(gx, gy - 200.0);
    e.pump_ex(1700);
    assert!(!events(&mut e).iter().any(|v| v["type"] == "nativeViewTouch"), "setting 关掉的地图不接手势");
    assert!(e.scroll_position() > 50.0, "页面照滚：{}", e.scroll_position());
    host_view::retain(&[]);
}

/// F2 camera / voip-room（同层渲染，`sameLayer: ["camera", "voip-room"]`）：
/// 1. camera 进 `sameLayerViews`（宿主据此开相机），默认 100% × 300px；
/// 2. `onCameraFrame` 由引擎从宿主交来的预览里取：`frame-size="small"` 长边 360，`data` 是 `ArrayBuffer`、长度 = 宽 × 高 × 4；
/// 3. `mode="scanCode"`：预览里有二维码 → `bindscancode`（result 是码里的字），同一个码不重复发；条形码（EAN-13）也认；
/// 4. voip-room `mode="video"`：进不了房，`binderror` 只派一次（errCode 是 number）；
/// 5. canIUse：宿主接了的 camera / voip-room 为 true，没接的 live-pusher 为 false。
#[test]
fn f2_相机帧走引擎_扫码_voip_room_进不了房() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("camera");
    std::fs::write(
        root.join("pages/p/p.wxml"),
        r#"<camera id="cam" mode="scanCode" frame-size="small" bindscancode="onScan"></camera>
<voip-room openid="o1" mode="video" binderror="onVoip"></voip-room>"#,
    )
    .unwrap();
    std::fs::write(root.join("pages/p/p.wxss"), "page { background-color: #ffffff; }").unwrap();
    std::fs::write(
        root.join("pages/p/p.js"),
        r#"Page({ data: { frames: [], scans: [], voip: [], started: false },
  onLoad() {
    const ctx = wx.createCameraContext();
    this.listener = ctx.onCameraFrame(f => this.data.frames.push([f.width, f.height, f.data.byteLength, f.data instanceof ArrayBuffer]));
    this.listener.start({ success: () => { this.data.started = true; } });
  },
  onScan(e) { this.data.scans.push(e.detail.result + '|' + e.detail.type); },
  onVoip(e) { this.data.voip.push(typeof e.detail.errCode); } });"#,
    )
    .unwrap();
    let opts = EngineOptions { same_layer: vec!["camera".into(), "voip-room".into()], ..Default::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&root.join("_data").to_string_lossy()), W, H, DPR, HostEnv::default(), opts)
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    let mut t = 1000u64;
    e.pump_ex(t);
    let evs = events(&mut e);
    let views = evs.iter().find(|v| v["type"] == "sameLayerViews").expect("sameLayerViews");
    let cam = views["views"].as_array().unwrap().iter().find(|v| v["tag"] == "camera").expect("camera 交给宿主");
    assert_eq!(cam["key"], "cam");
    assert_eq!(cam["box"], serde_json::json!([W as f32 * DPR, 300.0 * DPR]), "默认 100% × 300px：{cam}");
    assert_eq!(e.eval("String(getCurrentPages()[0].data.started)").unwrap(), "true", "listener.start 成功");
    assert_eq!(e.eval("String(wx.canIUse('camera')) + wx.canIUse('voip-room') + wx.canIUse('live-pusher')").unwrap(), "truetruefalse");

    // 宿主交一帧预览：白底上一个二维码
    let (bw, bh) = ((W as f32 * DPR) as usize, (300.0 * DPR) as usize);
    let code = qrcode::QrCode::new(b"https://example.com/f2").unwrap();
    let mods = code.width();
    let colors = code.to_colors();
    let scale = 400 / (mods + 8);
    let (ox, oy) = ((bw - mods * scale) / 2, (bh - mods * scale) / 2);
    let mut px = vec![255u8; bw * bh * 4];
    for my in 0..mods {
        for mx in 0..mods {
            if colors[my * mods + mx] == qrcode::Color::Dark {
                for y in oy + my * scale..oy + (my + 1) * scale {
                    for x in ox + mx * scale..ox + (mx + 1) * scale {
                        px[(y * bw + x) * 4..(y * bw + x) * 4 + 3].copy_from_slice(&[0, 0, 0]);
                    }
                }
            }
        }
    }
    let started = std::time::Instant::now();
    let mut pushes = 0;
    // 解码在后台线程：按墙上时钟等，最多 5 秒
    while started.elapsed().as_secs() < 5 {
        if pushes < 3 {
            assert!(host_view::push_rgba("cam", bw as u32, bh as u32, bw * 4, &px));
            pushes += 1;
        }
        t += 300;
        e.pump_ex(t);
        if e.eval("String(getCurrentPages()[0].data.scans.length)").unwrap() != "0" && pushes >= 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    // 再给两轮：同一个码在 2 秒内不重复发
    for _ in 0..3 {
        assert!(host_view::push_rgba("cam", bw as u32, bh as u32, bw * 4, &px));
        t += 300;
        e.pump_ex(t);
        std::thread::sleep(std::time::Duration::from_millis(60));
        e.pump_ex(t + 1);
    }
    assert_eq!(e.eval("JSON.stringify(getCurrentPages()[0].data.scans)").unwrap(), r#"["https://example.com/f2|QR_CODE"]"#);
    // 条形码（EAN-13）：同一个相机、换一帧画面
    let bars = {
        use rxing::Writer;
        rxing::MultiFormatWriter.encode("6901234567892", &rxing::BarcodeFormat::EAN_13, 600, 200).expect("编条形码")
    };
    let mut px2 = vec![255u8; bw * bh * 4];
    let (ox2, oy2) = ((bw - bars.getWidth() as usize) / 2, (bh - bars.getHeight() as usize) / 2);
    for yy in 0..bars.getHeight() {
        for xx in 0..bars.getWidth() {
            if bars.get(xx, yy) {
                let i = ((oy2 + yy as usize) * bw + ox2 + xx as usize) * 4;
                px2[i..i + 3].copy_from_slice(&[0, 0, 0]);
            }
        }
    }
    let started = std::time::Instant::now();
    while started.elapsed().as_secs() < 5 {
        assert!(host_view::push_rgba("cam", bw as u32, bh as u32, bw * 4, &px2));
        t += 300;
        e.pump_ex(t);
        std::thread::sleep(std::time::Duration::from_millis(40));
        e.pump_ex(t + 1);
        if e.eval("String(getCurrentPages()[0].data.scans.length)").unwrap() == "2" {
            break;
        }
    }
    assert_eq!(
        e.eval("JSON.stringify(getCurrentPages()[0].data.scans[1])").unwrap(),
        r#""6901234567892|EAN_13""#,
        "条形码也认，码制名与 wx.scanCode 的 scanType 一样"
    );
    let frames = e.eval("JSON.stringify(getCurrentPages()[0].data.frames[0])").unwrap();
    assert_eq!(frames, "[360,288,414720,true]", "small：长边 360，RGBA 的 ArrayBuffer");
    assert_eq!(e.eval("JSON.stringify(getCurrentPages()[0].data.voip)").unwrap(), r#"["number"]"#, "voip-room 只报一次错");
    host_view::retain(&[]);
}
