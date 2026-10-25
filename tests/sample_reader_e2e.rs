//! 书香阅读（sample/reader）：每页能打开；原生 tabBar 切换；书城轮播自动播放、分类榜单；书架进度百分比、长按操作菜单移出；
//! 详情简介展开收起、加入书架、开始阅读；阅读页点中间呼出 fixed 顶栏 / 底栏（点栏不穿透到正文）、换章、字号 slider、
//! 背景 radio、滚动进度存 storage 再进来恢复；目录 scroll-into-view 定位当前章；书签与阅读时长在「我的」页。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::json;

const APP: &str = "reader";

/// 点原生 tabBar 第 `i` 个（共 3 个，底部 50px 等分）
fn tab_bar(a: &mut App, i: usize) {
    let w = W / 3.0;
    a.tap_xy(w * i as f32 + w / 2.0, H - 25.0);
    a.settle();
}

/// 点阅读页正文正中间（呼出 / 收起菜单）
fn tap_middle(a: &mut App) {
    a.tap_xy(W / 2.0, H / 2.0);
    a.frames(3);
}

fn slide_to(a: &mut App, sel: &str, to_x: f32) {
    let (x, y, w, h) = a.rect(sel);
    let from = x + w * 0.5;
    a.drag(from, y + h / 2.0, to_x - from, 0.0, 10);
    a.frames(4);
}

fn pixel(a: &mut App, x: f32, y: f32) -> [u8; 3] {
    a.frames(2);
    let px = a.e.rgba().to_vec();
    let i = (((y * DPR) as usize) * (W * DPR) as usize + (x * DPR) as usize) * 4;
    [px[i], px[i + 1], px[i + 2]]
}

/// 阅读页实例上记的当前滚动位置（bindscroll 写的）
fn last_top(a: &App) -> f64 {
    a.val("getCurrentPages().slice(-1)[0].lastTop").as_f64().unwrap_or(-1.0)
}

#[test]
fn 书香_每页都能打开() {
    let _g = serial();
    smoke_all_pages(APP, &[("pages/detail/detail", "id=b1"), ("pages/catalog/catalog", "id=b1"), ("pages/reader/reader", "id=b1")]);
}

#[test]
fn 书香_原生tabBar切换_书城轮播自动播放_分类榜单() {
    let _g = serial();
    let mut a = App::open(APP, None, "tabs");
    assert_eq!(a.route(), "pages/shelf/shelf");
    tab_bar(&mut a, 1);
    assert_eq!(a.route(), "pages/store/store");
    assert_eq!(a.depth(), 1, "switchTab 不叠页");
    assert_eq!(a.data("banner"), 0);
    // autoplay 按墙上时钟走：机器忙（全量回归里负载 30+）时帧间隔拉长，最多再多等 3 秒
    a.wait_ms(3600);
    for _ in 0..30 {
        if a.data("banner") != 0 {
            break;
        }
        a.wait_ms(100);
    }
    assert_eq!(a.data("banner"), 1, "autoplay 3 秒翻到第 2 张");
    assert_eq!(a.data("bannerSource"), "autoplay");
    assert_eq!(a.data("list.length"), 8);
    assert_eq!(a.data("list[0].id"), "b1", "全部榜第一：9.4 分");
    a.tap_nth(".cat", 4);
    assert_eq!(a.data("cat"), 4);
    assert_eq!(a.data("list").as_array().unwrap().iter().map(|b| b["id"].as_str().unwrap().to_string()).collect::<Vec<_>>(), vec!["b1", "b7"]);
    assert_eq!(a.count(".item"), 2);
    a.tap_settle(".item");
    assert_eq!(a.route(), "pages/detail/detail");
    assert_eq!(a.data("book.id"), "b1");
    a.back();
    tab_bar(&mut a, 2);
    assert_eq!(a.route(), "pages/mine/mine");
    assert_eq!(a.data("shelfCount"), 4);
    tab_bar(&mut a, 0);
    assert_eq!(a.route(), "pages/shelf/shelf");
    a.assert_no_errors();
}

