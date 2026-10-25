//! 团队看板（sample/kanban）：每页能打开；三列看板（横向 scroll-view 拖到第三列再点卡片）、
//! 卡片「⋯」移动到别的列（catchtap 不穿透到卡片）、详情页优先级 / 负责人 / 截止日期 picker、
//! 子任务勾选与进度、新建卡片的表单校验、统计（完成率、逾期按「今天」算）、成员任务数、删除确认。
//! 截止日期都是相对今天生成的，断言里不写死任何日期。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::Value;

const APP: &str = "kanban";

fn card(a: &App, id: i64) -> Value {
    a.storage("kb_cards").as_array().unwrap().iter().find(|c| c["id"] == id).cloned().unwrap_or(Value::Null)
}

fn counts(a: &App) -> Vec<i64> {
    a.data("cols").as_array().unwrap().iter().map(|c| c["count"].as_i64().unwrap()).collect()
}

fn ids(v: &Value) -> Vec<i64> {
    v.as_array().unwrap().iter().map(|c| c["id"].as_i64().unwrap()).collect()
}

/// 操作菜单点完再等它收起（收起动画按引擎帧时钟走，紧接着的触摸会被它吃掉）
fn sheet(a: &mut App, i: usize, n: usize) {
    a.sheet_tap(i, n);
    a.wait_ms(400);
}

/// 引擎里的「今天 ± n 天」，与示例 utils 同一算法
fn day_offset(a: &App, n: i32) -> String {
    a.val(&format!(
        "(function(){{var d=new Date();d.setDate(d.getDate()+({n}));var p=function(x){{return x<10?'0'+x:''+x}};return d.getFullYear()+'-'+p(d.getMonth()+1)+'-'+p(d.getDate())}})()"
    ))
    .as_str()
    .unwrap()
    .to_string()
}

#[test]
fn 看板_每页都能打开() {
    let _g = serial();
    smoke_all_pages(APP, &[("pages/card/card", "id=1"), ("pages/new/new", "col=doing")]);
}

#[test]
fn 看板_三列数量与列内排序() {
    let _g = serial();
    let mut a = App::open(APP, None, "board");
    assert_eq!(counts(&a), [3, 4, 3]);
    assert_eq!(ids(&a.data("cols[0].cards")), [1, 2, 3], "优先级高的在前");
    assert_eq!(ids(&a.data("cols[1].cards")), [4, 5, 6, 7], "同优先级截止早的在前");
    assert_eq!(a.data("overdue"), 2);
    assert_eq!(a.data("cols[0].cards[1].overdue"), true);
    assert_eq!(a.data("cols[2].cards[0].overdue"), false, "已完成的不算逾期");
    assert_eq!(a.count(".col-count"), 3);
    assert_eq!(a.count(".is-overdue"), 2);
    a.assert_no_errors();
}

#[test]
fn 看板_横向拖到第三列_点卡片进详情() {
    let _g = serial();
    let mut a = App::open(APP, None, "hscroll");
    let (x0, _, _, _) = a.rect(".card-done");
    assert!(x0 > W, "第三列一开始在屏幕外：x={x0}");
    let (_, y, _, h) = a.rect(".col-head");
    a.drag(330.0, y + h / 2.0 + 40.0, -320.0, 0.0, 16);
    a.wait_ms(900);
    a.drag(330.0, y + h / 2.0 + 40.0, -320.0, 0.0, 16);
    a.wait_ms(900);
    assert_eq!(a.route(), "pages/board/board", "拖动不算点击");
    let (x1, _, w1, _) = a.rect(".card-done");
    assert!(x1 >= 0.0 && x1 + w1 <= W + 1.0, "拖完第三列整列在视口里：x={x1}");
    a.tap_settle(".card-done");
    assert_eq!(a.route(), "pages/card/card");
    assert_eq!(a.data("card.id"), 10, "已完成列第一张是「修复 iOS 键盘遮挡」");
    assert_eq!(a.data("colName"), "已完成");
    a.assert_no_errors();
}

