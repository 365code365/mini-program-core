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
