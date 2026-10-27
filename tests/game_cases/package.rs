//! GT-P（分包）与 GT-LC（生命周期），game-doc/05 §3。夹具 sample-game/g-subpkg、g-lifecycle。
use super::*;

fn ev(h: &GameHarness, name: &str, key: &str, val: &str) -> serde_json::Value {
    h.events(name).into_iter().find(|e| e[key] == val).unwrap_or_else(|| panic!("缺 {name} {key}={val}：{:?}", h.events(name)))
}

/// P1~P4：按名字 / 按 root 加载，进度 100、执行完才 success；重复加载不再执行；不存在的 fail；入口抛错 fail 且进 onError；
/// 加载之前分包里的模块 require 不到
#[test]
fn p1_p4_分包加载() {
    let _g = serial();
    let mut h = GameHarness::open("g-subpkg", Opts::default());
    h.engine.eval("go('pre')").unwrap();
    h.vsyncs(1);
    assert!(h.events("pre")[0]["err"].as_str().unwrap().contains("stage1"), "加载之前 require 不到：{:?}", h.events("pre"));
    for (step, tag) in [("a", "byName"), ("b", "byRoot"), ("c", "again"), ("d", "missing"), ("e", "broken")] {
        h.engine.eval(&format!("go('{step}')")).unwrap();
        h.until(&format!("\"tag\":\"{tag}\""), 10);
    }
    let a = ev(&h, "load", "tag", "byName");
    assert_eq!((a["ok"].as_bool(), a["errMsg"].as_str()), (Some(true), Some("loadSubpackage:ok")));
    assert_eq!(a["prog"], serde_json::json!([100]), "本地包一次 100%");
    assert_eq!(a["ran"], serde_json::json!(["stage1:42"]), "success 时入口已经执行过（分包内相对 require 可用）");
    assert_eq!(ev(&h, "load", "tag", "byRoot")["ran"], serde_json::json!(["stage1:42", "one"]), "单文件分包按 root 加载");
    let again = ev(&h, "load", "tag", "again");
    assert_eq!((again["ok"].as_bool(), &again["ran"]), (Some(true), &serde_json::json!(["stage1:42", "one"])), "重复加载不再执行");
    let miss = ev(&h, "load", "tag", "missing");
    assert_eq!(miss["ok"], false);
    assert!(miss["errMsg"].as_str().unwrap().starts_with("loadSubpackage:fail"), "{miss}");
    let br = ev(&h, "load", "tag", "broken");
    assert!(br["ok"] == false && br["errMsg"].as_str().unwrap().contains("故意抛错"), "{br}");
    assert!(h.events("onError").iter().any(|e| e["message"].as_str().unwrap_or("").contains("故意抛错")), "入口的异常进 wx.onError");
    let main = h.events("run");
    assert_eq!(main.len(), 1, "主包只执行一次：{main:?}");
}

/// P5：从独立分包启动：主包没执行、主包模块 require 不到；loadSubpackage('__GAME__') 之后主包 game.js 执行一次
#[test]
fn p5_独立分包启动() {
    let _g = serial();
    let mut h = GameHarness::open("g-subpkg", Opts { entry: Some("indep"), ..Default::default() });
    h.until("T:READY g-subpkg-indep", 5);
    let runs = h.events("run");
    assert_eq!(runs.len(), 1);
    assert_eq!((runs[0]["who"].as_str(), runs[0]["mainRan"].as_bool()), (Some("indep"), Some(false)));
    let pre = &h.events("indepPre")[0];
    assert!(pre["err"].as_str().unwrap().contains("stage1") && pre["errLib"].as_str().unwrap().contains("__GAME__"), "主包模块锁着：{pre}");
    h.engine.eval("goMain()").unwrap();
    h.until("T:EV main ", 10);
    let m = &h.events("main")[0];
    assert_eq!((m["ok"].as_bool(), m["runs"].as_u64()), (Some(true), Some(1)), "{m}");
    assert!(h.events("run").iter().any(|r| r["who"] == "main"));
}

/// LC1~LC3：冷启动不派发 onShow；前后台 ×3 成对、onShow 参数 = 进入参数、onHide 时音频已停；offShow；exit / restart 交给宿主
#[test]
fn lc1_lc3_生命周期() {
    let _g = serial();
    let mut h = GameHarness::open("g-lifecycle", Opts { query: "lv=3", ..Default::default() });
    h.vsyncs(3);
    assert_eq!(h.events("launch")[0]["query"]["lv"], "3");
    assert!(h.events("show").is_empty(), "冷启动不派发 onShow");
    for _ in 0..3 {
        h.engine.set_background(true);
        h.engine.set_background(true); // 重复的不再派发
        h.vsyncs(1);
        h.engine.set_background(false);
        h.vsyncs(1);
    }
    let (hides, shows) = (h.events("hide"), h.events("show"));
    assert_eq!((hides.len(), shows.len()), (3, 3), "{hides:?} {shows:?}");
    assert_eq!((shows[2]["scene"].as_u64(), shows[2]["query"]["lv"].as_str()), (Some(1001), Some("3")));
    assert_eq!(hides[0]["bgmPaused"], true);
    h.engine.eval("goOff()").unwrap();
    h.engine.set_background(true);
    h.engine.set_background(false);
    h.vsyncs(1);
    assert_eq!(h.events("show").len(), 3, "offShow 之后不再派发");
    h.engine.eval("goExit('exit'); goExit('restart')").unwrap();
    h.until("\"kind\":\"restart\"", 10);
    let kinds: Vec<&str> = h.host.iter().filter_map(|e| e["type"].as_str()).filter(|t| *t == "exit" || *t == "restart").collect();
    assert_eq!(kinds, ["exit", "restart"], "{:?}", h.host);
    assert_eq!(h.events("exit")[0]["errMsg"], "exitMiniProgram:ok");
}

