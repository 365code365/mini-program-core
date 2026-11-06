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
/// 引擎的小程序根目录是进程级的（单实例）：两个用例各开一个包，不能同时跑
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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

/// 播放页那种写法（云听音乐）：整页铺一张成组透明度的背景 + 渐变遮罩，中间一张一直在转的唱片（圆、带阴影圈、
/// 圆角封面），上面压着一根带 transition 的唱针，底下一条进度条。
///
/// 只有唱片在转的帧走「动画底图」（wxml_renderer/backdrop.rs：唱片底下那层贴回上一帧留的），
/// 中途改唱片底下的颜色、拨唱针、拖进度条，每一帧都要与整帧重画逐字节相同
fn player_fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-backdrop");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="wrap">
  <view class="bg" style="background-color: {{bg}};"></view>
  <view class="mask"></view>
  <view class="area">
    <view class="needle {{on ? 'needle-on' : ''}}"></view>
    <view class="disc {{playing ? 'spin' : ''}}"><view class="cover">封面</view></view>
  </view>
  <view class="prog"><text class="time">{{pos}}</text><slider class="slider" min="0" max="100" value="{{pos}}" bindchanging="onChanging" /></view>
  <view class="btn"><view class="pause" wx:if="{{playing}}"><view class="pbar"></view><view class="pbar"></view></view><view class="tri" wx:else><view class="tri-in"></view></view></view>
</view>"#;
    let wxss = ".wrap { position: relative; height: 667px; background-color: #14121e; overflow: hidden; }
