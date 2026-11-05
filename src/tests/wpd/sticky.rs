//! D1 吸顶：`position: sticky`（页面 / scroll-view）、sticky-header / sticky-section。
//! 判据：吸住的位置（像素）、被推走、事件 detail 的类型、点击落在吸住的元素上；两台引擎（局部 / 整帧）逐字节相同；
//! 页面滚动时没有整帧，吸住之前只合成（一个字节都不多画）。
use super::*;

const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];
const GRAY: [u8; 3] = [0xcc, 0xcc, 0xcc];

const LIST_JS: &str = "Page({ data: { items: Array.from({ length: 40 }, (_, i) => i) },
  onBar() { getApp().log.push('bar'); }, onItem() { getApp().log.push('item'); } });";

#[test]
fn d1_页面吸顶_吸住之前只合成_吸住后只重画那一块_点得到() {
    let root = fixture(
        "page-sticky",
        r#"<view class="head"></view><view class="bar" bindtap="onBar"></view>
<view wx:for="{{items}}" wx:key="*this" class="item {{index % 2 ? 'b' : 'g'}}" bindtap="onItem"></view>"#,
        "page { background: #ffffff; } .head { height: 100px; background: #cccccc; }
.bar { position: sticky; top: 0; height: 40px; background: #ff0000; }
.item { height: 50px; } .b { background: #0000ff; } .g { background: #00ff00; }",
        LIST_JS,
        &[],
    );
    let mut p = Pair::new(&root);
    assert_eq!(p.rgb(10, 50), GRAY);
    assert_eq!(p.rgb(10, 120), RED, "没滚时在原位（head 下面）");
    p.kinds.clear();
    // 吸住之前：随内容走，页面画布一个像素不用动
    for k in 1..=16 {
        p.scroll(k as f32 * 6.0);
    }
    // （Band 是滚动本来就有的提前补条带，与吸顶无关）
    assert_eq!(p.count(FrameKind::Compose) + p.count(FrameKind::Band), 16, "吸住之前只合成：{:?}", p.kinds);
    assert!(p.count(FrameKind::Compose) >= 12, "{:?}", p.kinds);
    assert_eq!(p.rgb(10, 120 - 96), RED);
    // 吸住之后：每帧只重画它上次与这次那两块，不退回整条带 / 整帧
    p.kinds.clear();
    for k in 17..=100 {
        p.scroll(k as f32 * 6.0);
    }
    assert_eq!(p.fulls(), 0, "吸顶滚动不许整帧：{:?}", p.kinds);
    let damage = p.count(FrameKind::Damage);
    assert!(damage >= 60, "吸住后的滚动帧是局部重画（{damage} 帧），{:?}", p.kinds);
    assert_eq!(p.rgb(10, 5), RED, "滚到 600：吸在视窗顶上");
    assert_eq!(p.rgb(10, 39), RED);
    let below = p.rgb(10, 45);
    assert!(below == GREEN || below == BLUE, "吸顶条下面是滚上来的列表：{below:?}");
    // 点吸住的条：点的是它，不是压在底下的列表项
    p.a.pointer_down(100.0, 20.0);
    p.a.pump(p.t + 16);
    p.a.pointer_up(100.0, 20.0);
    p.a.pump(p.t + 48);
    p.a.pump(p.t + 96);
    assert_eq!(p.log(), vec!["bar".to_string()], "点到吸住的条");
}

#[test]
fn d1_吸住时_set_data_改它里面的内容_吸住那份也重画() {
    let root = fixture(
        "page-sticky-setdata",
        r#"<view class="head"></view><view class="bar"><view class="dot {{on ? 'on' : ''}}"></view></view>
<view wx:for="{{items}}" wx:key="*this" class="item {{index % 2 ? 'b' : 'g'}}"></view>"#,
        ".head { height: 100px; background: #cccccc; } .bar { position: sticky; top: 0; height: 40px; background: #ff0000; }
.dot { width: 20px; height: 20px; background: #ffffff; } .on { background: #000000; }
.item { height: 50px; } .b { background: #0000ff; } .g { background: #00ff00; }",
        "Page({ data: { on: false, items: Array.from({ length: 40 }, (_, i) => i) } });",
        &[],
    );
    let mut p = Pair::new(&root);
    for k in 1..=60 {
        p.scroll(k as f32 * 5.0);
    }
    assert_eq!(p.rgb(5, 5), [255, 255, 255]);
    p.kinds.clear();
    p.set("{ on: true }");
    p.step("停");
    assert_eq!(p.rgb(5, 5), [0, 0, 0], "吸住那份跟着变");
    assert_eq!(p.fulls(), 0, "{:?}", p.kinds);
}

#[test]
fn d1_被父容器底边推走_bottom_吸底() {
    let root = fixture(
        "page-sticky-push",
        r#"<view class="sec"><view class="bar"></view><view class="fill"></view></view><view class="sec2"></view>
<view class="foot"></view><view class="tail"></view>"#,
        "page { background: #ffffff; } .sec { height: 400px; } .bar { position: sticky; top: 0; height: 40px; background: #ff0000; }
.fill { height: 360px; background: #cccccc; } .sec2 { height: 2000px; background: #0000ff; }
.foot { position: -webkit-sticky; bottom: 0; height: 40px; background: #00ff00; } .tail { height: 300px; background: #cccccc; }",
        "Page({});",
        &[],
    );
    let mut p = Pair::new(&root);
    assert_eq!(p.rgb(10, 650), GREEN, "吸底：原位在 2400，先钉在视窗底边");
    assert_eq!(p.rgb(10, 620), BLUE);
    p.kinds.clear();
    for k in 1..=50 {
        p.scroll(k as f32 * 4.0);
    }
    assert_eq!(p.rgb(10, 5), RED, "滚到 200：吸住");
    assert_eq!(p.rgb(10, 45), GRAY);
    for k in 1..=45 {
        p.scroll(200.0 + k as f32 * 4.0);
    }
    // 滚到 380：本节底边在 400，条被推到 360..400（屏上 -20..20）
    assert_eq!(p.rgb(10, 5), RED, "被推走一半");
    assert_eq!(p.rgb(10, 25), BLUE, "推走的那截下面是下一节");
    for k in 1..=10 {
        p.scroll(380.0 + k as f32 * 4.0);
    }
    assert_eq!(p.rgb(10, 5), BLUE, "整个推出去了");
    assert_eq!(p.fulls(), 0, "{:?}", p.kinds);
    // 滚到底：吸底的回到原位（2400，屏上 2400 - max）
    let max = 400.0 + 2000.0 + 40.0 + 300.0 - 667.0;
    p.scroll(max);
    for _ in 0..3 {
        p.step("停住");
    }
    let foot_y = (2400.0 - max) as u32;
    assert_eq!(p.rgb(10, foot_y + 20), GREEN, "到底了在原位");
    assert_eq!(p.rgb(10, foot_y + 60), GRAY);
}

/// scroll-view 里三节 sticky-section，每节一个 sticky-header（30px）+ 5 行（40px）；`wrap`：套一层 view（走分块缓存），否则是顶层 scroll-view（离屏缓存）
fn sections(name: &str, wrap: bool) -> Pair {
    let sv = r#"<scroll-view scroll-y type="custom" class="sv" scroll-top="{{top}}">
  <sticky-section wx:for="{{secs}}" wx:key="*this">
    <sticky-header bindstickontopchange="onStick" bindtap="onHead" data-i="{{index}}"><view class="h h{{index}}"></view></sticky-header>
    <view wx:for="{{rows}}" wx:for-item="r" wx:key="*this" class="row" bindtap="onRow"></view>
  </sticky-section>
</scroll-view>"#;
    let wxml = if wrap { format!("<view class=\"wrap\">{sv}</view>") } else { sv.to_string() };
    let root = fixture(
        name,
        &wxml,
        "page { background: #ffffff; } .sv { height: 300px; background: #ffffff; } .h { height: 30px; }
.h0 { background: #ff0000; } .h1 { background: #00ff00; } .h2 { background: #0000ff; } .row { height: 40px; background: #cccccc; }",
        "Page({ data: { top: 0, secs: [0, 1, 2], rows: [0, 1, 2, 3, 4] },
  onStick(e) { getApp().log.push(e.currentTarget.dataset.i + ':' + typeof e.detail.isStickOnTop + ':' + e.detail.isStickOnTop); },
  onHead(e) { getApp().log.push('head' + e.currentTarget.dataset.i); }, onRow() { getApp().log.push('row'); } });",
        &[],
    );
    Pair::new(&root)
}

fn check_sections(p: &mut Pair) {
    assert_eq!(p.rgb(10, 5), RED);
    assert_eq!(p.rgb(10, 235), GREEN, "第二节的头在 230");
    assert!(p.log().is_empty(), "没吸住不发：{:?}", p.log());
    p.set("{ top: 100 }");
    p.step("停");
    assert_eq!(p.rgb(10, 5), RED, "第一节的头吸在顶上");
    assert_eq!(p.rgb(10, 35), GRAY);
    assert_eq!(p.rgb(10, 135), GREEN, "第二节的头随内容走（230 - 100）");
    assert_eq!(p.log(), vec!["0:boolean:true".to_string()]);
    p.set("{ top: 215 }");
    p.step("停");
    // 第一节底边 230：头被推到 200..230（屏上 -15..15），第二节的头在 15..45
    assert_eq!(p.rgb(10, 5), RED, "推走一半");
    assert_eq!(p.rgb(10, 20), GREEN);
    p.set("{ top: 300 }");
    p.step("停");
    assert_eq!(p.rgb(10, 5), GREEN, "第二节的头吸住");
    assert_eq!(p.rgb(10, 35), GRAY);
    assert_eq!(p.log(), vec!["0:boolean:true", "0:boolean:false", "1:boolean:true"], "状态变了才发、isStickOnTop 是布尔");
    // 点吸住的头：是头（第二节），不是压在底下的行
    p.both(|e| e.pointer_down(100.0, 10.0));
    p.step("按下");
    p.both(|e| e.pointer_up(100.0, 10.0));
    p.step("抬起");
    p.step("派发");
    assert_eq!(p.log().last().map(String::as_str), Some("head1"), "{:?}", p.log());
    assert!(!p.log().iter().any(|s| s == "row"), "{:?}", p.log());
    // 拖动：每帧逐字节一致、不整帧
    p.kinds.clear();
    p.drag(100.0, 250.0, 120.0, 10, 30);
    p.drag(100.0, 120.0, 260.0, 10, 30);
    assert_eq!(p.fulls(), 0, "{:?}", p.kinds);
}

#[test]
fn d1_scroll_view_里的_sticky_header_吸顶推走事件_分块缓存() {
    let mut p = sections("sv-sections-tiles", true);
    check_sections(&mut p);
}

#[test]
fn d1_scroll_view_里的_sticky_header_吸顶推走事件_顶层离屏缓存() {
    let mut p = sections("sv-sections-root", false);
    check_sections(&mut p);
}

#[test]
fn d1_sticky_header_不许重叠时叠成一摞_允许时盖住() {
    let root = fixture(
        "sv-stack",
        r#"<view><scroll-view scroll-y class="sv" scroll-top="{{top}}">
  <sticky-header><view class="h h0"></view></sticky-header>
  <view wx:for="{{[0,1,2]}}" wx:key="*this" class="row"></view>
  <sticky-header allow-overlapping="{{ov}}" offset-top="0"><view class="h h1"></view></sticky-header>
  <view wx:for="{{rows}}" wx:key="*this" class="row"></view>
</scroll-view></view>"#,
        ".sv { height: 300px; background: #ffffff; } .h { height: 30px; } .h0 { background: #ff0000; } .h1 { background: #00ff00; }
.row { height: 40px; background: #cccccc; }",
        "Page({ data: { top: 0, ov: false, rows: Array.from({ length: 20 }, (_, i) => i) } });",
        &[],
    );
    let mut p = Pair::new(&root);
    p.set("{ top: 200 }");
    assert_eq!(p.rgb(10, 15), RED, "第一个吸在 0");
    assert_eq!(p.rgb(10, 45), GREEN, "第二个吸在第一个下边（30）");
    p.set("{ ov: true }");
    p.step("停");
    assert_eq!(p.rgb(10, 15), GREEN, "允许重叠：吸在 offset-top 处，盖住第一个");
    assert_eq!(p.rgb(10, 45), GRAY);
}

#[test]
fn d1_offset_top_与_css_top_两节都不推时叠成一摞() {
    // 页面：CSS top: 10px —— 吸在离视窗顶 10px 处，上面那 10px 露出底下滚上来的内容
    let root = fixture(
        "page-sticky-top10",
        r#"<view class="head"></view><view class="bar"></view><view class="tall"></view>"#,
        "page { background: #ffffff; } .head { height: 100px; background: #cccccc; }
.bar { position: sticky; top: 10px; height: 40px; background: #ff0000; } .tall { height: 2000px; background: #0000ff; }",
        "Page({});",
        &[],
    );
    let mut p = Pair::new(&root);
    p.kinds.clear();
    for k in 1..=40 {
        p.scroll(k as f32 * 5.0);
    }
    assert_eq!(p.rgb(10, 5), BLUE, "top: 10px 上面是滚上来的内容");
    assert_eq!(p.rgb(10, 11), RED);
    assert_eq!(p.rgb(10, 49), RED);
    assert_eq!(p.rgb(10, 51), BLUE);
    assert_eq!(p.fulls(), 0, "{:?}", p.kinds);
    drop(p);
    // scroll-view：offset-top=20，两节 push-pinned-header=false：第一节的头一直吸在 20，第二节的头吸在它下边（50）
    let root = fixture(
        "sv-offset-nopush",
        r#"<view><scroll-view scroll-y type="custom" class="sv" scroll-top="{{top}}">
  <sticky-section wx:for="{{[0, 1]}}" wx:key="*this" push-pinned-header="{{false}}">
    <sticky-header offset-top="20" bindstickontopchange="onStick" data-i="{{index}}"><view class="h h{{index}}"></view></sticky-header>
    <view wx:for="{{rows}}" wx:for-item="r" wx:key="*this" class="row"></view>
  </sticky-section>
</scroll-view></view>"#,
        ".sv { height: 300px; background: #ffffff; } .h { height: 30px; } .h0 { background: #ff0000; } .h1 { background: #00ff00; }
.row { height: 40px; background: #cccccc; }",
        "Page({ data: { top: 0, rows: Array.from({ length: 10 }, (_, i) => i) },
  onStick(e) { getApp().log.push(e.currentTarget.dataset.i + ':' + e.detail.isStickOnTop); } });",
        &[],
    );
    let mut p = Pair::new(&root);
    p.set("{ top: 100 }");
    assert_eq!(p.rgb(10, 10), GRAY, "offset-top 上面是行");
    assert_eq!(p.rgb(10, 21), RED, "第一节的头吸在 20");
    assert_eq!(p.rgb(10, 49), RED);
    assert_eq!(p.rgb(10, 55), GRAY);
    // 第二节的头原位 430：滚到 500 时本该在 -70，叠在第一节的头下边（50..80）
    p.set("{ top: 500 }");
    assert_eq!(p.rgb(10, 21), RED, "不推：第一节的头还吸着");
    assert_eq!(p.rgb(10, 51), GREEN, "第二节的头叠在下边");
    assert_eq!(p.rgb(10, 79), GREEN);
    assert_eq!(p.rgb(10, 85), GRAY);
    assert_eq!(p.log(), vec!["0:true", "1:true"]);
}

#[test]
fn d1_没写_top_的_sticky_不吸_sticky_section_padding() {
    let root = fixture(
        "sticky-none",
        r#"<view class="head"></view><view class="bar"></view><view class="tall"></view>
<view><scroll-view scroll-y class="sv"><sticky-section padding="{{[10, 0, 0, 20]}}"><sticky-header padding="{{[4, 4, 4, 4]}}"><view class="h"></view></sticky-header></sticky-section></scroll-view></view>"#,
        ".head { height: 100px; background: #cccccc; } .bar { position: sticky; height: 40px; background: #ff0000; } .tall { height: 2000px; }
.sv { height: 100px; } .h { height: 30px; background: #00ff00; }",
        "Page({});",
        &[],
    );
    let mut p = Pair::new(&root);
    p.scroll(300.0);
    assert_ne!(p.rgb(10, 5), RED, "没写 top / bottom 的 sticky 不吸（同浏览器）");
    // padding 属性：section 左 20 上 10，header 再 4 → 绿块从 (24, 2214) 起（页面坐标）
    // 内容 2240 高，滚到底（1573）：scroll-view 在屏上 567 处
    p.scroll(2240.0 - 667.0);
    let sv_top = (2140.0 - (2240.0 - 667.0)) as u32;
    assert_ne!(p.rgb(22, sv_top + 16), GREEN, "左边 padding 里没有绿");
    assert_eq!(p.rgb(26, sv_top + 16), GREEN);
    assert_ne!(p.rgb(30, sv_top + 12), GREEN, "上边 padding（10 + 4）里没有绿");
}

/// 吸顶页面滚动每帧的代价（SDK，375×667 @3x，CPU）：同一页 `position: sticky` 与 `relative` 各滚 150 帧（每帧 6px），
/// 打出每帧 pump 的中位 / 最慢与帧种类。数字记在 plan.md 的 WP-D 一节。`MINI_DAMAGE_LOG=1` 另有逐帧的 🩹 行（重画面积 + 耗时）。
/// 跑法：`cargo test --release --lib d1_bench -- --ignored --nocapture`
#[test]
#[ignore]
fn d1_bench_吸顶滚动每帧代价() {
    let wxml = r#"<view class="head">头图</view>
<view class="tabs"><view class="tab" wx:for="{{tabs}}" wx:key="*this">{{item}}</view></view>
<view wx:for="{{items}}" wx:key="*this" class="item"><view class="pic"></view><view class="txt">第 {{item}} 条 · 吸顶列表的内容行</view></view>"#;
    let js = "Page({ data: { tabs: ['推荐','热门','附近','关注'], items: Array.from({ length: 80 }, (_, i) => i) } });";
    let _g = crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    for pos in ["sticky", "relative"] {
        let css = format!(
            "page {{ background: #f5f5f5; }} .head {{ height: 200px; background: linear-gradient(135deg, #ff7a45, #ffc53d); color: #fff; font-size: 24px; }}
.tabs {{ position: {pos}; top: 0; display: flex; height: 44px; background: #ffffff; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
.tab {{ flex: 1; text-align: center; line-height: 44px; font-size: 15px; }}
.item {{ display: flex; height: 72px; margin: 8px 12px; background: #fff; border-radius: 8px; }}
.pic {{ width: 56px; height: 56px; margin: 8px; background: #91d5ff; border-radius: 6px; }}
.txt {{ flex: 1; line-height: 72px; font-size: 14px; color: #333; }}"
        );
        let root = fixture(&format!("bench-{pos}"), wxml, &css, js, &[]);
        let env = HostEnv { damage_log: std::env::var_os("MINI_DAMAGE_LOG").is_some(), ..HostEnv::default() };
        let data = root.join("_data");
        let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 3.0, env, EngineOptions::default()).unwrap();
        e.set_animation_clock(0.0);
        e.launch(None).unwrap();
        let mut t = 0u64;
        for _ in 0..3 {
            t += 16;
            e.pump(t);
        }
        let mut ms = Vec::new();
        let mut kinds = std::collections::BTreeMap::new();
        for k in 1..=150 {
            e.scroll_to(k as f32 * 6.0);
            t += 16;
            let t0 = std::time::Instant::now();
            e.pump(t);
            ms.push(t0.elapsed().as_secs_f32() * 1000.0);
            *kinds.entry(format!("{:?}", e.last_frame_kind())).or_insert(0) += 1;
        }
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!(
            "📊 {pos}: 150 帧 pump 中位 {:.2}ms  p90 {:.2}ms  最慢 {:.2}ms  帧种类 {kinds:?}",
            ms[75],
            ms[135],
            ms[149]
        );
    }
}
