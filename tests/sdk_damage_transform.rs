//! setData 改带 `transform` 的节点（手势 / worklet 每帧改 `style="transform:…"`）：
//! 局部重画（invalidate_transform.rs 按绘制几何算旧、新两次画到哪）与整帧重画**逐帧逐字节一致**。
//!
//! 从前这种子树一律整帧：skyline 演示页拖一下 pan 小圆点，之后每帧 `__nodeStyles` 都整条带重画。
//! 覆盖三条绘制路径：纯平移（子树带溢出的绝对定位孩子 + 阴影）、旋转 + 缩放（离屏仿射）、
//! 从无 transform 变成有 transform。中途滚动，确认滚过之后照样对。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-transform");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="hd">顶部</view>
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>
<view class="zone">
  <view class="dot" style="transform: translate({{x}}px, {{y}}px);"><view class="tip">{{x}}</view></view>
  <view class="card" style="transform: rotate({{r}}deg) scale({{s}});">卡片</view>
  <view class="late" style="{{late}}">后加</view>
</view>
<view wx:for="{{rows}}" wx:key="*this" class="row">下半第 {{item}} 行</view>"#;
    let wxss = ".hd { height: 40px; background-color: #ffffff; }
.row { height: 44px; line-height: 44px; padding: 0 16px; border-bottom: 1px solid #eeeeee; background-color: #ffffff; }
.zone { position: relative; height: 220px; margin: 10px; background-color: #f0f4ff; }
.dot { position: absolute; left: 40px; top: 40px; width: 40px; height: 40px; border-radius: 20px; background-color: #07c160; box-shadow: 0 4px 10px rgba(0,0,0,0.35); }
.tip { position: absolute; left: 30px; top: -18px; width: 50px; height: 16px; font-size: 12px; background-color: #333333; color: #ffffff; }
.card { position: absolute; left: 200px; top: 60px; width: 90px; height: 60px; background-color: #fa9d3b; color: #ffffff; }
.late { position: absolute; left: 120px; top: 150px; width: 60px; height: 40px; background-color: #10aeff; }";
    let js = "Page({ data: { x: 0, y: 0, r: 0, s: 1, late: '', rows: Array.from({length: 12}, (_, i) => i + 1) } });";
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
    root
}

fn engine(root: &PathBuf, no_damage: bool) -> MiniEngine {
    let data = root.join("_data");
    let env = HostEnv { no_damage, damage_log: !no_damage && std::env::var("MINI_DAMAGE_LOG").is_ok(), ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 3.0, env, EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e
}

#[test]
fn set_data_改_transform_走局部重画且逐字节一致() {
    let root = fixture();
    mini_render::renderer::components::reset_shared_component_state();
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut kinds = Vec::new();
    let mut t = 1u64;
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str, kinds: &mut Vec<FrameKind>| {
        t += 16;
        let (fa, fb) = (a.pump_ex(t) & FRAME_NEW, b.pump_ex(t) & FRAME_NEW);
        assert!(fa == 0 || fb != 0, "{what}：参照没出帧，局部却出了");
        if fb == 0 {
            return;
        }
        if fa != 0 {
            kinds.push(a.last_frame_kind());
        }
        let (pa, pb) = (a.rgba().to_vec(), b.rgba());
        if pa.as_slice() != pb {
            let i = pa.chunks_exact(4).zip(pb.chunks_exact(4)).position(|(x, y)| x != y).unwrap_or(0);
            let w = a.pixel_size().0 as usize;
            panic!("{what}（局部做的是「{}」）：第一个差异像素在 ({},{})", a.last_frame_kind().label(), i % w, i / w);
        }
    };
    step(&mut a, &mut b, "首屏", &mut kinds);
    step(&mut a, &mut b, "第二帧", &mut kinds);
    a.scroll_to(300.0);
    b.scroll_to(300.0);
    step(&mut a, &mut b, "滚到手势区", &mut kinds);
    kinds.clear();
    for i in 1..=40 {
        let (x, y) = (i as f32 * 4.5, ((i as f32) * 0.4).sin() * 50.0 + 20.0);
        let (r, s) = (i as f32 * 9.0, 1.0 + (i % 7) as f32 * 0.08);
        let late = if i >= 20 { format!("transform: translateY({}px) scale(1.2);", -(i as f32)) } else { String::new() };
        let js = format!("__currentPage.setData({{ x: {x:.1}, y: {y:.1}, r: {r}, s: {s:.2}, late: {late:?} }})");
        a.eval(&js).unwrap();
        b.eval(&js).unwrap();
        if i % 10 == 0 {
            // 中途滚一下：滚过之后的局部范围要按新条带算
            let to = 300.0 + i as f32 * 3.0;
            a.scroll_to(to);
            b.scroll_to(to);
        }
        step(&mut a, &mut b, &format!("第 {i} 次改 transform"), &mut kinds);
    }
    let damage = kinds.iter().filter(|k| **k == FrameKind::Damage).count();
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    // 空转守卫：真的走了局部重画，不是两边都整帧
    assert!(damage >= 30, "改 transform 的帧没走局部重画（局部 {damage} 帧）：{kinds:?}");
    assert_eq!(full, 0, "改 transform 不该整帧：{kinds:?}");
}