#[test]
fn 书香_书架进度百分比_长按移出书架() {
    let _g = serial();
    let mut a = App::open(APP, None, "shelf");
    assert_eq!(a.data("books.length"), 4);
    assert_eq!(a.data("books[0].status"), "未读");
    assert_eq!(a.data("books[1].pct"), 20);
    assert_eq!(a.data("books[1].status"), "读到 20%");
    assert_eq!(a.data("books[2].status"), "读到 67%");
    assert_eq!(a.data("recent.id"), "b2", "继续阅读：第一本有进度的");
    assert_eq!(a.data("recent.chapterTitle"), "第七章 灯火");
    a.long_press("#book-b5");
    a.sheet_tap(0, 2);
    assert_eq!(a.data("books.length"), 3);
    assert_eq!(a.storage("rd_shelf"), json!(["b1", "b2", "b4"]));
    assert!(!a.exists("#book-b5"));
    a.wait_ms(1600);
    a.long_press("#book-b4");
    a.sheet_tap(1, 2);
    a.settle();
    assert_eq!(a.route(), "pages/detail/detail", "菜单第二项：书籍详情");
    assert_eq!(a.data("book.id"), "b4");
    assert_eq!(a.data("inShelf"), true);
    a.back();
    a.tap_settle("#book-b2");
    assert_eq!(a.route(), "pages/reader/reader");
    assert_eq!(a.data("chapter"), 6, "从进度处接着读");
    a.assert_no_errors();
}

#[test]
fn 书香_详情_简介展开收起_加入书架_开始阅读() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/detail/detail?id=b7"), "detail");
    assert_eq!(a.data("inShelf"), false);
    let short = a.data("introText").as_str().unwrap().to_string();
    assert!(short.ends_with('…') && short.chars().count() == 49, "收起：48 字 + 省略号：{short}");
    a.tap(".toggle");
    assert_eq!(a.data("expanded"), true);
    let full = a.data("introText").as_str().unwrap().to_string();
    assert!(full.chars().count() > 60 && full.ends_with('。'));
    a.tap(".toggle");
    assert_eq!(a.data("introText"), json!(short), "再点收起");
    a.tap(".b-shelf");
    assert_eq!(a.data("inShelf"), true);
    assert_eq!(a.storage("rd_shelf")[0], "b7", "加到书架最前");
    a.wait_ms(1600);
    a.tap(".b-shelf");
    assert_eq!(a.storage("rd_shelf").as_array().unwrap().len(), 5, "不重复加");
    a.wait_ms(1600);
    a.tap_settle(".b-read");
    assert_eq!(a.route(), "pages/reader/reader");
    assert_eq!(a.depth(), 2);
    assert_eq!(a.data("bookTitle"), "深空回声");
    assert_eq!(a.data("chapter"), 0);
    a.back();
    assert_eq!(a.data("progress.chapter"), 0, "回到详情：有进度了");
    assert!(a.exists(".prog"));
    a.assert_no_errors();
}

#[test]
fn 书香_阅读页_点中间呼出菜单_fixed栏不穿透_换章() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/reader/reader?id=b1"), "menu");
    assert_eq!(a.data("menu"), false);
    assert!(!a.exists(".topbar"));
    a.tap_xy(W / 2.0, 40.0); // 上 30% 不呼出
    assert_eq!(a.data("taps"), 1);
    assert_eq!(a.data("menu"), false);
    tap_middle(&mut a);
    assert_eq!(a.data("menu"), true);
    assert_eq!(a.data("taps"), 2);
    let top = a.rect(".topbar");
    assert!(top.1.abs() < 0.5 && (top.2 - W).abs() < 0.5, "顶栏贴顶通栏：{top:?}");
    let bottom = a.rect(".bottombar");
    assert!((bottom.1 + bottom.3 - H).abs() < 0.5, "底栏贴底：{bottom:?}");
    // 点顶栏标题、底栏进度文字：栏盖在正文上，正文收不到 tap
    a.tap(".top-title");
    a.tap(".pct");
    let (bx, by, _, _) = bottom;
    a.tap_xy(bx + 4.0, by + 4.0);
    assert_eq!(a.data("taps"), 2, "点 fixed 栏不穿透到正文");
    assert_eq!(a.data("menu"), true, "菜单还开着");
    // 底栏下一章 / 上一章
    a.tap(".next");
    assert_eq!(a.data("chapter"), 1);
    assert_eq!(a.data("chapterTitle"), "第二章 远行");
    assert_eq!(a.storage("rd_progress")["b1"]["chapter"], 1);
    assert_eq!(a.data("pct"), 3, "全书 1/30 ≈ 3%");
    a.tap(".prev");
    assert_eq!(a.data("chapter"), 0);
    a.tap(".prev");
    assert_eq!(a.data("chapter"), 0, "第一章再往前不动");
    assert_eq!(a.data("taps"), 2);
    // 点正文收起
    a.tap_xy(W / 2.0, H * 0.45);
    assert_eq!(a.data("menu"), false);
    assert_eq!(a.data("taps"), 3);
    // 顶栏返回
    a.wait_ms(1600);
    a.assert_no_errors();
}

