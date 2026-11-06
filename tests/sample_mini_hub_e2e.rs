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

// ───────── 示例之间的互跳：云翼机票 → 途栖酒店民宿、健康挂号 → 康源大药房 ─────────
// 打开方调 wx.navigateToMiniProgram（测试里没有宿主接它：弹出失败原因，不崩）；被打开方按
// getEnterOptionsSync().referrerInfo.extraData 改自己的页面，顶上一条「返回 xx」（navigateBackMiniProgram）

/// 被打开时的启动参数：scene 1037 + referrerInfo
fn referred(path: &str, app_id: &str, extra: &str) -> EngineOptions {
    EngineOptions::from_json(&format!(
        r#"{{"launchOptions":{{"path":"{path}","query":{{}},"scene":1037,"referrerInfo":{{"appId":"{app_id}","extraData":{extra}}}}}}}"#
    ))
    .unwrap()
}

/// 机票支付结果页：已出票时有「订酒店」卡片，点了调 navigateToMiniProgram（目标 = 途栖酒店民宿的 AppID）
#[test]
fn 机票_支付结果_订酒店跳到酒店小程序() {
    let _g = serial();
    let mut a = open("flight", None, "jump");
    let order = r#"{id:'J1',status:'paid',total:860,payMethod:'微信支付',
        legs:[{flight:{date:'2030-01-10',depText:'08:30',fromName:'杭州',toName:'上海',airlineName:'东方航空',airline:'MU',no:'5402',depAirport:'萧山'},cabin:{name:'经济舱'}}],
        pax:[{name:'林晓舟',ticket:'781-0000000001'}]}"#;
    a.js(&format!("wx.setStorageSync('fl_orders', [{order}]); wx.navigateTo({{ url: '/pages/pay-result/pay-result?id=J1' }})"));
    a.settle();
    assert_eq!(a.route(), "pages/pay-result/pay-result");
    assert_eq!(a.data("dest"), "上海");
    assert!(a.exists(".hotel"), "已出票：有「订酒店」卡片");
    a.js("const _n = wx.navigateToMiniProgram; wx.navigateToMiniProgram = function (o) { getApp().__jump = { appId: o.appId, path: o.path, extra: o.extraData }; return _n.call(wx, o); };");
    a.tap(".hotel");
    a.settle();
    let j = a.val("getApp().__jump");
    assert_eq!(j["appId"], "wxe590495f6258b3e6", "{j}");
    assert_eq!(j["path"], "pages/list/list");
    assert_eq!(j["extra"]["city"], "上海");
    assert_eq!(j["extra"]["checkin"], "2030-01-10");
    assert!(a.modal_open(), "没有宿主接跳转：弹出失败原因");
    a.modal_tap(true);
    a.assert_no_errors();
}

/// 酒店列表被机票打开：城市、入住日期按带来的改（存进搜索条件），顶上「返回云翼机票」
#[test]
fn 酒店_被机票打开_按目的地与日期筛选() {
    let _g = serial();
    let extra = r#"{"from":"flight","fromName":"云翼机票","city":"上海","checkin":"2030-01-10","checkout":"2030-01-12"}"#;
    let mut a = open_opts("hotel", Some("pages/list/list"), "ref", referred("pages/list/list", "wx2af35e4a54a867a1", extra));
    assert_eq!(a.data("city"), "上海");
    assert_eq!(a.data("referrer.name"), "云翼机票");
    assert!(a.exists(".ref"), "顶上的来源条");
    let s = a.val("wx.getStorageSync('hotel_search')");
    assert_eq!(s["city"], "上海", "搜索条件存下来了：{s}");
    assert_eq!(s["checkin"], "2030-01-10");
    assert_eq!(s["checkout"], "2030-01-12");
    assert!(a.data("list").as_array().map(|l| !l.is_empty()).unwrap_or(false), "上海有酒店");
    a.assert_no_errors();
}

/// 酒店列表直接打开（没有来源）：没有来源条，搜索条件不动
#[test]
fn 酒店_直接打开_没有来源条() {
    let _g = serial();
    let mut a = open("hotel", Some("pages/list/list"), "plain");
    assert!(a.data("referrer").is_null());
    assert!(!a.exists(".ref"));
    assert_eq!(a.data("city"), "杭州");
    a.assert_no_errors();
}

/// 挂号单「处方与检查」里的药品有「去药房」；药房搜索页被挂号打开时按药名直接出结果、不弹键盘
#[test]
fn 挂号处方_去药房_药房按药名搜() {
    let _g = serial();
    let mut a = open("hospital", None, "jump");
    // 默认数据里第二张挂号单（7 天前、呼吸内科）有已缴的门诊处方，单号按当天日期算
    a.js("(function(){var d=new Date();d.setDate(d.getDate()-7);var p=function(n){return (n<10?'0':'')+n};\
          wx.navigateTo({url:'/pages/order-detail/order-detail?id=R'+d.getFullYear()+p(d.getMonth()+1)+p(d.getDate())+'083'});})()");
    a.settle();
    assert_eq!(a.route(), "pages/order-detail/order-detail");
    assert!(a.count(".buy") >= 1, "处方里的药品有「去药房」");
    a.js("const _n = wx.navigateToMiniProgram; wx.navigateToMiniProgram = function (o) { getApp().__jump = { appId: o.appId, path: o.path, extra: o.extraData }; return _n.call(wx, o); };");
    a.tap_nth(".buy", 0);
    a.settle();
    let j = a.val("getApp().__jump");
    assert_eq!(j["appId"], "wx372a5c26935a3559", "{j}");
    assert_eq!(j["path"], "pages/search/search");
    assert!(!j["extra"]["kw"].as_str().unwrap_or("").contains(' '), "药名去掉了规格：{j}");
    assert!(a.modal_open(), "没有宿主接跳转：弹出失败原因");
    a.modal_tap(true);
    a.assert_no_errors();

    let extra = r#"{"from":"hospital","fromName":"健康挂号","kw":"奥美拉唑肠溶胶囊","drug":"奥美拉唑肠溶胶囊 20mg×14"}"#;
    let mut p = open_opts("pharmacy", Some("pages/search/search"), "ref", referred("pages/search/search", "wx68a60e3639608901", extra));
    assert_eq!(p.data("kw"), "奥美拉唑肠溶胶囊");
    assert_eq!(p.data("searched"), true);
    assert!(p.data("results").as_array().map(|l| !l.is_empty()).unwrap_or(false), "药房里有这个药");
    assert_eq!(p.data("focus"), false, "带着药名来的不弹键盘");
    assert_eq!(p.data("referrer.name"), "健康挂号");
    assert!(p.exists(".ref"));
    p.assert_no_errors();
}
