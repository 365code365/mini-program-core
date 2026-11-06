//! C3 / C6 / C10：draggable-sheet、SDK 上的 image 加载事件、navigation-bar / page-meta / movable-view / audio
use super::*;
use base64::Engine as _;

const LOG_FN: &str = "function L(s){ getApp().log.push(s); }";
/// 8×8 的红色 PNG
const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAEklEQVR4nGP4z8CAFWEXHbQSACj/P8Fu7N9hAAAAAElFTkSuQmCC";

#[test]
fn sdk_上_image_的_bindload_binderror_派发() {
    let root = fixture(
        "image-events",
        r#"<image src="/red.png" data-n="1" bindload="ok" style="width:40px;height:40px"></image>
           <image src="/nope.png" binderror="bad" style="width:40px;height:40px"></image>"#,
        &format!("{LOG_FN} Page({{ ok(e){{ L('load ' + e.detail.width + 'x' + e.detail.height + ' ' + (typeof e.detail.width) + ' ' + e.currentTarget.dataset.n); }}, \
                   bad(e){{ L('error ' + e.detail.errMsg); }} }});"),
        &[],
    );
    std::fs::write(root.join("red.png"), base64::engine::general_purpose::STANDARD.decode(RED_PNG).unwrap()).unwrap();
    let mut e = engine(&root, EngineOptions::default());
    assert!(pump_until(&mut e, 2000, |e| log(e).len() >= 2), "{:?}", log(&e));
    let l = log(&e);
    assert!(l.contains(&"load 8x8 number 1".to_string()), "detail 是原始像素尺寸、数字：{l:?}");
    assert!(l.iter().any(|s| s.starts_with("error ")), "{l:?}");
    pump_until(&mut e, 200, |_| false);
    assert_eq!(log(&e).len(), 2, "每张图只发一次");
}