#[test]
fn 书香_阅读页_字号slider_设置持久化_点主题不穿透() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/reader/reader?id=b2"), "font");
    assert_eq!(pixel(&mut a, 6.0, H * 0.5), [0xf7, 0xf3, 0xea], "日间底色");
    let h0 = a.rect(".para").3;
    tap_middle(&mut a);
    let (x, _, w, _) = a.rect(".font");
    slide_to(&mut a, ".font", x + w + 30.0);
    assert_eq!(a.data("fontSize"), 26);
    assert!(a.data("fontSize").is_u64(), "slider 值是 number");
    assert_eq!(a.storage("rd_settings")["fontSize"], 26);
    let h1 = a.rect(".para").3;
    assert!(h1 > h0 * 1.2, "字号变大段落变高：{h0} → {h1}");
    slide_to(&mut a, ".font", x - 30.0);
    assert_eq!(a.data("fontSize"), 14);
    assert_eq!(a.data("fontSize"), a.storage("rd_settings")["fontSize"]);
    // 点底栏里的主题选项：不穿透到正文、菜单不收
    a.tap_nth(".th-radio", 1);
    a.tap_nth(".th", 2);
    assert_eq!(a.data("taps"), 1, "只有呼出菜单那一下");
    assert_eq!(a.data("menu"), true);
    tap_middle(&mut a);
    assert_eq!(a.data("menu"), false);
    relaunch_then(&mut a, "/pages/reader/reader?id=b4");
    assert_eq!(a.data("fontSize"), 14, "另一本书也用同一套设置");
    assert_eq!(a.data("bookTitle"), "万剑朝宗");
    tab_bar_mine(&mut a);
    a.assert_no_errors();
}

/// 重启到书架，再 navigateTo 带参数的页面（reLaunch 直接带 query 会丢，该问题已修，见 回归_reLaunch丢query）
fn relaunch_then(a: &mut App, url: &str) {
    a.js("wx.reLaunch({ url: '/pages/shelf/shelf' })");
    a.settle();
    a.js(&format!("wx.navigateTo({{ url: '{url}' }})"));
    a.settle();
    assert_eq!(a.depth(), 2);
}

/// 到「我的」页看阅读设置（阅读页不是 tab 页，用 switchTab）
fn tab_bar_mine(a: &mut App) {
    a.js("wx.switchTab({ url: '/pages/mine/mine' })");
    a.settle();
    assert_eq!(a.route(), "pages/mine/mine");
    assert_eq!(a.data("settings.fontSize"), 14);
}

/// 曾经的引擎问题（已修，留作回归）：`position: fixed` 容器里的 `<radio-group bindchange>`，点 radio（或包着它的 label）不派发 change。
/// 正常流里同样的写法可用（智慧家空调页的模式 radio 就是）
#[test]
fn 回归_fixed底栏里的radio_group点了不触发change() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/reader/reader?id=b2"), "theme");
    tap_middle(&mut a);
    a.tap_nth(".th-radio", 1);
    assert_eq!(a.data("theme.key"), "night", "点「夜间」radio");
    a.tap_nth(".th", 2);
    assert_eq!(a.data("theme.key"), "eye", "点「护眼」label");
    a.tap_nth(".th-radio", 1);
    tap_middle(&mut a);
    assert_eq!(pixel(&mut a, 6.0, H * 0.5), [0x1b, 0x1d, 0x22], "夜间底色");
    assert_eq!(a.storage("rd_settings")["theme"], "night");
    relaunch_then(&mut a, "/pages/reader/reader?id=b4");
    assert_eq!(a.data("theme.key"), "night", "重进恢复主题");
}

#[test]
fn 书香_阅读进度_滚动位置存storage_再进来恢复() {
    let _g = serial();
    let mut a = App::open(APP, None, "progress");
    a.tap_settle("#book-b1");
    assert_eq!(a.route(), "pages/reader/reader");
    let t0 = a.rect(".ch-title").1;
    a.scroll_by(W / 2.0, 520.0, 360.0);
    a.scroll_by(W / 2.0, 520.0, 360.0);
    // 惯性尾巴很长（最后几像素要好几秒），等它基本停住再比
    a.wait_ms(2500);
    let top = last_top(&a);
    assert!(top > 300.0, "滚下去了：{top}");
    let p = a.storage("rd_progress")["b1"].clone();
    assert_eq!(p["chapter"], 0);
    let saved = p["scrollTop"].as_f64().unwrap();
    assert!((saved - top).abs() <= 2.0, "滚动位置落盘：存 {saved}，实际 {top}");
    let top = saved;
    let pct = p["pct"].as_u64().unwrap();
    assert!((1..=4).contains(&pct), "第一章读了一部分：{pct}%");
    a.back();
    assert_eq!(a.route(), "pages/shelf/shelf");
    assert_eq!(a.data("books[0].pct"), pct, "书架上的百分比跟着变");
    // 再进来：恢复到原来的位置
    a.tap_settle("#book-b1");
    assert_eq!(a.data("scrollTop").as_f64().unwrap(), top, "scroll-top 用存下的值");
    let t1 = a.rect(".ch-title").1;
    assert!((t0 - t1 - top as f32).abs() < 2.0, "标题上移了保存的距离：{t0} → {t1}（{top}）");
    // 滚过之后点中间照样呼出菜单
    tap_middle(&mut a);
    assert_eq!(a.data("menu"), true);
    // 下一章回到顶部
    a.tap(".next");
    a.frames(4);
    assert_eq!(a.data("chapter"), 1);
    assert!((a.rect(".ch-title").1 - t0).abs() < 2.0, "换章回顶");
    a.assert_no_errors();
}

