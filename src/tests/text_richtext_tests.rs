//! WP-A：text 的行数限制 / decode / space，rich-text 的子树展开，span / cover-view，list-builder / grid-builder。
//!
//! 每条都是可判定的：布局数值（物理像素，sf = 2）、像素有没有墨、或事件 detail 的字段与类型。
use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::WxmlRenderer;
use crate::ui::interaction::InteractionManager;
use crate::Canvas;
use serde_json::{json, Value};

const SF: f32 = 2.0;

fn render(css: &str, wxml: &str, data: Value) -> (WxmlRenderer, Canvas, InteractionManager) {
    let sheet = WxssParser::new(css).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, SF);
    let nodes = WxmlParser::new(wxml).parse().unwrap_or_default();
    let mut c = Canvas::new(750, 1334);
    c.clear(crate::Color::WHITE);
    let mut im = InteractionManager::new();
    r.render_with_interaction(&mut c, &nodes, &data, &mut im);
    (r, c, im)
}

fn size(r: &WxmlRenderer, class: &str) -> (f32, f32) {
    r.node_size_by_class(class).unwrap_or_else(|| panic!("找不到 .{class}"))
}

/// 逻辑坐标矩形里非白像素的个数
fn ink(c: &Canvas, x0: f32, y0: f32, x1: f32, y1: f32) -> usize {
    let (w, h) = (c.width() as i32, c.height() as i32);
    let mut n = 0;
    for y in ((y0 * SF) as i32).max(0)..((y1 * SF) as i32).min(h) {
        for x in ((x0 * SF) as i32).max(0)..((x1 * SF) as i32).min(w) {
            let p = c.get_pixel(x as u32, y as u32);
            if p.r < 200 || p.g < 200 || p.b < 200 {
                n += 1;
            }
        }
    }
    n
}

const LONG: &str = "一二三四五六七八九十一二三四五六七八九十一二三四五六七八九十一二三四五六七八九十";

// ───────────── A1 行数限制 ─────────────