#[test]
fn draggable_sheet_scrollto_动画_onsizeupdate_与不带_id_的_node() {
    let root = fixture(
        "sheet",
        r#"<view style="height:600px;position:relative"><draggable-sheet class="sheet" initial-child-size="0.5" worklet:onsizeupdate="sz">
             <view style="height:100%;background-color:#00ff00"></view></draggable-sheet></view>"#,
        &format!("{LOG_FN} Page({{ sz(e){{ L('size ' + e.size.toFixed(2) + ' ' + Math.round(e.pixels)); }}, \
                   go(){{ this.createSelectorQuery().select('.sheet').node().exec(r => {{ L('node ' + typeof r[0].node.scrollTo); \
                     r[0].node.scrollTo({{ size: 0.8, duration: 200 }}); }}); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    // 面板占下半（600 的一半）：y=400 是绿、y=200 不是
    assert_eq!(rgb_at(&mut e, 100, 400), [0, 255, 0]);
    assert_ne!(rgb_at(&mut e, 100, 200), [0, 255, 0]);
    clear_log(&e);
    js(&e, "getCurrentPages()[0].go()");
    assert!(pump_until(&mut e, 2000, |e| log(e).iter().any(|s| s.starts_with("size 0.80"))), "{:?}", log(&e));
    let l = log(&e);
    assert_eq!(l[0], "node function", "select('.sheet').node() 不带 id 也拿到 DraggableSheetContext：{l:?}");
    assert_eq!(l.last().map(|s| s.as_str()), Some("size 0.80 480"), "pixels = size × 容器高：{l:?}");
    pump_until(&mut e, 200, |_| false);
    assert_eq!(rgb_at(&mut e, 100, 200), [0, 255, 0], "动画走完停在 0.8");
}

#[test]
fn draggable_sheet_里的列表_面板没到最大先拖面板_到了最大列表滚() {
    let items: String = (0..40).map(|i| format!(r#"<view style="height:50px">{i}</view>"#)).collect();
    let root = fixture(
        "sheet-list",
        &format!(
            r#"<view style="height:600px;position:relative"><draggable-sheet id="ds" initial-child-size="0.5" max-child-size="0.8" bind:sizeupdate="sz">
                 <scroll-view id="sv" scroll-y style="height:100%">{items}</scroll-view></draggable-sheet></view>"#
        ),
        &format!("{LOG_FN} Page({{ sz(e){{ L('size ' + e.detail.size.toFixed(2)); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    // 面板 0.5（顶边在 300）：在列表上往上推 90px → 面板长到 0.65，列表不滚
    drag(&mut e, (100.0, 500.0), (100.0, 410.0), 6);
    pump_until(&mut e, 200, |_| false);
    let l = log(&e);
    assert_eq!(l.last().map(|s| s.as_str()), Some("size 0.65"), "{l:?}");
    assert_eq!(e.scroll_view_offset("sv").unwrap_or(0.0), 0.0, "面板没到最大：列表不滚");
    // 推到最大（0.8）之后再推：列表滚、面板不动
    drag(&mut e, (100.0, 500.0), (100.0, 400.0), 6);
    pump_until(&mut e, 200, |_| false);
    assert_eq!(log(&e).last().map(|s| s.as_str()), Some("size 0.80"));
    clear_log(&e);
    drag(&mut e, (100.0, 500.0), (100.0, 350.0), 6);
    pump_until(&mut e, 300, |_| false);
    assert!(e.scroll_view_offset("sv").unwrap_or(0.0) > 100.0, "到了最大：列表滚 {:?}", e.scroll_view_offset("sv"));
    assert!(log(&e).iter().all(|s| s == "size 0.80"), "面板退回原大小、没被带着动：{:?}", log(&e));
}

#[test]
fn navigation_bar_颜色动画参数下传_movable_view_摩擦系数兜底() {
    let root = fixture(
        "navbar",
        r##"<page-meta><navigation-bar front-color="#ffffff" background-color="#ff0000" color-animation-duration="300" color-animation-timing-func="easeIn"/></page-meta>
           <movable-area style="width:200px;height:200px"><movable-view id="mv" friction="0" damping="-1" direction="all"></movable-view></movable-area>"##,
        "Page({});",
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    let evs = host_events(&mut e);
    let nb = evs.iter().find(|v| v["type"] == "navigationBar" && v.get("backgroundColor").is_some()).expect("navigationBar");
    assert_eq!(nb["animation"]["duration"], 300);
    assert_eq!(nb["animation"]["timingFunc"], "easeIn");
    let v = js(&e, "var m = getCurrentPages()[0].selectComponent('#mv'); m._friction() + ',' + m._damping()");
    assert_eq!(v, "2,20", "friction / damping ≤ 0 回默认值（官方：必须大于 0）");
}

#[test]
fn audio_binderror_的_errmsg_是数字码() {
    let root = fixture(
        "audio-err",
        r#"<audio id="a" src="/nope.mp3" binderror="err"></audio>"#,
        &format!("{LOG_FN} Page({{ err(e){{ L((typeof e.detail.errMsg) + ' ' + e.detail.errMsg); }} }});"),
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    pump_until(&mut e, 300, |_| false);
    // 宿主报播放器出错（与 InnerAudioContext 同一个 onError 通道，audio.js `__resolveAudio`）：10002 网络错误 → 2
    clear_log(&e);
    js(&e, r#"for (var k in __audios) __resolveAudio(k, 'error', '{"errCode":10002,"errMsg":"x"}')"#);
    assert!(pump_until(&mut e, 300, |e| !log(e).is_empty()));
    let l = log(&e);
    assert!(l.contains(&"number 2".to_string()) && l.iter().all(|s| s.starts_with("number ")), "官方 detail = {{errMsg: MediaError.code}}，数字：{l:?}");
}

/// 触点坐标：`pageY` 相对文档（含页面滚动）、`clientY` 相对视口（官方 Touch 对象）。页面滚过 300 之后
/// 在视口 (100, 100) 按下松手：touchstart / tap 都是 pageY 400、clientY 100；tap 的 detail 是文档坐标。
/// 从前触摸事件两个都给视口坐标、tap 两个都给文档坐标
#[test]
fn 触点的_page_与_client_坐标_页面滚过之后差一个_scrollTop() {
    let root = fixture(
        "touch-page-client",
        r#"<view style="height: 2000px;">
  <view class="hit" style="position: absolute; top: 0; left: 0; width: 375px; height: 2000px;" bindtouchstart="onStart" bindtap="onTap"></view>
</view>"#,
        "Page({ onStart(e) { getApp().log.push('start ' + e.touches[0].pageY + ' ' + e.touches[0].clientY); },
  onTap(e) { getApp().log.push('tap ' + e.changedTouches[0].pageY + ' ' + e.changedTouches[0].clientY + ' ' + e.detail.y); } });",
        &[],
    );
    let mut e = engine(&root, EngineOptions::default());
    e.scroll_to(300.0);
    pump_until(&mut e, 200, |_| false);
    clear_log(&e);
    e.pointer_down(100.0, 100.0);
    e.pump(0);
    e.pointer_up(100.0, 100.0);
    pump_until(&mut e, 200, |e| log(e).iter().any(|l| l.starts_with("tap")));
    let l = log(&e);
    assert!(l.contains(&"start 400 100".to_string()), "touchstart 的 pageY / clientY：{l:?}");
    assert!(l.contains(&"tap 400 100 400".to_string()), "tap 的 pageY / clientY / detail.y：{l:?}");
}

/// Skyline 的 `<large-image>`：默认 widthFix（宽 80 的 8×8 图高也是 80），`bind:load` 的 detail 是原图尺寸
#[test]
fn large_image_默认_width_fix_load_带原图尺寸() {
    let root = fixture(
        "large-image",
        r#"<large-image id="li" src="/red.png" style="width: 80px;" bindload="ok"></large-image>"#,
        &format!("{LOG_FN} Page({{ ok(e){{ L('load ' + e.detail.width + 'x' + e.detail.height); }} }});"),
        &[],
    );
    std::fs::write(root.join("red.png"), base64::engine::general_purpose::STANDARD.decode(RED_PNG).unwrap()).unwrap();
    let mut e = engine(&root, EngineOptions::default());
    assert!(pump_until(&mut e, 2000, |e| !log(e).is_empty()), "{:?}", log(&e));
    assert_eq!(log(&e), vec!["load 8x8".to_string()]);
    js(&e, "globalThis.__q = null; wx.createSelectorQuery().select('#li').boundingClientRect(r => { globalThis.__q = r; }).exec();");
    pump_until(&mut e, 200, |e| js(e, "!!globalThis.__q") == "true");
    assert_eq!(js(&e, "__q.width + 'x' + __q.height"), "80x80", "widthFix：高按原图比例");
}
