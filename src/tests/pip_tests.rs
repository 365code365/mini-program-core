//! 小窗（`picture-in-picture-mode`，同层渲染的 live-pusher；components/pip.rs + host/engine_pip.rs）：
//! push 进小窗派 enter、宿主照报它（`pip: true`，box 是小窗尺寸）、帧画在右下角、新帧重新合成；
//! 点小窗回到原页、收起、派 leave、照常报回页面里；pop 进小窗、点 × 关掉（不再报给宿主）。
use crate::compositor::HostEnv;
use crate::host::{EngineOptions, MiniEngine};
use crate::renderer::components::host_view;
use serde_json::Value;

const W: u32 = 375;
const H: u32 = 667;
const DPR: f32 = 2.0;

fn fixture(name: &str, mode: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join("mini-pip-tests").join(name);
    std::fs::remove_dir_all(&root).ok();
    let pusher = format!(
        r#"<live-pusher id="lp" class="lp" url="rtmp://x/y" picture-in-picture-mode="{mode}"
  bindenterpictureinpicture="onEnter" bindleavepictureinpicture="onLeave"></live-pusher>"#
    );
    let js = "Page({ onEnter() { getApp().log.push('enter'); }, onLeave() { getApp().log.push('leave'); } });";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/a/a","pages/b/b"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({ log: [] });"),
        ("app.wxss", ""),
        ("pages/a/a.wxml", pusher.as_str()),
        ("pages/a/a.wxss", "page { background: #ffffff; } .lp { width: 300px; height: 400px; }"),
        ("pages/a/a.js", js),
        ("pages/a/a.json", "{}"),
        ("pages/b/b.wxml", "<view class=\"b\">第二页</view>"),
        ("pages/b/b.wxss", "page { background: #00ff00; } .b { height: 100px; }"),
        ("pages/b/b.js", js),
        ("pages/b/b.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

struct T {
    e: MiniEngine,
    t: u64,
    _g: std::sync::RwLockWriteGuard<'static, ()>,
}

impl T {
    fn new(root: &std::path::Path, route: &str) -> Self {
        let g = crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
        let opts = EngineOptions { same_layer: vec!["live-pusher".into()], ..Default::default() };
        let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&root.join("_data").to_string_lossy()), W, H, DPR, HostEnv::default(), opts)
            .expect("创建引擎");
        e.set_animation_clock(0.0);
        e.freeze_clock_for_test();
        e.launch(Some(route)).expect("打开页面");
        let mut t = T { e, t: 0, _g: g };
        t.pump(3);
        t
    }
    fn pump(&mut self, n: u32) {
        for _ in 0..n {
            self.t += 16;
            self.e.pump(self.t);
        }
    }
    /// 这几帧里最后一次报的同层组件列表
    fn views(&mut self) -> Option<Vec<Value>> {
        let mut last = None;
        while let Some(s) = self.e.poll_event() {
            let v: Value = serde_json::from_str(&s).unwrap();
            if v["type"] == "sameLayerViews" {
                last = v["views"].as_array().cloned();
            }
        }
        last
    }
    fn log(&self) -> Vec<String> {
        serde_json::from_str(&self.e.eval("JSON.stringify(getApp().log)").unwrap_or_default()).unwrap_or_default()
    }
    fn px(&mut self, x: f32, y: f32) -> [u8; 3] {
        let (w, _) = self.e.pixel_size();
        let p = self.e.rgba();
        let i = ((y * DPR) as usize * w as usize + (x * DPR) as usize) * 4;
        [p[i], p[i + 1], p[i + 2]]
    }
    fn tap(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.pump(1);
        self.e.pointer_up(x, y);
        self.pump(3);
    }
}

fn solid(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    (0..w * h).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect()
}

