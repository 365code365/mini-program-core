//! 切后台 / 回前台时当前页也收到 onHide / onShow（与微信一致）。从前 `MiniEngine::on_hide` / `on_show`
//! 只派发 App 级，页面里「回前台就刷新」的写法不生效（原生示例 route 页的生命周期流水，
//! `tools/android-native-func.sh`「回前台后生命周期流水多了 onHide / onShow」一直不过）。
//! 走 `MiniEngine`，独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use std::path::PathBuf;

#[test]
fn background_dispatches_page_and_app_lifecycle_once_in_order() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("app-bg-page-life");
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/a/a"]}"#),
        ("app.js", "App({ globalData:{log:[]}, onShow(){ this.globalData.log.push('App.onShow') }, onHide(){ this.globalData.log.push('App.onHide') } });"),
        ("app.wxss", ""),
        ("pages/a/a.json", "{}"),
        ("pages/a/a.wxml", "<view>A</view>"),
        ("pages/a/a.wxss", ""),
        (
            "pages/a/a.js",
            "const g = () => getApp().globalData.log; Page({ onShow(){ g().push('Page.onShow') }, onHide(){ g().push('Page.onHide') } });",
        ),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let data = root.join("_data");
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.launch(Some("pages/a/a")).expect("打开页面");
    e.pump(16);
    e.eval("getApp().globalData.log = []").unwrap();
    // 启动后已在前台：多来的 on_show 不派发；一次切后台宿主可能报两三条（Flutter 的 hidden + paused），只派发一次
    e.on_show();
    e.on_hide();
    e.on_hide();
    e.on_show();
    e.on_show();
    let log = e.eval("getApp().globalData.log.join(',')").unwrap();
    std::fs::remove_dir_all(&root).ok();
    assert_eq!(log, "Page.onHide,App.onHide,App.onShow,Page.onShow");
}
