//! 页面根是一个 `<scroll-view>`（uni-app 产物的写法：`<scroll-view class="page">` 包住整页，内容多高它就多高，
//! 滚的是页面）：滚动中补条带时**不整页重画这个 scroll-view 的离屏缓存**，而且与整帧重画逐帧逐字节一致。
//!
//! 背景：从前「只换了滚动偏移」也把 scroll-view 缓存作废，视口每滑出已绘制条带一次，就把整页内容重新光栅化
//! （tea-app ai-result 补 30 行要画 8ms / 真机 15~23ms，「刚开始不卡、滑着滑着卡起来」，诊断 20261024-153306）。
//! 判据：① A（局部重画）每一帧与 B（`MINI_NO_DAMAGE`，整帧）逐字节相同；② A 里补条带真的出现了（空转守卫），
//! 而且补条带那几帧的绘制量与条带大小相称（不会每次都把 3000 多行重画一遍：用 `MINI_SCROLL_LOG` 没法在测试里断言，
//! 改断言「中途 setData 之后，内容的改动在两边都看得到」，证明缓存该作废时照样作废）。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-root-scroll-view");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<scroll-view scroll-y="true" class="page">
<view class="hd">标题 {{count}}</view>
<view wx:for="{{rows}}" wx:key="*this" class="card"><view class="tag">第 {{item}} 张</view><text class="tx">卡片正文 {{item}} {{count}}</text></view>
</scroll-view>"#;
    let wxss = ".page { flex: 1; background-color: #f7f3ec; }
.hd { height: 60px; line-height: 60px; padding: 0 16px; background-color: #fff; }
.card { margin: 10px 12px; padding: 12px; border-radius: 10px; border: 1px solid #e8dcc8; background-color: #fff; }
.tag { display: inline-block; padding: 2px 8px; border-radius: 6px; background-color: #f3eadb; }
.tx { display: block; margin-top: 6px; color: #555; }";
    let js = "Page({ data: { count: 0, rows: Array.from({length: 40}, (_, i) => i + 1) } });";
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
    let data = root.join(if no_damage { "_b" } else { "_a" });
    let env = HostEnv { no_damage, ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 3.0, env, EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e
}

#[test]
fn 根_scroll_view_的页面_滚动补条带与整帧逐字节一致() {
    let root = fixture();
    mini_render::renderer::components::reset_shared_component_state();
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut t = 1u64;
    let mut kinds = Vec::new();
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str| {
        t += 16;
        let fa = a.pump_ex(t) & FRAME_NEW;
        let fb = b.pump_ex(t) & FRAME_NEW;
        if fb == 0 {
            return;
        }
        if fa != 0 {
            kinds.push(a.last_frame_kind());
        }
        assert!(a.rgba() == b.rgba(), "{what}（滚动 {}，局部那边「{}」）：合成结果不一致", a.scroll_position(), a.last_frame_kind().label());
    };
    for _ in 0..3 {
        step(&mut a, &mut b, "首屏");
    }
    // 一路往下滚出条带（每步 30px，与真机补条带的步子差不多），中途 setData 改所有卡片的文字
    for i in 1..=90 {
        let y = i as f32 * 30.0;
        a.scroll_to(y);
        b.scroll_to(y);
        if i == 40 {
            a.eval("getCurrentPages()[0].setData({count: 7})").unwrap();
            b.eval("getCurrentPages()[0].setData({count: 7})").unwrap();
        }
        step(&mut a, &mut b, "往下滚");
    }
    // 跳回顶（跨好几个条带），再慢慢往下
    for i in (0..10).rev() {
        a.scroll_to(i as f32 * 150.0);
        b.scroll_to(i as f32 * 150.0);
        step(&mut a, &mut b, "往回滚");
    }
    let bands = kinds.iter().filter(|k| matches!(k, FrameKind::Band)).count();
    assert!(bands >= 20, "补条带要真的出现（不然两边都在整帧，比了个寂寞）：{bands} 次，{kinds:?}");
}