#[test]
fn 看板_卡片移动到_不穿透_重启后仍在() {
    let _g = serial();
    let mut a = App::open(APP, None, "move");
    a.tap_nth(".card-move", 0); // 待办第一张：首页改版需求评审
    assert_eq!(a.route(), "pages/board/board", "⋯ 是 catchtap，不该打开卡片");
    sheet(&mut a, 1, 3);
    assert_eq!(counts(&a), [2, 5, 3]);
    assert_eq!(card(&a, 1)["col"], "doing");
    assert_eq!(ids(&a.data("cols[1].cards")), [4, 5, 1, 6, 7], "进行中：紧急 > 高（同为高的截止早的在前）> 中");
    // 选当前列：什么都不变
    a.tap_nth(".card-move", 0);
    sheet(&mut a, 0, 3);
    assert_eq!(counts(&a), [2, 5, 3]);
    // 取消
    a.tap_nth(".card-move", 0);
    sheet(&mut a, 3, 3);
    assert_eq!(counts(&a), [2, 5, 3]);
    a.js("wx.reLaunch({url:'/pages/board/board'})");
    a.settle();
    assert_eq!(counts(&a), [2, 5, 3], "reLaunch 后从 storage 读回");
    a.assert_no_errors();
}

#[test]
fn 看板_详情页_picker_子任务_标题编辑() {
    let _g = serial();
    let mut a = App::open(APP, None, "detail");
    a.tap_nth(".card-todo", 1); // 登录页埋点方案：中、赵磊、逾期 2 天
    a.settle();
    assert_eq!(a.route(), "pages/card/card");
    assert_eq!(a.depth(), 2);
    assert_eq!(a.data("card.id"), 2);
    assert_eq!(a.data("overdue"), true);
    // 优先级：中 → 紧急
    a.picker(".p-prio");
    a.picker_row(0, 1, 2);
    a.picker_confirm();
    assert!(a.data("raw.prio").is_number(), "selector 的 value 是 number：{}", a.data("raw.prio"));
    assert_eq!(a.data("raw.prio"), 3);
    assert_eq!(card(&a, 2)["prio"], 3);
    // 负责人：赵磊（第 4 个）→ 孙悦
    a.picker(".p-owner");
    a.picker_row(0, 1, 1);
    a.picker_confirm();
    assert_eq!(a.data("raw.owner"), 4);
    assert_eq!(card(&a, 2)["owner"], "m5");
    // 截止日期：年份往后拨一年 → 不再逾期
    let due0 = card(&a, 2)["due"].as_str().unwrap().to_string();
    let y0: i32 = due0[..4].parse().unwrap();
    a.picker(".p-due");
    a.picker_row(0, 3, 1);
    a.picker_confirm();
    let raw = a.data("raw.due");
    assert!(raw.is_string(), "date picker 的 value 是字符串：{raw}");
    assert_eq!(raw.as_str().unwrap(), format!("{}{}", y0 + 1, &due0[4..]));
    assert_eq!(a.data("overdue"), false);
    // 子任务：1/2 → 勾上第二个 2/2 → 取消第一个 1/2
    assert_eq!(a.data("pct"), 50);
    a.tap_nth(".sub-cb", 1);
    a.frames(3);
    assert_eq!(a.data("pct"), 100);
    assert_eq!(a.data("progressText"), "2/2");
    a.wait_ms(1600); // 「子任务全部完成」toast 消失
    a.tap_nth(".sub-cb", 0);
    a.frames(3);
    assert_eq!(a.data("pct"), 50);
    assert_eq!(card(&a, 2)["subs"][0]["done"], false);
    assert_eq!(card(&a, 2)["subs"][1]["done"], true);
    // 新增子任务：空的不加
    a.tap(".sub-add");
    assert_eq!(a.data("card.subs").as_array().unwrap().len(), 2);
    a.type_into(".sub-input", "上线验证");
    a.blur();
    a.tap(".sub-add");
    a.frames(3);
    assert_eq!(a.data("card.subs").as_array().unwrap().len(), 3);
    assert_eq!(a.data("pct"), 33);
    assert_eq!(a.data("subDraft"), "");
    // 标题
    a.type_into(".f-title", "（v2）");
    a.blur();
    assert_eq!(card(&a, 2)["title"], "登录页埋点方案（v2）");
    a.back();
    assert_eq!(a.route(), "pages/board/board");
    assert_eq!(a.data("overdue"), 1, "改了截止日期，逾期少一张");
    assert_eq!(a.data("cols[0].cards[0].title"), "登录页埋点方案（v2）", "升到紧急后排第一");
    assert_eq!(a.data("cols[0].cards[0].owner"), "孙悦");
    a.assert_no_errors();
}

