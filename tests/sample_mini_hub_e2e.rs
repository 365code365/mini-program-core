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

/// 列表从商店接口分页拉；点「打开」调 wx.navigateToMiniProgram（测试里没有宿主接它：记下失败原因，不崩）
#[test]
fn 跳转中心_调_navigateToMiniProgram_并记录结果() {
    let _g = serial();
    let mut a = open("mini-hub", None, "home");
    let d = "getCurrentPages().slice(-1)[0].data";
    assert!(wait_for(&mut a, &format!("{d}.status !== 'loading'"), 5000), "商店列表请求应有结果");
    assert!(a.data("featured").as_array().map(|l| !l.is_empty()).unwrap_or(false), "推荐跳转目标");
    a.tap_nth(".open", 0);
    assert!(wait_for(&mut a, &format!("{d}.logs.length > 0"), 3000), "跳转结果记进「最近跳转」");
    a.assert_no_errors();
}

/// 被别的小程序打开时：顶上「本次启动参数」显示 scene 1037 与来源 AppID
#[test]
fn 跳转中心_显示启动来源() {
    let _g = serial();
    let opts = EngineOptions::from_json(r#"{"launchOptions":{"path":"pages/index/index","query":{},"scene":1037,
        "referrerInfo":{"appId":"wxc0bec341b9d3e883","extraData":{"from":"music-online"}}}}"#).unwrap();
    let mut a = open_opts("mini-hub", None, "ref", opts);
    a.wait_ms(300);
    assert_eq!(a.data("launch.scene"), 1037);
    assert_eq!(a.data("launch.refAppId"), "wxc0bec341b9d3e883");
    assert!(a.exists(".pill-on"), "「被小程序打开」标记");
    a.assert_no_errors();
}

#[test]
fn 跳转中心_每页都能打开() {
    let _g = serial();
    for p in ["pages/index/index", "pages/form/form"] {
        let mut a = open("mini-hub", Some(p), "smoke");
        assert_eq!(a.route(), p);
        a.assert_rendered(6);
        a.assert_no_errors();
    }
}
