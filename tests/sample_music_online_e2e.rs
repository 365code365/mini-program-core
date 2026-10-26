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
fn 云听音乐_首页歌单_歌单详情_播放页歌词() {
    let _g = serial();
    let mut a = open("music-online", None, "home");
    let d = "getCurrentPages().slice(-1)[0].data";
    assert!(wait_for(&mut a, &format!("{d}.playlists.length > 0"), 5000), "推荐歌单应从接口拉到");
    a.js("wx.navigateTo({ url: '/pages/playlist/playlist?id=' + getCurrentPages().slice(-1)[0].data.playlists[0].id })");
    a.wait_ms(400);
    assert_eq!(a.route(), "pages/playlist/playlist");
    assert!(wait_for(&mut a, &format!("!!{d}.songs && {d}.songs.length > 0"), 5000), "歌单里的歌应拉到");
    a.assert_rendered(10);
    a.assert_no_errors();
}

/// 被别的小程序打开（scene 1037 + referrerInfo）：首页顶上显示来源，带着歌单 id 的直接进歌单
#[test]
fn 云听音乐_被光影放映厅打开_显示来源并进歌单() {
    let _g = serial();
    let opts = EngineOptions::from_json(r#"{"launchOptions":{"path":"pages/index/index","query":{"playlistId":"101"},"scene":1037,
        "referrerInfo":{"appId":"wxaaf1983a0caced0c","extraData":{"movieTitle":"长河落日","playlistId":101}}}}"#).unwrap();
    let mut a = open_opts("music-online", Some("pages/index/index?playlistId=101"), "ref", opts);
    a.wait_ms(600);
    let ref_app = a.val("getApp().globalData.referrer && getApp().globalData.referrer.appId");
    assert_eq!(ref_app, "wxaaf1983a0caced0c", "App 记下来源小程序");
    assert!(wait_for(&mut a, "getCurrentPages().slice(-1)[0].route === 'pages/playlist/playlist'", 4000), "带着 playlistId 直接进歌单：{}", a.route());
    a.assert_no_errors();
}

#[test]
fn 云听音乐_每页都能打开() {
    let _g = serial();
    for (p, q) in [("pages/index/index", ""), ("pages/charts/charts", ""), ("pages/mine/mine", ""), ("pages/playlists/playlists", ""), ("pages/playlist/playlist", "id=1"), ("pages/player/player", "id=1")] {
        let route = if q.is_empty() { p.to_string() } else { format!("{p}?{q}") };
        let mut a = open("music-online", Some(&route), "smoke");
        a.wait_ms(600);
        assert_eq!(a.route(), p);
        a.assert_rendered(6);
        a.assert_no_errors();
    }
}
