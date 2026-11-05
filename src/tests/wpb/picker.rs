//! B3 / B4：picker（region 的 code）与 picker-view（indicator / mask 的样式与类、项高跟选中框）
use super::*;

#[test]
fn b3_region_确定之后_detail_带_code() {
    let wxml = r#"<picker mode="region" value="{{r}}" custom-item="全部" bindchange="onC" style="height:60px"><view>选</view></picker>"#;
    let p_js = format!(
        "{LOG_FN} Page({{ data: {{ r: ['广东省', '广州市', '海珠区'] }}, \
           onC(e){{ L(JSON.stringify(e.detail.value) + ' ' + JSON.stringify(e.detail.code) + ' ' + typeof e.detail.postcode); }} }});"
    );
    let root = fixture("wpb-region", wxml, &p_js, &[]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    tap(&mut e, 100.0, 30.0, 2000);
    // 面板入场 0.22s：等它停稳再点「确定」（头部右半）
    pump_until(&mut e, 400, |_| false);
    let (w, h) = (375.0f32, 667.0f32);
    let top = h - crate::host::picker_sheet::SHEET_H;
    tap(&mut e, w - 20.0, top + 15.0, 4000);
    assert!(pump_until(&mut e, 800, |e| !log(e).is_empty()), "确定要发 change");
    assert_eq!(log(&e), vec![r#"["广东省","广州市","海珠区"] ["440000","440100","440105"] undefined"#]);
}

#[test]
fn b4_picker_view_选中框样式与类_蒙层_项高跟选中框() {
    let wxml = r##"<picker-view indicator-style="height:100rpx;border:2px solid #ff0000" mask-style="background-image:none" value="{{[1]}}" style="height:250px;width:300px">
        <picker-view-column><view id="i0" style="font-size:40px">A</view><view>B</view><view>C</view></picker-view-column>
      </picker-view>
      <picker-view indicator-class="ind" mask-class="m" style="height:200px;width:300px">
        <picker-view-column><view id="j0">X</view><view>Y</view></picker-view-column>
      </picker-view>"##;
    let wxss = ".ind{height:60px;background-color:#0000ff}.m{background-color:#00ff00}";
    let root = fixture("wpb-picker-view", wxml, "Page({});", &[("pages/p/p.wxss", wxss)]);
    let mut e = engine(&root, EngineOptions::default());
    settle(&mut e);
    assert!((rect_of(&mut e, "i0")[3] - 50.0).abs() < 0.5, "indicator-style 的 height（100rpx=50px）设成项高");
    assert!((rect_of(&mut e, "j0")[3] - 60.0).abs() < 0.5, "indicator-class 的 height 也算");
    // 第一个：选中框 [100, 150)，border 2px 红；mask-style 去掉蒙层 → 第 0 项「A」原样露着（深色字）
    assert_eq!(rgb_at(&mut e, 150, 100), [255, 0, 0], "border 画成选中框上边");
    assert_eq!(rgb_at(&mut e, 150, 149), [255, 0, 0], "下边");
    let dark = (50..100u32).flat_map(|y| (0..300u32).map(move |x| (x, y))).filter(|&(x, y)| rgb_at(&mut e, x, y).iter().all(|c| *c < 80)).count();
    assert!(dark > 20, "没有蒙层，上面那一项的字是实色：{dark}");
    // 第二个（y 从 250 起，高 200）：选中框 [320, 380) 蓝底，上面的蒙层是 mask-class 的纯绿
    assert_eq!(rgb_at(&mut e, 5, 330), [0, 0, 255], "indicator-class 的底色");
    assert_eq!(rgb_at(&mut e, 5, 260), [0, 255, 0], "mask-class 的纯色蒙层");
}
