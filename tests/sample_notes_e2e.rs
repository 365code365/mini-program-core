//! 灵感笔记（sample/notes）：每页能打开；搜索实时过滤与空态、标签筛选、长按置顶 / 移到回收站、
//! 编辑页防抖自动保存（字数、标签多选、颜色单选）、新建（fixed 悬浮按钮不穿透）、回收站恢复 / 彻底删除 / 清空、
//! 标签管理校验、设置页排序 picker 与预览开关。全部走真指针。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::Value;

const APP: &str = "notes";

fn titles(a: &App, group: &str) -> Vec<String> {
    a.data(group).as_array().unwrap().iter().map(|n| n["title"].as_str().unwrap().to_string()).collect()
}

/// storage 里某条笔记
fn note(a: &App, id: i64) -> Value {
    a.storage("notes_list").as_array().unwrap().iter().find(|n| n["id"] == id).cloned().unwrap_or(Value::Null)
}

/// 点操作菜单第 `i` 项（共 `n` 项）再等它收起：收起动画按引擎帧时钟走，比 wait_ms 的墙上时钟慢，
/// 紧接着按下去会被正在收起的菜单吃掉（见 harness::sheet_tap）
fn sheet(a: &mut App, i: usize, n: usize) {
    a.sheet_tap(i, n);
    a.wait_ms(400);
}

/// 删掉输入框里光标前的 `n` 个字
fn backspace(a: &mut App, n: usize) {
    for _ in 0..n {
        a.key("backspace");
    }
}

#[test]
fn 笔记_每页都能打开() {
    let _g = serial();
    smoke_all_pages(APP, &[("pages/edit/edit", "id=3")]);
}

#[test]
fn 笔记_首页分组与种子数据() {
    let _g = serial();
    let a = App::open(APP, None, "home");
    assert_eq!(a.data("total"), 8, "回收站里的两篇不算");
    assert_eq!(titles(&a, "pinned"), ["周末徒步路线", "读书摘录：《置身事内》"], "置顶组按修改时间倒序");
    assert_eq!(a.data("others[0].title"), "Rust 所有权笔记", "其他组最近修改的在最前");
    assert_eq!(a.data("others").as_array().unwrap().len(), 6);
    assert_eq!(a.data("sortName"), "按修改时间");
    a.assert_no_errors();
}

#[test]
fn 笔记_搜索实时过滤_空态_清除() {
    let _g = serial();
    let mut a = App::open(APP, None, "search");
    a.type_into(".search-input", "徒步");
    assert_eq!(a.data("kw"), "徒步");
    assert_eq!(a.data("shown"), 1, "只剩标题含「徒步」的一篇");
    assert_eq!(a.data("pinned[0].title"), "周末徒步路线");
    // 正文也参与搜索：「借用检查器」只在 Rust 那篇的正文里
    backspace(&mut a, 2);
    a.e.text_input("借用");
    a.frames(3);
    assert_eq!(a.data("shown"), 1);
    assert_eq!(a.data("others[0].title"), "Rust 所有权笔记");
    // 再打几个字：一篇都不剩 → 空态
    a.e.text_input("不存在的词");
    a.frames(3);
    assert_eq!(a.data("shown"), 0);
    assert!(a.exists(".empty"), "没有结果时显示空态");
    assert_eq!(a.count(".note"), 0);
    a.blur();
    a.tap_settle(".search-clear");
    assert_eq!(a.data("kw"), "");
    assert_eq!(a.data("shown"), 8, "清除搜索后全部回来");
    assert!(!a.exists(".empty"));
    a.assert_no_errors();
}

#[test]
fn 笔记_标签_chip_筛选() {
    let _g = serial();
    let mut a = App::open(APP, None, "chip");
    // chip 顺序：全部、工作、生活、读书、旅行、灵感
    a.tap_nth(".chip", 4);
    a.frames(3);
    assert_eq!(a.data("tagId"), "t4");
    assert_eq!(a.data("shown"), 2);
    assert_eq!(titles(&a, "pinned"), ["周末徒步路线"]);
    assert_eq!(titles(&a, "others"), ["年度旅行清单"]);
    // 筛选与搜索叠加
    a.type_into(".search-input", "冰岛");
    assert_eq!(a.data("shown"), 1);
    assert_eq!(a.data("pinned").as_array().unwrap().len(), 0);
    a.blur();
    a.tap_nth(".chip", 0);
    a.frames(3);
    assert_eq!(a.data("tagId"), "");
    assert_eq!(a.data("shown"), 1, "回到「全部」后搜索词仍生效：只剩年度旅行清单");
    a.assert_no_errors();
}

