//! setData 的增量布局（renderer/wxml_renderer/layout_incremental.rs）：
//! 1. 每次增量重建都与整棵重建逐节点一致（`MINI_LAYOUT_VERIFY`，[`layout_verify_failures`]）；
//! 2. 局部重画的像素与整帧重画逐字节一致（同一串输入喂给开 / 关局部重画的两个引擎）；
//! 3. 真的留下了没变的子树（空转守卫）；
//! 4. 布局树里没有漏拿掉的节点（一共几个 == 从根能走到几个，连续改几十次也不涨）。
//!
//! 页面里什么都放一点：普通列表、带 `:nth-child` 的行、会换行的长文字、绝对定位、swiper、scroll-view、
//! 图片、`wx:if` 开关、fixed 悬浮条（覆盖层会改缓存树上它的样式）、带 transform 的块。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::{EngineOptions, MiniEngine};
use mini_render::renderer::wxml_renderer::layout_verify_failures;
use std::path::PathBuf;

fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("incremental-layout");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="hd">标题 {{title}}</view>
<view class="list">
  <view wx:for="{{rows}}" wx:key="*this" class="row"><text class="t">第 {{item}} 行</text><view class="badge">{{item % 3}}</view></view>
</view>
<view class="long"><text>{{long}}</text></view>
<view class="box"><view class="abs">角标 {{n}}</view><view class="inner">盒子里 {{n * 2}}</view></view>
<swiper class="sw" indicator-dots="{{true}}"><swiper-item wx:for="{{[1,2,3]}}" wx:key="*this"><view class="si">第 {{item}} 页</view></swiper-item></swiper>
<scroll-view scroll-y class="sv"><view wx:for="{{rows}}" wx:key="*this" class="svi">滚 {{item}}</view></scroll-view>
<view wx:if="{{show}}" class="toggle">开关里的 {{n}}</view>
<view class="tf" style="transform: translateX({{n}}px);">平移 {{n}}</view>
<view class="plain"><view wx:for="{{items}}" wx:key="*this" class="it"><text>条目 {{item}}</text><image class="thumb" src="/none.png" mode="aspectFill"></image></view></view>
<view class="form"><input class="ipt" value="{{v}}" placeholder="写点什么" /><picker range="{{['甲','乙','丙']}}" value="{{n % 3}}"><view>选 {{n % 3}}</view></picker><switch checked="{{show}}" /></view>
<view class="tail">底部 {{n}}</view>
<view class="bar">悬浮 {{n}}</view>"#;
    let wxss = ".hd { height: 44px; background-color: #fff; font-size: 18px; }
.row { display: flex; flex-direction: row; height: 40px; padding: 0 12px; border-bottom: 1px solid #eee; }
.row:nth-child(2n) { background-color: #fafafa; }
.t { flex: 1; } .badge { width: 24px; height: 24px; border-radius: 12px; background-color: #07c160; color: #fff; }
.long { padding: 8px 12px; } .box { position: relative; margin: 10px; padding: 12px; background-color: #f0f4ff; }
.abs { position: absolute; right: 0; top: 0; background-color: #fa5151; color: #fff; font-size: 10px; }
.sw { height: 80px; } .si { height: 80px; background-color: #ddeeff; }
.sv { height: 120px; } .svi { height: 30px; }
.toggle { height: 36px; background-color: #ffeedd; } .tf { width: 120px; height: 30px; background-color: #e0e0ff; }
.tail { height: 300px; }
.it { height: 28px; flex-direction: row; } .thumb { width: 24px; height: 24px; } .ipt { height: 36px; }
.bar { position: fixed; left: 0; right: 0; bottom: 0; height: 50px; background-color: #333; color: #fff; }";
    let js = "Page({ data: { title: 'A', n: 0, show: true, long: '一段会换行的长文字，'.repeat(6), rows: [1,2,3,4,5,6], items: ['a','b','c','d'], v: '' } });";
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
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0, env, EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e
}

#[test]
fn 增量布局与整棵重建一致_不漏节点_像素逐字节相同() {
    // 校验开关要在第一次读之前设好（OnceLock）；这个测试文件里只有这一个测试
    std::env::set_var("MINI_LAYOUT_VERIFY", "1");
    let root = fixture();
    mini_render::renderer::components::reset_shared_component_state();
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut t = 1u64;
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str| {
        t += 16;
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
    step(&mut a, &mut b, "首屏");
    step(&mut a, &mut b, "第二帧");
    let baseline = a.layout_node_counts();
    assert_eq!(baseline.0, baseline.1, "整棵建完就有没挂在树上的节点：{baseline:?}");
    let mut reused = 0;
    let edits = [
        "__currentPage.setData({ n: 1 })",
        "__currentPage.setData({ title: 'B' })",
        "__currentPage.setData({ n: 2, show: false })",
        "__currentPage.setData({ long: '短的' })",
        "__currentPage.setData({ show: true, n: 3 })",
        "__currentPage.setData({ rows: [1,2,3,4,5,6,7] })",
        "__currentPage.setData({ 'rows[0]': 9 })",
        "__currentPage.setData({ long: '又一段会换行的长文字，'.repeat(8) })",
        "__currentPage.setData({ rows: [1,2,3] })",
        "__currentPage.setData({ items: ['a','b','c','d','e'] })",
        "__currentPage.setData({ items: ['a','b','d','e'] })",
        "__currentPage.setData({ items: ['z','a','b','d','e'] })",
        "__currentPage.setData({ v: '输入了字' })",
        "__currentPage.setData({ items: ['a','b','c','d'], v: '' })",
    ];
    for round in 0..4 {
        for (i, js) in edits.iter().enumerate() {
            a.eval(js).unwrap();
            b.eval(js).unwrap();
            step(&mut a, &mut b, &format!("第 {round} 轮 第 {i} 次：{js}"));
            reused += a.layout_reuse().0;
            let (total, reach) = a.layout_node_counts();
            assert_eq!(total, reach, "第 {round} 轮 第 {i} 次之后树里有 {} 个节点没挂在根上", total - reach);
        }
        // 中途滚一下（覆盖层与条带都会重画）
        a.scroll_to(120.0 * (round + 1) as f32);
        b.scroll_to(120.0 * (round + 1) as f32);
        step(&mut a, &mut b, "滚动");
    }
    assert_eq!(layout_verify_failures(), 0, "增量布局与整棵重建不一致（看上面 ❌ 那几行）");
    // 空转守卫：真的走了增量（每次改一处，页面上大半子树都该留着）
    assert!(reused >= 120, "增量重建几乎没留下子树（{reused}）：可能一直退回整棵重建");
}
