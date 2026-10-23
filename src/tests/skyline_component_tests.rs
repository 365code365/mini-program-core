//! Skyline 补齐的组件：`grid-view`（对齐网格 / 瀑布流）、`cover-image`、没被宿主接管的原生组件
//! （map / camera / web-view …）的占位与默认尺寸。
//!
//! 判据都是可判定的数字：每一项的逻辑坐标（经事件绑定的包围盒读出）、像素是否相同。
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::Canvas;
use serde_json::json;

fn render(css: &str, wxml: &str, native: &[&str]) -> (WxmlRenderer, Canvas) {
    let sheet = WxssParser::new(css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, 2.0);
    r.set_native_components(&native.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let nodes = WxmlParser::new(wxml).parse().unwrap_or_default();
    let mut c = Canvas::new(750, 1334);
    c.clear(crate::Color::WHITE);
    r.render(&mut c, &nodes, &json!({}));
    (r, c)
}

/// 处理函数名 → (x, y, w, h)，逻辑像素
fn bounds(r: &WxmlRenderer, handler: &str) -> (f32, f32, f32, f32) {
    let b = r
        .get_event_bindings()
        .iter()
        .find(|b| b.handler == handler)
        .unwrap_or_else(|| panic!("没登记 {handler}"));
    let round = |v: f32| (v * 10.0).round() / 10.0;
    (round(b.bounds.x), round(b.bounds.y), round(b.bounds.width), round(b.bounds.height))
}

#[test]
fn grid_view_对齐网格按列数与间距排() {
    // 3 列、列距 15、行距 10、容器宽 375 → 列宽 (375 - 30) / 3 = 115
    let wxml = r#"<grid-view cross-axis-count="3" cross-axis-gap="15" main-axis-gap="10">
        <view bindtap="a0" style="height:40px"></view><view bindtap="a1" style="height:60px"></view>
        <view bindtap="a2" style="height:20px"></view><view bindtap="a3" style="height:30px"></view>
    </grid-view><view bindtap="after" style="height:10px"></view>"#;
    let (r, _) = render("", wxml, &[]);
    assert_eq!(bounds(&r, "a0"), (0.0, 0.0, 115.0, 40.0));
    assert_eq!(bounds(&r, "a1"), (130.0, 0.0, 115.0, 60.0));
    assert_eq!(bounds(&r, "a2").0, 260.0);
    // 第二行从第一行最高的那项（60）+ 行距开始：对齐网格同一行共用行高
    assert_eq!(bounds(&r, "a3"), (0.0, 70.0, 115.0, 30.0));
    assert_eq!(bounds(&r, "after").1, 100.0, "容器高 = 两行 + 一个行距");
}

#[test]
fn grid_view_瀑布流按顺序填最矮的列() {
    // 2 列、两向间距 10 → 列宽 (375 - 10) / 2 = 182.5
    // 高 100 / 50 / 80 / 40：第 3 项进右列（50 < 100），第 4 项进左列（110 < 140）
    let wxml = r#"<grid-view type="masonry" cross-axis-count="2" cross-axis-gap="10" main-axis-gap="10">
        <view bindtap="m0" style="height:100px"></view><view bindtap="m1" style="height:50px"></view>
        <view bindtap="m2" style="height:80px"></view><view bindtap="m3" style="height:40px"></view>
    </grid-view><view bindtap="after" style="height:10px"></view>"#;
    let (r, _) = render("", wxml, &[]);
    assert_eq!(bounds(&r, "m0"), (0.0, 0.0, 182.5, 100.0));
    assert_eq!(bounds(&r, "m1"), (192.5, 0.0, 182.5, 50.0));
    assert_eq!(bounds(&r, "m2"), (192.5, 60.0, 182.5, 80.0), "进右列，不被同行的 100 拉高");
    assert_eq!(bounds(&r, "m3"), (0.0, 110.0, 182.5, 40.0));
    assert_eq!(bounds(&r, "after").1, 150.0, "容器高 = 最高那列（左列 110 + 40）");
}

#[test]
fn grid_view_瀑布流放在带内边距的容器里_前面还有兄弟() {
    // 容器 margin 12 + padding 12 → 内容从 x=24 起、宽 327；前面一行 20px 高 + 下边距 10 → grid 从 y=24+30 起
    let wxml = r#"<view style="margin:12px;padding:12px">
        <view style="height:20px;margin-bottom:10px"></view>
        <grid-view type="masonry" cross-axis-count="2" cross-axis-gap="8" main-axis-gap="8">
          <view bindtap="q0" style="height:80px"></view><view bindtap="q1" style="height:140px"></view>
          <view bindtap="q2" style="height:100px"></view>
        </grid-view></view>"#;
    let (r, _) = render("", wxml, &[]);
    assert_eq!(bounds(&r, "q0"), (24.0, 54.0, 159.5, 80.0));
    assert_eq!(bounds(&r, "q1"), (191.5, 54.0, 159.5, 140.0));
    assert_eq!(bounds(&r, "q2"), (24.0, 142.0, 159.5, 100.0));
}

#[test]
fn grid_view_瀑布流按外边距盒摆放() {
    // 每项 margin 10：列宽 (375 - 8) / 2 = 183.5，项宽扣掉左右外边距 = 163.5；
    // 左列 0 号占 10+80+10 = 100（+ 行距 8），2 号从 108 起、它的边框盒再下移 10
    let css = ".it { margin: 10px; }";
    let wxml = r#"<grid-view type="masonry" cross-axis-count="2" cross-axis-gap="8" main-axis-gap="8">
        <view class="it" bindtap="k0" style="height:80px"></view><view class="it" bindtap="k1" style="height:140px"></view>
        <view class="it" bindtap="k2" style="height:100px"></view>
    </grid-view><view bindtap="after" style="height:10px"></view>"#;
    let (r, _) = render(css, wxml, &[]);
    assert_eq!(bounds(&r, "k0"), (10.0, 10.0, 163.5, 80.0));
    assert_eq!(bounds(&r, "k1"), (201.5, 10.0, 163.5, 140.0));
    assert_eq!(bounds(&r, "k2"), (10.0, 118.0, 163.5, 100.0));
    // 容器高 = 最高那列：右列 10+140+10 = 160，左列 108+120 = 228
    assert_eq!(bounds(&r, "after").1, 228.0);
}

#[test]
fn grid_view_padding_与按最大列宽推列数() {
    // padding 上 8 左 20 右 20；max-cross-axis-extent=100 → 内宽 335 / 100 向上取整 = 4 列
    let wxml = r#"<grid-view max-cross-axis-extent="100" padding="[8, 20, 0, 20]">
        <view bindtap="p0" style="height:10px"></view><view bindtap="p1" style="height:10px"></view>
    </grid-view>"#;
    let (r, _) = render("", wxml, &[]);
    let (x0, y0, w0, _) = bounds(&r, "p0");
    assert_eq!((x0, y0), (20.0, 8.0));
    // 335 / 4 = 83.75：taffy 按设备像素取整（sf = 2 时一个设备像素是 0.5 逻辑像素）
    assert!((w0 - 335.0 / 4.0).abs() <= 0.5, "列宽 {w0}");
    assert!((bounds(&r, "p1").0 - (20.0 + 335.0 / 4.0)).abs() <= 0.5);
}

#[test]
fn cover_image_与_image_画得一样() {
    let css = ".pic { width: 120px; height: 80px; background-color: #3366ff; border-radius: 8px; }";
    let (ra, a) = render(css, r#"<image class="pic" mode="aspectFill" src="/nope.png" bindtap="t"/>"#, &[]);
    let (rb, b) = render(css, r#"<cover-image class="pic" mode="aspectFill" src="/nope.png" bindtap="t"/>"#, &[]);
    assert_eq!(bounds(&ra, "t"), bounds(&rb, "t"));
    assert!(a.pixels() == b.pixels(), "cover-image 按 image 画，像素逐个相同");
}

#[test]
fn 未接管的原生组件有默认尺寸与占位_接管了就不画() {
    let wxml = r#"<map bindtap="m"></map><view bindtap="after" style="height:10px"></view>"#;
    let (r, c) = render("", wxml, &[]);
    assert_eq!(bounds(&r, "m"), (0.0, 0.0, 300.0, 150.0), "微信 map 默认 300×150");
    assert_eq!(bounds(&r, "after").1, 150.0);
    // 占位底色 #f0f0f0（设备像素，sf = 2）
    let px = c.get_pixel(20, 20);
    assert_eq!((px.r, px.g, px.b), (0xf0, 0xf0, 0xf0), "宿主没接管：画浅灰占位");

    let (_, hosted) = render("", wxml, &["map"]);
    let px = hosted.get_pixel(20, 20);
    assert_eq!((px.r, px.g, px.b), (0xff, 0xff, 0xff), "宿主接管了：控件盖在上面，引擎不画占位");

    // 页面 CSS 照样覆盖默认尺寸
    let (r, _) = render(".m { width: 100%; height: 200px; }", r#"<map class="m" bindtap="m"></map>"#, &[]);
    assert_eq!(bounds(&r, "m"), (0.0, 0.0, 375.0, 200.0));
}

#[test]
fn 居中的裸文本在靠内容撑宽的父节点里不塌成零宽() {
    // `.btn{text-align:center}` 的按钮放在 align-items:center 的弹层里：按钮宽靠内容撑。
    // 从前裸文本按 width:100% 排、没有参照物就是 0，按钮只剩内边距（18），字一个一个往下掉
    let css = ".btn { padding: 9px; text-align: center; } .body { align-items: center; }";
    let (bare, _) = render(css, r#"<view class="body"><view class="btn" bindtap="a">关闭</view></view>"#, &[]);
    let (wrapped, _) = render(css, r#"<view class="body"><view class="btn" bindtap="a"><text>关闭</text></view></view>"#, &[]);
    let (b, w) = (bounds(&bare, "a"), bounds(&wrapped, "a"));
    assert!(b.2 > 40.0, "按钮宽 = 两个字 + 内边距：{b:?}");
    assert_eq!(b, w, "与包在 <text> 里一样宽");
}