#[test]
fn 笔记_长按置顶与移到回收站_重启后仍在() {
    let _g = serial();
    let mut a = App::open(APP, None, "long");
    a.long_press(".other-note");
    assert_eq!(a.route(), "pages/index/index", "长按不该同时触发点击进编辑页");
    sheet(&mut a, 0, 2); // 置顶
    assert_eq!(titles(&a, "pinned"), ["Rust 所有权笔记", "周末徒步路线", "读书摘录：《置身事内》"]);
    assert_eq!(note(&a, 7)["pinned"], true);
    // 再长按其他组第一篇（产品周会要点）→ 移到回收站
    a.long_press(".other-note");
    sheet(&mut a, 1, 2);
    assert_eq!(a.route(), "pages/index/index", "长按 + 菜单不该打开笔记（depth={}）", a.depth());
    assert_eq!(a.data("total"), 7);
    assert_eq!(note(&a, 3)["deleted"], true);
    // 长按已置顶那篇 → 菜单第一项是「取消置顶」
    a.long_press(".pinned-note");
    sheet(&mut a, 0, 2);
    assert_eq!(note(&a, 7)["pinned"], false);
    // 取消菜单：什么都不变
    a.long_press(".pinned-note");
    sheet(&mut a, 2, 2);
    assert_eq!(a.data("pinned").as_array().unwrap().len(), 2);
    a.js("wx.reLaunch({url:'/pages/index/index'})");
    a.settle();
    assert_eq!(a.depth(), 1);
    assert_eq!(a.data("total"), 7, "reLaunch 后从 storage 读回");
    assert_eq!(a.data("others[0].title"), "Rust 所有权笔记");
    a.tap_settle(".go-trash");
    assert_eq!(a.route(), "pages/trash/trash");
    assert_eq!(a.data("items").as_array().unwrap().len(), 3);
    a.assert_no_errors();
}

#[test]
fn 笔记_编辑页防抖自动保存_字数_标签多选_颜色单选() {
    let _g = serial();
    let mut a = App::open(APP, None, "edit");
    a.tap_settle(".other-note"); // Rust 所有权笔记
    assert_eq!(a.route(), "pages/edit/edit");
    assert_eq!(a.depth(), 2);
    assert_eq!(a.data("isNew"), false);
    let w0 = a.data("words").as_i64().unwrap();
    a.type_into(".title-input", "（二）");
    assert_eq!(a.data("form.title"), "Rust 所有权笔记（二）");
    assert_eq!(a.data("saveState"), "编辑中…", "停下打字之前不落盘");
    assert_eq!(note(&a, 7)["title"], "Rust 所有权笔记");
    a.wait_ms(800);
    assert_eq!(a.data("saveState"), "已保存");
    assert_eq!(note(&a, 7)["title"], "Rust 所有权笔记（二）");
    a.blur();
    // 点在正文中间光标就落在那里：先移到末尾再打字
    a.tap(".body-input");
    a.key("doc-end");
    a.e.text_input(" 生命周期 'a");
    a.frames(3);
    a.wait_ms(800);
    assert_eq!(a.data("words").as_i64().unwrap(), w0 + 6, "空白不计字数");
    assert!(note(&a, 7)["body"].as_str().unwrap().ends_with("生命周期 'a"), "{}", note(&a, 7)["body"]);
    a.blur();
    // 标签多选：原有 工作(t1)、灵感(t5)；勾上 生活(t2)、取消 工作
    a.tap_nth(".tag-cb", 1);
    a.tap_nth(".tag-cb", 0);
    let mut tags: Vec<String> = a.data("form.tags").as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    tags.sort();
    assert_eq!(tags, ["t2", "t5"]);
    // 颜色单选：第 5 个是樱粉
    a.tap_nth(".color-rd", 4);
    assert_eq!(a.data("form.color"), "pink");
    assert_eq!(a.data("bg"), "#ffe1e6");
    a.wait_ms(800);
    assert_eq!(note(&a, 7)["color"], "pink");
    a.back();
    assert_eq!(a.route(), "pages/index/index");
    assert_eq!(a.data("others[0].title"), "Rust 所有权笔记（二）");
    assert_eq!(a.data("others[0].bg"), "#ffe1e6");
    a.assert_no_errors();
}

