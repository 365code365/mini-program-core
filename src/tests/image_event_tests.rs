//! `<image bindload>` / `<image binderror>` 的派发语义。
//!
//! 这条链路很容易悄悄坏掉，而且坏了以后**页面看起来是好的**，只是某些逻辑不跑：
//! tea-app 启动页就是「背景图 load 之后才开始倒计时」，事件不发时用户会干看
//! 3 秒不动的「5s」（等页面自己的兜底定时器）。
//!
//! 三条判据：
//! 1. 本地图加载成功后发 `load`，detail 带**原始**像素尺寸；
//! 2. 同一个 `(组件, src)` 只发一次（否则每帧都刷一条，逻辑层被打爆）；
//! 3. 找不到的文件发 `error`，且不会再退化成 `load`。

use crate::parser::{WxmlParser, WxssParser};
use crate::renderer::{ImageEventDetail, WxmlRenderer};
use crate::ui::InteractionManager;
use crate::{Canvas, Color};
use serde_json::json;

const LOCAL_JPG: &str = "sample/sample-app/assets/p_phone.jpg";

/// 渲染若干帧，收集期间产出的图片事件。
///
/// 为什么要多帧：`probe_load` 是**只查缓存**的（不能顺手触发加载，否则视口外
/// 被裁掉的图会被拖下来），所以第一帧是「绘制时把图读进缓存」，
/// 事件要到下一帧探测时才有结论。
fn collect_image_events(wxml: &str, css: &str, frames: usize) -> Vec<(String, String, ImageEventDetail)> {
    let sheet = WxssParser::new(css).parse().unwrap_or_default();
    let nodes = WxmlParser::new(wxml).parse().expect("wxml");
    let mut r = WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, 1.0);
    let mut im = InteractionManager::new();
    let mut out = Vec::new();
    for _ in 0..frames {
        let mut canvas = Canvas::new(375, 400);
        canvas.clear(Color::WHITE);
        r.render_with_interaction(&mut canvas, &nodes, &json!({}), &mut im);
        for e in r.take_image_events() {
            out.push((e.event_type.to_string(), e.handler.clone(), e.detail));
        }
    }
    out
}

#[test]
fn local_image_fires_load_with_natural_size() {
    if !std::path::Path::new(LOCAL_JPG).exists() {
        return; // 资源缺失时跳过
    }
    let wxml = format!(
        r#"<view><image class="p" src="{LOCAL_JPG}" bindload="onImgLoad"></image></view>"#
    );
    let evs = collect_image_events(&wxml, ".p{ width:100px; height:100px; }", 3);
    let load: Vec<_> = evs.iter().filter(|(t, _, _)| t == "load").collect();
    assert_eq!(load.len(), 1, "load 应恰好发一次，实际 {:?}", evs);
    assert_eq!(load[0].1, "onImgLoad", "handler 名要透出");
    match load[0].2 {
        ImageEventDetail::Load { width, height } => {
            // detail 是图片的原始像素尺寸，不是 CSS 的 100x100
            assert!(width > 0 && height > 0, "尺寸应为原始像素，实际 {width}x{height}");
            assert!(
                width != 100 || height != 100,
                "不能把 CSS 尺寸当成原始尺寸报上去"
            );
        }
        ImageEventDetail::Error => panic!("本地存在的图不该报 error"),
    }
}

#[test]
fn load_event_is_not_repeated_every_frame() {
    if !std::path::Path::new(LOCAL_JPG).exists() {
        return;
    }
    let wxml = format!(
        r#"<view><image class="p" src="{LOCAL_JPG}" bindload="onImgLoad"></image></view>"#
    );
    // 多跑几帧，事件总数不该跟着帧数涨
    let evs = collect_image_events(&wxml, ".p{ width:80px; height:80px; }", 8);
    assert_eq!(
        evs.iter().filter(|(t, _, _)| t == "load").count(),
        1,
        "同一个 (组件, src) 只能发一次 load，实际 {:?}",
        evs
    );
}