/// O1：横屏包（宿主按 deviceOrientation 锁方向）windowWidth > windowHeight、主画布默认是横屏尺寸、启动时告诉宿主锁 landscape；
/// O3：setDeviceOrientation 交给宿主，宿主转完 resize → onWindowResize（小游戏形状）与 onDeviceOrientationChange、系统信息更新；
/// 非法值 fail
#[test]
fn o1_o3_横屏与转屏() {
    let _g = serial();
    let mut h = GameHarness::open("g-landscape", Opts::default());
    h.vsyncs(2);
    let b = &h.events("boot")[0];
    assert_eq!((b["ww"].as_f64(), b["wh"].as_f64(), b["cw"].as_u64(), b["ch"].as_u64()), (Some(667.0), Some(375.0), Some(667), Some(375)), "{b}");
    assert_eq!(b["orient"], "landscape");
    assert!(h.host.iter().any(|e| e["type"] == "deviceOrientation" && e["value"] == "landscape" && e["initial"] == true), "{:?}", h.host);
    let (_, _, px) = h.engine.frame_rgba();
    assert_eq!(&px[..3], &[255, 0, 0], "左上角是红块");
    h.engine.eval("goRotate('portrait'); goRotate('upside')").unwrap();
    h.vsyncs(2);
    assert!(h.host.iter().any(|e| e["type"] == "deviceOrientation" && e["value"] == "portrait" && e.get("initial").is_none()));
    assert!(h.events("setOk").len() == 1 && h.events("setFail")[0]["errMsg"].as_str().unwrap().starts_with("setDeviceOrientation:fail"));
    h.engine.resize(375, 667, 2.0, None);
    h.engine.resize(375, 667, 2.0, None); // 没变不再派发
    h.vsyncs(2);
    let r = h.events("resize");
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0]["r"], serde_json::json!({ "windowWidth": 375, "windowHeight": 667 }), "小游戏形状：只有 windowWidth / windowHeight");
    assert_eq!((r[0]["ww"].as_f64(), r[0]["orient"].as_str(), r[0]["cw"].as_u64()), (Some(375.0), Some("portrait"), Some(375)));
    assert_eq!(h.events("orient"), vec![serde_json::json!({ "value": "portrait" })]);
    let (w, hh, px) = h.engine.frame_rgba();
    assert_eq!((w, hh), (375, 667), "游戏在回调里改了主画布尺寸（主画布按逻辑尺寸）");
    let i = ((hh as usize - 1) * w as usize + (w as usize - 1)) * 4;
    assert_eq!(&px[i..i + 3], &[255, 255, 0], "右下角是黄块");
}

/// O2：rotateInEngine（宿主竖屏、包横屏）：游戏看到横屏窗口；输出帧顺时针转 90°（游戏左上 → 设备右上）；
/// 点设备坐标，回调里是游戏坐标，画出来的点在设备上就在点下去的地方（±1px）
#[test]
fn o2_引擎内旋转() {
    let _g = serial();
    let mut h = GameHarness::open("g-landscape", Opts { rotate: true, ..Default::default() });
    h.vsyncs(2);
    let b = &h.events("boot")[0];
    assert_eq!((b["ww"].as_f64(), b["wh"].as_f64(), b["cw"].as_u64()), (Some(667.0), Some(375.0), Some(667)), "{b}");
    let px = |h: &mut GameHarness, x: usize, y: usize| {
        let (w, _, p) = h.engine.frame_rgba();
        let i = (y * w as usize + x) * 4;
        [p[i], p[i + 1], p[i + 2]]
    };
    assert_eq!(h.engine.frame_rgba().0, 375, "输出帧是竖的");
    assert_eq!(px(&mut h, 370, 5), [255, 0, 0], "游戏左上（红）→ 设备右上");
    assert_eq!(px(&mut h, 370, 660), [0, 255, 0], "游戏右上（绿）→ 设备右下");
    assert_eq!(px(&mut h, 5, 5), [0, 0, 255], "游戏左下（蓝）→ 设备左上");
    assert_eq!(px(&mut h, 5, 660), [255, 255, 0], "游戏右下（黄）→ 设备左下");
    let t = h.now_us();
    h.engine.touch(0, 1, 300.0, 200.0, 1.0, t);
    h.engine.touch(2, 1, 300.0, 200.0, 0.0, t);
    h.vsyncs(2);
    let tap = &h.events("tap")[0];
    assert_eq!((tap["x"].as_f64(), tap["y"].as_f64()), (Some(200.0), Some(75.0)), "游戏坐标 = (yd, Wd − xd)");
    assert_eq!(px(&mut h, 300, 200), [255, 255, 255], "点在设备上画在手指下面");
    h.engine.resize(375, 667, 2.0, Some("portrait"));
    h.vsyncs(1);
    assert!(h.events("resize").is_empty() && h.events("orient").is_empty(), "宿主报的还是同一个竖屏：游戏看到的横屏没变");
}
