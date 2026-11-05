//! voip-room 交给宿主自己的通话后台（host/engine_camera.rs、SDK 的 `MiniVoip`）：宿主 `hostApis` 里有 `joinVoIPChat` 时
//! `wx.joinVoIPChat` 走 hostCall、回包原样给页面，`<voip-room mode="video">` 不再报「进不了房」；没接的照旧报一次。
use crate::compositor::HostEnv;
use crate::host::{EngineOptions, MiniEngine};
use serde_json::Value;

fn run(name: &str, host_apis: Vec<String>) -> (MiniEngine, Vec<Value>, std::sync::RwLockWriteGuard<'static, ()>) {
    let g = crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let root = std::env::temp_dir().join("mini-voip-tests").join(name);
    std::fs::remove_dir_all(&root).ok();
    let js = "Page({ data: { log: [] },
  onLoad() { wx.joinVoIPChat({ signature: 's', nonceStr: 'n', timeStamp: 1, groupId: 'g',
    success: r => this.data.log.push('join:' + r.openIdList.join(',')), fail: e => this.data.log.push('fail:' + e.errMsg) }); },
  onErr(e) { this.data.log.push('error'); } });";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", r#"<voip-room openid="o2" mode="video" binderror="onErr" style="width:200px;height:200px"></voip-room>"#),
        ("pages/p/p.wxss", ""),
        ("pages/p/p.js", js),
        ("pages/p/p.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let opts = EngineOptions { same_layer: vec!["voip-room".into()], host_apis, ..Default::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&root.join("_data").to_string_lossy()), 375, 667, 2.0, HostEnv::default(), opts)
        .expect("创建引擎");
    e.freeze_clock_for_test();
    e.launch(None).expect("打开首页");
    let mut evs = Vec::new();
    for t in 1..=5 {
        e.pump(t * 16);
        while let Some(s) = e.poll_event() {
            evs.push(serde_json::from_str(&s).unwrap());
        }
    }
    (e, evs, g)
}

fn log(e: &MiniEngine) -> String {
    e.eval("JSON.stringify(getCurrentPages()[0].data.log)").unwrap_or_default()
}

#[test]
fn 宿主接了通话后台_join_走宿主_voip_room_不报进不了房() {
    let (mut e, evs, _g) = run("host", vec!["joinVoIPChat".into(), "exitVoIPChat".into()]);
    let call = evs.iter().find(|v| v["type"] == "hostCall" && v["api"] == "joinVoIPChat").expect("joinVoIPChat 交给宿主");
    assert_eq!(call["args"]["groupId"], "g");
    let id = call["id"].as_u64().unwrap() as u32;
    assert!(e.host_reply(id, true, r#"{"openIdList":["o1","o2"],"errCode":0,"errMsg":"joinVoIPChat:ok"}"#));
    for t in 6..=8 {
        e.pump(t * 16);
    }
    assert_eq!(log(&e), r#"["join:o1,o2"]"#, "回包原样给页面，voip-room 不报错");
}

#[test]
fn 宿主没接_voip_room_照旧报一次进不了房() {
    let (e, _evs, _g) = run("none", Vec::new());
    let l = log(&e);
    assert!(l.contains("error") && l.contains("fail:"), "{l}");
    assert_eq!(l.matches("error").count(), 1, "{l}");
}
