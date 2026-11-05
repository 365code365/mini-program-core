//! scroll-view 横竖两个方向同时滚（`scroll-x` + `scroll-y`，ui/interaction/scroll_2d.rs）：
//! 斜着拖两个轴一起动、横着拖也归它（不交给页面）；`bindscroll` 带 scrollLeft / deltaX；
//! 滚到右边发 `scrolltolower`（direction=right）；`scroll-left` 属性、ScrollViewContext.scrollTo({left, top})、
//! scrollOffset 都认横向；滚过之后点格子命中的是那一格。每一帧局部重画与整帧逐字节相同，滚动不逼出整帧。
use super::*;

/// 6×6 的格子，每格 100×100，颜色 = rgb(行×40, 列×40, 200)。盒子 200×200，左上角在 (20, 80)。
/// `root` 为真时 scroll-view 就是页面根（走 draw_with_interaction 那条路），否则套在 view 里（嵌套那条路）
fn page(name: &str, root: bool) -> Pair {
    let grid = r#"<view wx:for="{{rows}}" wx:for-item="r" wx:key="*this" class="row">
    <view wx:for="{{rows}}" wx:for-item="c" wx:key="*this" class="cell" data-i="{{r}}-{{c}}" bindtap="tap"
      style="background: rgb({{r * 40}}, {{c * 40}}, 200)"></view>
  </view>"#;
    let sv = format!(
        r#"<scroll-view id="sv" scroll-x scroll-y class="sv" scroll-left="{{{{left}}}}" bindscroll="onScroll" bindscrolltolower="onLower">
  {grid}
</scroll-view>"#
    );
    let wxml = if root { sv } else { format!(r#"<view class="wrap">{sv}</view>"#) };
    let js = "Page({ data: { rows: [0, 1, 2, 3, 4, 5], left: 0 },
  onLoad() { wx.createSelectorQuery().select('#sv').node().exec(res => { globalThis.__ctx = res[0].node; }); },
  onScroll(e) { getApp().last = e.detail; },
  onLower(e) { getApp().log.push('lower:' + e.detail.direction); },
  tap(e) { getApp().log.push('tap:' + e.currentTarget.dataset.i); } });";
    let css = if root {
        "page { background: #ffffff; } .sv { margin: 80px 0 0 20px; width: 200px; height: 200px; background: #eeeeee; }"
    } else {
        "page { background: #ffffff; } .wrap { padding: 80px 0 0 20px; } .sv { width: 200px; height: 200px; background: #eeeeee; }"
    };
    let css = format!("{css} .row {{ display: flex; flex-direction: row; width: 600px; height: 100px; }} .cell {{ width: 100px; height: 100px; flex-shrink: 0; }}");
    let root_dir = fixture(name, &wxml, &css, js, &[]);
    let mut p = Pair::new(&root_dir);
    for _ in 0..3 {
        p.step("拿 node");
    }
    p
}

/// 屏上 (x, y) 处应该是哪一格的颜色（滚动量 sx / sy）
fn cell_at(sx: f32, sy: f32, x: f32, y: f32) -> [u8; 3] {
    let (r, c) = (((y - 80.0 + sy) / 100.0).floor() as u8, ((x - 20.0 + sx) / 100.0).floor() as u8);
    [r * 40, c * 40, 200]
}

fn offsets(p: &Pair) -> (f32, f32) {
    (p.a.scroll_view_offset("sv#x").unwrap_or(-1.0), p.a.scroll_view_offset("sv").unwrap_or(-1.0))
}

/// 两台同一串直线拖动（每步一帧），松手后走 `settle` 帧
fn drag(p: &mut Pair, from: (f32, f32), to: (f32, f32), steps: u32, settle: u32) {
    p.both(|e| e.pointer_down(from.0, from.1));
    p.step("按下");
    for i in 1..=steps {
        let k = i as f32 / steps as f32;
        let (x, y) = (from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k);
        p.both(|e| e.pointer_move(x, y));
        p.step(&format!("拖到 ({x}, {y})"));
    }
    p.both(|e| e.pointer_up(to.0, to.1));
    for i in 0..settle {
        p.step(&format!("松手 {i}"));
    }
}

fn js(p: &mut Pair, code: &str) {
    p.a.eval(code).ok();
    p.b.eval(code).ok();
}

