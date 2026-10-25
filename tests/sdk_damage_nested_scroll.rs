//! 嵌在容器里的 scroll-view（定高、滚过一段）里 setData：改文字、`wx:if` 增删一个孩子（选中条 / 角标），
//! 开 / 关局部重画两个引擎逐字节相同；外面那排分类的 `wx:if` 选中条一换也一样，而且不再整帧（孩子 2 -> 1）。
//!
//! 守的是三件事一起：分块缓存（scroll_tiles.rs，数据一变整个作废重建）、失效分析在孩子个数变了时
//! 按两串孩子画到哪取范围（invalidate_pair.rs）、scroll-view 里的失效范围要按滚动后的位置算。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-nested-scroll");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="page">
<view class="cats"><view wx:for="{{cats}}" wx:key="*this" class="cat"><view class="bar" wx:if="{{index === active}}"></view><text>分类 {{item}}</text></view></view>
<scroll-view scroll-y class="list" style="height: 420px;" scroll-top="{{top}}">
<view wx:for="{{rows}}" wx:key="id" class="row"><view class="pic" style="background: linear-gradient(135deg, #f7d27a, #f0a030);"></view><text class="t">菜品 {{item.id}} · {{item.n}} 份</text><view class="badge" wx:if="{{item.n > 0}}"><text>{{item.n}}</text></view></view>
</scroll-view>
<view class="bar-bottom"><text>合计 {{total}}</text></view>
</view>"#;
    let wxss = ".page { background-color: #f5f6f8; }
.cats { display: flex; flex-direction: row; height: 44px; background-color: #fff; }
.cat { flex: 1; position: relative; padding: 12px 6px; }
.bar { position: absolute; left: 0; top: 10px; width: 3px; height: 20px; background-color: #ff6b3d; }
.row { display: flex; flex-direction: row; align-items: center; margin: 8px 12px; padding: 10px; border-radius: 10px; background-color: #fff; }
.pic { width: 56px; height: 56px; border-radius: 8px; }
.t { flex: 1; margin-left: 10px; }
.badge { width: 20px; height: 20px; border-radius: 10px; background-color: #ff3b30; color: #fff; text-align: center; }
.bar-bottom { height: 50px; background-color: #222; color: #fff; }";
    let js = "Page({ data: { top: 0, active: 0, cats: [1,2,3,4], total: 0, rows: Array.from({length: 30}, (_, i) => ({ id: i + 1, n: 0 })) } });";
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
fn 嵌套_scroll_view_滚过之后_set_data_增删孩子_局部与整帧逐字节一致() {
    let root = fixture();
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut t = 1u64;
    let mut kinds = Vec::new();
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str, kinds: &mut Vec<FrameKind>| {
        t += 16;
        let fa = a.pump_ex(t) & FRAME_NEW;
        let fb = b.pump_ex(t) & FRAME_NEW;
        if fa != 0 {
            kinds.push(a.last_frame_kind());
        }
        if fa != 0 || fb != 0 {
            assert!(a.rgba() == b.rgba(), "{what}：局部那边「{}」与整帧不一致", a.last_frame_kind().label());
        }
    };
    for _ in 0..4 {
        step(&mut a, &mut b, "首屏", &mut kinds);
    }
    // 内层列表滚到 253.3（`scroll-top`，两边位置一模一样；小数让行落在像素中间）
    for e in [&mut a, &mut b] {
        e.eval("getCurrentPages()[0].setData({top: 253.3})").unwrap();
    }
    for _ in 0..6 {
        step(&mut a, &mut b, "滚到 253.3", &mut kinds);
    }
    assert!(a.scroll_view_offset("scroll-view_0_44").unwrap_or(0.0) > 250.0, "scroll-top 没生效：{:?}", a.scroll_view_offset("scroll-view_0_44"));
    kinds.clear();
    // 列表里的改动（改字、角标 wx:if）：只比逐字节一致 —— 滚过的 scroll-view 里布局坐标没扣偏移，失效分析
    // 重画它整个盒子（这一页它占视口六成，超过局部重画的面积上限，出整帧也对）
    let inside = [
        "getCurrentPages()[0].setData({'rows[6].id': 77})",
        "getCurrentPages()[0].setData({'rows[7].n': 2, total: 2})",
        "getCurrentPages()[0].setData({'rows[9].n': 1, 'rows[7].n': 0, total: 1})",
    ];
    // 列表外那排分类的选中条（孩子 2 -> 1 / 1 -> 2）：要局部
    let outside = ["getCurrentPages()[0].setData({active: 2})", "getCurrentPages()[0].setData({active: 3})"];
    for js in inside.iter().chain(&outside) {
        kinds.clear();
        a.eval(js).unwrap();
        b.eval(js).unwrap();
        for _ in 0..3 {
            step(&mut a, &mut b, js, &mut kinds);
        }
        if outside.contains(js) {
            assert!(!kinds.iter().any(|k| matches!(k, FrameKind::Full(_))), "{js}：增删一个孩子不该整帧：{kinds:?}");
        }
    }
}