#[test]
fn 看板_详情页移动到已完成() {
    let _g = serial();
    let mut a = App::open(APP, None, "detail-move");
    // 进行中列只露出左边一截：点露出来的那部分
    let (x, y, _, h) = a.rect(".card-doing");
    a.tap_xy(x + 30.0, y + h / 2.0); // 订单列表虚拟滚动（逾期）
    a.settle();
    assert_eq!(a.data("card.id"), 4);
    a.tap(".btn-move");
    sheet(&mut a, 2, 3);
    assert_eq!(a.data("colName"), "已完成");
    assert_eq!(a.data("overdue"), false, "完成了就不算逾期");
    a.back();
    assert_eq!(counts(&a), [3, 3, 4]);
    a.assert_no_errors();
}

#[test]
fn 看板_新建卡片_标题必填_悬浮按钮不穿透() {
    let _g = serial();
    let mut a = App::open(APP, None, "new");
    let (fx, fy, fw, fh) = a.rect(".fab");
    let (cx, cy) = (fx + fw / 2.0, fy + fh / 2.0);
    let under = a.rects(".card").iter().any(|&(x, y, w, h)| cx > x && cx < x + w && cy > y && cy < y + h);
    assert!(under, "悬浮按钮下面要压着一张卡片，否则测不到穿透：fab={:?} cards={:?}", (fx, fy, fw, fh), a.rects(".card"));
    a.tap_settle(".fab");
    assert_eq!(a.route(), "pages/new/new", "点的是新建，不是底下的卡片");
    assert_eq!(a.data("form.col"), 0);
    assert_eq!(a.data("form.due"), day_offset(&a, 3).as_str(), "默认三天后截止");
    a.tap(".n-submit");
    assert_eq!(a.data("error"), "标题不能为空");
    assert_eq!(a.depth(), 2, "校验不过不返回");
    a.type_into(".n-title", "   ");
    a.blur();
    a.tap(".n-submit");
    assert_eq!(a.data("error"), "标题不能为空", "全是空格也不行");
    a.type_into(".n-title", "写周报");
    a.blur();
    assert_eq!(a.data("error"), "", "重新输入后清掉错误");
    a.picker(".n-col");
    a.picker_row(0, 1, 1);
    a.picker_confirm();
    assert_eq!(a.data("form.col"), 1);
    a.picker(".n-prio");
    a.picker_row(0, 1, 1);
    a.picker_confirm();
    assert_eq!(a.data("form.prio"), 2);
    a.tap(".n-submit");
    a.settle();
    assert_eq!(a.route(), "pages/board/board");
    assert_eq!(a.depth(), 1);
    assert_eq!(counts(&a), [3, 5, 3]);
    let c = card(&a, 11);
    assert_eq!(c["title"], "写周报", "标题去掉首尾空白");
    assert_eq!(c["col"], "doing");
    assert_eq!(c["prio"], 2);
    assert_eq!(c["owner"], "m1");
    a.assert_no_errors();
}

#[test]
fn 看板_列底添加_带上所在列() {
    let _g = serial();
    let mut a = App::open(APP, None, "col-add");
    a.tap_nth(".col-add", 0);
    a.settle();
    assert_eq!(a.route(), "pages/new/new");
    assert_eq!(a.data("form.col"), 0);
    a.back();
    let mut a = App::open(APP, Some("pages/new/new?col=done"), "col-add2");
    assert_eq!(a.data("form.col"), 2);
    a.frames(1);
    a.assert_no_errors();
}

