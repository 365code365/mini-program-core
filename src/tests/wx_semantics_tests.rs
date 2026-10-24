//! 对照微信语义核过一遍之后修掉的那批（2026-10-24 审计）：每条一个用例，改回去就失败。
//!
//! - 异步 API 的回调不在调用栈里同步发生（showToast / setNavigationBarTitle / pageScrollTo / 下拉刷新）；
//! - `setTimeout(cb, delay, ...rest)` 把 rest 交给回调；
//! - storage 读回来类型不变（'42' 还是字符串）；
//! - `RequestTask.abort()` 走 fail；下载任务 abort 也回调；x-www-form-urlencoded 的对象按表单编码。
use crate::runtime::MiniApp;

fn new_app() -> MiniApp {
    let mut app = MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init MiniApp");
    app
}

fn get(app: &MiniApp, expr: &str) -> String {
    app.eval(expr).unwrap_or_default()
}

/// 零延时定时器跑完：第一轮 update 登记，第二轮到点
fn flush(app: &mut MiniApp) {
    app.update().unwrap();
    app.update().unwrap();
}

#[test]
fn 异步_api_的回调在下一轮事件循环() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __log = [];
        wx.setNavigationBarTitle({ title: 't', success: function () { __log.push('nav'); } });
        wx.pageScrollTo({ scrollTop: 10, success: function () { __log.push('scroll'); } });
        wx.stopPullDownRefresh({ success: function (r) { __log.push(r.errMsg); } });
        __log.push('sync');
        var __promise = typeof wx.startPullDownRefresh().then;
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "__log.join(',')"), "sync", "回调不能同步发生");
    assert_eq!(get(&app, "__promise"), "function", "下拉刷新不给回调时返回 Promise");
    flush(&mut app);
    assert_eq!(get(&app, "__log.join(',')"), "sync,nav,scroll,stopPullDownRefresh:ok");
}

#[test]
fn set_timeout_把多余参数交给回调() {
    let mut app = new_app();
    app.load_script("var __a = ''; setTimeout(function (x, y) { __a = x + '|' + y; }, 0, 'p', 7);").unwrap();
    flush(&mut app);
    assert_eq!(get(&app, "__a"), "p|7");
}

#[test]
fn storage_读回来类型不变() {
    let app = new_app();
    app.load_script(
        r#"
        wx.setStorageSync('__t_num_str', '42');
        wx.setStorageSync('__t_bool_str', 'true');
        wx.setStorageSync('__t_json_str', '{"a":1}');
        wx.setStorageSync('__t_num', 42);
        wx.setStorageSync('__t_empty', '');
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "typeof wx.getStorageSync('__t_num_str') + ':' + wx.getStorageSync('__t_num_str')"), "string:42");
    assert_eq!(get(&app, "typeof wx.getStorageSync('__t_bool_str')"), "string");
    assert_eq!(get(&app, "typeof wx.getStorageSync('__t_json_str')"), "string");
    assert_eq!(get(&app, "typeof wx.getStorageSync('__t_num')"), "number");
    assert_eq!(get(&app, "JSON.stringify(wx.getStorageSync('__t_empty'))"), "\"\"");
    // 旧版本存下的裸字符串（没加 JSON 引号）照旧按原文读回
    app.eval("__storage['__t_legacy'] = 'hello';").unwrap();
    assert_eq!(get(&app, "wx.getStorageSync('__t_legacy')"), "hello");
    app.eval("['__t_num_str','__t_bool_str','__t_json_str','__t_num','__t_empty','__t_legacy'].forEach(function(k){ wx.removeStorageSync(k); })").unwrap();
}

#[test]
fn request_abort_走_fail_再_complete() {
    let app = new_app();
    app.load_script(
        r#"
        var __r = [];
        __native_request = function () {};   // 不真的发
        __native_download = function () {};
        var t = wx.request({ url: 'http://example.invalid/', fail: function (r) { __r.push('fail:' + r.errMsg); }, complete: function (r) { __r.push('complete'); } });
        var id1 = __requestSeq;
        t.abort();
        __resolveRequest(id1, 200, '{}', '', '');
        var d = wx.downloadFile({ url: 'http://example.invalid/f', fail: function (r) { __r.push('dfail:' + r.errMsg); }, complete: function () { __r.push('dcomplete'); } });
        var id2 = __requestSeq;
        d.abort();
        __resolveRequest(id2, 200, '{}', '', '');
        "#,
    )
    .unwrap();
    assert_eq!(
        get(&app, "__r.join(',')"),
        "fail:request:fail abort,complete,dfail:downloadFile:fail abort,dcomplete"
    );
}

#[test]
fn request_表单编码与_get_字符串参数() {
    let app = new_app();
    app.load_script(
        r#"
        var __sent = [];
        __native_request = function (id, method, url, header, body) { __sent.push(method + ' ' + url + ' ' + body); };
        wx.request({ url: 'http://h/a', method: 'POST', header: { 'Content-Type': 'application/x-www-form-urlencoded' }, data: { a: 1, b: 'x y' } });
        wx.request({ url: 'http://h/b', method: 'POST', data: { a: 1 } });
        wx.request({ url: 'http://h/c', data: 'k=v' });
        "#,
    )
    .unwrap();
    assert_eq!(
        get(&app, "__sent.join('\\n')"),
        "POST http://h/a a=1&b=x%20y\nPOST http://h/b {\"a\":1}\nGET http://h/c?k=v "
    );
}

#[test]
fn 不带值的属性是_true_绑定求空是假() {
    use crate::parser::wxml::WxmlParser;
    use crate::renderer::components::attr_is_true;
    let nodes = WxmlParser::new(r#"<button disabled plain="false"></button>"#).parse().unwrap();
    let b = &nodes[0];
    assert_eq!(b.get_attr("disabled"), Some("true"));
    assert!(attr_is_true(b.get_attr("disabled").unwrap()));
    assert!(!attr_is_true(b.get_attr("plain").unwrap()));
    // `checked="{{x}}"` 在 x 为 undefined 时求出空串：按布尔属性是假
    assert!(!attr_is_true(""));
    assert!(!attr_is_true("0"));
    assert!(attr_is_true("checked"));
}
