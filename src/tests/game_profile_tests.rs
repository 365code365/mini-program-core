//! 小游戏 G0：前置代码分组、包种类入口、`call_function` 的转义（game-doc/04 G0.4 / G0.5 / G0.7）

use crate::js::{MiniAppApi, PreludeProfile, GAME_CORE};
use crate::runtime::MiniApp;

/// 小程序装的段：名字与顺序**写死**。G0.4 是纯搬迁，这张表与引入 profile 之前的 `PRELUDE` 逐项相同；
/// 以后谁改了小程序的 prelude 集合或顺序，必须同时改这里并说明原因（顺序有意义，见 js/api.rs 文件头）
const MINI_PROGRAM_ORDER: &[&str] = &[
    "module", "console", "fake-clock", "wx-object", "codec", "timer", "storage", "ui", "canvas", "media",
    "query", "canvas2d", "app-page", "app-hooks", "lifecycle", "page-components", "route", "device", "network",
    "host", "host-media", "system", "tabbar", "socket", "fs", "audio", "app-info", "host-capabilities",
    "base-ext", "host-objects", "net-socket", "route-ext", "page-ext", "media-ext", "worker-mq", "animate",
    "worklet", "webaudio", "component", "component-tree", "wxs", "api-probe",
];

#[test]
fn 小程序的前置代码集合与顺序不变() {
    assert_eq!(MiniAppApi::prelude_names(), MINI_PROGRAM_ORDER);
    assert_eq!(MiniAppApi::prelude_names_for(PreludeProfile::MiniProgram), MINI_PROGRAM_ORDER);
}

#[test]
fn 小游戏的通用段都在表里且按表的顺序() {
    let all = MiniAppApi::prelude_names();
    for n in GAME_CORE {
        assert!(all.contains(n), "GAME_CORE 里的 {n} 不在 PRELUDE 里");
    }
    let game = MiniAppApi::prelude_names_for(PreludeProfile::MiniGame);
    assert_eq!(game.len(), GAME_CORE.len());
    // 顺序跟着表走，不跟着 GAME_CORE 的书写顺序走
    let idx: Vec<usize> = game.iter().map(|n| all.iter().position(|a| a == n).unwrap()).collect();
    assert!(idx.windows(2).all(|w| w[0] < w[1]), "小游戏段必须保持表内相对顺序：{game:?}");
    assert_eq!(game.last(), Some(&"api-probe"), "api-probe 必须最后装");
    for n in ["app-page", "route", "tabbar", "component", "query", "wxs"] {
        assert!(!game.contains(&n), "小程序专属段 {n} 不该装进小游戏");
    }
}

#[test]
fn 小游戏的通用段能单独装上且_wx_基础能力可用() {
    // 只装通用段时不能有任何一段因为引用了小程序专属段的全局而在加载期报错
    let mut app = MiniApp::new(375, 667).expect("MiniApp");
    app.init_profile(PreludeProfile::MiniGame).expect("小游戏前置代码加载失败");
    let r = app
        .eval(
            "[typeof wx, typeof wx.request, typeof wx.getSystemInfoSync, typeof wx.getFileSystemManager, \
              typeof wx.createInnerAudioContext, typeof setTimeout, typeof require, typeof Page, typeof wx.navigateTo].join(',')",
        )
        .unwrap();
    assert_eq!(r, "object,function,function,function,function,function,function,undefined,undefined");
}
