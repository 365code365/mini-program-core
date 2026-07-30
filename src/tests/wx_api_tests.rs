//! `wx.*` API 通路测试（离线，不依赖窗体）
//!
//! 之前只有 `wx.request` 有测试，界面类/存储类/路由类 API 全靠人肉点。这里按
//! 「JS 调用 → 桥事件 → 宿主可见的 UiEvent / 全局变量」这条通路逐个钉住，
//! 顺带把几条**容易被改错的微信语义**固化下来：
//! - `getStorageSync` 缺 key 返回 `''`（不是 undefined / null）；
//! - 未实现的 API 必须是 `undefined`，不能返回 no-op 函数；
//! - `wx.onError` 与 `App({onError})` 并存，两边都会收到。

use crate::runtime::{MiniApp, UiEvent};

fn new_app() -> MiniApp {
    let mut app = MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init MiniApp");
    app
}

fn get(app: &MiniApp, expr: &str) -> String {
    app.eval(expr).unwrap_or_default()
}

/// 跑一轮 update 把桥事件搬到 UiEvent 队列，再全部取出
fn ui_events(app: &mut MiniApp) -> Vec<UiEvent> {
    app.update().expect("update");
    app.drain_ui_events()
}

// ───────────────────────────── 界面类 ─────────────────────────────

#[test]
fn show_toast_carries_title_icon_and_duration() {
    let mut app = new_app();
    app.load_script("wx.showToast({ title: '已保存', icon: 'success', duration: 800 });")
        .unwrap();
    let evs = ui_events(&mut app);
    match evs.first() {
        Some(UiEvent::ShowToast { title, icon, duration }) => {
            assert_eq!(title, "已保存");
            assert_eq!(icon, "success");
            assert_eq!(*duration, 800);
        }
        other => panic!("应产出 ShowToast，实际 {:?}", other),
    }
}

#[test]
fn show_toast_defaults_match_wechat() {
    // 微信默认 icon=success、duration=1500
    let mut app = new_app();
    app.load_script("wx.showToast({ title: 'x' });").unwrap();
    match ui_events(&mut app).first() {
        Some(UiEvent::ShowToast { icon, duration, .. }) => {
            assert_eq!(icon, "success");
            assert_eq!(*duration, 1500);
        }
        other => panic!("实际 {:?}", other),
    }
}

#[test]
fn hide_toast_and_loading_produce_events() {
    let mut app = new_app();
    app.load_script(
        "wx.showLoading({ title: '加载中' }); wx.hideLoading(); wx.showToast({title:'t'}); wx.hideToast();",
    )
    .unwrap();
    let evs = ui_events(&mut app);
    assert!(matches!(evs.first(), Some(UiEvent::ShowLoading { title }) if title == "加载中"));
    assert!(evs.iter().any(|e| matches!(e, UiEvent::HideLoading)));
    assert!(evs.iter().any(|e| matches!(e, UiEvent::HideToast)));
}

#[test]
fn show_modal_defaults_and_result_callback() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __m = { confirm: null, cancel: null, done: 0 };
        wx.showModal({
            title: '提示', content: '确定删除？',
            success: function (r) { __m.confirm = r.confirm; __m.cancel = r.cancel; },
            complete: function () { __m.done++; }
        });
        "#,
    )
    .unwrap();
    match ui_events(&mut app).first() {
        Some(UiEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text }) => {
            assert_eq!(title, "提示");
            assert_eq!(content, "确定删除？");
            assert!(*show_cancel, "showCancel 缺省为 true");
            assert_eq!(cancel_text, "取消");
            assert_eq!(confirm_text, "确定");
        }
        other => panic!("应产出 ShowModal，实际 {:?}", other),
    }
    // 宿主点「确定」后回调：confirm=true / cancel=false 必须成对
    app.load_script("__handleModalResult(true);").unwrap();
    assert_eq!(get(&app, "String(__m.confirm)"), "true");
    assert_eq!(get(&app, "String(__m.cancel)"), "false");
    assert_eq!(get(&app, "String(__m.done)"), "1", "complete 要调一次");
}

#[test]
fn show_modal_cancel_result_is_mirrored() {
    let app = new_app();
    app.load_script(
        "var __r = null; wx.showModal({ title:'t', success: function(x){ __r = x; } }); __handleModalResult(false);",
    )
    .unwrap();
    assert_eq!(get(&app, "String(__r.confirm)"), "false");
    assert_eq!(get(&app, "String(__r.cancel)"), "true");
}

#[test]
fn pull_down_refresh_apis_produce_events() {
    let mut app = new_app();
    app.load_script("wx.startPullDownRefresh(); wx.stopPullDownRefresh();")
        .unwrap();
    let evs = ui_events(&mut app);
    assert!(evs.iter().any(|e| matches!(e, UiEvent::StartPullDownRefresh)));
    assert!(evs.iter().any(|e| matches!(e, UiEvent::StopPullDownRefresh)));
}

