//! Skyline scroll-view `clip="{{false}}"`：溢出盒子的内容照画，滚动 / setData 时局部重画与整帧逐字节相同
//! （scroll_views.rs `clip_false_paint` 给滚动的损伤区；invalidate.rs 里面变了整帧）
use super::*;

fn page(name: &str, clip: &str) -> Pair {
    let wxml = format!(
        r#"<view class="wrap"><scroll-view id="sv" scroll-x class="sv" {clip}>
  <view class="row">
    <view wx:for="{{{{cards}}}}" wx:key="*this" class="card {{{{index % 2 ? 'b' : 'g'}}}}"><view class="tip" style="background: {{{{item}}}}"></view></view>
  </view>
</scroll-view></view>"#
    );
    let js = "Page({ data: { cards: ['#ff0000', '#ff0000', '#ff0000', '#ff0000', '#ff0000', '#ff0000', '#ff0000', '#ff0000'] },
  onLoad() { wx.createSelectorQuery().select('#sv').node().exec(res => { globalThis.__ctx = res[0].node; }); } });";
    let root = fixture(
        name,
        &wxml,
        "page { background: #ffffff; } .wrap { padding-top: 80px; } .sv { height: 100px; background: #eeeeee; }
         .row { display: flex; flex-direction: row; width: 960px; height: 100px; }
         .card { position: relative; width: 120px; height: 100px; flex-shrink: 0; }
         .b { background: #0000ff; } .g { background: #00ff00; }
         .tip { position: absolute; left: 10px; top: -40px; width: 100px; height: 30px; }",
        js,
        &[],
    );
    let mut p = Pair::new(&root);
    for _ in 0..3 {
        p.step("拿 node");
    }
    p
}

fn js(p: &mut Pair, code: &str) {
    p.a.eval(code).ok();
    p.b.eval(code).ok();
}

#[test]
fn clip_false_盒子外的溢出照画_滚动与_set_data_局部重画与整帧一致() {
    let mut p = page("sv-clip-on", "");
    assert_eq!(p.rgb(50, 55), [255, 255, 255], "默认裁剪：盒子上面（溢出的那条）是页面底色");
    drop(p);
    let mut p = page("sv-clip-off", r#"clip="{{false}}""#);
    assert_eq!(p.rgb(50, 55), [255, 0, 0], "clip=false：卡片上面溢出的红条照画");
    assert_eq!(p.rgb(50, 120), [0, 255, 0], "盒子里照常");
    // 不带动画滚 130：红条跟着走
    js(&mut p, "__ctx.scrollTo({ left: 130, animated: false })");
    for i in 0..3 {
        p.step(&format!("滚到 130 · {i}"));
    }
    assert_eq!(p.a.scroll_view_offset("sv"), Some(130.0));
    // 第二张卡的红条在内容 130~230 → 屏上 0~100；第三张的在 250~350 → 屏上 120~220，中间 100~120 是空的
    assert_eq!(p.rgb(5, 55), [255, 0, 0], "红条跟着滚过来了");
    assert_eq!(p.rgb(110, 55), [255, 255, 255], "两条红条之间：旧位置（滚动前这里是第一张卡的红条）擦掉了");
    // 带动画滚到 300：每一帧都比。ScrollViewContext 调用的那一帧本来就是整帧（结构，裁不裁都一样），从下一帧起数
    js(&mut p, "__ctx.scrollTo({ left: 300 })");
    p.step("动画起步");
    let before = p.fulls();
    for i in 0..40 {
        p.step(&format!("动画 {i}"));
    }
    assert_eq!(p.a.scroll_view_offset("sv"), Some(300.0));
    // 横着拖一下（手指在盒子里）
    p.both(|e| e.pointer_down(300.0, 130.0));
    p.step("按下");
    for k in 1..=10 {
        p.both(|e| e.pointer_move(300.0 - k as f32 * 15.0, 130.0));
        p.step(&format!("拖 {k}"));
    }
    p.both(|e| e.pointer_up(150.0, 130.0));
    for i in 0..40 {
        p.step(&format!("松手 {i}"));
    }
    assert_eq!(p.fulls(), before, "滚动只重画盒子加溢出那圈，不整帧：{:?}", p.kinds);
    // 里面改颜色：盒子外那条也换（整帧，见 invalidate.rs）
    p.set("{ 'cards[3]': '#000000' }");
    p.step("改色之后");
}
