//! D4 swiper：swiper-item `item-id` → change / animationfinish 的 `currentItemId`、`current-item-id`、
//! `clip=false`、`snap-to-edge`。判据：事件 detail 的字段与类型、像素。
use super::*;

const COLORS: &str = ".i0 { background: #ff0000; } .i1 { background: #00ff00; } .i2 { background: #0000ff; }";

#[test]
fn d4_item_id_带进_current_item_id_current_item_id_按项选() {
    let root = fixture(
        "swiper-item-id",
        r#"<swiper id="sw-item" class="sw" current-item-id="{{cid}}" bindchange="onC" bindanimationfinish="onF">
  <swiper-item item-id="a" class="i0"></swiper-item><swiper-item item-id="b" class="i1"></swiper-item><swiper-item item-id="c" class="i2"></swiper-item>
</swiper>"#,
        &format!(".sw {{ height: 200px; }} {COLORS}"),
        "Page({ data: { cid: '' },
  onC(e) { getApp().log.push('change:' + typeof e.detail.current + ':' + e.detail.current + ':' + e.detail.currentItemId + ':' + e.detail.source); },
  onF(e) { getApp().log.push('finish:' + e.detail.currentItemId); } });",
        &[],
    );
    let (mut e, _g) = single(&root);
    let rgb = |e: &mut MiniEngine| {
        let (w, _) = e.pixel_size();
        let i = ((100 * w + 100) * 4) as usize;
        let px = e.rgba();
        [px[i], px[i + 1], px[i + 2]]
    };
    assert_eq!(rgb(&mut e), [255, 0, 0]);
    e.eval("getCurrentPages()[0].setData({ cid: 'c' })").unwrap();
    for i in 0..60u64 {
        e.set_animation_clock((100 + i * 16) as f32 / 1000.0);
        e.pump(100 + i * 16);
    }
    assert_eq!(rgb(&mut e), [0, 0, 255], "current-item-id='c' 选第三项");
    let log: Vec<String> = serde_json::from_str(&e.eval("JSON.stringify(getApp().log)").unwrap_or_default()).unwrap_or_default();
    assert_eq!(log, vec!["change:number:2:c:", "finish:c"], "currentItemId 带上了；source 是空串（页面改的）");
}

fn margins(name: &str, attrs: &str) -> Pair {
    let root = fixture(
        name,
        &format!(
            r#"<view class="box"><swiper id="sw-{name}" class="sw" previous-margin="40px" next-margin="40px" {attrs}>
  <swiper-item class="i0"></swiper-item><swiper-item class="i1"></swiper-item><swiper-item class="i2"></swiper-item>
</swiper></view>"#
        ),
        &format!("page {{ background: #ffffff; }} .box {{ padding: 0 80px; }} .sw {{ height: 200px; background: #cccccc; }} {COLORS}"),
        "Page({});",
        &[],
    );
    Pair::new(&root)
}

#[test]
fn d4_snap_to_edge_首项贴边_不写照旧留边距() {
    // 盒子 80..295（宽 215），前后边距 40：项宽 135，第一项从 120 起
    let mut p = margins("swiper-snap-off", "");
    assert_eq!(p.rgb(100, 100), [0xcc, 0xcc, 0xcc], "没写 snap-to-edge：第一项前面留着前边距");
    assert_eq!(p.rgb(125, 100), [255, 0, 0]);
    drop(p);
    let mut p = margins("swiper-snap-on", "snap-to-edge");
    assert_eq!(p.rgb(85, 100), [255, 0, 0], "snap-to-edge：第一项贴左边");
    assert_eq!(p.rgb(225, 100), [0, 255, 0], "后面露出第二项");
}

#[test]
fn d4_clip_false_盒子外的相邻项照画() {
    let mut p = margins("swiper-clip-on", r#"current="1""#);
    assert_eq!(p.rgb(40, 100), [255, 255, 255], "默认裁剪：盒子外是页面底色");
    drop(p);
    let mut p = margins("swiper-clip-off", r#"current="1" clip="{{false}}""#);
    // 第二项在 120..255，第一项在它左边 -15..120：盒子外 40 处露出第一项
    assert_eq!(p.rgb(40, 100), [255, 0, 0], "clip=false：盒子外露出前一项");
    assert_eq!(p.rgb(300, 100), [0, 0, 255], "右边露出后一项");
}