// ───────────────────────────── 存储类 ─────────────────────────────

#[test]
fn storage_sync_roundtrip_keeps_types() {
    let app = new_app();
    app.load_script(
        r#"
        wx.setStorageSync('s', 'text');
        wx.setStorageSync('n', 42);
        wx.setStorageSync('o', { a: [1, 2] });
        var __s = wx.getStorageSync('s');
        var __n = wx.getStorageSync('n');
        var __o = wx.getStorageSync('o');
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "__s"), "text");
    assert_eq!(get(&app, "String(__n)"), "42");
    assert_eq!(get(&app, "typeof __o"), "object");
    assert_eq!(get(&app, "String(__o.a[1])"), "2");
}

/// 缺 key 时微信返回空字符串。看着像小事，实际很致命：uni-app/UTS 编译产物里
/// 有 `if (raw != null) raw.toMap()` 这种写法，返回 `''` 会走进去然后
/// `TypeError`；返回 `undefined` 又会让「有没有缓存」的判断全部失效。
/// 保持与微信一致，把责任留在小程序侧。
#[test]
fn missing_storage_key_returns_empty_string_like_wechat() {
    let app = new_app();
    app.load_script("var __v = wx.getStorageSync('never-set');").unwrap();
    assert_eq!(get(&app, "typeof __v"), "string");
    assert_eq!(get(&app, "String(__v.length)"), "0");
}

#[test]
fn storage_remove_and_clear() {
    let app = new_app();
    app.load_script(
        r#"
        wx.setStorageSync('a', 1); wx.setStorageSync('b', 2);
        wx.removeStorageSync('a');
        var __afterRemove = [wx.getStorageSync('a'), wx.getStorageSync('b')].join('|');
        var __info = wx.getStorageInfoSync();
        wx.clearStorageSync();
        var __afterClear = wx.getStorageInfoSync().keys.length;
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "__afterRemove"), "|2", "删掉的 key 回空串，另一个保留");
    assert_eq!(get(&app, "String(__info.keys.length)"), "1");
    assert_eq!(get(&app, "String(__info.limitSize)"), "10240", "微信上限 10MB");
    assert_eq!(get(&app, "__afterClear"), "0");
}

#[test]
fn async_storage_wraps_the_sync_version() {
    let app = new_app();
    app.load_script(
        r#"
        var __got = null, __done = 0;
        wx.setStorage({ key: 'k', data: { v: 9 }, success: function () { __done++; } });
        wx.getStorage({ key: 'k', success: function (r) { __got = r.data; } });
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "String(__done)"), "1");
    assert_eq!(get(&app, "String(__got.v)"), "9");
}

// ───────────────────────────── 路由类 ─────────────────────────────

#[test]
fn navigation_apis_write_pending_navigation() {
    for (call, kind, url) in [
        ("wx.navigateTo({url:'/pages/a/a'})", "navigateTo", "/pages/a/a"),
        ("wx.redirectTo({url:'/pages/b/b'})", "redirectTo", "/pages/b/b"),
        ("wx.switchTab({url:'/pages/c/c'})", "switchTab", "/pages/c/c"),
        ("wx.reLaunch({url:'/pages/d/d'})", "reLaunch", "/pages/d/d"),
    ] {
        let app = new_app();
        app.load_script(call).unwrap();
        let got_kind = get(&app, "__pendingNavigation ? __pendingNavigation.type : ''");
        let got_url = get(&app, "__pendingNavigation ? String(__pendingNavigation.url) : ''");
        assert_eq!(got_kind, kind, "{call} 的类型");
        assert_eq!(got_url, url, "{call} 的目标");
    }
}

#[test]
fn navigate_back_defaults_to_delta_one() {
    let app = new_app();
    app.load_script("wx.navigateBack();").unwrap();
    assert_eq!(get(&app, "__pendingNavigation.type"), "navigateBack");
    assert_eq!(get(&app, "String(__pendingNavigation.delta)"), "1");
    let app2 = new_app();
    app2.load_script("wx.navigateBack({ delta: 3 });").unwrap();
    assert_eq!(get(&app2, "String(__pendingNavigation.delta)"), "3");
}

// ───────────────────────────── 设备信息 ─────────────────────────────

#[test]
fn system_info_reports_the_render_size() {
    let app = new_app();
    app.load_script("var __i = wx.getSystemInfoSync();").unwrap();
    assert_eq!(get(&app, "String(__i.windowWidth)"), "375");
    assert!(
        get(&app, "String(__i.windowHeight)").parse::<f32>().unwrap_or(0.0) > 0.0,
        "windowHeight 要有值"
    );
    assert!(!get(&app, "String(__i.platform)").is_empty());
    assert!(
        get(&app, "String(__i.pixelRatio)").parse::<f32>().unwrap_or(0.0) >= 1.0,
        "pixelRatio 至少 1"
    );
    // 新版拆分接口应与 getSystemInfoSync 对得上
    assert_eq!(
        get(&app, "String(wx.getWindowInfo().windowWidth)"),
        get(&app, "String(__i.windowWidth)")
    );
    assert!(!get(&app, "String(wx.getDeviceInfo().platform)").is_empty());
    assert!(!get(&app, "String(wx.getAppBaseInfo().SDKVersion)").is_empty());
}

