//! C4 / C5：button 的 open-type 与 navigator 的 target="miniProgram"
use super::*;

const LOG_FN: &str = "function L(s){ getApp().log.push(s); }";

/// 点 (x, y) 一下
fn tap(e: &mut MiniEngine, x: f32, y: f32, t: u64) {
    e.pointer_down(x, y);
    e.pump(t);
    e.pointer_up(x, y);
    e.pump(t + 16);
}

#[test]
fn button_open_type_宿主没接时按_fail_回_接了走_hostcall() {
    let wxml = r#"<button id="ph" open-type="getPhoneNumber" bindgetphonenumber="onPhone" bindtap="onTap" style="height:60px">手机号</button>
                  <button open-type="launchApp" app-parameter="abc" binderror="onErr" style="height:60px">打开 App</button>
                  <button open-type="chooseAvatar" bindchooseavatar="onAvatar" disabled style="height:60px">头像</button>"#;
    let p_js = format!(
        "{LOG_FN} Page({{ onTap(){{ L('tap'); }}, onPhone(e){{ L('phone ' + e.detail.errMsg + ' ' + e.target.id); }}, \
           onErr(e){{ L('err ' + e.detail.errMsg); }}, onAvatar(){{ L('avatar'); }} }});"
    );
    let root = fixture("open-type", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 200, |_| false);
    tap(&mut e, 100.0, 30.0, 2000);
    assert!(pump_until(&mut e, 1000, |e| log(e).iter().any(|s| s.starts_with("phone"))), "{:?}", log(&e));
    let l = log(&e);
    assert_eq!(l[0], "tap", "bindtap 先发：{l:?}");
    assert!(l[1].starts_with("phone getPhoneNumber:fail") && l[1].ends_with(" ph"), "回调派给按钮自己、errMsg 按官方前缀：{l:?}");
    // launchApp 失败走 binderror（官方：bind:error 是 open-type=launchApp 时的错误回调）
    clear_log(&e);
    tap(&mut e, 100.0, 90.0, 3000);
    assert!(pump_until(&mut e, 1000, |e| log(e).iter().any(|s| s.starts_with("err launchApp:fail"))), "{:?}", log(&e));
    // disabled 的按钮不做开放能力
    clear_log(&e);
    tap(&mut e, 100.0, 150.0, 4000);
    pump_until(&mut e, 200, |_| false);
    assert!(log(&e).is_empty(), "{:?}", log(&e));

    // 宿主接了：发 hostCall（api = button.getPhoneNumber），宿主回结果后回调拿到 code
    let opts = EngineOptions { host_apis: vec!["button.getPhoneNumber".into()], ..Default::default() };
    drop(e);
    let mut e = engine(&root, opts);
    pump_until(&mut e, 200, |_| false);
    host_events(&mut e);
    tap(&mut e, 100.0, 30.0, 2000);
    e.pump(2100);
    let call = host_events(&mut e).into_iter().find(|v| v["type"] == "hostCall").expect("hostCall");
    assert_eq!(call["api"], "button.getPhoneNumber");
    let id = call["id"].as_u64().unwrap();
    js(&e, &format!("__resolveHostCall({id}, true, JSON.stringify({{ code: 'C1' }}))"));
    assert!(pump_until(&mut e, 500, |e| log(e).iter().any(|s| s == "phone getPhoneNumber:ok ph")), "{:?}", log(&e));
}

#[test]
fn button_share_调_onshareappmessage_再发宿主事件() {
    let root = fixture(
        "open-type-share",
        r#"<button open-type="share" data-k="7" style="height:60px">转发</button>"#,
        &format!("{LOG_FN} Page({{ onShareAppMessage(o){{ L(o.from + ' ' + o.target.dataset.k); return {{ title: '标题' }}; }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 200, |_| false);
    host_events(&mut e);
    tap(&mut e, 100.0, 30.0, 2000);
    assert_eq!(log(&e), ["button 7"], "from 是 button、target 带 dataset（数字还原）");
    let share = host_events(&mut e).into_iter().find(|v| v["type"] == "share").expect("share 宿主事件");
    assert_eq!(share["title"], "标题");
    assert_eq!(share["path"], "/pages/p/p", "path 缺省是当前页");
}

#[test]
fn button_get_user_info_与隐私同意() {
    let root = fixture(
        "open-type-ui",
        r#"<button open-type="getUserInfo" bindgetuserinfo="u" style="height:60px">a</button>
           <button id="pv" open-type="agreePrivacyAuthorization" bindagreeprivacyauthorization="p" style="height:60px">b</button>"#,
        &format!("{LOG_FN} Page({{ u(e){{ L('ui ' + e.detail.errMsg); }}, p(e){{ L('pv ' + e.detail.buttonId); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 200, |_| false);
    tap(&mut e, 100.0, 30.0, 2000);
    tap(&mut e, 100.0, 90.0, 2100);
    assert!(pump_until(&mut e, 1000, |e| log(e).len() == 2), "{:?}", log(&e));
    let l = log(&e);
    assert!(l.iter().any(|s| s.starts_with("ui getUserInfo:fail")), "宿主没接 getUserInfo：fail 也回调：{l:?}");
    assert!(l.contains(&"pv pv".to_string()), "{l:?}");
}

#[test]
fn navigator_小程序跳转按_open_type_分流并回调() {
    let root = fixture(
        "nav-mp",
        r#"<navigator target="miniProgram" open-type="navigateBack" extra-data="{{ {a:1} }}" bindfail="f" bindcomplete="c" style="display:block;height:60px">返回</navigator>
           <navigator target="miniProgram" app-id="wx123" path="pages/x" bindfail="f2" style="display:block;height:60px">去</navigator>"#,
        &format!("{LOG_FN} Page({{ f(e){{ L('fail ' + e.detail.errMsg.split(' ')[0]); }}, c(){{ L('complete'); }}, f2(e){{ L('fail2 ' + e.detail.errMsg.split(' ')[0]); }} }});"),
        &[],
    );
    let opts = EngineOptions { host_apis: vec!["navigateToMiniProgram".into()], ..Default::default() };
    let mut e = engine(&root, opts);
    pump_until(&mut e, 200, |_| false);
    host_events(&mut e);
    tap(&mut e, 100.0, 30.0, 2000);
    assert!(pump_until(&mut e, 1000, |e| log(e).len() >= 2), "{:?}", log(&e));
    assert_eq!(log(&e), ["fail navigateBackMiniProgram:fail", "complete"], "navigateBack → navigateBackMiniProgram");
    // navigate → navigateToMiniProgram（宿主接了：hostCall 带 appId / path）
    tap(&mut e, 100.0, 90.0, 3000);
    e.pump(3100);
    let call = host_events(&mut e).into_iter().find(|v| v["type"] == "hostCall").expect("hostCall");
    assert_eq!(call["api"], "navigateToMiniProgram");
    assert_eq!(call["args"]["appId"], "wx123");
    let id = call["id"].as_u64().unwrap();
    js(&e, &format!("__resolveHostCall({id}, false, '{{}}')"));
    assert!(pump_until(&mut e, 500, |e| log(e).iter().any(|s| s == "fail2 navigateToMiniProgram:fail")), "{:?}", log(&e));
}
