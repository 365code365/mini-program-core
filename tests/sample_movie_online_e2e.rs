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
fn 光影放映厅_首页列表_分页加载_详情() {
    let _g = serial();
    let mut a = open("movie-online", None, "home");
    let d = "getCurrentPages().slice(-1)[0].data";
    assert!(wait_for(&mut a, &format!("{d}.status === 'ok' && {d}.list.length > 0"), 5000), "首页列表应从接口拉到：{}", a.data("status"));
    let n0 = a.data("list").as_array().map(|l| l.len()).unwrap_or(0);
    let total = a.data("total").as_u64().unwrap_or(0);
    assert!(total as usize > n0, "正在热映有多页（共 {total}，首屏 {n0}）");
    // 上拉触底：加载第二页，追加在后面
    a.js("getCurrentPages().slice(-1)[0].onReachBottom()");
    assert!(wait_for(&mut a, &format!("{d}.list.length > {n0}"), 5000), "触底应加载下一页");
    // 切到「即将上映」：列表换掉
    a.tap_nth(".tab", 1);
    assert!(wait_for(&mut a, &format!("{d}.tab === 'coming' && {d}.status === 'ok'"), 5000), "切 tab 后重新拉");
    // 点第一部进详情
    a.js("wx.navigateTo({ url: '/pages/detail/detail?id=' + getCurrentPages().slice(-1)[0].data.list[0].id })");
    a.wait_ms(400);
    assert_eq!(a.route(), "pages/detail/detail");
    assert!(wait_for(&mut a, &format!("!!{d}.m && {d}.m.stills.length > 0"), 5000), "详情（含剧照）应从接口拉到");
    a.assert_rendered(10);
    a.assert_no_errors();
}

#[test]
fn 光影放映厅_每页都能打开() {
    let _g = serial();
    for (p, q) in [("pages/index/index", ""), ("pages/mine/mine", ""), ("pages/search/search", ""), ("pages/detail/detail", "id=1"), ("pages/comments/comments", "id=1")] {
        let route = if q.is_empty() { p.to_string() } else { format!("{p}?{q}") };
        let mut a = open("movie-online", Some(&route), "smoke");
        a.wait_ms(600);
        assert_eq!(a.route(), p);
        a.assert_rendered(6);
        a.assert_no_errors();
    }
}
