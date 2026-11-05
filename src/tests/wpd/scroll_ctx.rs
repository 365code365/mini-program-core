//! D3 ScrollViewContext（可写属性、scrollTo、scrollIntoView 的 alignment、triggerRefresh / closeRefresh）与
//! D2 scroll-view 属性（bounces、show-scrollbar、paging-enabled、dragend 的 velocity、scroll-anchoring、reverse、
//! enable-back-to-top 自动开、refresherstatuschange）。判据：滚动位置数值、事件 detail 的类型、像素。
use super::*;

/// 300px 高的 scroll-view，20 行 × 100px（`r0`~`r19`，带 wx:key），行色交替；`attrs` 加在 scroll-view 上
fn list(name: &str, attrs: &str, js_extra: &str) -> Pair {
    let wxml = format!(
        r#"<view><scroll-view id="sv" scroll-y class="sv" {attrs}>
  <view wx:for="{{{{rows}}}}" wx:key="*this" id="r{{{{item}}}}" class="row {{{{item % 2 ? 'b' : 'g'}}}}"></view>
</scroll-view></view>"#
    );
    let js = format!(
        "Page({{ data: {{ rows: Array.from({{ length: 20 }}, (_, i) => i) }},
  onLoad() {{ wx.createSelectorQuery().select('#sv').node().exec(res => {{ globalThis.__ctx = res[0].node; }}); }},
  {js_extra} }});"
    );
    let root = fixture(
        name,
        &wxml,
        "page { background: #ffffff; } .sv { height: 300px; background: #ffffff; } .row { height: 100px; } .b { background: #0000ff; } .g { background: #00ff00; }",
        &js,
        &[],
    );
    let mut p = Pair::new(&root);
    for _ in 0..3 {
        p.step("拿 node");
    }
    p
}

fn sv(p: &Pair) -> f32 {
    p.a.scroll_view_offset("sv").unwrap_or(-1.0)
}

fn js(p: &mut Pair, code: &str) -> String {
    let out = p.a.eval(code).unwrap_or_default();
    p.b.eval(code).ok();
    out
}

fn settle(p: &mut Pair, n: usize) {
    for i in 0..n {
        p.step(&format!("等 {i}"));
    }
}

#[test]
fn d3_可写属性_scroll_enabled_paging_bounces_读回是写的值() {
    let mut p = list("ctx-props", "enhanced", "");
    assert_eq!(js(&mut p, "typeof __ctx.scrollEnabled + ':' + __ctx.scrollEnabled"), "boolean:true");
    js(&mut p, "__ctx.scrollEnabled = false");
    settle(&mut p, 2);
    assert_eq!(js(&mut p, "String(__ctx.scrollEnabled)"), "false");
    p.drag(100.0, 250.0, 100.0, 8, 30);
    assert_eq!(sv(&p), 0.0, "scrollEnabled = false：拖不动");
    js(&mut p, "__ctx.scrollEnabled = true");
    settle(&mut p, 2);
    p.drag(100.0, 250.0, 150.0, 8, 30);
    assert!(sv(&p) > 50.0, "打开后拖得动：{}", sv(&p));
    // 分页：一页 = 视口高 300
    js(&mut p, "__ctx.scrollTo({ top: 0, animated: false }); __ctx.pagingEnabled = true");
    settle(&mut p, 2);
    p.drag(100.0, 250.0, 130.0, 40, 60);
    assert_eq!(sv(&p), 0.0, "慢拖 120（不到半页）回到第 0 页");
    p.drag(100.0, 280.0, 100.0, 60, 60);
    assert_eq!(sv(&p), 300.0, "慢拖 180（过半页）到第 1 页");
    // bounces = false：在顶上往下拉不出橡皮筋
    js(&mut p, "__ctx.pagingEnabled = false; __ctx.scrollTo({ top: 0, animated: false }); __ctx.bounces = false");
    settle(&mut p, 2);
    p.both(|e| e.pointer_down(100.0, 50.0));
    p.step("按下");
    for k in 1..=8 {
        p.both(|e| e.pointer_move(100.0, 50.0 + k as f32 * 20.0));
        p.step("往下拉");
        assert!(sv(&p) >= 0.0, "bounces=false 不越界：第 {k} 步 {}", sv(&p));
    }
    p.both(|e| e.pointer_up(100.0, 210.0));
    settle(&mut p, 20);
}

