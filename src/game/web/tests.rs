//! 高性能模式原生侧的单测（页面里的行为由 examples/web_game_check.rs 在 Chrome 里跑同一批夹具守着）
use super::*;

fn pkg(name: &str, files: &[(&str, &str)]) -> (std::path::PathBuf, std::path::PathBuf) {
    let base = std::env::temp_dir().join(format!("wg-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let dir = base.join("pkg");
    for (p, s) in files {
        let f = dir.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, s).unwrap();
    }
    (dir, base.join("data"))
}

fn game(name: &str, files: &[(&str, &str)]) -> (WebGame, std::path::PathBuf) {
    let (dir, data) = pkg(name, files);
    let opts = GameOptions { app_id: Some(format!("wx{name}")), ..Default::default() };
    // 不改数据目录（进程级，别的单测在用）：只靠 appId 命名空间隔离（SCOPE_TEST_LOCK 守着）
    let _ = data;
    let g = WebGame::new(dir.to_str().unwrap(), None, 375, 667, 2.0, &opts).unwrap();
    (g, dir.parent().unwrap().to_path_buf())
}

#[test]
fn 消息体解析_参数一律字符串() {
    let (n, a) = natives::parse_message(r#"{"n":"fs","a":["stat",{"path":"x"},3,null]}"#).unwrap();
    assert_eq!(n, "fs");
    assert_eq!(a, vec!["stat".to_string(), r#"{"path":"x"}"#.to_string(), "3".into(), "".into()]);
    assert!(natives::parse_message("not json").is_none());
}

#[test]
fn 资源_包内文件_不许跳出包根_模块包装同一行() {
    let _scope = crate::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let (g, base) = game("res", &[("game.json", "{}"), ("game.js", "var a = 1;"), ("js/b.js", "\"use strict\"; module.exports = 2; // 尾注释"), ("img/a.png", "PNG")]);
    let r = g.resource("/img/a.png").unwrap();
    assert_eq!((r.bytes.as_slice(), r.mime), (b"PNG".as_slice(), "image/png"));
    assert!(g.resource("/../secret").is_none(), "不许跳出包根");
    assert!(g.resource("/img/%2e%2e/%2e%2e/x").is_none(), "编码过的 .. 也不行");
    assert!(g.resource("/__wg/m/../game.js").is_none());
    let m = String::from_utf8(g.resource("/__wg/m/js/b.js").unwrap().bytes).unwrap();
    // 源码第一行与包装头同一行（行号对得上），"use strict" 仍是内层函数的指令序言
    assert!(m.starts_with("__defineModule(\"js/b\", function(module, exports, require) { return (function(GameGlobal, window, self, globalThis) {\"use strict\";"), "{m}");
    assert_eq!(m.lines().count(), 3, "尾注释不能吃掉右括号：{m}");
    assert_eq!(g.module_keys(), vec!["game".to_string(), "js/b".into()]);
    let html = String::from_utf8(g.resource("/__wg.html").unwrap().bytes).unwrap();
    for s in boot::SEGMENTS {
        assert!(html.contains(&format!("/__wg/seg/{s}.js")), "启动页缺 {s}");
        assert!(g.resource(&format!("/__wg/seg/{s}.js")).is_some(), "取不到段 {s}");
    }
    assert!(g.resource("/__wg/seg/not-a-segment.js").is_none());
    let boot = String::from_utf8(g.resource("/__wg/boot.js").unwrap().bytes).unwrap();
    assert!(boot.starts_with("var __wgBoot = {") && boot.contains("\"modules\":[\"game\",\"js/b\"]"));
    std::fs::remove_dir_all(base).ok();
}

#[test]
fn storage_与_quickjs_模式同一个文件_轮询时落盘() {
    let _scope = crate::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let (mut g, base) = game("st", &[("game.json", "{}"), ("game.js", "")]);
    g.post("storage_clear", &[]);
    g.post("storage_set", &["k".into(), "{\"a\":1}".into()]);
    g.post("storage_set", &["gone".into(), "1".into()]);
    g.post("storage_remove", &["gone".into()]);
    let _ = g.take_js();
    assert_eq!(crate::storage_file::load().get("k").map(String::as_str), Some("{\"a\":1}"));
    assert!(crate::storage_file::load().get("gone").is_none());
    // 下一次启动：启动数据里带着整张表（页面读不过桥）
    drop(g);
    let opts = GameOptions { app_id: Some("wxst".into()), ..Default::default() };
    let dir = base.join("pkg");
    let g2 = WebGame::new(dir.to_str().unwrap(), None, 375, 667, 2.0, &opts).unwrap();
    assert!(g2.boot_js().contains(r#""storage":{"k":"{\"a\":1}"}"#), "{}", &g2.boot_js()[..300]);
    std::fs::remove_dir_all(base).ok();
}

#[test]
fn 尺寸与方向_变了才派发_同_quickjs_模式() {
    let _scope = crate::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let (mut g, base) = game("rz", &[("game.json", "{}"), ("game.js", "")]);
    g.resize(375, 667, 2.0, None);
    assert!(g.take_js().is_none(), "没变不派发");
    g.resize(667, 375, 2.0, Some("landscapeReverse"));
    let js = g.take_js().unwrap();
    assert!(js.contains(r#"__gameWindowResize({"windowHeight":375,"windowWidth":667})"#), "{js}");
    assert!(js.contains(r#"__gameOrientationChange({"value":"landscapeReverse"})"#), "{js}");
    assert_eq!(g.system_info_json()["windowWidth"], 667.0);
    std::fs::remove_dir_all(base).ok();
}

#[test]
fn 宿主事件_hostcall_补_tempdir_宿主回包导入文件() {
    let _scope = crate::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let (mut g, base) = game("hc", &[("game.json", "{}"), ("game.js", "")]);
    g.post("host_event", &[r#"{"type":"hostCall","id":3,"api":"login","args":{}}"#.into()]);
    g.post("host_event", &["not json".into()]);
    let ev: Value = serde_json::from_str(&g.take_event().unwrap()).unwrap();
    assert_eq!((ev["type"].as_str(), ev["id"].as_u64()), (Some("hostCall"), Some(3)));
    assert!(ev["tempDir"].is_string(), "{ev}");
    assert!(g.take_event().is_none(), "不是 JSON 的丢掉");
    let f = base.join("pick.png");
    std::fs::write(&f, b"x").unwrap();
    g.host_reply(3, true, &format!(r#"{{"tempFilePath":"file://{}"}}"#, f.display()));
    let js = g.take_js().unwrap();
    assert!(js.starts_with("__resolveHostCall(3, true, ") && js.contains("wxfile://tmp"), "回包里的 file:// 导入沙盒：{js}");
    std::fs::remove_dir_all(base).ok();
}

#[test]
fn 文件系统与临时文件走同一份_rust_实现() {
    let _scope = crate::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _root = crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let (mut g, base) = game("fs", &[("game.json", "{}"), ("game.js", "")]);
    let w = g.call("fs", &["writeFile".into(), r#"{"filePath":"wxfile://usr/a.txt","data":"hi","encoding":"utf8"}"#.into()]);
    assert!(w.contains("\"ok\":true"), "{w}");
    let r = g.call("fs", &["readFile".into(), r#"{"filePath":"wxfile://usr/a.txt","encoding":"utf8"}"#.into()]);
    assert!(r.contains("hi"), "{r}");
    // /__wg/file/ 取得到 wxfile 的内容（图片 src 给 wxfile:// 时页面走这条）
    assert_eq!(g.resource("/__wg/file/usr/a.txt").unwrap().bytes, b"hi");
    let png = crate::wxfs::codec::b64(&[0x89, b'P', b'N', b'G', 13, 10, 26, 10]);
    let uri = g.call("temp_write", &[png]);
    assert!(uri.starts_with("wxfile://tmp") && uri.ends_with(".png"), "{uri}");
    assert_eq!(g.call("file_exists", &[uri.clone()]), "true");
    g.post("temp_remove", &[uri.clone()]);
    assert_eq!(g.call("file_exists", &[uri]), "false");
    std::fs::remove_dir_all(base).ok();
}

#[test]
fn 百分号解码() {
    assert_eq!(resource::decode_for_test("/a%20b/%E5%9B%BE.png"), "/a b/图.png");
    assert_eq!(resource::decode_for_test("/x%"), "/x%");
    assert_eq!(resource::decode_for_test("/x%4"), "/x%4");
}
