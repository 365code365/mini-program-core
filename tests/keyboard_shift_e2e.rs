//! 键盘顶起画面（输入框 `adjust-position`，host/engine_keyboard.rs）：引擎合成时把导航栏以下整体上移，
//! 导航栏留在原处；触摸坐标宿主照原样给，引擎换回没顶起时的位置。
//!
//! 背景：从前 Android SDK 给 SurfaceView 做 `translationY` 动画，引擎画的导航栏跟着被推出屏幕，
//! 页面内容顶到状态栏底下（招财银行转账页「附言」），而且顶起那 200ms 一顿一顿。
//! 判据：① 顶起后导航栏那几行与顶起前逐字节相同；② 导航栏以下第 y 行 == 顶起前第 y + shift 行；
//! ③ 顶起过程只重新合成、不重画页面；④ 点顶起后画面上的「附言」输入框，聚焦的就是它；⑤ 收回后与顶起前逐字节相同
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine, SafeAreaInsets};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn 键盘顶起_导航栏不动_内容整体上移_点击坐标跟着换() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/bank");
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("kb-shift");
    std::fs::remove_dir_all(&data).ok();
    let opts = EngineOptions {
        navigation_bar: true,
        safe_area: Some(SafeAreaInsets { top: 44.0, right: 0.0, bottom: 34.0, left: 0.0 }),
        ..EngineOptions::default()
    };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 360, 785, 3.0, Default::default(), opts)
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(Some("pages/transfer/transfer")).expect("打开转账");
    let mut t = 1000u64;
    let mut kinds = Vec::new();
    let mut pump = |e: &mut MiniEngine, n: u32, kinds: &mut Vec<FrameKind>| {
        for _ in 0..n {
            t += 16;
            std::thread::sleep(Duration::from_millis(16));
            if e.pump_ex(t) & FRAME_NEW != 0 {
                kinds.push(e.last_frame_kind());
            }
        }
    };
    pump(&mut e, 10, &mut kinds);
    // 先找「附言」输入框在画面上的位置：没顶起时自上往下点，第一次聚焦到它（placeholder 带「房租」）的 y
    let mut remark_y = None;
    for k in 0..60 {
        let y = 420.0 + k as f32 * 6.0;
        while e.poll_event().is_some() {}
        e.pointer_down(200.0, y);
        pump(&mut e, 1, &mut kinds);
        e.pointer_up(200.0, y);
        pump(&mut e, 2, &mut kinds);
        let mut hit = false;
        while let Some(ev) = e.poll_event() {
            hit |= ev.contains("\"inputFocus\"") && ev.contains("房租");
        }
        if hit {
            remark_y = Some(y + 6.0);
            break;
        }
    }
    let remark_y = remark_y.expect("没找到「附言」输入框");
    // 点别处失焦，等光标收掉
    e.pointer_down(20.0, 300.0);
    e.pointer_up(20.0, 300.0);
    e.eval("wx.hideKeyboard && wx.hideKeyboard()").ok();
    pump(&mut e, 40, &mut kinds);
    let (w, h) = e.pixel_size();
    let before = e.rgba().to_vec();
    let row = |p: &[u8], y: u32| p[(y * w * 4) as usize..((y + 1) * w * 4) as usize].to_vec();
    let nav_px = (e.nav_height() * 3.0).round() as u32;
    assert!(nav_px > 100, "要有引擎画的导航栏：{nav_px}");
    kinds.clear();
    e.set_keyboard_shift(200.0);
    pump(&mut e, 25, &mut kinds);
    let after = e.rgba().to_vec();
    let shift = 600u32;
    for y in (0..nav_px - 2).step_by(7) {
        assert!(row(&before, y) == row(&after, y), "导航栏第 {y} 行顶起后变了");
    }
    for y in (nav_px + 2..h - shift).step_by(11) {
        assert!(row(&after, y) == row(&before, y + shift), "顶起后第 {y} 行 != 顶起前第 {} 行", y + shift);
    }
    assert!(kinds.iter().all(|k| matches!(k, FrameKind::Compose)), "顶起过程该只重新合成：{kinds:?}");
    assert!(kinds.len() >= 8, "顶起要按帧走（缓动），只出了 {} 帧", kinds.len());
    // 顶起后画面上「附言」在原位置上面 200 逻辑像素
    while e.poll_event().is_some() {}
    e.pointer_down(200.0, remark_y - 200.0);
    pump(&mut e, 2, &mut kinds);
    e.pointer_up(200.0, remark_y - 200.0);
    pump(&mut e, 4, &mut kinds);
    let mut focus = None;
    while let Some(ev) = e.poll_event() {
        if ev.contains("\"inputFocus\"") {
            focus = Some(ev);
        }
    }
    let focus = focus.expect("点顶起后的「附言」要聚焦一个输入框");
    assert!(focus.contains("房租"), "聚焦的不是「附言」：{focus}");
    // 收回：与顶起前逐字节相同（附言框聚焦后多了光标，只比它上面那段）
    e.set_keyboard_shift(0.0);
    pump(&mut e, 25, &mut kinds);
    let back = e.rgba().to_vec();
    let remark_px = ((remark_y - 30.0) * 3.0) as u32;
    for y in (0..remark_px).step_by(5) {
        assert!(row(&back, y) == row(&before, y), "收回后第 {y} 行与顶起前不同");
    }
}
