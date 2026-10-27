//! `mr_game_*` 的契约（game-doc/08 §5）：空指针、包种类、panic 防护、事件与 eval 的指针生命周期

use super::*;
use std::ffi::{CStr, CString};

fn game_dir(name: &str, js: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mr-ffi-game-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("game.json"), "{}").unwrap();
    std::fs::write(d.join("game.js"), js).unwrap();
    d
}

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// 建游戏实例会改进程级的数据命名空间：与 storage_file 的用例互斥
fn scope_lock() -> (std::sync::MutexGuard<'static, ()>, std::sync::RwLockWriteGuard<'static, ()>) {
    // 建游戏实例也会换掉进程级的包根：拿写锁，起 MiniEngine 的用例这时等着（data_dir::ROOT_TEST_LOCK）
    (crate::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner()),
     crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner()))
}

fn create(dir: &std::path::Path, opts: &str) -> *mut MRGame {
    let d = c(dir.to_str().unwrap());
    let data = c(dir.with_extension("data").to_str().unwrap());
    mr_game_create(d.as_ptr(), data.as_ptr(), 375, 667, 2.0, c(opts).as_ptr())
}

fn drain(g: *mut MRGame) -> Vec<String> {
    let mut v = Vec::new();
    loop {
        let p = mr_game_poll_event(g);
        if p.is_null() {
            return v;
        }
        v.push(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned());
    }
}

#[test]
fn 空指针都不崩() {
    let n = std::ptr::null_mut();
    assert_eq!(mr_game_pump_ex(n, 16), 0);
    assert!(mr_game_poll_event(n).is_null());
    assert!(mr_game_eval(n, c("1").as_ptr()).is_null());
    assert!(mr_game_last_error(n).is_null());
    assert_eq!(mr_game_is_poisoned(n), 1);
    mr_game_destroy(n);
    assert_eq!(mr_package_kind(std::ptr::null()), 0);
    assert!(mr_game_create(std::ptr::null(), std::ptr::null(), 1, 1, 1.0, std::ptr::null()).is_null());
}

#[test]
fn 包种类与能力() {
    let _scope = scope_lock();
    let g = game_dir("kind", "");
    assert_eq!(mr_package_kind(c(g.to_str().unwrap()).as_ptr()), 2);
    let caps: serde_json::Value = serde_json::from_str(unsafe { CStr::from_ptr(mr_capabilities()) }.to_str().unwrap()).unwrap();
    assert_eq!(caps["game"], true);
    assert_eq!(caps["sdkVersion"], crate::game::GAME_SDK_VERSION);
    assert_eq!(mr_capabilities(), mr_capabilities(), "静态字符串，指针不变");
    // 小程序目录不能按小游戏建
    let mini = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/sample-app");
    if mini.join("app.json").is_file() {
        assert_eq!(mr_package_kind(c(mini.to_str().unwrap()).as_ptr()), 1);
        assert!(create(&mini, "").is_null());
    }
    let _ = std::fs::remove_dir_all(g);
}

#[test]
fn 日志与_eval() {
    let _scope = scope_lock();
    let d = game_dir("eval", "console.log('hi-ffi');");
    let g = create(&d, r#"{"appId":"wxffi"}"#);
    assert!(!g.is_null());
    assert!(drain(g).iter().any(|e| e.contains("hi-ffi")));
    let r = mr_game_eval(g, c("6 * 7").as_ptr());
    assert_eq!(unsafe { CStr::from_ptr(r) }.to_str().unwrap(), "42");
    assert!(mr_game_eval(g, c("throw new Error('x')").as_ptr()).is_null());
    assert!(unsafe { CStr::from_ptr(mr_game_last_error(g)) }.to_str().unwrap().contains('x'));
    assert_eq!(mr_game_is_poisoned(g), 0);
    mr_game_destroy(g);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn panic_不穿过_ffi_实例失效() {
    let _scope = scope_lock();
    for at in ["panic@pump", "panic@eval"] {
        let d = game_dir(&at.replace('@', "-"), "");
        let g = create(&d, &format!(r#"{{"fault":"{at}"}}"#));
        assert!(!g.is_null());
        drain(g);
        // 触发
        if at == "panic@pump" {
            let f = mr_game_pump_ex(g, 16);
            assert_ne!(f & crate::host::engine_backend::FRAME_EVENTS as i32, 0, "要带 EVENTS 位让宿主取 panic 事件");
        } else {
            assert!(mr_game_eval(g, c("1").as_ptr()).is_null());
        }
        assert_eq!(mr_game_is_poisoned(g), 1);
        let ev = drain(g);
        assert_eq!(ev.len(), 1, "{ev:?}");
        assert!(ev[0].contains("\"kind\":\"panic\"") && ev[0].contains(at), "{ev:?}");
        let err = unsafe { CStr::from_ptr(mr_game_last_error(g)) }.to_str().unwrap().to_string();
        assert!(err.contains("实例已失效"), "{err}");
        // 之后的调用都返回失败值，不再 panic
        assert_eq!(mr_game_pump_ex(g, 32), 0);
        assert!(mr_game_eval(g, c("1").as_ptr()).is_null());
        assert!(mr_game_poll_event(g).is_null());
        mr_game_destroy(g);
        let _ = std::fs::remove_dir_all(d);
    }
}
