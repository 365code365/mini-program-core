//! 覆盖层里的 scroll-view（点餐页「已选菜品」抽屉）：拖它时页面一个像素不动，只重画覆盖层里它那块
//! （host/engine_fixed_partial.rs），与整帧重画逐字节相同。
//!
//! 单独一个测试二进制：小程序根目录是进程级的，与 sdk_damage_nested_scroll.rs 的用例并行跑会串目录。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;
use std::time::Duration;

fn engine(root: &PathBuf, no_damage: bool) -> MiniEngine {
    let data = root.join(if no_damage { "_b" } else { "_a" });
    let env = HostEnv { no_damage, ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 3.0, env, EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e
}

/// 覆盖层里的 scroll-view（点餐页「已选菜品」抽屉）：拖它时页面一个像素不动，只重画覆盖层里它那块
/// （engine_fixed_partial.rs），与整帧重画逐字节相同。拖动位置有一点按墙上时钟走，只比两边位置相同的帧
#[test]
fn 覆盖层里的_scroll_view_拖动只重画它那块_与整帧逐字节一致() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-fixed-scroll");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="page"><view wx:for="{{rows}}" wx:key="*this" class="prow"><text>页面第 {{item}} 行</text></view></view>
<view class="mask"><view class="drawer"><text class="dt">已选菜品</text>
<scroll-view scroll-y class="dl"><view wx:for="{{rows}}" wx:key="*this" class="dr"><view class="pic" style="background: linear-gradient(135deg, #8fd3f4, #84fab0);"></view><text>菜品 {{item}}</text></view></scroll-view>
</view></view>"#;
    let wxss = ".page { background-color: #f5f6f8; }
.prow { height: 60px; margin: 6px 10px; background-color: #fff; border-radius: 8px; }
.mask { position: fixed; left: 0; top: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.45); }
.drawer { position: absolute; left: 0; right: 0; bottom: 0; height: 360px; background: #fff; border-radius: 16px 16px 0 0; }
.dt { display: block; height: 40px; line-height: 40px; padding: 0 16px; }
.dl { height: 300px; }
.dr { display: flex; flex-direction: row; align-items: center; height: 64px; padding: 0 16px; border-bottom: 1px solid #eee; }
.pic { width: 44px; height: 44px; border-radius: 8px; margin-right: 10px; }";
    let js = "Page({ data: { rows: Array.from({length: 24}, (_, i) => i + 1) } });";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", wxml),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", js),
        ("pages/p/p.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut t = 1u64;
    let (mut compared, mut kinds) = (0, Vec::new());
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str, kinds: &mut Vec<FrameKind>| {
        t += 16;
        std::thread::sleep(Duration::from_millis(8));
        let fa = a.pump_ex(t) & FRAME_NEW;
        let fb = b.pump_ex(t) & FRAME_NEW;
        if fa != 0 {
            kinds.push(a.last_frame_kind());
        }
        if (fa != 0 || fb != 0) && a.rgba() == b.rgba() {
            compared += 1;
        } else if fa != 0 || fb != 0 {
            // 位置不同（按时间采样差了半像素）时不比；位置相同却不一样才是错
            assert!(
                a.scroll_view_offsets() != b.scroll_view_offsets(),
                "{what}：两边位置一样，局部那边「{}」却与整帧不一致",
                a.last_frame_kind().label()
            );
        }
    };
    for _ in 0..4 {
        step(&mut a, &mut b, "首屏", &mut kinds);
    }
    kinds.clear();
    // 抽屉列表在屏幕底部 300px 里：从 y=600 往上拖 200px，停 150ms 再松手；再往回拖
    for (from, to) in [(600.0f32, 400.0f32), (420.0, 620.0)] {
        for e in [&mut a, &mut b] {
            e.pointer_down(180.0, from);
        }
        step(&mut a, &mut b, "按下", &mut kinds);
        for i in 1..=20 {
            let y = from + (to - from) * i as f32 / 20.0;
            for e in [&mut a, &mut b] {
                e.pointer_move(180.0, y);
            }
            step(&mut a, &mut b, "拖", &mut kinds);
        }
        std::thread::sleep(Duration::from_millis(150));
        for e in [&mut a, &mut b] {
            e.pointer_up(180.0, to);
        }
        for _ in 0..20 {
            step(&mut a, &mut b, "松手", &mut kinds);
        }
    }
    assert!(compared >= 10, "比过的帧太少：{compared}");
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    assert!(full == 0, "拖覆盖层里的列表不该整帧：{kinds:?}");
}