#[test]
fn missing_file_fires_error_once() {
    let wxml = r#"<view><image class="p" src="/assets/definitely-not-here.png" binderror="onImgError"></image></view>"#;
    let evs = collect_image_events(wxml, ".p{ width:60px; height:60px; }", 4);
    let errs: Vec<_> = evs.iter().filter(|(t, _, _)| t == "error").collect();
    assert_eq!(errs.len(), 1, "error 应恰好发一次，实际 {:?}", evs);
    assert_eq!(errs[0].1, "onImgError");
    assert!(
        matches!(errs[0].2, ImageEventDetail::Error),
        "detail 应为 Error"
    );
    assert!(
        !evs.iter().any(|(t, _, _)| t == "load"),
        "失败的图不能再发 load"
    );
}

#[test]
fn no_binding_means_no_event() {
    if !std::path::Path::new(LOCAL_JPG).exists() {
        return;
    }
    // 没绑 bindload 时不该攒事件（去重表也不该白涨）
    let wxml = format!(r#"<view><image class="p" src="{LOCAL_JPG}"></image></view>"#);
    let evs = collect_image_events(&wxml, ".p{ width:50px; height:50px; }", 3);
    assert!(evs.is_empty(), "没有绑定就不该有事件，实际 {:?}", evs);
}

#[test]
fn empty_src_is_ignored() {
    let wxml = r#"<view><image class="p" src="" bindload="onL" binderror="onE"></image></view>"#;
    let evs = collect_image_events(wxml, ".p{ width:50px; height:50px; }", 3);
    assert!(evs.is_empty(), "空 src 既不 load 也不 error，实际 {:?}", evs);
}

#[test]
fn dataset_and_id_ride_along_with_the_event() {
    if !std::path::Path::new(LOCAL_JPG).exists() {
        return;
    }
    // 列表里靠 data-index 找回是哪张图 load 完了
    let wxml = format!(
        r#"<view><image id="hero" class="p" src="{LOCAL_JPG}" data-index="3" bindload="onImgLoad"></image></view>"#
    );
    let sheet = WxssParser::new(".p{ width:70px; height:70px; }")
        .parse()
        .unwrap_or_default();
    let nodes = WxmlParser::new(&wxml).parse().expect("wxml");
    let mut r = WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, 1.0);
    let mut im = InteractionManager::new();
    let mut found = None;
    for _ in 0..4 {
        let mut canvas = Canvas::new(375, 200);
        canvas.clear(Color::WHITE);
        r.render_with_interaction(&mut canvas, &nodes, &json!({}), &mut im);
        if let Some(e) = r.take_image_events().into_iter().next() {
            found = Some(e);
            break;
        }
    }
    let e = found.expect("应产出一条 load 事件");
    assert_eq!(e.id, "hero", "id 要带上");
    assert_eq!(e.data.get("index").map(String::as_str), Some("3"), "data-* 要带上");
    assert!(e.owner.is_empty(), "页面模板里的绑定 owner 为空串");
}

// ───────────── WP-E2：image 的 SVG / data: / fade-in / cloud:// / 长按菜单（官方 component/image）─────────────

/// 渲染一页（白底，sf = 1），返回画布
fn render_once(r: &mut WxmlRenderer, wxml: &str) -> Canvas {
    let nodes = WxmlParser::new(wxml).parse().expect("wxml");
    let mut im = InteractionManager::new();
    let mut canvas = Canvas::new(375, 200);
    canvas.clear(Color::WHITE);
    r.render_with_interaction(&mut canvas, &nodes, &json!({}), &mut im);
    canvas
}

/// 纯色 PNG 的 data: URI
fn png_data_uri(w: u32, h: u32, rgba: [u8; 4]) -> String {
    use base64::Engine as _;
    let mut buf = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(w, h, image::Rgba(rgba)).write_to(&mut buf, image::ImageFormat::Png).unwrap();
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(buf.into_inner()))
}