// ───────────────────────── 动画 / 画布 ─────────────────────────

#[test]
fn create_animation_exports_ordered_actions() {
    let app = new_app();
    app.load_script(
        r#"
        var a = wx.createAnimation({ duration: 300, timingFunction: 'ease' });
        a.opacity(0.5).translateX(20).step();
        a.rotate(45).step({ duration: 100 });
        var __acts = a.export().actions;
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "String(__acts.length)"), "2", "两次 step 两个动作");
    assert_eq!(get(&app, "String(__acts[0].option.transition.duration)"), "300");
    assert_eq!(get(&app, "__acts[0].option.transition.timingFunction"), "ease");
    assert_eq!(
        get(&app, "String(__acts[1].option.transition.duration)"),
        "100",
        "step 里的 duration 要覆盖构造时的默认值"
    );
    // export 之后动作队列清空（微信语义：一次 export 消费一次）
    assert_eq!(get(&app, "String(a.export().actions.length)"), "0");
}

#[test]
fn canvas_context_draw_reaches_the_bridge() {
    let mut app = new_app();
    app.load_script(
        r#"
        var ctx = wx.createCanvasContext('c1');
        ctx.setFillStyle('#ff0000');
        ctx.fillRect(0, 0, 10, 20);
        ctx.draw();
        "#,
    )
    .unwrap();
    // CanvasDraw 不走 UiEvent，落在 canvas 指令队列里；这里只验证 update 不报错
    // 且 JS 侧队列已被 draw() 清空（否则重复 draw 会叠加历史指令）
    app.update().unwrap();
    assert_eq!(
        get(&app, "String(ctx.__cmds ? ctx.__cmds.length : 0)"),
        "0",
        "draw() 之后指令队列应清空"
    );
}

// ─────────────────── 未实现 API 的探针语义 ───────────────────

/// 框架层普遍用 `typeof wx.X === 'function'` 做能力探测（uni-app、Taro 都是）。
/// 所以未实现的 API 必须**保持 undefined**：给个 no-op 函数会让框架以为能力存在，
/// 后面走进去拿不到回调，反而更难查。
#[test]
fn unimplemented_apis_stay_undefined_for_capability_probes() {
    let app = new_app();
    for name in ["login", "getUserProfile", "requestPayment", "chooseImage", "getLocation"] {
        assert_eq!(
            get(&app, &format!("typeof wx.{name}")),
            "undefined",
            "wx.{name} 未实现时必须是 undefined，不能是假函数"
        );
    }
    // 已实现的必须是函数
    for name in ["request", "showToast", "setStorageSync", "navigateTo", "getSystemInfoSync"] {
        assert_eq!(get(&app, &format!("typeof wx.{name}")), "function", "wx.{name}");
    }
}

/// `wx.onError` 是真实存在的监听型 API，且与 `App({onError})` 并存。
/// 缺了它，uni-app 的包装层会 `wx.onError.apply` 直接 TypeError，整个 app 起不来。
#[test]
fn app_level_listeners_coexist_with_app_options() {
    let app = new_app();
    app.load_script(
        r#"
        var __hits = [];
        App({ onError: function (e) { __hits.push('app:' + e); } });
        wx.onError(function (e) { __hits.push('listener:' + e); });
        __dispatchApp('onError', 'boom');
        "#,
    )
    .unwrap();
    let hits = get(&app, "__hits.join(',')");
    assert!(hits.contains("app:boom"), "App({{onError}}) 要收到: {hits}");
    assert!(hits.contains("listener:boom"), "wx.onError 要收到: {hits}");
}

#[test]
fn app_listeners_can_be_removed() {
    let app = new_app();
    app.load_script(
        r#"
        var __n = 0;
        function h() { __n++; }
        wx.onUnhandledRejection(h);
        __dispatchApp('onUnhandledRejection', {});
        wx.offUnhandledRejection(h);
        __dispatchApp('onUnhandledRejection', {});
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "String(__n)"), "1", "off 之后不该再触发");
}

#[test]
fn console_has_every_method_frameworks_reduce_over() {
    // uni-app 的运行时会对 ['log','warn','error','info','debug'] 逐个
    // `console[t].bind(console)`，少一个就整包挂掉
    let app = new_app();
    for m in ["log", "warn", "error", "info", "debug", "trace", "group", "groupEnd", "time", "timeEnd"] {
        assert_eq!(
            get(&app, &format!("typeof console.{m}")),
            "function",
            "console.{m} 必须存在"
        );
    }
}