#[test]
fn a1_webkit_line_clamp_夹住父级里的文字为两行() {
    let css = ".c{width:100px;font-size:16px;line-height:20px;display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:2;overflow:hidden}\
               .n{width:100px;font-size:16px;line-height:20px} .tn{-webkit-line-clamp:2;overflow:hidden}";
    let (r, c, _) = render(css, &format!(r#"<view class="c">{LONG}</view><view class="n"><text class="tn">{LONG}</text></view>"#), json!({}));
    assert_eq!(size(&r, "c"), (100.0 * SF, 40.0 * SF), "display:-webkit-box + line-clamp:2 → 按容器宽折行、两行高");
    assert!(size(&r, "n").1 > 100.0 * SF, "不是 -webkit-box 时 line-clamp 不生效（同浏览器），实际 {}", size(&r, "n").1);
    // 两行都有字、第二行末尾是省略号（最后 12px 里有墨：「...」画在行尾），第三行没字
    assert!(ink(&c, 0.0, 0.0, 100.0, 20.0) > 0 && ink(&c, 0.0, 20.0, 100.0, 40.0) > 0);
    // 第二行末尾是「...」：行尾 10px 里有墨（汉字排满一行时那里本来是字，换成三个点后墨少很多）
    let (_, c1, _) = render(css, &format!(r#"<view class="c">{LONG}</view>"#), json!({}));
    assert_eq!(ink(&c1, 0.0, 41.0, 375.0, 80.0), 0, "第三行不画");
    assert!(ink(&c1, 88.0, 20.0, 100.0, 40.0) < ink(&c1, 88.0, 0.0, 100.0, 20.0), "第二行行尾是省略号，比第一行行尾的汉字墨少");
}

#[test]
fn a1_text_自己的_line_clamp_与_max_lines() {
    let css = ".t{width:100px;font-size:16px;line-height:20px;display:-webkit-box;-webkit-line-clamp:3;overflow:hidden}\
               .m{width:100px;font-size:16px;line-height:20px}";
    let (r, _, _) = render(css, &format!(r#"<view><text class="t">{LONG}</text></view><view><text class="m" max-lines="2">{LONG}</text></view>"#), json!({}));
    assert_eq!(size(&r, "t").1, 60.0 * SF);
    assert_eq!(size(&r, "m").1, 40.0 * SF, "Skyline max-lines=2 → 两行高");
}

#[test]
fn a1_max_lines_overflow_clip_不画第三行_visible_照画() {
    let css = ".m{width:100px;font-size:16px;line-height:20px;color:#000}";
    let w = |ov: &str| format!(r#"<view style="height:300px"><text class="m" max-lines="2" overflow="{ov}">{LONG}</text></view>"#);
    let (_, clip, _) = render(css, &w("clip"), json!({}));
    let (_, vis, _) = render(css, &w("visible"), json!({}));
    let (_, ell, _) = render(css, &w("ellipsis"), json!({}));
    assert_eq!(ink(&clip, 0.0, 41.0, 110.0, 80.0), 0, "clip：第三行不画");
    assert!(ink(&vis, 0.0, 41.0, 110.0, 80.0) > 20, "visible：盒子两行高，字照画到外面");
    assert!(ink(&ell, 0.0, 0.0, 110.0, 40.0) > 0 && ink(&ell, 0.0, 41.0, 110.0, 80.0) == 0, "ellipsis：只两行");
}

// ───────────── A2 decode / space ─────────────

#[test]
fn a2_decode_解码官方实体_不写不解() {
    let css = ".d,.e,.f{font-size:16px}";
    // 横排容器里文字按内容定宽（纵排的父级会把 text 拉满整宽，比不出宽窄）
    let row = r#"style="display:flex;flex-direction:row""#;
    let (r, _, _) = render(css, &format!(r#"<view {row}><text class="d" decode>x&amp;y</text></view><view {row}><text class="e">x&y</text></view><view {row}><text class="f">x&amp;y</text></view>"#), json!({}));
    assert_eq!(size(&r, "d").0, size(&r, "e").0, "decode 后 &amp; 与 & 一样宽");
    assert!(size(&r, "f").0 > size(&r, "e").0, "不写 decode 时实体原样显示");
}

#[test]
fn a2_space_emsp_比普通空格宽() {
    let css = ".s,.n{font-size:16px}";
    let row = r#"style="display:flex;flex-direction:row""#;
    let (r, _, _) = render(css, &format!(r#"<view {row}><text class="s" space="emsp">a  b</text></view><view {row}><text class="n">a  b</text></view>"#), json!({}));
    let (s, n) = (size(&r, "s").0, size(&r, "n").0);
    // 两个空格各换成一个中文字宽（16px）：至少宽出 2×(16 − 普通空格宽 ≈ 4.5) 逻辑像素
    assert!(s - n >= 20.0 * SF, "emsp 空格应约 1em 宽：{s} vs {n}");
}

// ───────────── A3 rich-text ─────────────

#[test]
fn a3_html_字符串_块级与_ua_样式() {
    let css = ".rt{font-size:16px}";
    let html = r#"<h1 class="h">标题</h1><p class="p">正文<b>粗</b>后</p><p class="q">第二段<br>换行</p>"#;
    let (r, _, _) = render(css, r#"<rich-text class="rt" nodes="{{html}}"></rich-text>"#, json!({ "html": html }));
    // h1：2em 字号；h1 / p 都有外边距（第二个起不落 margin-top，见 rich_text/ua.rs）
    let (_, h_h) = size(&r, "h");
    let (_, p_h) = size(&r, "p");
    let (_, q_h) = size(&r, "q");
    assert!(h_h > p_h * 1.5, "h1 是 2em：{h_h} vs {p_h}");
    assert!(q_h > p_h * 1.8, "<br> 换行成两行：{q_h} vs {p_h}");
    let (rt_w, rt_h) = size(&r, "rt");
    assert!(rt_w > 300.0 * SF);
    // 标题 + 两段的盒高之和，加上外边距：.67em×32 + 16(第一段下) + 16(第二段下) 至少
    assert!(rt_h > h_h + p_h + q_h + 40.0 * SF, "块之间有外边距：{rt_h}");
}

#[test]
fn a3_class_匹配页面样式_style_全量生效() {
    let css = ".red{color:#ff0000} .big{font-size:40px}";
    let nodes = json!([{ "name": "div", "attrs": { "class": "big", "style": "color:#0000ff;padding-left:20px" }, "children": [
        { "type": "text", "text": "蓝色大字" }, { "name": "span", "attrs": { "class": "red" }, "children": [{ "type": "text", "text": "红" }] }
    ]}]);
    let (r, c, _) = render(css, r#"<rich-text nodes="{{nodes}}"></rich-text>"#, json!({ "nodes": nodes }));
    let (fs, color, _) = r.node_style_by_class("big").expect(".big 生效");
    assert_eq!(fs, 40.0);
    assert_eq!(color.map(|c| (c.r, c.g, c.b)), Some((0, 0, 255)), "内联 style 的 color 盖过继承");
    // 红字：在第一行里找纯红的像素
    let mut red = 0;
    for y in 0..(60.0 * SF) as u32 {
        for x in 0..(375.0 * SF) as u32 {
            let p = c.get_pixel(x, y);
            if p.r > 200 && p.g < 80 && p.b < 80 {
                red += 1;
            }
        }
    }
    assert!(red > 30, "span.red 的 class 匹配页面 WXSS，画成红色（{red} 个红像素）");
    // padding-left:20px：第一行左边 20px 内没有墨
    assert_eq!(ink(&c, 0.0, 0.0, 19.0, 50.0), 0);
}

#[test]
fn a3_白名单外连子树移除_图片变成_image() {
    let html = r#"<p>a<iframe>不该出现</iframe><img src="x.png" width="80" height="40"></p><script>bad()</script>"#;
    let kids = crate::renderer::components::rich_text_expand(&json!(html), None);
    let dump = format!("{kids:?}");
    assert!(!dump.contains("不该出现") && !dump.contains("bad()"), "{dump}");
    assert!(dump.contains("width:80px;height:40px"), "img 的宽高属性折成样式：{dump}");
    let (r, _, im) = render("", r#"<rich-text nodes="{{h}}"></rich-text>"#, json!({ "h": html }));
    let img = r.query_selector("image", false, &im);
    assert_eq!(img.len(), 1);
    assert_eq!((img[0].rect.width, img[0].rect.height), (80.0, 40.0));
    // 内部节点不带事件：展开出来的节点上一个 bind 属性都没有
    assert!(!dump.contains("bind"), "{dump}");
}

#[test]
fn a3_内部节点不派事件_点击归_rich_text_自己() {
    let html = r#"<p>a<b>b</b><img src="x.png" width="20" height="20"></p><div>c</div>"#;
    let (r, _, _) = render("", r#"<rich-text bindtap="onRt" nodes="{{h}}"></rich-text>"#, json!({ "h": html }));
    let taps: Vec<_> = r.get_event_bindings().iter().filter(|b| b.event_type == "tap").collect();
    assert_eq!(taps.len(), 1, "只有 rich-text 自己的 bindtap");
    assert_eq!(taps[0].handler, "onRt");
}

#[test]
fn a3_列表项目符号与_space() {
    let (r, c, _) = render(".l{font-size:16px}", r#"<rich-text class="l" nodes="<ul><li>甲</li><li>乙</li></ul>"></rich-text>"#, json!({}));
    let (_, h) = size(&r, "l");
    assert!(h > 2.0 * 20.0 * SF, "两个 li 各一行");
    // ul 的 padding-left 40px：项目符号画在 40px 左边（20~40 之间），文字从 40 起
    assert!(ink(&c, 20.0, 16.0, 39.0, 40.0) > 0, "项目符号在内边距里");
    fn all_text(n: &[crate::parser::wxml::WxmlNode]) -> String {
        n.iter().map(|c| format!("{}{}", c.text_content, all_text(&c.children))).collect()
    }
    let kids = crate::renderer::components::rich_text_expand(&json!("a  b"), Some("nbsp"));
    assert_eq!(all_text(&kids), "a\u{a0}\u{a0}b", "space=nbsp：连续空格保留并换成 nbsp");
    let kids = crate::renderer::components::rich_text_expand(&json!("a  \n b"), None);
    assert_eq!(all_text(&kids), "a b", "默认按 HTML 合并空白");
}

#[test]
fn a3_长文一万字_建树布局耗时() {
    // 300 段、每段 ~34 字（共 ~1 万字），其中每 3 段有一段带行内标签（走 span 拆 run）
    let mut html = String::new();
    for i in 0..300 {
        if i % 3 == 0 {
            html.push_str(&format!("<p>第{i}段<b>加粗</b>的文字，这里是一段比较长的正文内容用来测量排版耗时。</p>"));
        } else {
            html.push_str(&format!("<p>第{i}段普通的文字，这里是一段比较长的正文内容用来测量排版的耗时情况。</p>"));
        }
    }
    let chars = html.chars().count();
    let t = std::time::Instant::now();
    let (r, _, _) = render(".rt{font-size:16px}", r#"<rich-text class="rt" nodes="{{h}}"></rich-text>"#, json!({ "h": html }));
    let ms = t.elapsed().as_secs_f32() * 1000.0;
    let nodes = r.render_node_count();
    eprintln!("📏 rich-text 长文 {chars} 字符：首次渲染（展开 + 建树 + 布局 + 画一屏）{ms:.1}ms，渲染节点 {nodes}");
    assert!(size(&r, "rt").1 > 3000.0 * SF);
    assert!(nodes < 6000, "纯文字段落一段一个节点，带行内标签的才拆 run：{nodes}");
    // setData 换一份同样长的文字（最后一段改一个字）：再排一次的耗时
    let mut r = r;
    let html2 = html.replacen("第299段", "第二九九段", 1);
    let nodes_x = WxmlParser::new(r#"<rich-text class="rt" nodes="{{h}}"></rich-text>"#).parse().unwrap_or_default();
    let mut c = Canvas::new(750, 1334);
    let mut im = InteractionManager::new();
    r.render_with_interaction(&mut c, &nodes_x, &json!({ "h": html }), &mut im);
    let t = std::time::Instant::now();
    r.render_with_interaction(&mut c, &nodes_x, &json!({ "h": html2 }), &mut im);
    let ms2 = t.elapsed().as_secs_f32() * 1000.0;
    let t = std::time::Instant::now();
    let kids = crate::renderer::components::rich_text_expand(&json!(html), None);
    let ms_expand = t.elapsed().as_secs_f32() * 1000.0;
    eprintln!("📏 setData 后重排 {ms2:.1}ms；其中 HTML 解析 + 转子树 {ms_expand:.2}ms（{} 个块）", kids.len());
}

// ───────────── A5 cover-view / span ─────────────

#[test]
fn a5_cover_view_默认不换行_行高_1_2() {
    let css = ".cv,.v{width:60px;font-size:20px}";
    let (r, _, _) = render(css, r#"<cover-view class="cv">很长很长很长很长的一行</cover-view><view class="v">很长很长很长很长的一行</view>"#, json!({}));
    assert_eq!(size(&r, "cv").1, 20.0 * 1.2 * SF, "一行、line-height:1.2");
    assert!(size(&r, "v").1 > 25.0 * SF, "普通 view 是字体自然行高（中文 1.375）");
    // nowrap 交给了里面的字（裸文本本来就不按容器宽折行，这条防的是以后加上折行时 cover-view 跟着折）
    let mut found = None;
    fn walk(n: &crate::renderer::RenderNode, out: &mut Option<crate::renderer::components::WhiteSpace>) {
        if n.tag == "#text" && out.is_none() {
            *out = Some(n.style.white_space);
        }
        n.children.iter().for_each(|c| walk(c, out));
    }
    r.for_each_root_for_test(|n| walk(n, &mut found));
    assert!(found == Some(crate::renderer::components::WhiteSpace::NoWrap));
}

#[test]
fn a5_span_文字接在图片后面折行() {
    let css = ".sp{width:200px;font-size:16px;line-height:20px}";
    let wxml = r#"<span class="sp"><image style="width:50px;height:20px"></image><text>一二三四五六七八九十一二三四五六七八九十</text></span>"#;
    let (r, _, _) = render(css, wxml, json!({}));
    // 第一行：图 50 + 9 个字 144 = 194；剩下 11 个字第二行 —— 共两行（竖着叠会是 20 + 两行字 = 60）
    let (_, h) = size(&r, "sp");
    assert!(h <= 48.0 * SF, "文字接在图片后面同一行：{h}");
}

// ───────────── A4 list-builder / grid-builder ─────────────

fn builder_page(tag: &str, extra: &str) -> String {
    format!(
        r#"<scroll-view type="custom" scroll-y style="height:300px"><{tag} class="lb" list="{{{{list}}}}" child-height="50" padding="[10, 0, 0, 0]" {extra} bind:itembuild="onBuild" bind:itemdispose="onDispose"><view slot:item="it" slot:index="i" class="row">{{{{it.name}}}}-{{{{i}}}}</view></{tag}></scroll-view>"#
    )
}

fn list(n: usize) -> Value {
    json!({ "list": (0..n).map(|i| json!({ "name": format!("项{i}") })).collect::<Vec<_>>() })
}

#[test]
fn a4_list_builder_按_list_展开_定高与_padding() {
    let (r, _, im) = render("", &builder_page("list-builder", ""), list(20));
    let rows = r.query_selector(".row", true, &im);
    assert_eq!(rows.len(), 20, "slot:item 模板逐项展开");
    assert_eq!(rows[0].rect.y, 10.0, "padding-top 10");
    assert_eq!(rows[1].rect.y - rows[0].rect.y, 50.0, "type=static：child-height 定高");
    let w = r.builder_windows(&im, 0.0, 667.0);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].range, Some((0, 6)), "300 高的 scroll-view 里看得见第 0~5 项");
}

#[test]
fn a4_grid_builder_复用_grid_view_布局() {
    let (r, _, im) = render("", &builder_page("grid-builder", r#"cross-axis-count="3" cross-axis-gap="5" main-axis-gap="8""#), list(9));
    let rows = r.query_selector(".row", true, &im);
    assert_eq!(rows.len(), 9);
    assert_eq!(rows[0].rect.y, rows[2].rect.y, "一行三列");
    assert!(rows[3].rect.y > rows[0].rect.y);
    assert!((rows[1].rect.x - rows[0].rect.x - rows[0].rect.width - 5.0).abs() < 0.6, "列间距 5");
}

#[test]
fn a4_itembuild_itemdispose_detail_index_是_number() {
    let (r, _, mut im) = render("", &builder_page("list-builder", ""), list(40));
    let mut app = crate::runtime::MiniApp::new(375, 667).expect("MiniApp");
    app.init().expect("init");
    app.eval("globalThis.__got = []; globalThis.__dispatchEvent = function (h, ev) { __got.push(h + ':' + ev.type + ':' + typeof ev.detail.index + ':' + ev.detail.index); };")
        .unwrap();
    assert!(crate::host::builder_events::tick(&mut app, &r, &im, "p/a", 0.0, 667.0, 0));
    let got = app.eval("__got.join(',')").unwrap();
    assert_eq!(got, (0..6).map(|i| format!("onBuild:itembuild:number:{i}")).collect::<Vec<_>>().join(","));
    // 滚到 200：第 0~3 项出去（顶边 10+50k，底边 < 200 的是 0~2；第 3 项底边 210 仍可见）
    let sv = r.query_selector("scroll-view", false, &im)[0].component_id.clone();
    if let Some(c) = im.get_scroll_controller_mut(&sv) {
        c.set_position(200.0);
    } else {
        panic!("scroll-view 没有滚动控制器");
    }
    app.eval("__got = []").unwrap();
    assert!(crate::host::builder_events::tick(&mut app, &r, &im, "p/a", 0.0, 667.0, 0));
    let got = app.eval("__got.join(',')").unwrap();
    assert!(got.starts_with("onDispose:itemdispose:number:0,onDispose:itemdispose:number:1,onDispose:itemdispose:number:2,onBuild:itembuild:number:6"), "{got}");
    assert!(!crate::host::builder_events::tick(&mut app, &r, &im, "p/a", 0.0, 667.0, 0), "窗没变不再派发");
    crate::host::builder_events::clear();
}

// ───────────── 裸文本（`<view>一段字</view>`）的宽度与折行 ─────────────

#[test]
fn 裸文本_粗体按粗体量宽_长的到容器边折行() {
    // 组件实验室抓到的两处：粗体标题按常规体量宽、盒子窄一截，绘制按盒宽折行压到下面；长的裸文本不折行、一整行冲出容器
    let css = ".c{width:300px;font-size:16px;line-height:20px} .b{font-weight:bold} .t{width:300px;font-size:16px;line-height:20px}";
    let (r, _, _) = render(
        css,
        r#"<view class="c b">textarea auto-height</view>
           <view class="c">一段比容器宽很多的裸文本，到容器右边就该折到下一行接着排，而不是一整行冲出去</view>
           <view class="t"><text>一段比容器宽很多的裸文本，到容器右边就该折到下一行接着排，而不是一整行冲出去</text></view>"#,
        json!({}),
    );
    let (_, bold_h) = size(&r, "b");
    assert_eq!(bold_h, 20.0 * SF, "短的粗体标题一行：盒子按粗体字面量宽，不会被绘制折成两行");
    // 长的裸文本与 <text> 同样折行、同样高
    let (_, text_h) = size(&r, "t");
    assert!(text_h > 20.0 * SF, "<text> 折成多行：{text_h}");
    let nodes = WxmlParser::new(r#"<view class="c">一段比容器宽很多的裸文本，到容器右边就该折到下一行接着排，而不是一整行冲出去</view>"#).parse().unwrap_or_default();
    let sheet = WxssParser::new(css).parse().unwrap_or_default();
    let mut r2 = WxmlRenderer::new_with_scale(sheet, 375.0, 667.0, SF);
    let mut c = Canvas::new(750, 1334);
    let mut im = InteractionManager::new();
    r2.render_with_interaction(&mut c, &nodes, &json!({}), &mut im);
    assert_eq!(size(&r2, "c"), (300.0 * SF, text_h), "裸文本与 <text> 一样到容器边折行、补高");
}
