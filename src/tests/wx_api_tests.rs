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
        Some(UiEvent::ShowToast { title, icon, duration, .. }) => {
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
    assert!(matches!(evs.first(), Some(UiEvent::ShowLoading { title, .. }) if title == "加载中"));
    assert!(evs.iter().any(|e| matches!(e, UiEvent::HideLoading { .. })));
    assert!(evs.iter().any(|e| matches!(e, UiEvent::HideToast { .. })));
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
        Some(UiEvent::ShowModal { title, content, show_cancel, cancel_text, confirm_text, cancel_color, confirm_color }) => {
            assert_eq!(title, "提示");
            assert_eq!(content, "确定删除？");
            assert!(*show_cancel, "showCancel 缺省为 true");
            assert_eq!(cancel_text, "取消");
            assert_eq!(confirm_text, "确定");
            assert_eq!(cancel_color, "#000000", "cancelColor 缺省 #000000");
            assert_eq!(confirm_color, "#576B95", "confirmColor 缺省 #576B95");
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

// ── 提示框 / 弹窗与微信对齐的那几条（官方 api/ui/interaction/*）──

#[test]
fn toast_的_icon_非法值按_success_mask_与_image_透传() {
    let mut app = new_app();
    app.load_script("wx.showToast({ title: 'a', icon: 'fail', mask: true, image: '/img/x.png' });").unwrap();
    match ui_events(&mut app).first() {
        Some(UiEvent::ShowToast { icon, mask, image, .. }) => {
            assert_eq!(icon, "success", "合法值只有 success/error/loading/none");
            assert!(*mask);
            assert_eq!(image, "/img/x.png");
        }
        other => panic!("实际 {:?}", other),
    }
}

#[test]
fn 提示框回调带_err_msg_没回调时返回_promise() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __r = {};
        wx.showToast({ title: 'a', success: function (r) { __r.toast = r.errMsg; } });
        wx.showLoading({ title: 'b', complete: function (r) { __r.loading = r.errMsg; } });
        __r.promise = typeof wx.hideLoading().then;
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "__r.toast"), "showToast:ok");
    assert_eq!(get(&app, "__r.loading"), "showLoading:ok");
    assert_eq!(get(&app, "__r.promise"), "function");
    let _ = ui_events(&mut app);
}

#[test]
fn 标题不是字符串时_fail_且什么也不弹() {
    let mut app = new_app();
    app.load_script("var __e = ''; wx.showLoading({ fail: function (r) { __e = r.errMsg; } });").unwrap();
    assert_eq!(get(&app, "__e"), "showLoading:fail parameter error: parameter.title should be String instead of Undefined;");
    assert!(ui_events(&mut app).iter().all(|e| !matches!(e, UiEvent::ShowLoading { .. })));
}

#[test]
fn hide_默认混用_no_conflict_只关自己_空槽位_fail() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __h = [];
        function rec(r) { __h.push(r.errMsg); }
        wx.showLoading({ title: 'x' });
        wx.hideToast({ success: rec, fail: rec });              // 混用：关掉 Loading
        wx.hideLoading({ success: rec, fail: rec });            // 已经空了
        wx.showLoading({ title: 'y' });
        wx.hideToast({ noConflict: true, success: rec, fail: rec }); // 不碰 Loading
        wx.hideLoading({ noConflict: true, success: rec, fail: rec });
        "#,
    )
    .unwrap();
    assert_eq!(
        get(&app, "__h.join('|')"),
        "hideToast:ok|hideLoading:fail:toast can't be found|hideToast:fail:toast can't be found|hideLoading:ok"
    );
    let evs = ui_events(&mut app);
    assert!(evs.iter().any(|e| matches!(e, UiEvent::HideToast { no_conflict: false })));
    assert!(evs.iter().any(|e| matches!(e, UiEvent::HideLoading { no_conflict: true })));
}

