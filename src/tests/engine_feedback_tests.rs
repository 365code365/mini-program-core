//! 移动端 SDK（[`MiniEngine`]）的提示框与弹窗：`wx.showLoading` 的圈要真的转、
//! `mask` 才挡触摸、Toast 与 Loading 共用一个槽位。
//!
//! 为什么要在引擎上测：桌面窗体每帧都重新合成，SDK 只在「有理由」时才合成 ——
//! 从前 Loading 的圈在 SDK 上纹丝不动，原因有三层：时钟传的是「这一帧开始的时刻」
//! （转角恒为 0）、圈在转不算重新合成的理由、`wants_next_frame` 也不认它。
//! 静态快照与单元测试都看不出来，只有按帧推进引擎才测得到。

use crate::host::MiniEngine;

fn engine_in(route: &str, slot: &str) -> super::LockedEngine {
    let g = super::LockedEngine::lock();
    let root = crate::app_dir::resolve("sample-app");
    let data = crate::data_dir::subdir(&format!("engine-feedback-tests/{slot}"));
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0)
        .expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some(route)).expect("打开页面");
    e.pump(1);
    super::LockedEngine::new(e, g)
}

/// 提示框那块（136×136，中心在视口 40% 高处）的 RGBA 像素
fn tip_pixels(e: &mut MiniEngine) -> Vec<u8> {
    let (w, h) = e.pixel_size();
    let (cx, cy, half) = (w as usize / 2, (h as f32 * 0.4) as usize, 136usize);
    let rgba = e.rgba().to_vec();
    let mut out = Vec::new();
    for y in cy - half / 2..cy + half / 2 {
        let row = y * w as usize * 4;
        out.extend_from_slice(&rgba[row + (cx - half / 2) * 4..row + (cx + half / 2) * 4]);
    }
    out
}

#[test]
fn loading_的圈按时钟转_且一直要帧() {
    let mut e = engine_in("pages/profile/profile", "spin");
    e.eval("wx.showLoading({ title: '加载中' })").unwrap();
    e.pump(0);
    assert!(e.wants_next_frame(), "Loading 弹着时宿主必须续帧，圈才会转");
    let a = tip_pixels(&mut e);
    // 时钟走了 1/4 秒（15 步）：只合成就要出新画面，转角跟着变
    e.set_animation_clock(2.25);
    assert!(e.pump(0), "圈转到新的一步要出一帧新画面");
    let b = tip_pixels(&mut e);
    let changed = a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count();
    assert!(changed > 200, "转了 90° 提示框里应有一片像素变了，实际 {changed}");
    // 收起后不再因为它要帧（页面本身没别的动画时）
    e.eval("wx.hideLoading()").unwrap();
    e.pump(0);
    assert!(!crate::host::ui_overlay::tip_spinning(&e.toast, &e.loading));
}

/// 点第二个 tab（分类）
fn tap_second_tab(e: &mut MiniEngine) {
    let h = 667.0 - crate::host::tabbar_height() as f32;
    e.pointer_down(141.0, h + 25.0);
    e.pointer_up(141.0, h + 25.0);
    for _ in 0..4 {
        e.pump(0);
    }
}

#[test]
fn 带_mask_的_loading_挡住触摸() {
    let mut e = engine_in("pages/index/index", "mask-on");
    e.eval("wx.showLoading({ title: '处理中', mask: true })").unwrap();
    e.pump(0);
    tap_second_tab(&mut e);
    assert_eq!(e.current_route(), "pages/index/index", "mask: true 期间点不动页面");
}

#[test]
fn 不带_mask_的_loading_可以点穿() {
    // 微信的 mask 文档原话「是否显示透明蒙层，防止触摸穿透」—— 不给就是能点穿。
    // 从前 SDK 与桌面都是 Loading 一律挡
    let mut e = engine_in("pages/index/index", "mask-off");
    e.eval("wx.showLoading({ title: '加载中' })").unwrap();
    e.pump(0);
    tap_second_tab(&mut e);
    assert_eq!(e.current_route(), "pages/category/category");
}

#[test]
fn toast_顶掉_loading() {
    // demo 里「带 mask」那个按钮：2 秒后 hideLoading 再 showToast —— 两步合在一起也要只剩 Toast
    let mut e = engine_in("pages/profile/profile", "slot");
    e.eval("wx.showLoading({ title: '处理中' }); wx.showToast({ title: '处理完成' })").unwrap();
    e.pump(0);
    assert!(e.loading.is_none(), "showToast 之后 Loading 应被顶掉");
    assert_eq!(e.toast.as_ref().map(|t| t.title.as_str()), Some("处理完成"));
}
