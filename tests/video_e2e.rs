//! `<video>` 的组件事件与 `wx.createVideoContext`（G-6）。
//!
//! 依据官方 component/video.html 与 api/media/video/VideoContext.html：
//! `loadedmetadata {width, height, duration}`、`play`、`pause`、`timeupdate {currentTime, duration}`
//! （250ms 一次）、`ended`、`seekcomplete {position}`、`fullscreenchange {fullScreen, direction}`。
//!
//! （detail 由 serde_json 序列化，键按字母序；页面里看到的是同一个对象）
//!
//! 夹具是 `tests/assets/tiny_1s.mp4`（64×48、10fps、1 秒、无音轨，3KB）。播放器按墙上时钟走，
//! 所以这里的等待是真的 sleep。走 `MiniEngine`（SDK 链路），独立集成测试二进制的原因同
//! `component_e2e.rs`：小程序根目录是进程级的；播放器表按 src 进程级共享，每个用例用自己的文件名。
#![cfg(feature = "h264")]
use mini_render::host::MiniEngine;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static SERIAL: Mutex<()> = Mutex::new(());

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

const JS: &str = r#"
Page({
  data: { ev: [], times: [], taps: 0 },
  rec(name, e) {
    if (name === 'timeupdate') { this.data.times.push(e.detail.currentTime); return; }
    this.data.ev.push(name + ' ' + JSON.stringify(e.detail));
  },
  onMeta(e) { this.rec('loadedmetadata', e); },
  onPlay(e) { this.rec('play', e); },
  onPause(e) { this.rec('pause', e); },
  onEnded(e) { this.rec('ended', e); },
  onTime(e) { this.rec('timeupdate', e); },
  onSeek(e) { this.rec('seekcomplete', e); },
  onFs(e) { this.rec('fullscreenchange', e); },
  onToggle(e) { this.rec('controlstoggle', e); },
  onBtn() { this.data.taps++; },
});
"#;