#[test]
fn 笔记_返回时立即保存未落盘的输入() {
    let _g = serial();
    let mut a = App::open(APP, None, "flush");
    a.tap_settle(".pinned-note");
    a.type_into(".title-input", "!");
    a.blur();
    a.back(); // 防抖还没到点就返回
    assert_eq!(note(&a, 1)["title"], "周末徒步路线!");
    assert_eq!(a.data("pinned[0].title"), "周末徒步路线!");
    a.assert_no_errors();
}

#[test]
fn 笔记_悬浮新建按钮不穿透_新建后保存() {
    let _g = serial();
    let mut a = App::open(APP, None, "new");
    // 悬浮按钮压在某张笔记卡片上：点它只新建，不打开底下那篇
    let (fx, fy, fw, fh) = a.rect(".fab");
    let (cx, cy) = (fx + fw / 2.0, fy + fh / 2.0);
    let under = a.rects(".note").iter().any(|&(x, y, w, h)| cx > x && cx < x + w && cy > y && cy < y + h);
    assert!(under, "悬浮按钮下面要压着一张笔记，否则测不到穿透：fab={:?} notes={:?}", (fx, fy, fw, fh), a.rects(".note"));
    a.tap_settle(".fab");
    assert_eq!(a.route(), "pages/edit/edit");
    assert_eq!(a.data("isNew"), true, "打开的是新建，不是底下那篇");
    assert_eq!(a.data("form.id"), 11);
    a.wait_ms(800);
    assert_eq!(a.storage("notes_list").as_array().unwrap().len(), 10, "什么都没写不建空笔记");
    a.type_into(".title-input", "新的想法");
    a.wait_ms(800);
    assert_eq!(a.data("isNew"), false);
    assert_eq!(note(&a, 11)["title"], "新的想法");
    a.blur();
    a.back();
    assert_eq!(a.data("total"), 9);
    assert_eq!(a.data("others[0].title"), "新的想法");
    a.assert_no_errors();
}

#[test]
fn 笔记_编辑页移到回收站() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/edit/edit?id=4"), "edit-del");
    assert_eq!(a.data("form.title"), "番茄炒蛋的正确做法");
    a.tap_settle(".del");
    assert_eq!(note(&a, 4)["deleted"], true);
    a.assert_no_errors();
}

#[test]
fn 笔记_回收站_恢复_彻底删除确认_弹窗不穿透() {
    let _g = serial();
    let mut a = App::open(APP, None, "trash");
    a.tap_settle(".go-trash");
    assert_eq!(a.data("items").as_array().unwrap().len(), 2);
    a.tap_nth(".restore", 0); // 旧的购物清单
    a.settle();
    assert_eq!(a.data("items").as_array().unwrap().len(), 1);
    assert_eq!(note(&a, 9)["deleted"], false);
    // 彻底删除：弹窗弹着时点底下的「恢复」不生效，取消后什么都不变
    let (rx, ry, rw, rh) = a.rect(".restore");
    a.tap(".purge");
    assert!(a.modal_open());
    a.tap_xy(rx + rw / 2.0, ry + rh / 2.0);
    assert_eq!(note(&a, 10)["deleted"], true, "弹窗遮罩下的按钮不该被点到");
    if a.modal_open() {
        a.modal_tap(false);
    }
    assert_eq!(a.data("items").as_array().unwrap().len(), 1);
    a.tap(".purge");
    a.modal_tap(true);
    assert_eq!(a.data("items").as_array().unwrap().len(), 0);
    assert!(a.exists(".empty"));
    assert!(note(&a, 10).is_null(), "彻底删除后 storage 里没有了");
    a.back();
    assert_eq!(a.data("total"), 9, "恢复的那篇回到列表");
    a.assert_no_errors();
}

