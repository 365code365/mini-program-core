//! GT-OD（G9.7 开放数据域）：独立 JS 环境、sharedCanvas、单向消息、主域读像素受限、关系链 API 交宿主。夹具 sample-game/g-opendata。
use super::*;

#[test]
fn od1_共享画布与消息() {
    let _g = serial();
    let mut h = GameHarness::open("g-opendata", Opts::default());
    h.vsyncs(4);
    assert_eq!(h.events("main")[0], serde_json::json!({ "same": true, "ctx": null, "w": 200, "read": "denied" }));
    assert!(h.has("[开放数据域] 开放数据域就绪 200x200"), "{:?}", h.log);
    assert!(h.has("wx.request=undefined storage=undefined gl=null"), "开放数据域没有网络 / 存储 / WebGL：{:?}", h.log);
    // 开放数据域画红，主域 drawImage 到左上 200×200
    assert_eq!(webgl_px(&mut h, 100, 100), [255, 0, 0]);
    assert_eq!(webgl_px(&mut h, 300, 300), [32, 48, 64]);
    // 主域的全局在开放数据域里看不到
    h.engine.eval("GameGlobal.secret = 1").unwrap();
    assert!(h.errors.is_empty(), "{:?}", h.errors);
}

#[test]
fn od2_关系链交宿主() {
    let _g = serial();
    // 没点名：fail
    let mut h = GameHarness::open("g-opendata", Opts { query: "cmd=rank", ..Default::default() });
    h.vsyncs(5);
    assert!(h.has("rank fail getFriendCloudStorage:fail 宿主没有提供"), "{:?}", h.log);
    // 点名 openData.getFriendCloudStorage：hostCall → 宿主回两条
    let mut h = GameHarness::open("g-opendata", Opts { query: "cmd=rank", host_apis: &["openData.getFriendCloudStorage"], ..Default::default() });
    h.vsyncs(3);
    let call = h.host.iter().find(|e| e["type"] == "hostCall" && e["api"] == "openData.getFriendCloudStorage").cloned().expect("hostCall");
    assert_eq!(call["args"]["keyList"], serde_json::json!(["score"]));
    let id = call["id"].as_u64().unwrap() as u32;
    let data = serde_json::json!({ "data": [{ "openid": "a", "KVDataList": [{ "key": "score", "value": "150" }] }, { "openid": "b", "KVDataList": [{ "key": "score", "value": "80" }] }] });
    assert!(h.engine.host_reply(id, true, &data.to_string()));
    h.vsyncs(3);
    assert!(h.has("rank ok 2"), "{:?}", h.log);
    // 第一条 150 宽的白条在 (10..160, 10..30)
    assert_eq!(webgl_px(&mut h, 100, 20), [255, 255, 255]);
    assert_eq!(webgl_px(&mut h, 180, 20), [255, 0, 0]);
}

/// PC1：指针锁定 / 手柄 / 光标 / getGPU 的形状与宿主推的状态
#[test]
fn pc1_指针锁定_手柄_光标() {
    let _g = serial();
    let mut h = GameHarness::open_inline("pc1", r#"
var T = require('_lib/t.js');
var ev = [];
wx.onGamepadConnected(function (e) { ev.push('on:' + e.gamepad.id); });
wx.onGamepadDisconnected(function (e) { ev.push('off:' + e.gamepad.index); });
globalThis.snap = function () { T.ev('s', { locked: wx.isPointerLocked(), pads: wx.getGamepads(), ev: ev }); };
wx.requestPointerLock(); wx.setCursor('cursor.png', 2, 3);
wx.getGPU().requestAdapter().then(function (a) { T.ev('gpu', { adapter: a, fmt: wx.getGPU().getPreferredCanvasFormat() }); });
snap();
T.ready('pc1');
"#, Opts::default());
    h.vsyncs(2);
    assert_eq!(h.events("s")[0], serde_json::json!({ "locked": false, "pads": [], "ev": [] }));
    assert_eq!(h.events("gpu")[0], serde_json::json!({ "adapter": null, "fmt": "rgba8unorm" }));
    let kinds: Vec<_> = h.host.iter().filter(|e| e["type"] == "pointerLock" || e["type"] == "cursor").cloned().collect();
    assert_eq!(kinds, vec![serde_json::json!({ "type": "pointerLock", "lock": true }), serde_json::json!({ "type": "cursor", "path": "cursor.png", "x": 2, "y": 3 })]);
    h.engine.dispatch_event("pointerLockChange", r#"{"locked":true}"#);
    h.engine.dispatch_event("gamepad", r#"{"index":0,"id":"Xbox","buttons":[{"pressed":true,"value":1}],"axes":[0.5,-1]}"#);
    h.engine.eval("snap()").unwrap();
    h.vsyncs(1);
    let s = &h.events("s")[1];
    assert_eq!((s["locked"].as_bool(), s["pads"][0]["id"].as_str(), s["pads"][0]["axes"][1].as_f64(), s["ev"].clone()), (Some(true), Some("Xbox"), Some(-1.0), serde_json::json!(["on:Xbox"])));
    h.engine.dispatch_event("gamepad", r#"{"index":0,"id":"Xbox","connected":false}"#);
    h.engine.eval("snap()").unwrap();
    h.vsyncs(1);
    assert_eq!(h.events("s")[2]["pads"], serde_json::json!([]));
    assert_eq!(h.events("s")[2]["ev"], serde_json::json!(["on:Xbox", "off:0"]));
}
