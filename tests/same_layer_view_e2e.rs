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
