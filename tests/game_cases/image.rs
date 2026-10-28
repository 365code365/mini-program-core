//! GT-I 图片（game-doc/05 §3）：g-image 夹具 + 一个本地 HTTP 服务（/px.png 正常、其余 404）。
use super::*;

/// 只认 GET 的最小 HTTP 服务：每个连接一个请求，读完请求头就回
fn serve(png: Vec<u8>) -> u16 {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut s in l.incoming().flatten() {
            let mut buf = [0u8; 2048];
            let n = s.read(&mut buf).unwrap_or(0);
            let head = String::from_utf8_lossy(&buf[..n]);
            let ok = head.starts_with("GET /px.png ");
            let body: &[u8] = if ok { &png } else { b"not found" };
            let status = if ok { "200 OK" } else { "404 Not Found" };
            let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = s.write_all(body);
        }
    });
    port
}

fn open_image() -> GameHarness {
    let png = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/sample-game/g-image/img/px4x2.png")).unwrap();
    let q: &'static str = Box::leak(format!("http={}", serve(png)).into_boxed_str());
    let mut h = GameHarness::open("g-image", Opts { query: q, ..Opts::default() });
    h.until("T:DONE", 300);
    h
}

fn rgb(h: &mut GameHarness, x: u32, y: u32) -> [u8; 3] {
    let p = h.pixel(x, y);
    [p[0], p[1], p[2]]
}

/// I1 / I2 / I3：各来源与格式的尺寸回读、onload / onerror 的形状
#[test]
fn i1_i3_来源格式与错误() {
    let _g = serial();
    let h = open_image();
    h.assert_no_fail();
    let loads = h.events("load");
    let size = |k: &str| loads.iter().find(|e| e["k"] == k).map(|e| (e["after"][0].as_u64().unwrap(), e["after"][1].as_u64().unwrap()));
    for (k, wh) in [("png", (4, 2)), ("png-abs", (4, 2)), ("jpg", (16, 8)), ("webp", (5, 5)), ("gif", (6, 6)), ("data", (4, 2)), ("wxfile", (4, 2)), ("http", (4, 2))] {
        assert_eq!(size(k), Some(wh), "{k} 没加载或尺寸不对：{loads:#?}");
    }
    for e in &loads {
        assert_eq!(e["before"], serde_json::json!([0, 0, true]), "onload 之前 0×0、complete（还没设 src）：{e}");
        assert_eq!((e["complete"].as_bool(), e["self"].as_bool()), (Some(true), Some(true)), "{e}");
    }
    assert!(h.events("set").iter().all(|e| e["w"] == 0 && e["complete"] == false), "设 src 之后同步读：还是 0、complete=false");
    let errs = h.events("error");
    for k in ["missing", "broken", "escape", "http404"] {
        let e = errs.iter().find(|e| e["k"] == k).unwrap_or_else(|| panic!("{k} 应当 onerror：{errs:#?}"));
        assert!(e["arg"]["errMsg"].as_str().is_some_and(|m| !m.is_empty()), "onerror 参数是 {{errMsg}}：{e}");
        assert_eq!(e["w"], 0);
    }
}

/// I4 / I5：同 src 只解一次（`/a.png` 与 `a.png` 也算同一张）；drawImage 一万次不再解码
#[test]
fn i4_i5_解码次数() {
    let _g = serial();
    let mut h = open_image();
    assert!(h.has("T:EV same5"), "同 src 的 5 个 Image 都要 onload");
    // 不同的源：px4x2（两种写法算一张）、jpg、webp、gif、data URI、edge4、wxfile、http
    assert_eq!(h.engine.image_decode_count(), 8);
    h.vsyncs(5);
    assert_eq!(h.engine.image_decode_count(), 8, "之后的帧不再解码");
}

