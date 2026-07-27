//! `wx.request` 的语义测试（全部离线，不依赖外网）
//!
//! 两类断言：
//! - **失败路径**：不支持的协议由原生侧立即产出失败结果，宿主 `update()` 取回后走 `fail`
//! - **成功路径**：直接调 `__resolveRequest` 模拟原生回调，验证 4xx 仍走 success、
//!   `dataType` 的解析规则、以及 `abort` 语义
//!
//! 之所以能这么测：网络层被刻意切成「提交 → 后台跑 → 按帧取回」三段，
//! 中间那段是唯一需要外网的部分。

use crate::runtime::MiniApp;

fn new_app() -> MiniApp {
    let mut app = MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init MiniApp");
    app
}

/// 读一个 JS 全局变量的值（字符串形式）
fn get(app: &MiniApp, expr: &str) -> String {
    app.eval(expr).unwrap_or_default()
}

#[test]
fn unsupported_scheme_goes_to_fail() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __r = { ok: 0, fail: 0, msg: '', done: 0 };
        wx.request({
            url: 'ftp://example.com/x',
            success: function () { __r.ok++; },
            fail: function (e) { __r.fail++; __r.msg = e.errMsg; },
            complete: function () { __r.done++; }
        });
        "#,
    )
    .unwrap();

    // 失败结果是同步入队的，跑一次 update 就该被取回并回调
    app.update().unwrap();

    assert_eq!(get(&app, "String(__r.fail)"), "1", "不支持的协议应走 fail");
    assert_eq!(get(&app, "String(__r.ok)"), "0", "不该走 success");
    assert_eq!(get(&app, "String(__r.done)"), "1", "complete 必须调一次");
    assert!(
        get(&app, "__r.msg").contains("request:fail"),
        "errMsg 要带微信的 request:fail 前缀，实际: {}",
        get(&app, "__r.msg")
    );
}

/// 4xx / 5xx 在微信里**仍然走 success**，只是 statusCode 不是 2xx。
/// 这一条最容易实现错（很多人会把它当失败），所以单独钉住。
#[test]
fn http_error_status_still_calls_success() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __s = { ok: 0, fail: 0, code: 0 };
        wx.request({
            url: 'https://example.com/none',
            success: function (r) { __s.ok++; __s.code = r.statusCode; },
            fail: function () { __s.fail++; }
        });
        // 模拟原生把一个 404 响应喂回来（请求 id 从 1 开始自增）
        __resolveRequest(1, 404, '{}', '{"e":"nope"}', null);
        "#,
    )
    .unwrap();

    assert_eq!(get(&app, "String(__s.ok)"), "1", "4xx 也要走 success");
    assert_eq!(get(&app, "String(__s.fail)"), "0");
    assert_eq!(get(&app, "String(__s.code)"), "404", "statusCode 要如实透出");
}

#[test]
fn json_body_is_parsed_by_default_and_kept_raw_when_invalid() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __a = null, __b = null;
        wx.request({ url: 'https://example.com/a', success: function (r) { __a = r.data; } });
        __resolveRequest(1, 200, '{}', '{"n":7,"s":"x"}', null);
        // dataType 缺省是 'json'，但正文不是合法 JSON 时要原样给字符串
        wx.request({ url: 'https://example.com/b', success: function (r) { __b = r.data; } });
        __resolveRequest(2, 200, '{}', 'not json at all', null);
        "#,
    )
    .unwrap();

    assert_eq!(get(&app, "typeof __a"), "object", "JSON 正文要解析成对象");
    assert_eq!(get(&app, "String(__a.n)"), "7");
    assert_eq!(get(&app, "typeof __b"), "string", "非 JSON 正文要保持字符串");
    assert_eq!(get(&app, "__b"), "not json at all");
}

#[test]
fn response_headers_are_exposed() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __ct = '';
        wx.request({ url: 'https://example.com/h', success: function (r) { __ct = r.header['content-type'] || ''; } });
        __resolveRequest(1, 200, '{"content-type":"application/json"}', '{}', null);
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "__ct"), "application/json");
}

#[test]
fn abort_suppresses_success_but_still_completes() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __t = { ok: 0, done: 0, msg: '' };
        var task = wx.request({
            url: 'https://example.com/slow',
            success: function () { __t.ok++; },
            complete: function (r) { __t.done++; __t.msg = r.errMsg; }
        });
        task.abort();
        __resolveRequest(1, 200, '{}', '{}', null);
        "#,
    )
    .unwrap();
    assert_eq!(get(&app, "String(__t.ok)"), "0", "abort 之后不该再回 success");
    assert_eq!(get(&app, "String(__t.done)"), "1", "complete 仍要调用");
    assert!(get(&app, "__t.msg").contains("abort"));
}

/// GET 的 `data` 要拼进 query string（微信语义），而不是塞进请求体
#[test]
fn get_data_is_encoded_into_query() {
    let mut app = new_app();
    app.load_script(
        r#"
        var __url = '';
        // 用探针截获原生调用，避免真的发请求
        var __origNative = __native_request;
        __native_request = function (id, method, url) { __url = url; };
        wx.request({ url: 'https://example.com/q?keep=1', data: { a: '中文', b: 2 } });
        __native_request = __origNative;
        "#,
    )
    .unwrap();
    let url = get(&app, "__url");
    assert!(url.starts_with("https://example.com/q?keep=1&"), "已有 query 要用 & 续接: {}", url);
    assert!(url.contains("b=2"), "普通参数要带上: {}", url);
    assert!(url.contains("a=%E4%B8%AD%E6%96%87"), "中文要 URL 编码: {}", url);
}
