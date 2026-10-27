//! GT-U1（系统覆盖层）：Toast / Loading / Modal / ActionSheet 盖在游戏上、拦触摸、按钮回调值。夹具 sample-game/g-overlays。
use super::*;

const GREEN: [u8; 3] = [0x20, 0x80, 0x40];

fn px(h: &mut GameHarness, x: u32, y: u32) -> [u8; 3] {
    let (w, _, p) = h.engine.frame_rgba();
    let i = ((y * w + x) * 4) as usize;
    [p[i], p[i + 1], p[i + 2]]
}

/// 输出帧里接近白色（≥ 0xF0）的像素包围盒（输出像素）
fn white_bbox(h: &mut GameHarness, y_from: u32) -> Option<(u32, u32, u32, u32)> {
    let (w, hh, p) = h.engine.frame_rgba();
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for y in y_from..hh {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            if p[i] >= 0xF0 && p[i + 1] >= 0xF0 && p[i + 2] >= 0xF0 {
                b = Some(match b { None => (x, y, x, y), Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)) });
            }
        }
    }
    b
}

fn tap(h: &mut GameHarness, x: f32, y: f32) {
    let t = h.now_us();
    h.engine.touch(0, 7, x, y, 1.0, t);
    h.engine.touch(2, 7, x, y, 0.0, t);
    h.vsyncs(2);
}

fn ui(h: &GameHarness, tag: &str) -> Vec<serde_json::Value> {
    h.events("ui").into_iter().filter(|e| e["tag"] == tag).collect()
}

#[test]
fn u1_覆盖层盖在游戏上_拦触摸_回调值() {
    let _g = serial();
    let mut h = GameHarness::open("g-overlays", Opts::default());
    h.vsyncs(2);
    assert_eq!(h.engine.frame_rgba().0, 375, "没有覆盖层：输出就是主画布（逻辑尺寸）");
    assert_eq!(px(&mut h, 187, 333), GREEN);
    let plain = h.engine.frame_rgba().2.to_vec();

    // Toast：盖在中间、不挡触摸；到时收起，回到原样
    h.engine.eval("go('toast')").unwrap();
    h.vsyncs(2);
    assert_eq!(h.engine.frame_rgba().0, 750, "覆盖层弹着：按 min(dpr, 2) 合成");
    if std::env::var("U1_DUMP").is_ok() {
        let (w, hh, p) = h.engine.frame_rgba();
        image::RgbaImage::from_raw(w, hh, p.to_vec()).unwrap().save("/tmp/u1_toast.png").unwrap();
    }
    assert_ne!(px(&mut h, 375, 500), GREEN, "提示框（偏上，与微信一致）");
    assert_eq!(px(&mut h, 10, 10), GREEN, "提示框不带遮罩：四周还是游戏");
    tap(&mut h, 50.0, 50.0);
    assert_eq!(h.events("gameTouch").len(), 1, "不带 mask 的 Toast 不挡触摸");
    h.vsyncs(70);
    assert_eq!(h.engine.frame_rgba().2, &plain[..], "收起之后与没弹过逐字节相同");

    // Loading（mask）：挡触摸；hideLoading 之后放行
    h.engine.eval("go('loading')").unwrap();
    h.vsyncs(2);
    tap(&mut h, 50.0, 50.0);
    assert_eq!(h.events("gameTouch").len(), 1, "带 mask 的 Loading 挡触摸");
    h.engine.eval("go('hideLoading')").unwrap();
    h.vsyncs(2);
    tap(&mut h, 50.0, 50.0);
    assert_eq!(h.events("gameTouch").len(), 2);

    // Modal：遮罩压暗、点遮罩不回调也不进游戏；点「确定」回 confirm:true
    h.engine.eval("go('modal')").unwrap();
    h.vsyncs(2);
    let g = px(&mut h, 10, 10);
    assert!(g[1] < GREEN[1], "遮罩压暗了游戏：{g:?}");
    let (x0, y0, x1, y1) = white_bbox(&mut h, 0).expect("弹窗白底");
    tap(&mut h, 20.0, 20.0);
    assert!(ui(&h, "modal").is_empty() && h.events("gameTouch").len() == 2, "点遮罩：不回调、不进游戏");
    // 输出是 2 倍：确定按钮在白框右下角
    tap(&mut h, (x1 as f32 - 30.0) / 2.0, (y1 as f32 - 20.0) / 2.0);
    let m = ui(&h, "modal");
    assert_eq!(m.len(), 1, "{:?}", h.events("ui"));
    assert_eq!((m[0]["r"]["confirm"].as_bool(), m[0]["r"]["cancel"].as_bool()), (Some(true), Some(false)), "{m:?}");
    assert!(x1 - x0 > 400 && y1 > y0, "弹窗大小合理：{:?}", (x0, y0, x1, y1));
    h.vsyncs(2);
    assert_eq!(h.engine.frame_rgba().0, 375, "弹窗关了");

    // ActionSheet：滑入走完之后点第二项 → tapIndex 1；点遮罩 → fail cancel
    h.engine.eval("go('sheet')").unwrap();
    h.vsyncs(40);
    let (_, top, _, _) = white_bbox(&mut h, 667).expect("菜单白底（下半屏）");
    // 菜单从上往下：每项 56 逻辑像素
    tap(&mut h, 187.0, top as f32 / 2.0 + 56.0 + 28.0);
    h.vsyncs(40);
    let s = ui(&h, "sheet");
    assert_eq!((s.len(), s[0]["r"]["tapIndex"].as_u64()), (1, Some(1)), "{s:?}");
    h.engine.eval("go('sheet')").unwrap();
    h.vsyncs(40);
    tap(&mut h, 187.0, 100.0);
    h.vsyncs(40);
    let s = ui(&h, "sheet");
    assert_eq!((s.len(), s[1]["ok"].as_bool()), (2, Some(false)), "点遮罩 = 取消：{s:?}");
    assert!(s[1]["r"]["errMsg"].as_str().unwrap().contains("cancel"));
    assert_eq!(h.events("gameTouch").len(), 2, "菜单弹着时的触摸一个都没进游戏");
    assert_eq!(h.engine.frame_rgba().2, &plain[..], "都收起之后与没弹过逐字节相同");
}

