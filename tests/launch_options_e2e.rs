//! 小程序身份与启动参数：外部 App / H5 / 别的小程序打开小程序时，宿主在创建选项里给 `launchOptions`，
//! AppID 来自包里的 `project.config.json`。判据：
//! - `wx.getAccountInfoSync().miniProgram.appId` 等于 project.config.json 的 appid（从前恒为空串）；
//! - `App.onLaunch(options)` 与 `wx.getLaunchOptionsSync()` 拿到宿主给的 path / query（对象）/ scene（number）/ referrerInfo；
//! - 宿主什么都不给：scene 1001、query 是空对象、appid 是 touristappid 时算未设置（空串）。
//!
//! 独立的集成测试（同 `tests/embed_host.rs` 的理由）：小程序根目录是进程级的。
use mini_render::compositor::HostEnv;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;
use std::sync::Mutex;

static SERIAL: Mutex<()> = Mutex::new(());

fn fixture(name: &str, project: Option<&str>) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launch-options").join(name);
    std::fs::remove_dir_all(&root).ok();
    let mut files = vec![
        ("app.json", r#"{"pages":["pages/p/p"]}"#),
        ("app.js", "App({ onLaunch(o){ this.launched = JSON.stringify(o); } });"),
        ("pages/p/p.wxml", "<view>p</view>"),
        ("pages/p/p.js", "Page({});"),
        ("pages/p/p.json", "{}"),
    ];
    if let Some(p) = project {
        files.push(("project.config.json", p));
    }
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, opts: EngineOptions) -> MiniEngine {
    let data = root.join("_data");
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 1.0, HostEnv::default(), opts)
        .expect("创建引擎");
    e.launch(None).expect("打开首页");
    e
}

#[test]
fn 宿主给的启动参数与_project_config_的_appid_都能读到() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("full", Some(r#"{"appid":"wx3b7d9f1a2c4e6b80","projectname":"x"}"#));
    let opts = EngineOptions::from_json(
        r#"{"launchOptions":{"path":"pages/p/p","query":{"id":"7","from":"h5"},"scene":1065,
            "referrerInfo":{"appId":"wx0000000000000001","extraData":{"k":1}}}}"#,
    )
    .unwrap();
    let e = engine(&root, opts);
    assert_eq!(e.eval("wx.getAccountInfoSync().miniProgram.appId").unwrap(), "wx3b7d9f1a2c4e6b80");
    let lo = e.eval("JSON.stringify(wx.getLaunchOptionsSync())").unwrap();
    let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
    assert_eq!(v["path"], "pages/p/p");
    assert_eq!(v["scene"], 1065, "scene 是 number");
    assert_eq!(v["query"]["id"], "7", "query 是对象");
    assert_eq!(v["referrerInfo"]["appId"], "wx0000000000000001");
    assert_eq!(v["referrerInfo"]["extraData"]["k"], 1);
    // onLaunch 收到同一份
    let launched = e.eval("getApp().launched").unwrap();
    let l: serde_json::Value = serde_json::from_str(&launched).unwrap();
    assert_eq!(l["scene"], 1065);
    assert_eq!(l["query"]["from"], "h5");
}

#[test]
fn 什么都不给_默认值与从前一致_游客appid算未设置() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("bare", Some(r#"{"appid":"touristappid"}"#));
    let e = engine(&root, EngineOptions::default());
    assert_eq!(e.eval("wx.getAccountInfoSync().miniProgram.appId").unwrap(), "");
    assert_eq!(e.eval("String(wx.getLaunchOptionsSync().scene)").unwrap(), "1001");
    assert_eq!(e.eval("JSON.stringify(wx.getLaunchOptionsSync().query)").unwrap(), "{}");
    let l: serde_json::Value = serde_json::from_str(&e.eval("getApp().launched").unwrap()).unwrap();
    assert_eq!(l["scene"], 1001, "onLaunch 也收到默认启动参数");
}