#[test]
fn 书香_目录_scroll_into_view定位当前章_点章节跳转() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/reader/reader?id=b1&chapter=24"), "catalog");
    assert_eq!(a.data("chapter"), 24);
    tap_middle(&mut a);
    a.tap_settle(".catalog");
    assert_eq!(a.route(), "pages/catalog/catalog");
    assert_eq!(a.data("current"), 24);
    assert_eq!(a.data("intoView"), "ch-22");
    a.wait_ms(300);
    let (_, y, _, h) = a.rect("#ch-24");
    assert!(y > 0.0 && y + h < H, "当前章滚进视口：y={y}");
    assert_eq!(a.count(".ch.cur"), 1);
    let (_, y0, _, _) = a.rect("#ch-0");
    assert!(y0 < 0.0, "第一章被滚到上面去了：{y0}");
    a.tap_settle("#ch-23");
    assert_eq!(a.route(), "pages/reader/reader", "从阅读页进的目录：选完返回");
    assert_eq!(a.depth(), 1);
    assert_eq!(a.data("chapter"), 23);
    assert_eq!(a.data("chapterTitle"), "第二十四章 灯火");
    // 从详情页进目录：跳到阅读页
    relaunch_then(&mut a, "/pages/catalog/catalog?id=b5");
    assert_eq!(a.data("current"), 20, "种子进度：第 21 章");
    a.wait_ms(300);
    a.tap_settle("#ch-21");
    assert_eq!(a.route(), "pages/reader/reader");
    assert_eq!(a.data("chapter"), 21);
    a.assert_no_errors();
}

#[test]
fn 书香_书签_我的页列表点进对应章节_阅读时长() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/reader/reader?id=b3&chapter=4"), "marks");
    a.wait_ms(1300);
    tap_middle(&mut a);
    a.tap(".mark");
    assert_eq!(a.data("marked"), true);
    let marks = a.storage("rd_bookmarks");
    assert_eq!(marks.as_array().unwrap().len(), 1);
    assert_eq!(marks[0]["chapter"], 4);
    assert_eq!(marks[0]["chapterTitle"], "第五章 故人");
    a.wait_ms(1600);
    a.tap(".mark");
    assert_eq!(a.storage("rd_bookmarks").as_array().unwrap().len(), 1, "同一章不重复");
    a.wait_ms(1600);
    a.tap_settle(".marks");
    assert_eq!(a.route(), "pages/mine/mine");
    assert_eq!(a.depth(), 1, "switchTab 关掉了阅读页");
    assert_eq!(a.data("bookmarks.length"), 1);
    assert!(a.data("totalSeconds").as_u64().unwrap() >= 2, "阅读时长记下了：{}", a.data("totalSeconds"));
    assert_eq!(a.data("todaySeconds"), a.data("totalSeconds"));
    assert_eq!(a.data("days"), 1);
    a.tap_settle(".mark");
    assert_eq!(a.route(), "pages/reader/reader");
    assert_eq!(a.data("bookTitle"), "雾港疑案");
    assert_eq!(a.data("chapter"), 4);
    assert_eq!(a.data("marked"), true);
    a.back();
    a.long_press(".mark");
    a.modal_tap(true);
    assert_eq!(a.data("bookmarks.length"), 0);
    assert!(a.exists(".empty"));
    a.assert_no_errors();
}


/// 曾经的引擎问题（已修，留作回归）：`wx.reLaunch({ url: '/pages/x/x?id=b5' })` 打开的页面 onLoad 收到的 query 是空的（`options` 为 `{}`）；
/// navigateTo / redirectTo 带的参数都在
#[test]
fn 回归_reLaunch丢query() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/detail/detail?id=b3"), "relaunch");
    assert_eq!(a.data("book.id"), "b3");
    a.js("wx.reLaunch({ url: '/pages/detail/detail?id=b5' })");
    a.settle();
    assert_eq!(a.val("getCurrentPages()[0].options"), json!({ "id": "b5" }));
    assert_eq!(a.data("book.id"), "b5");
}
