//! GT-E（G9.1 兼容矩阵 / A8.2）：第三方引擎经 sample-game/_lib/adapter.js 跑在小游戏里。
//! 引擎本身不入库：先跑 tools/fetch-engines.sh（没有就失败并说明）。
use super::*;

fn need(fixture: &str, lib: &str) {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample-game").join(fixture).join("libs").join(lib);
    assert!(p.exists(), "{} 不存在：先跑 bash tools/fetch-engines.sh", p.display());
}

/// 输出帧的「非单色」程度：不同颜色数
pub fn colors(h: &mut GameHarness) -> usize {
    let (_, _, p) = h.engine.frame_rgba();
    let mut s = std::collections::HashSet::new();
    for c in p.chunks_exact(4) {
        s.insert([c[0], c[1], c[2]]);
    }
    s.len()
}

/// 把输出帧存成 PNG（对照 Chrome 用）
pub fn save_png(h: &mut GameHarness, path: &std::path::Path) {
    let (w, hh, p) = h.engine.frame_rgba();
    image::RgbaImage::from_raw(w, hh, p.to_vec()).unwrap().save(path).unwrap();
}

#[test]
fn e1_three() {
    let _g = serial();
    need("e-three", "three.js");
    let mut h = GameHarness::open("e-three", Opts { query: "frames=30&dpr=1", ..Default::default() });
    h.until("T:EV done", 120);
    assert!(h.errors.is_empty(), "{:?}\n{:?}", h.errors, h.log.iter().rev().take(20).collect::<Vec<_>>());
    let t = &h.events("three")[0];
    assert_eq!((t["rev"].as_str(), t["w"].as_u64(), t["h"].as_u64()), (Some("160"), Some(375), Some(667)), "{t}");
    assert!(!h.log.iter().any(|l| l.contains("[error]") || l.contains("console.error")), "{:?}", h.log);
    let n = colors(&mut h);
    assert!(n > 500, "首帧应当有光照渐变，颜色数 {n}");
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/engine-ref");
    std::fs::create_dir_all(&out).unwrap();
    save_png(&mut h, &out.join("three-mini.png"));
}

/// 点一下（按下 + 抬起，逻辑坐标）
fn tap(h: &mut GameHarness, x: f32, y: f32) {
    let t = h.now_us();
    h.engine.touch(0, 1, x, y, 1.0, t);
    h.vsyncs(1);
    let t = h.now_us();
    h.engine.touch(2, 1, x, y, 1.0, t);
    h.vsyncs(2);
}

fn matrix_checks(h: &mut GameHarness, name: &str) {
    assert!(h.errors.is_empty(), "{name}: {:?}\n{:?}", h.errors, h.log.iter().rev().take(25).collect::<Vec<_>>());
    assert!(!h.log.iter().any(|l| l.contains("[error]") || l.contains("console.error")), "{name}: {:?}", h.log.iter().filter(|l| l.contains("error")).collect::<Vec<_>>());
    let n = colors(h);
    assert!(n > 8, "{name}: 首帧颜色数 {n}（空白？）");
}

#[test]
fn e2_pixi() {
    let _g = serial();
    need("e-pixi", "pixi.js");
    let mut h = GameHarness::open("e-pixi", Opts { query: "dpr=1", ..Default::default() });
    h.until("T:EV frames", 200);
    matrix_checks(&mut h, "pixi");
    let p = &h.events("pixi")[0];
    assert_eq!((p["ver"].as_str(), p["type"].as_u64()), (Some("7.4.2"), Some(1)), "WebGL 渲染器：{p}");
    // 精灵在 (60..140, 480..560)
    assert_eq!(super::webgl_px(&mut h, 100, 520), [0, 255, 0]);
    tap(&mut h, 100.0, 520.0);
    assert_eq!(h.events("tap").len(), 1, "{:?}", h.log.iter().rev().take(10).collect::<Vec<_>>());
    h.vsyncs(2);
    assert_eq!(super::webgl_px(&mut h, 100, 520), [255, 0, 0], "点了之后变红");
    assert_eq!(super::webgl_px(&mut h, 140, 180), [240, 160, 32], "Graphics 圆角矩形");
}

fn phaser(ty: &'static str) {
    need("e-phaser", "phaser.js");
    let q: &'static str = if ty == "canvas" { "type=canvas" } else { "type=webgl" };
    let mut h = GameHarness::open("e-phaser", Opts { query: q, ..Default::default() });
    h.until("T:EV frames", 300);
    matrix_checks(&mut h, ty);
    let p = &h.events("phaser")[0];
    assert_eq!((p["ver"].as_str(), p["renderer"].as_str()), (Some("3.80.1"), Some(ty)), "{p}");
    assert!(h.events("frames")[0]["y"].as_i64().unwrap() > 100, "物理下落：{:?}", h.events("frames"));
    assert_eq!(super::webgl_px(&mut h, 140, 180), [240, 160, 32]);
    tap(&mut h, 200.0, 300.0);
    assert_eq!(h.events("tap").len(), 1, "{:?}", h.log.iter().rev().take(10).collect::<Vec<_>>());
}

#[test]
fn e3_phaser_webgl() {
    let _g = serial();
    phaser("webgl");
}

#[test]
fn e4_phaser_canvas() {
    let _g = serial();
    phaser("canvas");
}