#[test]
fn d3_scroll_to_与_scroll_into_view_的_alignment() {
    let mut p = list("ctx-into", "enhanced", "");
    js(&mut p, "__ctx.scrollTo({ top: 500, animated: false })");
    settle(&mut p, 2);
    assert_eq!(sv(&p), 500.0);
    js(&mut p, "__ctx.scrollIntoView('#r10', { alignment: 'center', animated: false })");
    settle(&mut p, 2);
    assert_eq!(sv(&p), 900.0, "center：1000 - (300 - 100) / 2");
    js(&mut p, "__ctx.scrollIntoView('#r12', { alignment: 'end', animated: false })");
    settle(&mut p, 2);
    assert_eq!(sv(&p), 1000.0, "end：1200 - (300 - 100)");
    js(&mut p, "__ctx.scrollIntoView('#r11', { alignment: 'nearest', animated: false })");
    settle(&mut p, 2);
    assert_eq!(sv(&p), 1000.0, "nearest：已经在视口里不动");
    js(&mut p, "__ctx.scrollIntoView('#r3', { offset: -20, animated: false })");
    settle(&mut p, 2);
    assert_eq!(sv(&p), 280.0, "默认 start + offset");
    js(&mut p, "__ctx.scrollTo({ top: 100 })");
    settle(&mut p, 40);
    assert_eq!(sv(&p), 100.0, "带动画滚过去");
}

#[test]
fn d3_trigger_refresh_close_refresh_与_refresherstatuschange() {
    let mut p = list(
        "ctx-refresh",
        r#"enhanced refresher-enabled bindrefresherrefresh="onR" bindrefresherrestore="onRs" bindrefresherstatuschange="onSt""#,
        "onR() { getApp().log.push('refresh'); }, onRs() { getApp().log.push('restore'); },
  onSt(e) { getApp().log.push('st:' + typeof e.detail.status + ':' + e.detail.status); }",
    );
    js(&mut p, "__ctx.triggerRefresh()");
    settle(&mut p, 40);
    assert!(sv(&p) < -40.0, "进刷新态：内容被按在阈值处（{}）", sv(&p));
    assert_eq!(p.log(), vec!["refresh", "st:number:2"]);
    js(&mut p, "__ctx.closeRefresh()");
    settle(&mut p, 60);
    assert!(sv(&p).abs() < 0.5, "收回：{}", sv(&p));
    assert_eq!(p.log(), vec!["refresh", "st:number:2", "st:number:3", "restore", "st:number:0"]);
}

