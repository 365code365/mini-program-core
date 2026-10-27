//! 移动端 SDK（[`MiniEngine`]）里 `<slider>` 的指针链路：拖动跟手并发 `change`、
//! 竖着划照样滚页面、点一下跳到手指处。
//!
//! 为什么要在 SDK 上单测：桌面窗体与 SDK 共用 `host::input` 的手势仲裁，但 slider 曾经在
//! 两边各写了一份 —— SDK 的 `pointer_move` 里根本没有滑块分支（拖动时滑块纹丝不动），
//! 两边的 `SliderChange` 也都没送到逻辑层。静态截图一模一样，只有直接驱动引擎才测得到。
//!
//! 用 sample-app 的 showcase 页：`<slider value="{{volume}}" bindchange="onVolume">`，
//! volume 初值 60，onVolume 里 `setData({volume: e.detail.value})`。

use crate::host::MiniEngine;
use crate::ui::interaction::SliderSpec;

fn engine_in(slot: &str) -> super::LockedEngine {
    let g = super::LockedEngine::lock();
    let root = crate::app_dir::resolve("sample-app");
    let data = crate::data_dir::subdir(&format!("engine-slider-tests/{slot}"));
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0)
        .expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some("pages/showcase/showcase")).expect("打开页面");
    e.pump(1);
    super::LockedEngine::new(e, g)
}

/// 把页面滚到滑块落在视口 y≈300 处，返回它的登记信息与它在视口里的纵坐标
fn bring_slider_into_view(e: &mut MiniEngine) -> (SliderSpec, f32) {
    // 先滚一遍让滑块被画到、登记上（视口外的元素不登记）
    for step in 0..12 {
        if !e.interaction.slider_ids().is_empty() {
            break;
        }
        e.scroll_to(step as f32 * 200.0);
        e.pump(0);
    }
    let id = e.interaction.slider_ids().into_iter().next().expect("showcase 页应该有 slider");
    let spec = *e.interaction.slider_spec(&id).unwrap();
    e.scroll_to((spec.bounds.y - 300.0).max(0.0));
    e.pump(0);
    let spec = *e.interaction.slider_spec(&id).unwrap();
    let vy = spec.bounds.y + spec.bounds.height / 2.0 - e.scroll_position();
    (spec, vy)
}

fn volume(e: &MiniEngine) -> String {
    e.eval("typeof getCurrentPages()[0].data.volume + ':' + getCurrentPages()[0].data.volume").unwrap_or_default()
}

fn swipe(e: &mut MiniEngine, from: (f32, f32), to: (f32, f32), steps: u32) {
    e.pointer_down(from.0, from.1);
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        e.pointer_move(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        e.pump(0);
    }
    e.pointer_up(to.0, to.1);
    e.pump(0);
}

#[test]
fn 抓着滑块横拖松手发change且值是number() {
    let mut e = engine_in("drag");
    let (s, vy) = bring_slider_into_view(&mut e);
    assert_eq!(volume(&e), "number:60");
    let thumb = s.thumb_x(60.0);
    let scroll_before = e.scroll_position();
    // 抓着圆点往右拖 20% 轨道长：60 → 80（抓取偏移保持，不会一抓就跳）
    swipe(&mut e, (thumb, vy), (thumb + s.track_w * 0.2, vy), 8);
    assert_eq!(volume(&e), "number:80", "松手应发 change，detail.value 是 number");
    assert_eq!(e.scroll_position(), scroll_before, "横拖滑块不该滚页面");
    assert!(!e.interaction.is_dragging_slider(), "松手后不该还在拖");
}

#[test]
fn 在滑块上竖着划照样滚页面() {
    let mut e = engine_in("vertical");
    let (s, vy) = bring_slider_into_view(&mut e);
    let before = e.scroll_position();
    let x = s.thumb_x(60.0);
    // 手指上滑 200px：页面应该滚下去，滑块不动
    swipe(&mut e, (x, vy), (x, vy - 200.0), 8);
    assert!(e.scroll_position() > before + 50.0, "竖着划应滚页面：{before} → {}", e.scroll_position());
    assert_eq!(volume(&e), "number:60", "竖着划不该改滑块");
}

#[test]
fn 点一下轨道跳到手指处() {
    let mut e = engine_in("tap");
    let (s, vy) = bring_slider_into_view(&mut e);
    let x = s.track_x + s.track_w * 0.25;
    e.pointer_down(x, vy);
    e.pointer_up(x, vy);
    e.pump(0);
    assert_eq!(volume(&e), "number:25", "点按应跳到手指处并发 change");
    assert!(!e.interaction.is_dragging_slider(), "点按之后不该进入拖动态");
}