#[test]
fn 看板_统计_完成率与逾期随移动变化() {
    let _g = serial();
    let mut a = App::open(APP, None, "stats");
    a.tap_settle(".go-stats");
    assert_eq!(a.route(), "pages/stats/stats");
    assert_eq!(a.data("total"), 10);
    assert_eq!(a.data("rate"), 30);
    assert_eq!(a.data("overdue"), 2);
    assert_eq!(a.data("dueToday"), 1, "新版插画今天到期");
    assert_eq!(ids(&a.data("lateList")), [2, 4]);
    assert_eq!(a.data("todayText"), day_offset(&a, 0).as_str());
    let pr: Vec<i64> = a.data("prios").as_array().unwrap().iter().map(|p| p["count"].as_i64().unwrap()).collect();
    assert_eq!(pr, [1, 3, 2, 1], "未完成按优先级 低/中/高/紧急");
    // 点逾期卡片进详情，移到已完成，回来统计跟着变
    a.tap_nth(".late-row", 1);
    a.settle();
    assert_eq!(a.data("card.id"), 4);
    a.tap(".btn-move");
    sheet(&mut a, 2, 3);
    a.back();
    assert_eq!(a.route(), "pages/stats/stats");
    assert_eq!(a.data("rate"), 40);
    assert_eq!(a.data("overdue"), 1);
    assert_eq!(a.data("cols[2].count"), 4);
    a.assert_no_errors();
}

#[test]
fn 看板_成员_任务数_展开收起() {
    let _g = serial();
    let mut a = App::open(APP, None, "members");
    a.tap_settle(".go-members");
    let order: Vec<String> = a.data("members").as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap().to_string()).collect();
    assert_eq!(order, ["m3", "m4", "m1", "m2", "m5"], "未完成多的在前");
    assert_eq!(a.data("members[1].open"), 2);
    assert_eq!(a.data("members[1].late"), 1, "赵磊有一张逾期");
    a.tap_nth(".member", 1);
    a.frames(3);
    assert_eq!(a.data("selected"), "m4");
    assert_eq!(ids(&a.data("cards")), [2, 5]);
    assert_eq!(a.count(".m-card"), 2);
    a.tap_nth(".m-card", 1);
    a.settle();
    assert_eq!(a.route(), "pages/card/card");
    assert_eq!(a.data("card.title"), "支付回调重试");
    a.back();
    assert_eq!(a.data("selected"), "m4", "返回后仍展开");
    a.tap_nth(".member", 1);
    a.frames(3);
    assert_eq!(a.data("selected"), "");
    assert_eq!(a.count(".m-card"), 0);
    a.assert_no_errors();
}

#[test]
fn 看板_删除卡片确认_弹窗不穿透() {
    let _g = serial();
    let mut a = App::open(APP, None, "delete");
    a.tap_nth(".card-todo", 2); // 图标库整理
    a.settle();
    assert_eq!(a.data("card.id"), 3);
    let (mx, my, mw, mh) = a.rect(".btn-move");
    a.tap(".btn-del");
    assert!(a.modal_open());
    a.tap_xy(mx + mw / 2.0, my + mh / 2.0); // 弹窗遮罩下的「移动到」
    a.wait_ms(400);
    assert_eq!(card(&a, 3)["col"], "todo");
    if a.modal_open() {
        a.modal_tap(false);
    }
    assert_eq!(a.route(), "pages/card/card");
    assert!(!card(&a, 3).is_null(), "取消不删");
    a.tap(".btn-del");
    a.modal_tap(true);
    assert_eq!(a.route(), "pages/board/board");
    assert!(card(&a, 3).is_null());
    assert_eq!(counts(&a), [2, 4, 3]);
    a.assert_no_errors();
}
