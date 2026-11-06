//! 挂起 / 恢复（host/engine_park.rs）：小程序 A 打开 B 时 A 留在内存里，B 关掉后 A 原样回来。
//! 照 SDK 的用法：A 在用例线程上，B 在另一条线程上建、跑、销毁（SDK 每个引擎一条线程，thread_local 天然分开）。
use super::*;

fn b_fixture() -> PathBuf {
    let root = std::env::temp_dir().join("mini-wpc-tests").join("park_b");
    std::fs::remove_dir_all(&root).ok();
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/i/i"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/i/i.wxml", "<view>b</view>"),
        ("pages/i/i.wxss", ""),
        ("pages/i/i.js", "Page({ onLoad() { wx.setStorageSync('k', 'b'); } });"),
        ("pages/i/i.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

/// 判据：A 恢复后页面栈（2 页，停在 q）、JS 里的变量、storage、包根都是 A 的；挂起期间 pump 不出帧；
/// onHide 只在挂起时一次；onShow 收到 scene 1038 + referrerInfo.extraData，getEnterOptionsSync 同值、
/// getLaunchOptionsSync 仍是启动时那份
#[test]
fn 挂起_另一个小程序跑完_恢复后原样() {
    let app_js = "App({ log: [], onHide() { this.log.push('hide'); }, onShow(o) { this.log.push('show ' + o.scene + ' ' + ((o.referrerInfo||{}).extraData||{}).x); } });";
    let root = fixture("park_a", "<view>p</view>", "Page({});", &[("app.js", app_js), ("pages/q/q.js", "Page({ onLoad() { this.n = 41; } });")]);
    let mut a = engine(&root, EngineOptions::default());
    js(&a, "wx.setStorageSync('k', 'a'); wx.navigateTo({ url: '/pages/q/q' })");
    assert!(pump_until(&mut a, 1000, |e| e.eval("getCurrentPages().length").as_deref() == Ok("2")), "进到第二页");
    js(&a, "getCurrentPages()[1].n += 1");
    clear_log(&a);

    assert!(a.suspend(), "挂起");
    assert!(!a.suspend(), "已经挂着再挂返回 false");
    assert!(a.is_suspended());
    assert!(!a.pump(5000), "挂起中 pump 不出帧");
    a.on_show(); // 宿主 Activity 回前台的那一下：挂着的不算回来
    assert!(a.is_suspended());
    assert_eq!(log(&a), ["hide"]);
    clear_log(&a);

    // B：另一条线程上建、跑、销毁（宿主保证 B 建之前 A 已挂起、A 恢复之前 B 已销毁）
    let b_root = b_fixture();
    let data = root.join("_data");
    std::thread::spawn(move || {
        let mut b = MiniEngine::new_with_options(&b_root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 1.0, HostEnv::default(), EngineOptions::default())
            .expect("建 B");
        b.use_virtual_clock();
        b.launch(None).expect("B 首页");
        for t in 1..20 {
            b.pump(t * 16);
        }
        assert_eq!(b.eval("wx.getStorageSync('k')").as_deref(), Ok("b"));
    })
    .join()
    .expect("B 线程");
    assert!(crate::host::page_loader::get_app_path().ends_with("park_b"), "B 建引擎时把包根换成了自己的");

    let enter = serde_json::json!({ "path": "", "query": {}, "scene": 1038, "referrerInfo": { "appId": "wxb", "extraData": { "x": 7 } } });
    assert!(a.resume(Some(&enter)), "恢复");
    assert!(!a.resume(None), "没挂着再恢复返回 false");
    assert!(crate::host::page_loader::get_app_path().ends_with("park_a"), "包根还原成 A 的");
    assert_eq!(log(&a), ["show 1038 7"], "onShow 带回 scene 与 extraData（onHide 没有多派）");
    assert_eq!(js(&a, "getCurrentPages().length + ' ' + getCurrentPages()[1].route + ' ' + getCurrentPages()[1].n"), "2 pages/q/q 42", "页面栈与 JS 状态都在");
    assert_eq!(js(&a, "wx.getStorageSync('k')"), "a", "storage 是 A 的");
    assert_eq!(js(&a, "wx.getEnterOptionsSync().scene + ' ' + wx.getEnterOptionsSync().referrerInfo.appId"), "1038 wxb");
    assert_eq!(js(&a, "String(wx.getLaunchOptionsSync().scene)"), "1001", "启动参数不变");
    assert!(pump_until(&mut a, 200, |_| true), "恢复后照常出帧");
}