#[test]
fn d2_dragend_的_velocity_是带方向的数字() {
    let mut p = list("dragend", r#"enhanced binddragend="onEnd""#, "onEnd(e) { getApp().log.push(typeof e.detail.velocity.y + ':' + (e.detail.velocity.y > 0)); }");
    p.drag(100.0, 280.0, 120.0, 4, 2);
    assert_eq!(p.log(), vec!["number:true"], "往上甩（scrollTop 变大）velocity.y 为正");
}

#[test]
fn d2_show_scrollbar_滚动时出现_停下渐隐只重画那一条() {
    let mut p = list("scrollbar", "enhanced", "");
    assert_eq!(p.rgb(371, 30), p.rgb(300, 30), "没滚过：没有滚动条");
    p.drag(100.0, 280.0, 180.0, 10, 0);
    let (on, beside) = (p.rgb(371, 30), p.rgb(300, 30));
    // 行色上叠 35% 黑
    assert!(on != beside && on.iter().zip(beside.iter()).all(|(a, b)| a <= b), "滚着的时候右边有滚动条：{on:?} 旁边 {beside:?}");
    settle(&mut p, 10);
    p.kinds.clear();
    settle(&mut p, 120);
    assert_eq!(p.fulls(), 0, "渐隐不整帧：{:?}", p.kinds);
    assert!(p.count(FrameKind::Damage) > 10, "渐隐那几帧是局部重画：{:?}", p.kinds);
    let off = p.rgb(371, 30);
    assert_eq!(off, p.rgb(300, 30), "停下 1s 后滚动条没了");
    // 包根写锁同一时间只能有一对引擎拿着：先放掉这一对
    drop(p);
    // 没开 enhanced：不画
    let mut q = list("scrollbar-off", "", "");
    q.drag(100.0, 280.0, 180.0, 10, 0);
    assert_eq!(q.rgb(371, 30), q.rgb(300, 30), "没开 enhanced 不画滚动条");
}

#[test]
fn d2_scroll_anchoring_上方插入内容可见内容不跳_关掉照旧() {
    for (attrs, keep) in [("", true), (r#"scroll-anchoring="{{false}}""#, false)] {
        let mut p = list(if keep { "anchor-on" } else { "anchor-off" }, attrs, "");
        js(&mut p, "__ctx.scrollTo({ top: 550, animated: false })");
        settle(&mut p, 2);
        let before = p.rgb(10, 10);
        p.set("{ rows: [-3, -2, -1].concat(Array.from({ length: 20 }, (_, i) => i)) }");
        settle(&mut p, 2);
        if keep {
            assert_eq!(sv(&p), 850.0, "上方插了 3 行（300px）：位置跟着挪");
            assert_eq!(p.rgb(10, 10), before, "视口里的内容不跳");
        } else {
            assert_eq!(sv(&p), 550.0, "scroll-anchoring=false：位置不动");
        }
    }
}

#[test]
fn d2_reverse_初始在底_内容变长贴底() {
    let mut p = list("reverse", "reverse", "");
    assert_eq!(sv(&p), 1700.0, "初始在末尾");
    p.set("{ rows: Array.from({ length: 22 }, (_, i) => i) }");
    settle(&mut p, 2);
    assert_eq!(sv(&p), 1900.0, "来了两条：接着贴底");
}

#[test]
fn d2_enable_back_to_top_占屏九成自动开_显式_false_不开() {
    for (attrs, on) in [("", true), (r#"enable-back-to-top="{{false}}""#, false)] {
        let wxml = format!(r#"<scroll-view id="sv" scroll-y class="sv" {attrs}><view class="tall"></view></scroll-view>"#);
        let root = fixture(if on { "btt-on" } else { "btt-off" }, &wxml, ".sv { height: 667px; } .tall { height: 3000px; }", "Page({});", &[]);
        let mut p = Pair::new(&root);
        js(&mut p, "wx.createSelectorQuery().select('#sv').node().exec(r => r[0].node.scrollTo({ top: 400, animated: false }))");
        settle(&mut p, 3);
        assert_eq!(sv(&p), 400.0);
        assert_eq!(p.a.tap_status_bar(), on, "占满一屏的 scroll-view {attrs}");
    }
}

#[test]
fn d2_type_nested_上推先滚外层的头_再滚里层_下拉先滚里层() {
    let root = fixture(
        "nested",
        r#"<scroll-view id="outer" type="nested" scroll-y class="outer">
  <nested-scroll-header><view class="head"></view></nested-scroll-header>
  <nested-scroll-body><scroll-view id="inner" scroll-y class="inner"><view wx:for="{{rows}}" wx:key="*this" class="row"></view></scroll-view></nested-scroll-body>
</scroll-view>"#,
        ".outer { height: 400px; } .head { height: 200px; background: #ff0000; } .inner { height: 400px; } .row { height: 100px; background: #cccccc; }",
        "Page({ data: { rows: Array.from({ length: 20 }, (_, i) => i) } });",
        &[],
    );
    let mut p = Pair::new(&root);
    p.kinds.clear();
    let at = |p: &Pair, id: &str| p.a.scroll_view_offset(id).unwrap_or(-1.0);
    // 手指落在里层（屏上 200..400）上推 120：头还没滚完，先滚外层
    p.drag(100.0, 350.0, 230.0, 40, 60);
    assert_eq!(at(&p, "inner"), 0.0, "外层的头没滚完之前里层不动");
    assert!(at(&p, "outer") > 100.0 && at(&p, "outer") <= 200.0, "外层：{}", at(&p, "outer"));
    // 再上推 200：外层滚到底（200）之后接着滚里层
    p.drag(100.0, 350.0, 150.0, 60, 60);
    assert_eq!(at(&p, "outer"), 200.0, "头滚完了");
    let inner = at(&p, "inner");
    assert!(inner > 50.0, "剩下的交给里层：{inner}");
    // 下拉 280：先把里层拉回顶，再拉外层
    p.drag(100.0, 100.0, 380.0, 90, 60);
    assert_eq!(at(&p, "inner"), 0.0, "里层先回到顶");
    assert!(at(&p, "outer") < 200.0, "然后外层往下：{}", at(&p, "outer"));
    assert_eq!(p.fulls(), 0, "里外层换手不整帧：{:?}", p.kinds);
}

#[test]
fn d2_下拉二级_拉过阈值打开_context_关闭_状态码_5_6_7_8() {
    let mut p = list(
        "two-level",
        r#"enhanced refresher-enabled refresher-two-level-enabled refresher-two-level-scroll-enabled refresher-two-level-threshold="60" bindrefresherrefresh="onR" bindrefresherstatuschange="onSt""#,
        "onR() { getApp().log.push('refresh'); }, onSt(e) { getApp().log.push('st:' + e.detail.status); }",
    );
    // 在顶上往下拉很远（橡皮筋阻尼下也要过 60）再松手：打开二级，不发 refresherrefresh
    p.drag(100.0, 20.0, 290.0, 30, 40);
    let log = p.log();
    assert!(!log.contains(&"refresh".to_string()), "二级不发 refresherrefresh：{log:?}");
    assert!(log.contains(&"st:5".to_string()) && log.contains(&"st:6".to_string()), "CanTwoLevel → TwoLevelOpening：{log:?}");
    assert_eq!(log.last().map(String::as_str), Some("st:7"), "弹簧走完是 TwoLeveling：{log:?}");
    assert!((sv(&p) + 300.0).abs() < 1.0, "内容推到盒子下面（-300）：{}", sv(&p));
    // 二级里往上推 40（< 80）松手：弹回打开的位置
    p.drag(100.0, 200.0, 160.0, 8, 40);
    assert!((sv(&p) + 300.0).abs() < 1.0, "推得不够弹回：{}", sv(&p));
    // closeTwoLevel：收回、8 → 0
    js(&mut p, "__ctx.closeTwoLevel()");
    settle(&mut p, 60);
    assert!(sv(&p).abs() < 0.5, "收回：{}", sv(&p));
    let log = p.log();
    let tail: Vec<_> = log.iter().rev().take(2).rev().cloned().collect();
    assert_eq!(tail, vec!["st:8", "st:0"], "{log:?}");
    // triggerTwoLevel：程序打开
    js(&mut p, "__ctx.triggerTwoLevel()");
    settle(&mut p, 60);
    assert!((sv(&p) + 300.0).abs() < 1.0, "triggerTwoLevel 打开：{}", sv(&p));
    // 推过 80 松手：关
    p.drag(100.0, 250.0, 120.0, 10, 60);
    assert!(sv(&p).abs() < 0.5, "推过关闭阈值松手关闭：{}", sv(&p));
}

#[test]
fn d2_min_drag_distance_没到不滚_没写照旧() {
    let mut p = list("min-drag", r#"min-drag-distance="40""#, "");
    // 慢慢移 30px 就停：没到 40，不滚
    p.both(|e| e.pointer_down(100.0, 250.0));
    p.step("按下");
    for k in 1..=6 {
        p.both(|e| e.pointer_move(100.0, 250.0 - k as f32 * 5.0));
        p.step("移");
    }
    assert_eq!(sv(&p), 0.0, "30 < 40：还没开始滚");
    for k in 1..=12 {
        p.both(|e| e.pointer_move(100.0, 220.0 - k as f32 * 10.0));
        p.step("接着移");
    }
    p.both(|e| e.pointer_up(100.0, 100.0));
    settle(&mut p, 30);
    assert!(sv(&p) > 50.0, "过了 40 照常滚：{}", sv(&p));
    // Pair 拿着包根的写锁：先放掉这一对再开下一对
    drop(p);
    // 没写：沿用宿主的 4px，移 30 已经在滚
    let mut q = list("min-drag-default", "", "");
    q.both(|e| e.pointer_down(100.0, 250.0));
    q.step("按下");
    for k in 1..=6 {
        q.both(|e| e.pointer_move(100.0, 250.0 - k as f32 * 5.0));
        q.step("移");
    }
    assert!(sv(&q) > 10.0, "没写 min-drag-distance：{}", sv(&q));
    q.both(|e| e.pointer_up(100.0, 220.0));
    settle(&mut q, 30);
}

#[test]
fn d2_bind_scroll_碰到边界时带_boundary_velocity() {
    let mut p = list(
        "boundary-v",
        r#"bindscroll="onS""#,
        "onS(e) { if (e.detail.boundaryVelocity !== undefined) getApp().log.push('bv:' + typeof e.detail.boundaryVelocity + ':' + (e.detail.boundaryVelocity < 0)); }",
    );
    // 在顶上往下拉：从 0 进到负的那一下带速度（负 = scrollTop 变小的方向）
    p.drag(100.0, 50.0, 200.0, 10, 30);
    let log = p.log();
    assert_eq!(log.first().map(String::as_str), Some("bv:number:true"), "{log:?}");
    assert_eq!(log.len(), 1, "只有碰边那一次带：{log:?}");
}

#[test]
fn d2_scroll_view_自己的属性变了只重画盒子_拉着刷新时_set_data_不整帧() {
    // 盒子小一点（视口的 0.34 以下）：失效范围再大就按整帧画（那是另一条规矩，见 engine_damage_data.rs）
    let root = fixture(
        "attr-change",
        r#"<view><scroll-view id="sv" scroll-y class="sv" refresher-enabled refresher-triggered="{{t}}" data-x="{{x}}">
  <view wx:for="{{rows}}" wx:key="*this" class="row {{item % 2 ? 'b' : 'g'}}"></view>
</scroll-view></view>"#,
        "page { background: #ffffff; } .sv { height: 160px; background: #ffffff; } .row { height: 100px; } .b { background: #0000ff; } .g { background: #00ff00; }",
        "Page({ data: { rows: Array.from({ length: 20 }, (_, i) => i), t: false, x: 0 },
  onLoad() { wx.createSelectorQuery().select('#sv').node().exec(res => { globalThis.__ctx = res[0].node; }); } });",
        &[],
    );
    let mut p = Pair::new(&root);
    settle(&mut p, 3);
    js(&mut p, "__ctx.scrollTo({ top: 250, animated: false })");
    settle(&mut p, 3);
    let before = p.fulls();
    p.set("{ x: 1 }");
    p.set("{ x: 2 }");
    assert_eq!(p.fulls(), before, "只改 scroll-view 自己的属性：重画盒子，不整帧 {:?}", p.kinds);
    // 回到顶上往下拉出刷新区，拉着的时候 setData 刷新态
    js(&mut p, "__ctx.scrollTo({ top: 0, animated: false })");
    settle(&mut p, 3);
    p.both(|e| e.pointer_down(100.0, 40.0));
    p.step("按下");
    for k in 1..=6 {
        p.both(|e| e.pointer_move(100.0, 40.0 + k as f32 * 15.0));
        p.step(&format!("往下拉 {k}"));
    }
    let before = p.fulls();
    p.set("{ t: true, x: 3 }");
    p.step("拉着 setData 之后");
    assert_eq!(p.fulls(), before, "拉着刷新时 setData：不整帧 {:?}", p.kinds);
    p.both(|e| e.pointer_up(100.0, 130.0));
    settle(&mut p, 30);
    p.set("{ t: false }");
    settle(&mut p, 30);
}