/// drawImage 3 / 5 / 9 参、最近邻、GIF 第一帧、Canvas 源、图案、旋转（像素断言）
#[test]
fn i_draw_像素() {
    let _g = serial();
    let mut h = open_image();
    assert!(h.has(r#"T:EV drawn {"pattern":true}"#), "图加载完 createPattern 不是 null");
    assert_eq!(rgb(&mut h, 0, 0), [255, 0, 0], "3 参原尺寸：(0,0) 红");
    assert_eq!(rgb(&mut h, 1, 0), [0, 255, 0]);
    assert_eq!(rgb(&mut h, 15, 12), [0, 255, 0], "5 参放大 10 倍最近邻：(15,12) → 源 (1,0) 绿");
    assert_eq!(rgb(&mut h, 35, 25), [0, 0, 0], "→ 源 (3,1) 黑");
    assert_eq!(rgb(&mut h, 62, 12), [0, 255, 255], "9 参只取源 (1,1)：青");
    assert_eq!(rgb(&mut h, 69, 12), [255, 255, 255], "9 参目标 8×8 以外是白底");
    assert_eq!(rgb(&mut h, 5, 65), [0, 200, 0], "GIF 只画第一帧");
    assert_eq!(rgb(&mut h, 105, 5), [0, 255, 0], "Canvas 源");
    assert_eq!(rgb(&mut h, 124, 1), [255, 0, 255], "图案平铺：(124,1) → 源 (0,1) 品红");
    assert_eq!(rgb(&mut h, 125, 0), [0, 255, 0], "图案平铺：(125,0) → 源 (1,0) 绿");
    assert_eq!(rgb(&mut h, 133, 1), [255, 255, 255], "图案只填 12 宽");
    assert_eq!(rgb(&mut h, 199, 21), [255, 0, 0], "旋转 90°：右上角是源 (0,0)");
    assert_eq!(rgb(&mut h, 181, 59), [0, 0, 0], "旋转 90°：左下角是源 (3,1)");
    assert_eq!(rgb(&mut h, 179, 30), [255, 255, 255], "旋转后的范围以外不画");
}

/// I7：跨运行逐字节稳定（GIF 只取第一帧，不按墙上时钟换帧）
#[test]
fn i7_两次运行像素相同() {
    let _g = serial();
    let a = open_image().engine.frame_rgba().2.to_vec();
    let b = open_image().engine.frame_rgba().2.to_vec();
    assert!(a == b, "两次运行的主画布不同");
}

/// I6：超预算只淘汰没有 Image 引用的缓存；全被引用还超 → wx.onMemoryWarning
#[test]
fn i6_预算() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
wx.onMemoryWarning(function (r) { T.ev('mem', r); });
var keep = [];
globalThis.load = function (src, hold, cb) { var i = wx.createImage(); i.onload = function () { if (hold) keep.push(i); cb(); }; i.src = src; };
T.ready('budget');
globalThis.step = function (n) {
  if (n === 1) load('img/a.png', false, function () { load('img/b.png', true, function () { T.ev('step1', {}); }); });
  if (n === 2) load('img/c.png', true, function () { T.ev('step2', {}); });
};
"#;
    let mut h = GameHarness::open_inline("budget", js, Opts::default());
    // 三张 64×64（16KB 解码后）
    let dir = h.dir.join("img");
    std::fs::create_dir_all(&dir).unwrap();
    for n in ["a", "b", "c"] {
        image::RgbaImage::from_pixel(64, 64, image::Rgba([1, 2, 3, 255])).save(dir.join(format!("{n}.png"))).unwrap();
    }
    h.until("T:READY budget", 10);
    h.engine.set_image_budget(40 * 1024);
    h.engine.eval("step(1)").unwrap();
    h.until("T:EV step1", 50);
    assert_eq!(h.engine.image_cache_bytes(), 32 * 1024, "a、b 都在缓存里（没超）");
    // a 的 Image 已经没人引用：GC 之后 FinalizationRegistry 通知原生放掉（不 GC 的话对象还活着，a 也就还被引用）
    h.engine.collect_garbage();
    h.engine.eval("0").unwrap();
    // 第三张进来：48KB > 40KB → 淘汰没人引用的 a，剩 b、c 32KB
    h.engine.eval("step(2)").unwrap();
    h.until("T:EV step2", 50);
    h.vsyncs(1);
    assert_eq!(h.engine.image_cache_bytes(), 32 * 1024, "只淘汰 a（没有 Image 引用）");
    assert!(h.events("mem").is_empty(), "淘汰得下就不告警");
    // 预算压到 20KB：b、c 都被引用，淘汰不动 → 告警一次
    h.engine.set_image_budget(20 * 1024);
    h.engine.eval("load('img/a.png', true, function(){})").unwrap();
    h.vsyncs(3);
    let mem = h.events("mem");
    assert_eq!(mem.len(), 1, "全被引用还超：告警一次，不重复：{mem:?}");
    assert_eq!(mem[0]["level"], 10);
}