#[test]
fn 笔记_清空回收站() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/trash/trash"), "clear");
    a.tap(".clear");
    a.modal_tap(true);
    assert_eq!(a.data("items").as_array().unwrap().len(), 0);
    assert_eq!(a.storage("notes_list").as_array().unwrap().len(), 8);
    a.js("wx.reLaunch({url:'/pages/trash/trash'})");
    a.settle();
    assert_eq!(a.data("items").as_array().unwrap().len(), 0, "重开仍是空的");
    a.assert_no_errors();
}

#[test]
fn 笔记_标签管理_校验重复_新增_删除() {
    let _g = serial();
    let mut a = App::open(APP, None, "tags");
    a.tap_settle(".go-tags");
    assert_eq!(a.route(), "pages/tags/tags");
    assert_eq!(a.data("tags[0].count"), 2, "工作：产品周会、Rust");
    a.tap(".tag-add");
    assert_eq!(a.data("error"), "请输入标签名");
    a.type_into(".tag-input", "工作");
    a.blur();
    a.tap(".tag-add");
    assert_eq!(a.data("error"), "标签「工作」已存在");
    assert_eq!(a.data("tags").as_array().unwrap().len(), 5);
    a.tap(".tag-input");
    backspace(&mut a, 2);
    a.e.text_input("摄影");
    a.frames(3);
    a.blur();
    a.tap_settle(".tag-add");
    assert_eq!(a.data("error"), "");
    assert_eq!(a.data("name"), "");
    assert_eq!(a.data("tags[5].name"), "摄影");
    assert_eq!(a.storage("notes_tags")[5]["id"], "t6");
    // 删除「工作」：确认后笔记上的 t1 一并摘掉
    a.tap_nth(".tag-del", 0);
    a.modal_tap(true);
    assert_eq!(a.data("tags").as_array().unwrap().len(), 5);
    assert_eq!(a.data("tags[0].name"), "生活");
    let still = a.storage("notes_list").as_array().unwrap().iter().filter(|n| n["tags"].as_array().unwrap().iter().any(|t| t == "t1")).count();
    assert_eq!(still, 0);
    a.back();
    assert_eq!(a.count(".chip"), 6, "全部 + 5 个标签");
    a.assert_no_errors();
}

#[test]
fn 笔记_设置_排序_picker_与预览开关() {
    let _g = serial();
    let mut a = App::open(APP, None, "settings");
    assert!(a.exists(".note-body"));
    a.tap_settle(".go-settings");
    a.picker(".sort-picker");
    a.picker_row(0, 1, 2);
    a.picker_confirm();
    assert!(a.data("rawSort").is_number(), "selector 的 e.detail.value 是 number：{}", a.data("rawSort"));
    assert_eq!(a.data("rawSort"), 2);
    assert_eq!(a.storage("notes_settings")["sort"], 2);
    a.tap(".preview-sw");
    assert_eq!(a.data("preview"), false);
    assert_eq!(a.storage("notes_settings")["preview"], false);
    a.back();
    assert_eq!(a.data("sortName"), "按标题");
    assert!(!a.exists(".note-body"), "关掉预览后列表不显示正文");
    // 按标题（码位序）排
    let got = titles(&a, "others");
    let mut want = got.clone();
    want.sort();
    assert_eq!(got, want);
    assert_eq!(got[0], "Rust 所有权笔记");
    // 按创建时间：年度旅行清单（400h 前）在最后
    a.tap_settle(".go-settings");
    a.picker(".sort-picker");
    a.picker_row(0, 1, -1);
    a.picker_confirm();
    assert_eq!(a.data("sort"), 1);
    a.back();
    assert_eq!(a.data("others[0].title"), "灵感：桌面收纳盒");
    assert_eq!(a.data("others[5].title"), "年度旅行清单");
    a.assert_no_errors();
}