/// U2：showKeyboard 的参数完整交给宿主；宿主推 keyboardInput / Confirm / Complete → 回调 {value}；
/// confirmHold:false 时确认之后发 hideKeyboard；keyboardHeightChange 到 onKeyboardHeightChange
#[test]
fn u2_键盘() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
['Input', 'Confirm', 'Complete'].forEach(function (k) { wx['onKeyboard' + k](function (r) { T.ev('kb', { k: k, v: r.value }); }); });
wx.onKeyboardHeightChange(function (r) { T.ev('kbh', r); });
globalThis.go = function () {
  wx.showKeyboard({ defaultValue: 'hi', maxLength: 20, confirmType: 'send', success: function (r) { T.ev('kbShown', r); } });
};
globalThis.upd = function () { wx.updateKeyboard({ value: 'x' }); };
T.ready('u2');
"#;
    let mut h = GameHarness::open_inline("u2", js, Opts::default());
    h.engine.eval("go()").unwrap();
    h.vsyncs(2);
    let show: Vec<&serde_json::Value> = h.host.iter().filter(|e| e["type"] == "showKeyboard").collect();
    assert_eq!(show.len(), 1, "{:?}", h.host);
    assert_eq!(show[0], &serde_json::json!({ "type": "showKeyboard", "defaultValue": "hi", "maxLength": 20, "multiple": false, "confirmHold": false, "confirmType": "send" }));
    assert_eq!(h.events("kbShown")[0]["errMsg"], "showKeyboard:ok");
    assert_eq!(h.engine.dispatch_event("keyboardInput", r#"{"value":"hi!"}"#), 1);
    h.engine.dispatch_event("keyboardHeightChange", r#"{"height":280}"#);
    h.engine.eval("upd()").unwrap();
    h.engine.dispatch_event("keyboardConfirm", r#"{"value":"hi!"}"#);
    h.vsyncs(1);
    assert!(h.host.iter().any(|e| e["type"] == "updateKeyboard" && e["value"] == "x"));
    assert!(h.host.iter().any(|e| e["type"] == "hideKeyboard"), "confirmHold:false → 确认之后收起");
    h.engine.dispatch_event("keyboardComplete", r#"{"value":"hi!"}"#);
    h.vsyncs(1);
    let kb: Vec<(String, String)> = h.events("kb").iter().map(|e| (e["k"].as_str().unwrap().into(), e["v"].as_str().unwrap().into())).collect();
    assert_eq!(kb, [("Input".into(), "hi!".into()), ("Confirm".into(), "hi!".into()), ("Complete".into(), "hi!".into())]);
    assert_eq!(h.events("kbh")[0]["height"], 280);
}

/// U3：原生按钮。样式字段（底色、边框、圆角、文字）画出来；点按钮 onTap（UserInfoButton 没接宿主 → getUserInfo:fail）且不进游戏；
/// 改 style.left 跟着动；hide 之后不命中、点下去进游戏；destroy 之后不再画（输出与没建过逐字节相同）；image 型按图铺满；
/// Feedback / GameClub 交宿主事件
#[test]
fn u3_原生按钮() {
    let _g = serial();
    let mut h = GameHarness::open("g-overlays", Opts::default());
    h.vsyncs(2);
    let plain = h.engine.frame_rgba().2.to_vec();
    let png = {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([10, 200, 250, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    };
    std::fs::write(h.dir.join("btn.png"), png).unwrap();
    h.engine.eval(r#"
        var T = require('_lib/t.js');
        globalThis.btn = wx.createUserInfoButton({ type: 'text', text: '登录', style: { left: 20, top: 100, width: 200, height: 40,
            backgroundColor: '#ff0000', borderColor: '#0000ff', borderWidth: 2, borderRadius: 8, color: '#ffffff', textAlign: 'center', fontSize: 16 } });
        btn.onTap(function (r) { T.ev('tap', r); });
        globalThis.ib = wx.createFeedbackButton({ type: 'image', image: 'btn.png', style: { left: 300, top: 500, width: 40, height: 40 } });
        ib.onTap(function (r) { T.ev('fb', r); });
    "#).unwrap();
    h.vsyncs(3);
    let px = |h: &mut GameHarness, x: u32, y: u32| { let (w, _, p) = h.engine.frame_rgba(); let i = ((y * w + x) * 4) as usize; [p[i], p[i + 1], p[i + 2]] };
    assert_eq!(h.engine.frame_rgba().0, 750);
    assert_eq!(px(&mut h, 2 * 30, 2 * 110), [255, 0, 0], "底色");
    assert_eq!(px(&mut h, 2 * 120, 2 * 100 + 1), [0, 0, 255], "上边框");
    assert_eq!(px(&mut h, 2 * 20 + 1, 2 * 100 + 1), GREEN, "圆角外是游戏");
    let white = { let (w, _, p) = h.engine.frame_rgba(); (2 * 20..2 * 220).filter(|&x| { let i = ((2 * 120 * w + x) * 4) as usize; p[i] > 200 && p[i + 1] > 200 }).count() };
    assert!(white > 5, "按钮中线上有白色文字像素：{white}");
    assert_eq!(px(&mut h, 2 * 320, 2 * 520), [10, 200, 250], "image 型按图铺满");
    tap(&mut h, 100.0, 120.0);
    let t = h.events("tap");
    assert_eq!(t.len(), 1);
    assert!(t[0]["errMsg"].as_str().unwrap().starts_with("getUserInfo:fail"), "没接宿主：{t:?}");
    assert!(h.events("gameTouch").is_empty(), "点按钮不进游戏");
    tap(&mut h, 320.0, 520.0);
    assert_eq!(h.events("fb").len(), 1);
    assert!(h.host.iter().any(|e| e["type"] == "openFeedback"));
    h.engine.eval("btn.style.left = 120").unwrap();
    h.vsyncs(1);
    assert_eq!(px(&mut h, 2 * 30, 2 * 110), GREEN, "左移走了");
    assert_eq!(px(&mut h, 2 * 130, 2 * 110), [255, 0, 0]);
    h.engine.eval("btn.hide()").unwrap();
    h.vsyncs(1);
    tap(&mut h, 150.0, 120.0);
    assert_eq!((h.events("tap").len(), h.events("gameTouch").len()), (1, 1), "hide 之后不命中，点下去进游戏");
    h.engine.eval("btn.destroy(); ib.destroy()").unwrap();
    h.vsyncs(2);
    assert_eq!(h.engine.frame_rgba().2, &plain[..], "destroy 之后与没建过逐字节相同");
}

/// H1 / H2：没接宿主的代理对象——方法都在、建出来下一轮 onError（errCode 1004）、show() 的 Promise reject、普通方法 fail；
/// 引擎就地做的 GameLogManager / ScenePerformanceManager 能用；setMenuStyle / setStatusBarStyle 交宿主
#[test]
fn h1_h2_宿主代理对象_没接宿主() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
var ad = wx.createBannerAd({ adUnitId: 'x', style: { left: 0, top: 0, width: 300 } });
ad.onError(function (e) { T.ev('adErr', e); });
ad.show().then(function () { T.ev('show', { ok: true }); }, function (e) { T.ev('show', { ok: false, e: e }); });
var rv = wx.createRewardedVideoAd({ adUnitId: 'y' });
var miss = ['show', 'load', 'destroy', 'onClose', 'offClose', 'setServerSideVerificationData'].filter(function (m) { return typeof rv[m] !== 'function'; });
var gs = wx.getGameServerManager();
gs.login({ fail: function (e) { T.ev('gsFail', e); } });
var rec = wx.getGameRecorder();
var lm = wx.getGameLogManager(); lm.updateCommonInfo({ a: 1 });
var sp = wx.getScenePerformanceManager(); sp.setCommonInfo({ b: 2 });
wx.setMenuStyle({ style: 'dark' }); wx.setStatusBarStyle({ style: 'white' });
T.ev('shape', { miss: miss, sameRecorder: rec === wx.getGameRecorder(), sound: rec.isSoundSupported(), log: lm.getCommonInfo(), perf: sp.getCommonInfo(),
  styleW: ad.style.width, fns: [typeof wx.createGridAd, typeof wx.createCustomAd, typeof wx.createCamera, typeof wx.createPageManager,
  typeof wx.createStoreGift, typeof wx.createVideoDecoder, typeof wx.getRankManager, typeof wx.getMiniReportManager, typeof wx.createAdSkipCard] });
T.ready('h1');
"#;
    let mut h = GameHarness::open_inline("h1", js, Opts::default());
    h.vsyncs(3);
    let s = &h.events("shape")[0];
    assert_eq!((s["miss"].as_array().map(|a| a.len()), s["sameRecorder"].as_bool(), s["sound"].as_bool()), (Some(0), Some(true), Some(false)), "{s}");
    assert_eq!((&s["log"], &s["perf"], s["styleW"].as_u64()), (&serde_json::json!({ "a": 1 }), &serde_json::json!({ "b": 2 }), Some(300)));
    assert!(s["fns"].as_array().unwrap().iter().all(|f| f == "function"), "{s}");
    let e = &h.events("adErr")[0];
    assert_eq!((e["errCode"].as_u64(), e["errMsg"].as_str()), (Some(1004), Some("BannerAd:fail no advertisement")));
    assert_eq!(h.events("show")[0]["ok"], false);
    assert!(h.events("gsFail")[0]["errMsg"].as_str().unwrap().starts_with("login:fail"));
    assert!(h.host.iter().any(|e| e["type"] == "menuStyle" && e["style"] == "dark"));
    assert!(h.host.iter().any(|e| e["type"] == "statusBar" && e["style"] == "white"));
}

/// H3：宿主接了 login 与 BannerAd.show：调用变成 hostCall 事件（args 带上 contextId），宿主 host_reply 之后 success 拿到原样的参数；
/// 宿主推的 contextEvent 到对象的 onLoad
#[test]
fn h3_宿主接管() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
wx.login({ timeout: 5000, success: function (r) { T.ev('login', r); } });
var ad = wx.createBannerAd({ adUnitId: 'u1' });
ad.onLoad(function (d) { T.ev('adLoad', d); });
ad.show().then(function (r) { T.ev('adShow', r); });
T.ev('adId', { id: ad.__id });
T.ready('h3');
"#;
    let mut h = GameHarness::open_inline("h3", js, Opts { host_apis: &["login", "BannerAd.show", "BannerAd.load"], ..Default::default() });
    h.vsyncs(2);
    let calls: Vec<serde_json::Value> = h.host.iter().filter(|e| e["type"] == "hostCall").cloned().collect();
    let login = calls.iter().find(|c| c["api"] == "login").expect("login 的 hostCall");
    assert_eq!(login["args"]["timeout"], 5000);
    let show = calls.iter().find(|c| c["api"] == "BannerAd.show").expect("BannerAd.show 的 hostCall");
    let ad_id = h.events("adId")[0]["id"].as_str().unwrap().to_string();
    assert_eq!(show["args"]["contextId"], ad_id.as_str());
    assert!(h.engine.host_reply(login["id"].as_u64().unwrap() as u32, true, r#"{"code":"abc","errMsg":"login:ok"}"#));
    h.engine.host_reply(show["id"].as_u64().unwrap() as u32, true, r#"{"errMsg":"show:ok"}"#);
    h.engine.dispatch_event("contextEvent", &serde_json::json!({ "context": "BannerAd", "contextId": ad_id, "event": "load", "data": { "w": 1 } }).to_string());
    h.vsyncs(2);
    assert_eq!(h.events("login")[0]["code"], "abc");
    assert_eq!(h.events("adShow")[0]["errMsg"], "show:ok");
    assert_eq!(h.events("adLoad")[0]["w"], 1);
    assert!(h.events("adErr").is_empty());
}

/// G7.5（引擎侧）：传感器由宿主采集。start 交宿主（hostCall 带 interval），宿主推 accelerometerChange 到 onAccelerometerChange；
/// 宿主没接时 start fail
#[test]
fn s1_传感器() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
wx.onAccelerometerChange(function (r) { T.ev('acc', r); });
wx.startAccelerometer({ interval: 'game', success: function (r) { T.ev('start', r); }, fail: function (r) { T.ev('startFail', r); } });
wx.startGyroscope({ fail: function (r) { T.ev('gyroFail', r); } });
T.ready('s1');
"#;
    let mut h = GameHarness::open_inline("s1", js, Opts { host_apis: &["startAccelerometer"], ..Default::default() });
    h.vsyncs(2);
    let call = h.host.iter().find(|e| e["type"] == "hostCall" && e["api"] == "startAccelerometer").expect("交宿主").clone();
    assert_eq!(call["args"]["interval"], "game");
    h.engine.host_reply(call["id"].as_u64().unwrap() as u32, true, r#"{"errMsg":"startAccelerometer:ok"}"#);
    assert_eq!(h.engine.dispatch_event("accelerometerChange", r#"{"x":0.1,"y":-0.2,"z":1.0}"#), 1);
    h.vsyncs(2);
    assert_eq!(h.events("start")[0]["errMsg"], "startAccelerometer:ok");
    assert_eq!(h.events("acc")[0], serde_json::json!({ "x": 0.1, "y": -0.2, "z": 1.0 }));
    assert!(h.events("gyroFail")[0]["errMsg"].as_str().unwrap().starts_with("startGyroscope:fail"), "宿主没接");
}