#[cfg(feature = "svg")]
#[test]
fn e2_svg_解码成位图_data_uri_也认_scale_to_fill_撑满() {
    // 20×10 的 SVG：左半红、右半透明；画进 80×20 的框（scaleToFill：Skyline 撑满，官方 image.md）
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>"##;
    let src = format!("data:image/svg+xml,{}", svg.replace('#', "%23").replace('"', "%22").replace('<', "%3C").replace('>', "%3E"));
    let mut r = WxmlRenderer::new_with_scale(WxssParser::new(".s{width:80px;height:20px}").parse().unwrap_or_default(), 375.0, 667.0, 1.0);
    let c = render_once(&mut r, &format!(r#"<view><image class="s" src="{src}"></image></view>"#));
    let p = c.get_pixel(20, 10);
    assert!(p.r > 240 && p.g < 20 && p.b < 20, "左半是红（横向拉伸到 40px 宽）：{p:?}");
    assert_eq!(c.get_pixel(60, 10), Color::WHITE, "右半透明，露出白底");
}

#[test]
fn e2_fade_in_从透明渐显_不写不渐显() {
    let src = png_data_uri(4, 4, [255, 0, 0, 255]);
    let other = png_data_uri(4, 4, [0, 0, 255, 255]);
    let css = ".s{width:40px;height:40px}";
    let mut r = WxmlRenderer::new_with_scale(WxssParser::new(css).parse().unwrap_or_default(), 375.0, 667.0, 1.0);
    let wxml = format!(r#"<view><image class="s" fade-in src="{src}"></image><image class="s" src="{other}"></image></view>"#);
    let c0 = render_once(&mut r, &wxml);
    let p0 = c0.get_pixel(20, 20);
    assert!(p0.g > 150, "刚出现几乎透明（白底透出来）：{p0:?}");
    assert_eq!(c0.get_pixel(20, 60), Color::new(0, 0, 255, 255), "没写 fade-in 的直接不透明");
    assert!(!r.animated_images_due(0.0, 1e6).is_empty() || r.has_animated_images(0.0, 1e6), "渐显中按动图续帧（宿主醒着、只重画这块）");
    std::thread::sleep(std::time::Duration::from_millis(crate::renderer::components::image::fade::DURATION_MS as u64 + 30));
    assert!(!r.animated_images_due(0.0, 1e6).is_empty(), "渐显到头：那块要重画成不透明");
    let c1 = render_once(&mut r, &wxml);
    assert_eq!(c1.get_pixel(20, 20), Color::new(255, 0, 0, 255), "渐显完不透明");
}

#[test]
fn e2_cloud_文件_id_走_binderror() {
    let evs = collect_image_events(r#"<view><image class="p" src="cloud://env-1.xxx/a.png" binderror="onErr"></image></view>"#, ".p{width:10px;height:10px}", 3);
    assert!(evs.iter().any(|(t, h, d)| t == "error" && h == "onErr" && matches!(d, ImageEventDetail::Error)), "cloud:// 宿主解析不了，发 error：{evs:?}");
}

#[test]
fn e2_长按菜单_宿主没接弹_action_sheet_返回已处理() {
    let a = {
        let mut a = crate::runtime::MiniApp::new(375, 667).expect("create MiniApp");
        a.init().expect("init");
        a
    };
    let out = a
        .eval(
            r#"(function() {
  var got = null; wx.showActionSheet = function(o) { got = o.itemList; };
  var handled = __dispatchHandler('__imgmenu:/img/a.png', { type: 'longpress', detail: {} });
  return JSON.stringify({ handled: handled, items: got });
})()"#,
        )
        .unwrap();
    assert_eq!(out, r#"{"handled":true,"items":["发送给朋友","收藏","保存图片"]}"#);
}
