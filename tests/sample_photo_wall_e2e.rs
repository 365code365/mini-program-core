#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use mini_render::host::EngineOptions;

/// 打开示例，数据接口指到本机起的商店服务（见 sample_harness/demo_server.rs）
fn open(name: &str, route: Option<&str>, tag: &str) -> App {
    open_opts(name, route, tag, EngineOptions::default())
}

fn open_opts(name: &str, route: Option<&str>, tag: &str, opts: EngineOptions) -> App {
    let base = demo_server::base();
    let mut a = App::open_seeded(name, route, tag, opts, &[("demoApiBase", &base)]);
    a.wait_ms(900);
    a
}

/// 等到 `cond`（JS 表达式）为真，最多 `ms` 毫秒
fn wait_for(a: &mut App, cond: &str, ms: u64) -> bool {
    let t0 = std::time::Instant::now();
    while t0.elapsed().as_millis() < ms as u128 {
        if a.val(cond) == serde_json::Value::Bool(true) {
            return true;
        }
        a.wait_ms(100);
    }
    false
}

#[test]
fn 拾光图库_瀑布流分两列_触底加载_点图预览() {
    let _g = serial();
    let mut a = open("photo-wall", None, "home");
    let d = "getCurrentPages().slice(-1)[0].data";
    assert!(wait_for(&mut a, &format!("{d}.left.length > 0 && {d}.right.length > 0"), 5000), "照片应分到左右两列");
    let n0 = a.val(&format!("{d}.left.length + {d}.right.length")).as_u64().unwrap_or(0);
    a.js("getCurrentPages().slice(-1)[0].onReachBottom()");
    assert!(wait_for(&mut a, &format!("{d}.left.length + {d}.right.length > {n0}"), 5000), "触底加载下一页");
    // 两列高度差不多（按宽高比往矮的那一列放）
    let cols = a.rects(".wf-col");
    assert_eq!(cols.len(), 2, "两列");
    assert!((cols[0].3 - cols[1].3).abs() < 400.0, "两列高度应接近（按宽高比往矮的那一列放）：{cols:?}");
    a.assert_rendered(10);
    a.assert_no_errors();
}

#[test]
fn 拾光图库_每页都能打开() {
    let _g = serial();
    for (p, q) in [("pages/index/index", ""), ("pages/favorites/favorites", ""), ("pages/detail/detail", "id=1")] {
        let route = if q.is_empty() { p.to_string() } else { format!("{p}?{q}") };
        let mut a = open("photo-wall", Some(&route), "smoke");
        a.wait_ms(600);
        assert_eq!(a.route(), p);
        a.assert_rendered(6);
        a.assert_no_errors();
    }
}