#[test]
fn toast_到时只收自己_不会误关后来的_loading() {
    let mut app = new_app();
    // showToast 的计时器在 showLoading 时被清掉：计时器跑完也不该发 HideToast
    app.load_script("wx.showToast({ title: 'a', duration: 1 }); wx.showLoading({ title: 'b' });").unwrap();
    let _ = ui_events(&mut app);
    std::thread::sleep(std::time::Duration::from_millis(20));
    let evs = ui_events(&mut app);
    assert!(evs.iter().all(|e| !matches!(e, UiEvent::HideToast { .. })), "{:?}", evs);
}

#[test]
fn modal_颜色与按钮文字长度() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __e = '';
        wx.showModal({ title: 't', confirmText: '我知道了啊', fail: function (r) { __e = r.errMsg; } });
        wx.showModal({ title: 't', cancelColor: '#ff0000', confirmColor: 'red' });
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "__e"), "showModal:fail confirmText length should not larger than 4 Chinese characters");
    let evs = ui_events(&mut app);
    let modals: Vec<_> = evs.iter().filter(|e| matches!(e, UiEvent::ShowModal { .. })).collect();
    assert_eq!(modals.len(), 1, "超长的那个不弹");
    match modals[0] {
        UiEvent::ShowModal { cancel_color, confirm_color, .. } => {
            assert_eq!(cancel_color, "#ff0000");
            assert_eq!(confirm_color, "#576B95", "不是 16 进制串就用缺省");
        }
        _ => unreachable!(),
    }
}

#[test]
fn modal_连弹两个_第二个的回调照样执行() {
    let app = new_app();
    app.load_script(
        r#"
        var __done = '';
        wx.showModal({ title: '1', success: function (r) {
            if (r.confirm) wx.showModal({ title: '2', success: function (r2) { __done = r2.errMsg; } });
        } });
        __handleModalResult(true);
        __handleModalResult(true);
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "__done"), "showModal:ok");
}

#[test]
fn modal_没回调时返回_promise() {
    let app = new_app();
    app.load_script(
        "var __p = ''; wx.showModal({ title: 't' }).then(function (r) { __p = r.errMsg + ' ' + r.confirm; }); __handleModalResult(false);",
    )
    .unwrap();
    // Promise 的回调在微任务里跑：再 eval 一次让队列排空
    app.eval("1").ok();
    assert_eq!(get(&app, "__p"), "showModal:ok false");
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
/// 所以引擎根本没有的 API 必须**保持 undefined**：给个永远不回调的 no-op 会让框架以为能力存在，
/// 后面走进去拿不到回调，反而更难查。
///
/// 例外是微信里**总是存在**的宿主能力（login / chooseImage / getLocation …）：它们是函数，
/// 宿主没在 `hostApis` 里接管时**立刻 fail** 并写明原因（system.js）—— 回调一定会来，
/// 页面的 fail 分支接得住；而 undefined 会让 `uni.login → wx.login` 这类包装层直接 TypeError。
#[test]
fn unimplemented_apis_stay_undefined_for_capability_probes() {
    let app = new_app();
    // 蓝牙 / UDP / iBeacon：引擎与宿主契约里都没有，保持 undefined
    for name in ["startBluetoothDevicesDiscovery", "createUDPSocket", "startBeaconDiscovery"] {
        assert_eq!(
            get(&app, &format!("typeof wx.{name}")),
            "undefined",
            "wx.{name} 未实现时必须是 undefined，不能是假函数"
        );
    }
    for name in ["login", "getUserProfile", "requestPayment", "chooseImage", "getLocation"] {
        assert_eq!(get(&app, &format!("typeof wx.{name}")), "function", "wx.{name} 是会立刻 fail 的默认实现");
        assert_eq!(get(&app, &format!("String(wx.{name}.__default)")), "true", "wx.{name} 没被宿主接管");
    }
    // 已实现的必须是函数
    for name in ["request", "showToast", "setStorageSync", "navigateTo", "getSystemInfoSync",
                 "connectSocket", "createInnerAudioContext", "getFileSystemManager", "canIUse", "getMenuButtonBoundingClientRect"] {
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