#[test]
fn push_进小窗_宿主照报_帧画在右下角_点小窗回到原页收起() {
    let root = fixture("push", "push");
    let mut t = T::new(&root, "pages/a/a");
    let v = t.views().expect("报了同层组件");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0]["key"], "lp");
    assert!(v[0].get("pip").is_none());
    t.e.eval("wx.navigateTo({ url: '/pages/b/b' })").unwrap();
    t.pump(30);
    assert_eq!(t.e.current_route(), "pages/b/b");
    assert_eq!(t.log(), vec!["enter"], "进小窗之前派 enter");
    assert_eq!(t.e.eval("String(wx.checkIsPictureInPictureActive())").unwrap(), "true");
    let v = t.views().expect("换页之后重新报");
    assert_eq!(v.len(), 1, "原页的 live-pusher 照报：{v:?}");
    assert_eq!(v[0]["pip"], true);
    // 300×400 的框 → 小窗 120×160，右下角（离右 12、离底 12）
    assert_eq!((v[0]["width"].as_f64(), v[0]["height"].as_f64()), (Some(120.0), Some(160.0)));
    assert_eq!(v[0]["box"], serde_json::json!([240.0, 320.0]));
    let (cx, cy) = (375.0 - 12.0 - 60.0, 667.0 - 12.0 - 80.0);
    assert_eq!(t.px(cx, cy), [0, 0, 0], "还没来帧：黑底");
    assert_eq!(t.px(20.0, 200.0), [0, 255, 0], "小窗外是第二页");
    assert!(host_view::push_rgba("lp", 240, 320, 240 * 4, &solid(240, 320, [255, 0, 0])));
    t.pump(2);
    assert_eq!(t.px(cx, cy), [255, 0, 0], "宿主的帧画在小窗里");
    assert!(host_view::push_rgba("lp", 240, 320, 240 * 4, &solid(240, 320, [0, 0, 255])));
    t.pump(2);
    assert_eq!(t.px(cx, cy), [0, 0, 255], "新帧重新合成");
    // 点小窗：回到原页、收起、派 leave、照常报回页面里
    t.tap(cx, cy);
    t.pump(30);
    assert_eq!(t.e.current_route(), "pages/a/a");
    assert!(!crate::renderer::components::pip::visible(), "回到原页收起");
    assert_eq!(t.log(), vec!["enter", "leave"]);
    assert_eq!(t.e.eval("String(wx.checkIsPictureInPictureActive())").unwrap(), "false");
    let v = t.views().expect("回来之后重新报");
    assert_eq!(v.len(), 1);
    assert!(v[0].get("pip").is_none(), "报回页面里：{v:?}");
    assert_eq!(v[0]["box"], serde_json::json!([600.0, 800.0]));
    // 再进一次小窗，在第二页上调 LivePusherContext.exitPictureInPicture：关掉
    t.e.eval("wx.navigateTo({ url: '/pages/b/b' })").unwrap();
    t.pump(30);
    assert!(crate::renderer::components::pip::visible());
    t.e.eval("wx.createLivePusherContext().exitPictureInPicture({ success() { getApp().log.push('exit-ok'); } })").unwrap();
    t.pump(3);
    assert!(!crate::renderer::components::pip::visible(), "exitPictureInPicture 关掉小窗");
    assert_eq!(t.log().last().map(String::as_str), Some("exit-ok"));
    let v = t.views().expect("关掉之后重新报");
    assert!(v.is_empty(), "{v:?}");
}

#[test]
fn pop_进小窗_点关闭不再报给宿主() {
    let root = fixture("pop", "pop");
    let mut t = T::new(&root, "pages/b/b");
    t.e.eval("wx.navigateTo({ url: '/pages/a/a' })").unwrap();
    t.pump(30);
    t.views();
    // push 方向没写：再往下开一页不进小窗
    assert!(!crate::renderer::components::pip::visible());
    t.e.back();
    t.pump(30);
    assert_eq!(t.e.current_route(), "pages/b/b");
    assert!(crate::renderer::components::pip::visible(), "关掉那页之前进了小窗");
    let v = t.views().expect("退栈之后重新报");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0]["pip"], true);
    // 点画面：原页已经关了，不跳
    let (cx, cy) = (375.0 - 12.0 - 60.0, 667.0 - 12.0 - 80.0);
    t.tap(cx, cy);
    t.pump(10);
    assert_eq!(t.e.current_route(), "pages/b/b");
    assert!(crate::renderer::components::pip::visible());
    // 点 ×：关掉
    t.tap(375.0 - 12.0 - 14.0, 667.0 - 12.0 - 160.0 + 14.0);
    assert!(!crate::renderer::components::pip::visible());
    let v = t.views().expect("关掉之后重新报");
    assert!(v.is_empty(), "不再报给宿主（宿主放掉控件）：{v:?}");
    assert_eq!(t.px(cx, cy), [0, 255, 0], "小窗没了");
}
