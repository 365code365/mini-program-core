//! 宿主承载的原生组件被引擎自己画的层挡住时，`nativeViews` 报的遮挡（`clip` / `holes` /
//! `mask`，见 `src/host/native_occlusion.rs`）。
//!
//! 现象：picker 面板弹起来，宿主的视频控件压在面板和遮罩上面（宿主不知道面板在哪，
//! 只能把控件摆在最上层）。判据：面板滑到位后，视频的 `clip` 下沿正好是面板顶边、
//! `mask` = 0.5；面板收起后原样露出来；弹窗盒子落在视频上时挖洞；吸底的 fixed 栏削掉下沿。
//!
//! 独立成一个测试二进制（不放进 `embed_host.rs`）：引擎的小程序根目录是进程级的，
//! 每个集成测试文件是单独的进程，夹具目录互不干扰。
use mini_render::compositor::HostEnv;
use mini_render::host::picker_sheet::SHEET_H;
use mini_render::host::{EngineOptions, MiniEngine};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

const W: f32 = 375.0;
const H: f32 = 667.0;

fn fixture(name: &str, wxml: &str, wxss: &str, js: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("embed-native-occlusion").join(name);
    std::fs::remove_dir_all(&root).ok();
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#),
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

/// 取完队列，返回最后一条 `nativeViews` 里的第一个控件（没有这条事件返回 None）
fn last_view(e: &mut MiniEngine) -> Option<Value> {
    let evs: Vec<Value> = std::iter::from_fn(|| e.poll_event())
        .map(|s| serde_json::from_str(&s).unwrap())
        .collect();
    evs.iter().rev().find(|v| v["type"] == "nativeViews").map(|v| v["views"][0].clone())
}

fn f(v: &Value, k: &str) -> f32 {
    v[k].as_f64().unwrap_or_else(|| panic!("缺 {k}：{v}")) as f32
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

/// 按墙上时钟等动画走完（picker 面板入场 / 退场 0.22s），期间照常 pump
fn settle(e: &mut MiniEngine, t: &mut u64) {
    for _ in 0..30 {
        e.pump(*t);
        *t += 16;
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn picker_面板_弹窗_吸底栏盖住原生视频时报遮挡() {
    // 视频 0..300 × 50..650；吸底栏 627..667 横跨整个视口
    let root = fixture(
        "occl",
        r#"<picker mode="selector" range="{{items}}" value="0"><view class="p">选</view></picker><video id="v" src="https://example.invalid/a.mp4" style="width:300px;height:600px"></video><view class="bar"></view>"#,
        ".p { width: 200px; height: 50px; } \
         .bar { position: fixed; left: 0; right: 0; bottom: 0; height: 40px; background-color: #ffffff; }",
        "Page({data:{items:['甲','乙','丙']}});",
    );
    let opts = EngineOptions { native_components: vec!["video".into()], ..Default::default() };
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(
        &root.to_string_lossy(),
        Some(&data.to_string_lossy()),
        W as u32,
        H as u32,
        1.0,
        HostEnv::default(),
        opts,
    )
    .expect("创建引擎");
    e.launch(None).expect("打开首页");
    let mut t = 1u64;
    e.pump(t);

    // ① 只有吸底栏：横跨视频整个宽度、压住下沿 → 直接削掉，不挖洞
    let v = last_view(&mut e).expect("要报原生组件");
    assert!(close(f(&v, "y"), 50.0) && close(f(&v, "height"), 600.0), "完整矩形照旧：{v}");
    let clip = &v["clip"];
    assert!(close(f(clip, "y"), 50.0) && close(f(clip, "height"), H - 40.0 - 50.0), "吸底栏削掉下沿：{v}");
    assert_eq!(v["holes"].as_array().map(|a| a.len()), Some(0), "{v}");
    assert_eq!(f(&v, "mask"), 0.0);

    // ② 点 picker 弹出面板：视频露出的部分止于面板顶边，并被遮罩压暗
    e.pointer_down(50.0, 20.0);
    e.pump(t + 16);
    e.pointer_up(50.0, 20.0);
    t += 32;
    settle(&mut e, &mut t);
    assert!(e.picker_sheet_open(), "面板没弹出来（坐标落空的话下面的断言全是空转）");
    let v = last_view(&mut e).expect("面板弹出时要重报遮挡");
    let clip = &v["clip"];
    let top = H - SHEET_H;
    assert!(close(f(clip, "y") + f(clip, "height"), top), "clip 下沿应是面板顶边 {top}：{v}");
    assert!(close(f(&v, "mask"), 0.5), "picker 遮罩 0.5：{v}");
    assert!(close(f(&v, "height"), 600.0), "完整矩形不变（宿主不该缩放控件）：{v}");

    // 静止就不再重报
    e.pump(t);
    t += 16;
    assert!(last_view(&mut e).is_none(), "面板静止后不该每帧重报");

    // ③ 点遮罩关面板：退场动画走完后视频原样露出来
    e.pointer_down(350.0, 20.0);
    e.pump(t);
    e.pointer_up(350.0, 20.0);
    t += 16;
    settle(&mut e, &mut t);
    assert!(!e.picker_sheet_open());
    let v = last_view(&mut e).expect("面板收起后要重报");
    assert!(close(f(&v["clip"], "height"), H - 40.0 - 50.0) && f(&v, "mask") == 0.0, "{v}");

    // ④ showModal：全屏遮罩 0.6（WeUI 2.x 的 weui-mask），居中的弹窗盒子在视频上挖洞（只挖与视频相交的那块）
    e.eval("wx.showModal({title:'t',content:'c'})").ok();
    e.pump(t);
    t += 16;
    assert!(e.modal_open());
    let v = last_view(&mut e).expect("弹窗出现时要重报");
    assert!(close(f(&v, "mask"), 0.6), "{v}");
    let holes = v["holes"].as_array().expect("holes");
    assert_eq!(holes.len(), 1, "弹窗盒子在视频中间：{v}");
    let hole = &holes[0];
    // 盒子按 WeUI 2.x 是 320 宽（视口 ≥ 352 时，见 ui_overlay/geometry.rs）居中：x = (375 - 320) / 2 = 27.5，
    // 视频右沿 300 处截断
    assert!(close(f(hole, "x"), 27.5) && close(f(hole, "width"), 300.0 - 27.5), "{v}");
    let _ = t;
}
