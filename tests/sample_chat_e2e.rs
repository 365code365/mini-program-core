//! 轻聊（sample/chat）：每一页能打开、不报错；会话列表点进聊天、发消息、对方回复、返回后会话列表的最后一条跟着变。
//! 键盘与输入条的细节见 tests/chat_keyboard_e2e.rs。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;

#[test]
fn 轻聊_每页都能打开() {
    let _g = serial();
    smoke_all_pages("chat", &[("pages/chat/chat", "id=c19"), ("pages/contact/contact", "id=c19")]);
}

#[test]
fn 轻聊_发消息_对方回复_会话列表跟着变() {
    let _g = serial();
    let mut a = App::open("chat", Some("pages/chat/chat?id=c19"), "send");
    let n0 = a.data("rows").as_array().map(|r| r.len()).unwrap_or(0);
    a.type_into(".input", "在吗");
    assert_eq!(a.data("canSend"), true, "有字之后发送键可点");
    a.tap(".send");
    a.frames(3);
    assert_eq!(a.data("draft"), "", "发送后输入框清空");
    a.wait_ms(2200);
    let rows = a.data("rows");
    let rows = rows.as_array().unwrap();
    assert!(rows.len() >= n0 + 2, "发出去一条、回来一条：{} → {}", n0, rows.len());
    assert_eq!(rows.last().unwrap()["text"], "在的，怎么了？", "「在吗」的回复");
    a.assert_no_errors();
}

/// 驱动自检：逻辑层的异常真的被记下（不然「不报错」的判据是空转）
#[test]
fn 驱动自检_能抓到逻辑层报错() {
    let _g = serial();
    let mut a = App::open("chat", None, "hook");
    assert!(a.errors().is_empty());
    a.js("setTimeout(function () { null.x; }, 0)");
    a.wait_ms(100);
    let errs = a.errors();
    assert!(!errs.is_empty(), "定时器里抛的错没抓到");
    a.js("globalThis.__errs = []; getCurrentPages()[0].__boom = function () { undefinedFn(); }");
    a.js("__dispatchEvent('__boom', {})");
    assert!(!a.errors().is_empty(), "事件处理函数里抛的错没抓到");
}