.bg { position: absolute; left: -40px; top: -40px; width: 460px; height: 640px; opacity: 0.55; }
.mask { position: absolute; left: 0; top: 0; right: 0; bottom: 0; background: linear-gradient(180deg, rgba(20,18,30,0.55), rgba(20,18,30,0.92)); }
.area { position: relative; height: 300px; display: flex; align-items: center; justify-content: center; margin-top: 20px; }
.needle { position: absolute; top: 0; left: 180px; width: 6px; height: 80px; background-color: #dddddd; transform-origin: 3px 0; transform: rotate(-28deg); transition: transform 0.4s; z-index: 2; }
.needle-on { transform: rotate(-6deg); }
.disc { width: 240px; height: 240px; border-radius: 120px; background: radial-gradient(circle, #333333 0%, #111111 70%, #222222 100%); display: flex; align-items: center; justify-content: center; box-shadow: 0 0 0 8px rgba(255,255,255,0.08); }
.spin { animation: spin 20s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
.cover { width: 160px; height: 160px; border-radius: 80px; background-color: #fa9d3b; color: #ffffff; line-height: 160px; text-align: center; }
.prog { position: absolute; left: 0; right: 0; top: 400px; display: flex; flex-direction: row; align-items: center; }
.time { width: 40px; color: #ffffff; font-size: 12px; }
.slider { flex: 1; }
.btn { position: absolute; left: 160px; top: 480px; width: 56px; height: 56px; border-radius: 28px; background-color: #ffffff; display: flex; align-items: center; justify-content: center; }
.pause { display: flex; flex-direction: row; }
.pbar { width: 5px; height: 18px; margin: 0 3px; background-color: #14121e; }
.tri { width: 20px; height: 20px; overflow: hidden; }
.tri-in { width: 14px; height: 14px; margin-left: -7px; margin-top: 3px; background-color: #14121e; transform: scaleX(1.5) rotate(45deg); }";
    let js = "Page({ data: { bg: '#3a2a5a', on: false, pos: 20, playing: true }, onChanging(e) { this.setData({ pos: e.detail.value }); } });";
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

#[test]
fn 转圈的唱片_底下那层贴回上一帧的_改底色拨唱针拖进度条_仍逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = player_fixture();
    mini_render::renderer::components::reset_shared_component_state();
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut kinds = Vec::new();
    let mut t = 1u64;
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str, kinds: &mut Vec<FrameKind>| {
        t += 16;
        let secs = t as f32 / 1000.0;
        a.set_animation_clock(secs);
        b.set_animation_clock(secs);
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
    for i in 0..3 {
        step(&mut a, &mut b, &format!("首屏 {i}"), &mut kinds);
    }
    let before = mini_render::renderer::wxml_renderer::backdrop_restores();
    kinds.clear();
    for i in 0..30 {
        step(&mut a, &mut b, &format!("唱片转 {i}"), &mut kinds);
    }
    let reused = mini_render::renderer::wxml_renderer::backdrop_restores() - before;
    // 空转守卫：真的贴回了底图（不是每帧都照常把底下那层画一遍）
    assert!(reused >= 20, "只转唱片的 30 帧里只贴回 {reused} 次底图：{kinds:?}");
    assert!(kinds.iter().all(|k| *k == FrameKind::Damage), "只转唱片应当只局部重画：{kinds:?}");
    // 唱片底下的颜色变了：底图作废，这一帧照常画，之后重新留
    for (i, c) in ["#1f6f5a", "#6a2a2a"].iter().enumerate() {
        let js = format!("__currentPage.setData({{ bg: '{c}' }})");
        a.eval(&js).unwrap();
        b.eval(&js).unwrap();
        for j in 0..8 {
            step(&mut a, &mut b, &format!("改底色 {i} 之后第 {j} 帧"), &mut kinds);
        }
    }
    // 底色那块占了大半屏：改它整帧是对的（失效分析的取舍），只要求逐字节一致。之后只看唱针与进度条
    kinds.clear();
    // 唱针（压在唱片上面、带 transition）落下：唱针那块与唱片那块一起画
    a.eval("__currentPage.setData({ on: true })").unwrap();
    b.eval("__currentPage.setData({ on: true })").unwrap();
    for j in 0..32 {
        step(&mut a, &mut b, &format!("唱针落下第 {j} 帧"), &mut kinds);
    }
    // 拖进度条：滑块那块 + 唱片那块，回调 setData 改时间文字
    let (sx, sy) = (120.0, 400.0 + 14.0);
    a.pointer_down(sx, sy);
    b.pointer_down(sx, sy);
    let hits0 = mini_render::renderer::wxml_renderer::transform_cache_hits();
    for i in 1..=20 {
        let x = sx + i as f32 * 8.0;
        a.pointer_move(x, sy);
        b.pointer_move(x, sy);
        step(&mut a, &mut b, &format!("拖进度条第 {i} 步"), &mut kinds);
    }
    // 按着的是唱片外面的滑块：唱片照样只贴回缓存的那张（transform_cache.rs），不每帧重新光栅化。
    // 两个引擎各转一次，20 帧至少 30 次命中
    let hits = mini_render::renderer::wxml_renderer::transform_cache_hits() - hits0;
    assert!(hits >= 30, "拖进度条时唱片的离屏缓存只命中 {hits} 次");
    a.pointer_up(sx + 160.0, sy);
    b.pointer_up(sx + 160.0, sy);
    for j in 0..10 {
        step(&mut a, &mut b, &format!("松手后第 {j} 帧"), &mut kinds);
    }
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    assert_eq!(full, 0, "拨唱针 / 拖进度条都不该整帧：{kinds:?}");
    // 暂停 / 播放：唱片的 class 在 `disc` 与 `disc spin` 之间换（停在转到一半的角度、从 0° 重新转），
    // 只重画唱片转起来扫过的那个圆（invalidate_grow.rs `anim_extent`），不整帧
    kinds.clear();
    for (k, on) in [false, true, false, true].iter().enumerate() {
        let js = format!("__currentPage.setData({{ playing: {on} }})");
        a.eval(&js).unwrap();
        b.eval(&js).unwrap();
        for j in 0..(5 + k * 3) {
            step(&mut a, &mut b, &format!("第 {k} 次 playing={on} 之后第 {j} 帧"), &mut kinds);
        }
    }
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    assert_eq!(full, 0, "暂停 / 播放不该整帧：{kinds:?}");
}

/// 覆盖层里转着的封面（云听音乐的迷你播放条：fixed 底条 + 圆角阴影 + 旋转的封面图）：覆盖层只重画封面那块，
/// 与整帧逐字节一致；中途 setData 改条上的进度（覆盖层的数据变了）照样对
#[test]
fn 覆盖层里转圈的封面_只重画那一块_改进度仍逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-fixed-spin");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 首</view>
<view class="mp"><view class="prog" style="width: {{p}}%"></view><view class="cover spin"><view class="dot"></view></view><text class="name">歌名 {{p}}</text></view>"#;
    let wxss = ".row { height: 60px; line-height: 60px; padding: 0 16px; border-bottom: 1px solid #eeeeee; }
.mp { position: fixed; left: 8px; right: 8px; bottom: 8px; height: 54px; background: rgba(255,255,255,0.98); border-radius: 27px; box-shadow: 0 3px 12px rgba(0,0,0,0.12); display: flex; flex-direction: row; align-items: center; padding: 0 12px 0 7px; overflow: hidden; }
.prog { position: absolute; left: 0; bottom: 0; height: 2px; background-color: #f04c4c; }
.cover { width: 40px; height: 40px; border-radius: 20px; border: 3px solid #222222; background-color: #6a9cff; }
.dot { width: 10px; height: 10px; margin: 4px; background-color: #ffffff; }
.spin { animation: spin 12s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
.name { margin-left: 10px; font-size: 14px; }";
    let js = "Page({ data: { p: 10, rows: Array.from({length: 20}, (_, i) => i + 1) } });";
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
    mini_render::renderer::components::reset_shared_component_state();
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut t = 1u64;
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str| {
        t += 16;
        let secs = t as f32 / 1000.0;
        a.set_animation_clock(secs);
        b.set_animation_clock(secs);
        let (fa, fb) = (a.pump_ex(t) & FRAME_NEW, b.pump_ex(t) & FRAME_NEW);
        assert!(fa == 0 || fb != 0, "{what}：参照没出帧，局部却出了");
        if fb == 0 {
            return;
        }
        let (pa, pb) = (a.rgba().to_vec(), b.rgba());
        if pa.as_slice() != pb {
            let i = pa.chunks_exact(4).zip(pb.chunks_exact(4)).position(|(x, y)| x != y).unwrap_or(0);
            let w = a.pixel_size().0 as usize;
            panic!("{what}（局部做的是「{}」）：第一个差异像素在 ({},{})", a.last_frame_kind().label(), i % w, i / w);
        }
    };
    for i in 0..3 {
        step(&mut a, &mut b, &format!("首屏 {i}"));
    }
    let before = mini_render::host::fixed_partial_count();
    for i in 0..30 {
        step(&mut a, &mut b, &format!("封面转 {i}"));
        if i % 10 == 9 {
            let js = format!("__currentPage.setData({{ p: {} }})", 20 + i);
            a.eval(&js).unwrap();
            b.eval(&js).unwrap();
        }
    }
    a.scroll_to(200.0);
    b.scroll_to(200.0);
    for i in 0..10 {
        step(&mut a, &mut b, &format!("滚过之后封面转 {i}"));
    }
    let partial = mini_render::host::fixed_partial_count() - before;
    // 空转守卫：真的只重画了封面那块（引擎 b 是 MINI_NO_DAMAGE，一次也不走）
    assert!(partial >= 25, "40 帧里覆盖层只局部重画了 {partial} 次");
}