fn check(name: &str, root: bool) {
    let mut p = page(name, root);
    assert_eq!(p.rgb(30, 90), cell_at(0.0, 0.0, 30.0, 90.0), "起始：左上角是 0-0");
    let fulls0 = p.fulls();
    // 斜着拖：两个轴一起动（拖 60×60，松手有惯性，等它停）
    drag(&mut p, (180.0, 240.0), (120.0, 180.0), 10, 90);
    let (sx, sy) = offsets(&p);
    assert!(sx > 50.0 && sy > 50.0, "斜着拖两个轴都动：{sx} / {sy}");
    assert_eq!(p.rgb(30, 90), cell_at(sx, sy, 30.0, 90.0), "画面跟着两个轴挪");
    assert_eq!(p.rgb(190, 270), cell_at(sx, sy, 190.0, 270.0), "右下角也对");
    let last: serde_json::Value = serde_json::from_str(&p.a.eval("JSON.stringify(getApp().last)").unwrap()).unwrap();
    assert_eq!(last["scrollLeft"].as_f64().map(|v| v as f32), Some(sx), "bindscroll 的 scrollLeft：{last}");
    assert_eq!(last["scrollTop"].as_f64().map(|v| v as f32), Some(sy), "bindscroll 的 scrollTop：{last}");
    assert_eq!(last["scrollWidth"], 600.0, "scrollWidth 是内容宽");
    // 横着拖：归这个 scroll-view（页面本来也不能横滚），纵向不动
    drag(&mut p, (180.0, 200.0), (100.0, 202.0), 10, 90);
    let (sx2, sy2) = offsets(&p);
    assert!(sx2 > sx + 40.0, "横着拖横向接着走：{sx} → {sx2}");
    assert!((sy2 - sy).abs() < 3.0, "纵向基本不动：{sy} → {sy2}");
    assert_eq!(p.fulls(), fulls0, "拖动只重画盒子，不整帧：{:?}", p.kinds);
    // 点一格：命中的是滚过之后在那儿的那一格
    p.both(|e| e.pointer_down(70.0, 130.0));
    p.step("按下");
    p.both(|e| e.pointer_up(70.0, 130.0));
    p.step("抬起");
    let (r, c) = (((130.0 - 80.0 + sy2) / 100.0).floor() as i32, ((70.0 - 20.0 + sx2) / 100.0).floor() as i32);
    assert!(p.log().contains(&format!("tap:{r}-{c}")), "点到 {r}-{c}：{:?}", p.log());
    // ScrollViewContext.scrollTo 两个轴都认；滚到右边发 scrolltolower(direction=right)
    js(&mut p, "__ctx.scrollTo({ left: 400, top: 120, animated: false })");
    for i in 0..3 {
        p.step(&format!("scrollTo · {i}"));
    }
    assert_eq!(offsets(&p), (400.0, 120.0), "scrollTo({{left, top}})");
    assert!(p.log().contains(&"lower:right".to_string()), "滚到右边发 scrolltolower right：{:?}", p.log());
    assert_eq!(p.rgb(30, 90), cell_at(400.0, 120.0, 30.0, 90.0));
    // scrollOffset 给两个轴
    js(&mut p, "wx.createSelectorQuery().select('#sv').scrollOffset(r => { getApp().off = r; }).exec()");
    for i in 0..2 {
        p.step(&format!("scrollOffset · {i}"));
    }
    let off: serde_json::Value = serde_json::from_str(&p.a.eval("JSON.stringify(getApp().off)").unwrap()).unwrap();
    assert_eq!((off["scrollLeft"].as_f64(), off["scrollTop"].as_f64()), (Some(400.0), Some(120.0)), "scrollOffset：{off}");
    // scroll-left 属性（受控）：变了才动，只动横向
    p.set("{ left: 150 }");
    p.step("scroll-left 之后");
    assert_eq!(offsets(&p), (150.0, 120.0), "scroll-left 只动横向");
    assert_eq!(p.rgb(190, 270), cell_at(150.0, 120.0, 190.0, 270.0));
}

#[test]
fn 横竖两个方向同时滚_嵌套的() {
    check("sv-2d-nested", false);
}

#[test]
fn 横竖两个方向同时滚_页面根上的() {
    check("sv-2d-root", true);
}