fn fixture(name: &str) -> Fixture {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("video-e2e").join(name);
    std::fs::remove_dir_all(&root).ok();
    let wxml = format!(
        r#"<video id="v" src="/assets/{name}.mp4" style="width:300px;height:200px"
  bindloadedmetadata="onMeta" bindplay="onPlay" bindpause="onPause" bindended="onEnded"
  bindtimeupdate="onTime" bindseekcomplete="onSeek" bindfullscreenchange="onFs" bindcontrolstoggle="onToggle"></video>
<view style="height:300px;background:#ffffff" bindtap="onBtn">btn</view>"#
    );
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"],"window":{"navigationStyle":"custom"}}"#.to_string()),
        ("app.js", "App({});".to_string()),
        ("app.wxss", String::new()),
        ("pages/index/index.json", "{}".to_string()),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", "page{background:#ffffff}".to_string()),
        ("pages/index/index.js", JS.to_string()),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let clip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/tiny_1s.mp4");
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::copy(clip, root.join(format!("assets/{name}.mp4"))).unwrap();
    Fixture(root)
}

struct E {
    e: MiniEngine,
    t: u64,
}

impl E {
    fn new(fx: &Fixture) -> Self {
        let data = fx.0.join("_data");
        let mut e = MiniEngine::new(&fx.0.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).expect("创建引擎");
        e.launch(Some("pages/index/index")).expect("打开页面");
        let mut me = E { e, t: 1000 };
        me.wait(50);
        me
    }
    /// 真等 `ms` 毫秒，期间按 16ms 一帧 pump（播放器按墙上时钟走）
    fn wait(&mut self, ms: u64) {
        let until = Instant::now() + Duration::from_millis(ms);
        loop {
            self.t += 16;
            self.e.pump(self.t);
            if Instant::now() >= until {
                break;
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }
    fn tap(&mut self, x: f32, y: f32) {
        self.e.pointer_down(x, y);
        self.wait(16);
        self.e.pointer_up(x, y);
        self.wait(32);
    }
    fn ctx(&mut self, call: &str) {
        self.e.eval(&format!("wx.createVideoContext('v').{call}")).unwrap();
        self.wait(32);
    }
    fn js(&self, code: &str) -> Value {
        let s = self.e.eval(&format!("JSON.stringify({code})")).unwrap_or_default();
        let s: String = serde_json::from_str(&s).unwrap_or(s);
        serde_json::from_str(&s).unwrap_or(Value::Null)
    }
    fn events(&self) -> Vec<String> {
        serde_json::from_value(self.js("__currentPage.data.ev")).unwrap_or_default()
    }
    fn times(&self) -> Vec<f64> {
        serde_json::from_value(self.js("__currentPage.data.times")).unwrap_or_default()
    }
    fn clear(&mut self) {
        self.e.eval("__currentPage.data.ev = []; __currentPage.data.times = []").unwrap();
    }
}

#[test]
fn metadata_play_timeupdate_ended() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("ended");
    let mut e = E::new(&fx);
    assert_eq!(e.events(), vec![r#"loadedmetadata {"duration":1,"height":48,"width":64}"#], "加载完先报一次元数据");

    e.clear();
    e.ctx("play()");
    assert_eq!(e.events(), vec!["play {}"]);
    e.wait(1400);
    let ev = e.events();
    assert_eq!(ev, vec!["play {}", "ended {}"], "播到末尾发 ended（不发 pause）");
    let times = e.times();
    // 1 秒的片子、250ms 一次：起播那一下 + 中间三四次 + 末尾补一次 = 4~6 次
    assert!((4..=6).contains(&times.len()), "timeupdate 次数 {times:?}");
    assert!(times.windows(2).all(|w| w[1] >= w[0]), "currentTime 单调不减 {times:?}");
    assert_eq!(*times.last().unwrap(), 1.0, "最后一次 timeupdate 停在时长上");

    // 播完再 play：从头开始
    e.clear();
    e.ctx("play()");
    e.wait(300);
    let t = e.times();
    assert!(!t.is_empty() && t[0] < 0.3, "播完再播应从头开始 {t:?}");
    e.ctx("pause()");
}

#[test]
fn pause_seek_rate_and_tap_to_play() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("seek");
    let mut e = E::new(&fx);
    e.clear();

    // 暂停着点中间的播放键：起播
    e.tap(150.0, 80.0);
    assert_eq!(e.events(), vec!["play {}"], "点中间的播放键应起播");
    // 播放中点画面：控制条收起（与微信一致，不切播放）
    e.tap(60.0, 60.0);
    let hide = r#"controlstoggle {"show":false}"#;
    let show = r#"controlstoggle {"show":true}"#;
    assert_eq!(e.events(), vec!["play {}", hide]);
    // 收起着点控制条那块：先把控制条叫出来，不算点到播放键
    e.tap(20.0, 200.0 - 18.0);
    assert_eq!(e.events(), vec!["play {}", hide, show]);
    // 再点左下角的播放键：暂停
    e.tap(20.0, 200.0 - 18.0);
    assert_eq!(e.events(), vec!["play {}", hide, show, "pause {}"], "点控制条的播放键暂停");
    assert_eq!(e.js("__currentPage.data.taps"), 0, "点视频不该冒泡到下面的 view");

    e.clear();
    e.ctx("seek(0.5)");
    assert_eq!(e.events(), vec![r#"seekcomplete {"position":0.5}"#]);
    assert!(e.times().is_empty(), "暂停时 seek 不发 timeupdate");
    e.ctx("playbackRate(2)");
    e.ctx("play()");
    e.wait(130);
    let t = e.times();
    assert!(!t.is_empty() && t[0] >= 0.5, "从 seek 的位置接着播 {t:?}");
    e.wait(300);
    // 2 倍速：剩下 0.5 秒 0.25 秒就播完
    assert!(e.events().iter().any(|s| s == "ended {}"), "2 倍速应更快播完：{:?}", e.events());
}

#[test]
fn missing_file_fires_error_once() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("err");
    // 换成一个不存在的文件
    let wxml = std::fs::read_to_string(fx.0.join("pages/index/index.wxml")).unwrap().replace("/assets/err.mp4", "/assets/none.mp4");
    std::fs::write(fx.0.join("pages/index/index.wxml"), wxml.replace("bindloadedmetadata", "binderror=\"onErr\" bindloadedmetadata")).unwrap();
    std::fs::write(
        fx.0.join("pages/index/index.js"),
        JS.replace("onBtn() {", "onErr(e) { this.rec('error', e); },\n  onBtn() {"),
    )
    .unwrap();
    let mut e = E::new(&fx);
    e.wait(50);
    let ev = e.events();
    assert_eq!(ev.len(), 1, "只报一次：{ev:?}");
    assert!(ev[0].starts_with("error ") && ev[0].contains("none.mp4"), "{ev:?}");
}

#[test]
fn fullscreen_covers_page_and_eats_taps() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("fs");
    let mut e = E::new(&fx);
    e.clear();
    let probe = |e: &mut E| {
        // 页面下半部分（白底的 view）里一个像素，物理坐标 (100, 500) @2x
        let (w, _) = e.e.pixel_size();
        let px = e.e.rgba();
        let i = ((500 * 2) * w as usize + 100 * 2) * 4;
        (px[i], px[i + 1], px[i + 2])
    };
    assert_eq!(probe(&mut e), (255, 255, 255));

    e.ctx("requestFullScreen()");
    assert_eq!(
        e.events(),
        vec![r#"fullscreenchange {"direction":"horizontal","fullScreen":true}"#],
        "不给 direction 时按画面宽高比（64×48 是横的）"
    );
    e.wait(32);
    // 横屏：画面转过来画，667×375 的画面里 4:3 的视频左右是黑边 —— 也就是屏幕上方 / 下方那几十像素
    let at = |e: &mut E, x: usize, y: usize| {
        let (w, _) = e.e.pixel_size();
        let px = e.e.rgba();
        let i = ((y * 2) * w as usize + x * 2) * 4;
        (px[i], px[i + 1], px[i + 2])
    };
    assert_eq!(at(&mut e, 100, 640), (0, 0, 0), "全屏画面盖住页面（黑边）");
    // 全屏时点页面的位置：不穿透（点在画面上：控制条收起）
    e.tap(100.0, 500.0);
    assert_eq!(e.js("__currentPage.data.taps"), 0, "全屏时点击不能落到页面上");
    assert!(e.events().iter().any(|s| s == r#"controlstoggle {"show":false}"#), "{:?}", e.events());
    // 中间的播放键：横屏画面的中心在屏幕 (187.5, 333.5)
    e.tap(187.5, 333.5);
    assert!(e.events().iter().any(|s| s == "play {}"), "全屏里点中间的播放键起播：{:?}", e.events());
    // 顶栏的返回键：画面左上角，横屏（逆时针 90°）时在屏幕右上角
    e.tap(375.0 - 20.0, 20.0);
    assert!(
        e.events().iter().any(|s| s == r#"fullscreenchange {"direction":"horizontal","fullScreen":false}"#),
        "{:?}",
        e.events()
    );
    e.ctx("pause()");
    e.wait(32);
    assert_eq!(probe(&mut e), (255, 255, 255), "退出全屏后页面回来");
    e.tap(100.0, 500.0);
    assert_eq!(e.js("__currentPage.data.taps"), 1);
}

#[test]
fn vertical_fullscreen_seek_drag_and_back_key() {
    use mini_render::renderer::components::video_chrome::{bar_layout, Flags};
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("fsv");
    let mut e = E::new(&fx);
    e.clear();
    e.ctx("requestFullScreen({direction: 0})");
    assert_eq!(e.events(), vec![r#"fullscreenchange {"direction":"vertical","fullScreen":true}"#]);
    // 竖屏全屏：画面坐标就是屏幕坐标。拖底部控制条上的进度到正中，松手才 seek 一次
    let flags = { let mut f = Flags::of(&Default::default(), 375.0); f.fullscreen_btn = true; f };
    let bar = bar_layout(0.0, 0.0, 375.0, 667.0, &flags, 1.0, true, 1.0);
    let (x0, tw) = bar.track.expect("全屏有进度条");
    let y = bar.y + bar.h / 2.0;
    e.e.pointer_down(x0 + 4.0, y);
    e.wait(16);
    for i in 1..=5 {
        e.e.pointer_move(x0 + tw / 2.0 * i as f32 / 5.0, y);
        e.wait(16);
    }
    assert!(e.events().iter().all(|s| !s.starts_with("seekcomplete")), "拖的时候不 seek：{:?}", e.events());
    e.e.pointer_up(x0 + tw / 2.0, y);
    e.wait(32);
    let seeks: Vec<_> = e.events().into_iter().filter(|s| s.starts_with("seekcomplete")).collect();
    assert_eq!(seeks, vec![r#"seekcomplete {"position":0.5}"#], "松手跳到正中一次");
    assert!(e.events().iter().all(|s| !s.starts_with("controlstoggle")), "拖进度不算点画面");
    // 系统返回键：先退出全屏，不离开页面
    assert!(e.e.back(), "全屏时返回键被吃掉");
    e.wait(32);
    assert!(e.events().iter().any(|s| s == r#"fullscreenchange {"direction":"vertical","fullScreen":false}"#), "{:?}", e.events());
    assert!(!e.e.back(), "退出全屏后再按返回：只剩一页，交给宿主");
}

#[test]
fn inline_seek_drag_follows_finger_and_seeks_once() {
    let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
    let fx = fixture("drag");
    let mut e = E::new(&fx);
    e.clear();
    let (t0, tw) = mini_render::renderer::components::progress_track(300.0);
    let y = 200.0 - 20.0;
    e.e.pointer_down(t0 + 2.0, y);
    e.wait(16);
    for i in 1..=6 {
        e.e.pointer_move(t0 + 2.0 + (tw * 0.75 - 2.0) * i as f32 / 6.0, y + 0.5);
        e.wait(16);
    }
    e.e.pointer_up(t0 + tw * 0.75, y + 0.5);
    e.wait(32);
    assert_eq!(e.events(), vec![r#"seekcomplete {"position":0.75}"#], "横着拖进度条：只在松手时 seek 一次，也不切控制条");
    // 竖着划过进度条：不归它（页面不够长滚不动，但也不能 seek）
    e.clear();
    e.e.pointer_down(t0 + 10.0, y);
    e.wait(16);
    e.e.pointer_move(t0 + 10.0, y - 60.0);
    e.wait(16);
    e.e.pointer_up(t0 + 10.0, y - 60.0);
    e.wait(32);
    assert!(e.events().is_empty(), "{:?}", e.events());
}
