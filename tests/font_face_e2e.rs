//! `wx.loadFontFace`（官方 api/ui/font/wx.loadFontFace.html）与登记之后的重排（G-3）。
//!
//! 走 `MiniEngine`：加载包内的 TTF 之后，`font-family` 写它的文字换成那个字体，页面**自己**重排
//! （数据没变，靠「字体变了」让布局缓存作废）；WOFF 明确 fail，不假装成功。
//! 独立集成测试二进制的原因同 `component_e2e.rs`：小程序根目录是进程级的。
use mini_render::host::MiniEngine;
use serde_json::Value;
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn load_font_face_registers_relayouts_and_rejects_woff() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("font-face-e2e");
    std::fs::remove_dir_all(&root).ok();
    let fx = Fixture(root.clone());
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", r#"<view style="display:flex;flex-direction:row"><text id="t" class="t">iiiiiiii</text></view>"#),
        ("pages/index/index.wxss", ".t{font-family:'Loaded Mono';font-size:20px}"),
        (
            "pages/index/index.js",
            r#"Page({ data: { log: [], w: [] },
  measure() { this.createSelectorQuery().select('#t').boundingClientRect(r => this.data.w.push(r.width)).exec(); } });"#,
        ),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    std::fs::create_dir_all(root.join("fonts")).unwrap();
    std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/Monaco.ttf"), root.join("fonts/mono.ttf")).unwrap();
    std::fs::write(root.join("fonts/x.woff"), b"wOFF\0\0\0\0").unwrap();

    let data = root.join("_data");
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
    e.launch(Some("pages/index/index")).expect("打开页面");
    let mut t = 1000u64;
    let mut frames = |e: &mut MiniEngine, n: u32| {
        for _ in 0..n {
            t += 16;
            e.pump(t);
        }
    };
    frames(&mut e, 3);
    let js = |e: &MiniEngine, code: &str| -> Value {
        let s = e.eval(&format!("JSON.stringify({code})")).unwrap_or_default();
        let s: String = serde_json::from_str(&s).unwrap_or(s);
        serde_json::from_str(&s).unwrap_or(Value::Null)
    };

    e.eval("__currentPage.measure()").unwrap();
    frames(&mut e, 3);
    e.eval(
        "wx.loadFontFace({ family: 'Loaded Mono', source: 'url(\"/fonts/mono.ttf\")', \
           success: r => __currentPage.data.log.push(r.status), fail: r => __currentPage.data.log.push(r.errMsg) }); \
         wx.loadFontFace({ family: 'Bad', source: '/fonts/x.woff', fail: r => __currentPage.data.log.push(r.status + ' ' + r.errMsg) })",
    )
    .unwrap();
    frames(&mut e, 4);
    e.eval("__currentPage.measure()").unwrap();
    frames(&mut e, 3);

    let log = js(&e, "__currentPage.data.log");
    assert_eq!(log[0], "loaded", "{log}");
    assert!(log[1].as_str().unwrap_or("").starts_with("error loadFontFace:fail unsupported font format: woff"), "{log}");
    let w = js(&e, "__currentPage.data.w");
    let (before, after) = (w[0].as_f64().unwrap_or(0.0), w[1].as_f64().unwrap_or(0.0));
    assert!(after > before * 1.3, "登记等宽字体后页面自己重排，一串 i 变宽：{before} → {after}");
    drop(fx);
}
