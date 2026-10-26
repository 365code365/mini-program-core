//! 原生 tabBar 与底部安全区：tabBar 底下垫一条 `safeArea.bottom`，图标文字在上面那 50px 里。
//!
//! 真机现象（荣耀 Android 15，三键导航，Flutter 宿主铺满整屏）：系统导航键占底部 38px，
//! 引擎的 tabBar 仍贴着画面底边画 50px，整条被导航键盖住、几乎点不到。
//! 判据：375×667 @2x、`safeArea.bottom = 34` 时 tabBar 占 583..667（白底），583 以上是页面（绿底）；
//! 点 y=600 切到第二个 tab（走 `pointer_down` / `pointer_up` 真实指针链路）；
//! 安全区改回 0 后 tabBar 回到 617..667。
//!
//! 夹具写在 `CARGO_TARGET_TMPDIR` 下，串行跑（小程序根目录是进程级的，见 embed_host.rs）。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine, SafeAreaInsets};
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

const GREEN: (u8, u8, u8) = (0x07, 0xc1, 0x60);
const WHITE: (u8, u8, u8) = (0xff, 0xff, 0xff);

fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("tabbar-safe-area");
    std::fs::remove_dir_all(&root).ok();
    let app_json = r##"{"pages":["pages/a/a","pages/b/b"],
        "tabBar":{"backgroundColor":"#ffffff","list":[
            {"pagePath":"pages/a/a","text":"甲"},{"pagePath":"pages/b/b","text":"乙"}]}}"##;
    let page_wxss = "page{background-color:#07c160;} .body{height:2000px;}";
    let files = [
        ("app.json", app_json),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/a/a.wxml", r#"<view class="body"></view>"#),
        ("pages/a/a.wxss", page_wxss),
        ("pages/a/a.js", "Page({});"),
        ("pages/b/b.wxml", r#"<view class="body"></view>"#),
        ("pages/b/b.wxss", page_wxss),
        ("pages/b/b.js", "Page({});"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, bottom: f32) -> MiniEngine {
    let data = root.join("_data");
    let safe_area = Some(SafeAreaInsets { top: 20.0, right: 0.0, bottom, left: 0.0 });
    let opts = EngineOptions { safe_area, ..Default::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0, HostEnv::default(), opts)
        .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e.pump(1);
    e
}

/// 逻辑坐标处的像素（dpr 2）
fn px(e: &mut MiniEngine, x: f32, y: f32) -> (u8, u8, u8) {
    let (w, _) = e.pixel_size();
    let i = (((y * 2.0) as u32 * w + (x * 2.0) as u32) * 4) as usize;
    let p = e.rgba();
    (p[i], p[i + 1], p[i + 2])
}

#[test]
fn 原生_tabbar_底下垫安全区_点得到_安全区改回_0_贴底() {
    let _g = SERIAL.lock().unwrap();
    let root = fixture();
    let mut e = engine(&root, 34.0);
    // 两栏之间的竖线位置（x=187）没有文字：只看底色
    assert_eq!(px(&mut e, 2.0, 580.0), GREEN, "tabBar 上沿以上是页面");
    assert_eq!(px(&mut e, 2.0, 590.0), WHITE, "583 起是 tabBar（50 + 安全区 34）");
    assert_eq!(px(&mut e, 2.0, 660.0), WHITE, "安全区那条是 tabBar 底色，不是页面");

    // 点第二个 tab 的上半截（旧行为下这里是页面内容）
    let mut t = 1;
    e.pointer_down(280.0, 600.0);
    t += 16;
    e.pump(t);
    e.pointer_up(280.0, 600.0);
    for _ in 0..4 {
        t += 16;
        e.pump(t);
    }
    assert_eq!(e.current_route(), "pages/b/b");
    // page 事件带 tabBar 底色：Flutter 宿主在 Android 系统导航键那一截填这个色（view.dart）
    let pages: Vec<serde_json::Value> = std::iter::from_fn(|| e.poll_event())
        .map(|s| serde_json::from_str(&s).unwrap())
        .filter(|v: &serde_json::Value| v["type"] == "page")
        .collect();
    let last = pages.last().expect("切 tab 后有 page 事件");
    assert_eq!((last["route"].as_str(), last["tabBarBackgroundColor"].as_str()), (Some("pages/b/b"), Some("#ffffff")));

    // 宿主不再铺到系统导航键下面：安全区回 0，tabBar 贴底
    assert!(e.set_safe_area(SafeAreaInsets { top: 20.0, right: 0.0, bottom: 0.0, left: 0.0 }));
    e.pump(t + 16);
    assert_eq!(px(&mut e, 2.0, 610.0), GREEN, "安全区 0 时 617 以上是页面");
    assert_eq!(px(&mut e, 2.0, 620.0), WHITE);
}

/// 自定义 tabBar + iOS 的底部安全区（402×874 @3x，bottom 34）：图标与文字和没有安全区时**逐像素同位**。
///
/// 真机现象（iPhone，Flutter 与 Swift 宿主都一样）：底部图标被切掉一半、文字压上来。原因不在渲染：
/// Skyline 默认 `box-sizing: border-box`（官方 WXSS 文档的属性表），而示例的组件写的是
/// `height: 100rpx; padding-bottom: env(safe-area-inset-bottom)` —— 34px 的 padding 被算进 53.6px 的高里，
/// 内容区只剩 19.6px，居中的图标文字上下溢出、被 tabBar 画布裁掉。桌面与多数 Android 机安全区是 0，看不出来。
/// 示例已改成 `height: calc(100rpx + env(safe-area-inset-bottom))`；这条守住「有安全区时上面那条不变」。
#[test]
fn 自定义_tabbar_有底部安全区时_图标文字与无安全区同位() {
    let _g = SERIAL.lock().unwrap();
    let root = std::fs::canonicalize(concat!(env!("CARGO_MANIFEST_DIR"), "/sample/cinema")).unwrap();
    let shot = |bottom: f32| {
        let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("tabbar-safe-area-cinema");
        std::fs::remove_dir_all(&data).ok();
        let safe_area = Some(SafeAreaInsets { top: 62.0, right: 0.0, bottom, left: 0.0 });
        let opts = EngineOptions { safe_area, ..Default::default() };
        let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 402, 874, 3.0, HostEnv::default(), opts)
            .expect("创建引擎");
        e.launch(None).expect("打开首页");
        for t in 0..20 {
            e.pump(t * 16);
        }
        let (w, h) = e.pixel_size();
        assert_eq!((w, h), (1206, 2622));
        e.rgba().to_vec()
    };
    let (flat, inset) = (shot(0.0), shot(34.0));
    // tabBar 上沿（100rpx = 53.6px）往下 2..52px 这一条：图标 + 文字。有安全区时整条上移 34px（102 物理行）
    let row = |px: &[u8], ly: u32| -> Vec<u8> {
        let s = (ly * 3 * 1206 * 4) as usize;
        px[s..s + 1206 * 4].to_vec()
    };
    let top0 = 874 - 54;
    let mut inked = 0;
    for ly in top0 + 2..top0 + 52 {
        let a = row(&flat, ly);
        assert_eq!(a, row(&inset, ly - 34), "逻辑行 {ly}（有安全区时是 {}）不一致", ly - 34);
        inked += a.chunks(4).filter(|p| (p[0] as u32 + p[1] as u32 + p[2] as u32) < 600).count();
    }
    assert!(inked > 500, "这一条里应该有图标和文字（防止比的是两片空白）：{inked}");
    // 安全区那一截是 tabBar 底色，不是页面
    let pad = row(&inset, 874 - 10);
    assert!(pad.chunks(4).all(|p| p[..3] == [255, 255, 255]), "安全区那条应是 tabBar 白底");
}
