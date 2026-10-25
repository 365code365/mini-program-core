//! 停车宝（sample/parking）：自定义车牌键盘（fixed 底部面板、省份 → 字母数字两段、删除、新能源 8 位、
//! 点键盘不穿透到下面的车辆卡片）、我的车辆持久化、停车场列表的排序 / 筛选 / 搜索、预约费用实时计算、
//! 缴费页的时长与费用（首小时 + 每半小时、封顶、优惠券）、确认支付 → 结果页 → 订单记录。
//! 「现在」用 `?now=18:30` 注入；示例在场记录：京A12345 15:10 进 P1（首小时 6、每半小时 3、封顶 50）。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::Value;

/// 点确定，再多等面板退场：退场没走完时下一次点击会被面板吃掉（点了「下一步」没反应）
fn confirm(a: &mut App) {
    a.picker_confirm();
    a.wait_ms(250);
}

/// 面板里点一行，再等滚轮动画落定（按墙上时钟走，机器忙时 4 帧不够，确定键会读到旧值）
fn row(a: &mut App, col: usize, cols: usize, delta: i32) {
    a.picker_row(col, cols, delta);
    a.wait_ms(150);
}

/// 点开 picker。上一个面板的退场动画按墙上时钟走，机器忙时 `picker_confirm` 等的 300ms 不一定够，
/// 这时点击会被还在退场的面板吃掉：没弹出来就等一会儿再点（最多 5 次）
fn pick(a: &mut App, sel: &str) {
    for _ in 0..5 {
        a.tap(sel);
        if a.picker_open() {
            a.wait_ms(300);
            return;
        }
        a.wait_ms(250);
    }
    panic!("点 {sel} 没弹出 picker 面板");
}

const PROV: &str = "京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤青藏川宁琼";
const ALNUM: &str = "1234567890ABCDEFGHJKLMNPQRSTUVWXYZ";

/// 按键盘上的字点键（省份面板 / 字母数字面板）
fn press(a: &mut App, ch: char) {
    if let Some(i) = PROV.chars().position(|c| c == ch) {
        assert_eq!(a.data("panel"), "prov", "{ch} 在省份面板上");
        a.tap_nth(".key-prov", i);
    } else {
        let i = ALNUM.chars().position(|c| c == ch).unwrap_or_else(|| panic!("键盘上没有 {ch}"));
        assert_eq!(a.data("panel"), "alnum");
        a.tap_nth(".key-an", i);
    }
}

fn type_plate(a: &mut App, plate: &str) {
    for ch in plate.chars() {
        press(a, ch);
    }
}

fn ids(v: &Value) -> Vec<i64> {
    v.as_array().unwrap().iter().map(|l| l["id"].as_i64().unwrap()).collect()
}

fn plates(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|c| c["plate"].as_str().unwrap().to_string()).collect()
}

#[test]
fn 停车宝_每页都能打开() {
    let _g = serial();
    smoke_all_pages(
        "parking",
        &[("pages/lot/lot", "id=1"), ("pages/pay/pay", "plate=京A12345&now=18:30"), ("pages/result/result", "id=P1000"), ("pages/orders/orders", "tab=1")],
    );
}

#[test]
fn 车牌键盘_省份到字母数字_第二位只收字母_删除_添加并持久化() {
    let _g = serial();
    let mut a = App::open("parking", None, "plate");
    assert_eq!(plates(&a.data("cars")), vec!["京A12345", "沪AD12345"], "示例车辆");
    assert_eq!(a.data("showKb"), false);
    a.tap(".boxes");
    assert_eq!(a.data("showKb"), true, "点车牌格弹出自定义键盘");
    assert_eq!(a.data("panel"), "prov");
    assert_eq!(a.count(".key-prov"), 31);
    assert_eq!(a.count(".key-an"), 0);
    press(&mut a, '浙');
    assert_eq!(a.data("plate"), "浙");
    assert_eq!(a.data("panel"), "alnum", "选完省份切到字母数字");
    assert_eq!(a.count(".key-an"), 34, "去掉 I、O");
    // 第二位只能是字母：点数字没反应
    a.tap_nth(".key-an", 0);
    assert_eq!(a.data("plate"), "浙");
    type_plate(&mut a, "A12345");
    assert_eq!(a.data("plate"), "浙A12345");
    assert_eq!(a.data("boxes[6].ch"), "5");
    // 满 7 位再点不收
    press(&mut a, '9');
    assert_eq!(a.data("plate"), "浙A12345");
    a.tap(".key-del");
    assert_eq!(a.data("plate"), "浙A1234");
    press(&mut a, 'B');
    assert_eq!(a.data("plate"), "浙A1234B");
    // 删到空：回到省份面板
    for _ in 0..7 {
        a.tap(".key-del");
    }
    assert_eq!(a.data("plate"), "");
    assert_eq!(a.data("panel"), "prov");
    type_plate(&mut a, "浙A1234");
    // 不完整：校验失败
    a.tap(".add-car");
    assert_eq!(a.data("err"), "车牌号不完整，还差 1 位");
    assert_eq!(a.data("cars").as_array().unwrap().len(), 2);
    press(&mut a, 'B');
    a.tap(".add-car");
    a.frames(3);
    assert_eq!(a.data("err"), "");
    assert_eq!(a.data("showKb"), false, "添加后收起键盘");
    assert_eq!(a.data("plate"), "");
    assert_eq!(plates(&a.data("cars")), vec!["京A12345", "沪AD12345", "浙A1234B"]);
    assert_eq!(plates(&a.storage("pk_cars")), vec!["京A12345", "沪AD12345", "浙A1234B"]);
    // 重复添加
    a.tap(".boxes");
    type_plate(&mut a, "京A12345");
    a.tap(".add-car");
    assert_eq!(a.data("err"), "已添加过 京A12345");
    a.tap(".kb-done");
    assert_eq!(a.data("showKb"), false);
    // 重进：车辆还在；解绑走确认弹窗
    a.js("wx.reLaunch({ url: '/pages/index/index' })");
    a.settle();
    assert_eq!(a.data("cars").as_array().unwrap().len(), 3);
    a.tap_nth(".car-del", 2);
    a.frames(3);
    a.modal_tap(true);
    assert_eq!(plates(&a.storage("pk_cars")), vec!["京A12345", "沪AD12345"]);
    assert_eq!(a.route(), "pages/index/index", "解绑是 catchtap，不会顺带点进车辆");
    a.assert_no_errors();
}

#[test]
fn 车牌键盘_新能源8位切换() {
    let _g = serial();
    let mut a = App::open("parking", None, "energy");
    a.tap(".ne-chip");
    assert_eq!(a.data("newEnergy"), true);
    assert_eq!(a.data("maxLen"), 8);
    assert_eq!(a.count(".box"), 8);
    a.tap(".boxes");
    type_plate(&mut a, "粤BD12345");
    assert_eq!(a.data("plate"), "粤BD12345");
    assert_eq!(a.data("boxes[7].ch"), "5");
    // 切回普通车牌：第 8 位截掉
    a.tap(".kb-done");
    a.tap(".ne-chip");
    assert_eq!(a.data("maxLen"), 7);
    assert_eq!(a.data("plate"), "粤BD1234");
    assert_eq!(a.count(".box"), 7);
    a.tap(".ne-chip");
    a.tap(".boxes");
    press(&mut a, '6');
    assert_eq!(a.data("plate"), "粤BD12346");
    a.tap(".add-car");
    a.frames(3);
    let cars = a.storage("pk_cars");
    let last = cars.as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["plate"], "粤BD12346");
    assert_eq!(last["newEnergy"], true);
    a.assert_no_errors();
}

#[test]
fn 车牌键盘弹着时_点键盘不穿透到下面的车辆卡片() {
    let _g = serial();
    let mut a = App::open("parking", None, "through");
    let cars = a.rects(".car");
    assert_eq!(cars.len(), 2);
    a.tap(".boxes");
    type_plate(&mut a, "京A");
    let kb = a.rect(".kb");
    let keys = a.rects(".key-an");
    // 找一个与车辆卡片重叠的键，点在两者重叠区的中心
    let hit = keys.iter().enumerate().find_map(|(i, &(x, y, w, h))| {
        cars.iter().find_map(|&(l, t, cw, ch)| {
            let (x0, y0, x1, y1) = (x.max(l), y.max(t), (x + w).min(l + cw), (y + h).min(t + ch));
            (x1 - x0 > 4.0 && y1 - y0 > 4.0).then_some((i, (x0 + x1) / 2.0, (y0 + y1) / 2.0))
        })
    });
    let (i, px, py) = hit.unwrap_or_else(|| panic!("键盘 {kb:?} 没盖住车辆卡片 {cars:?}，测不到穿透"));
    let k = ALNUM.chars().nth(i).unwrap();
    a.tap_xy(px, py);
    a.settle();
    assert_eq!(a.data("plate"), format!("京A{k}"), "键按下去了");
    assert_eq!(a.data("carTaps"), 0, "下面的车辆卡片没收到 tap");
    assert_eq!(a.route(), "pages/index/index");
    assert_eq!(a.depth(), 1, "没有跳到缴费页");
    // 键盘本身的空隙（键与键之间的底色）也不穿透
    let (x, _, w, _) = keys[i];
    a.tap_xy(x + w + 1.0, py);
    a.settle();
    assert_eq!(a.data("plate"), format!("京A{k}"), "空隙不是键");
    assert_eq!(a.data("carTaps"), 0);
    assert_eq!(a.depth(), 1);
    // 收起键盘后同一位置点得到车辆卡片：证明下面确实有可点的东西
    a.tap(".kb-done");
    a.frames(3);
    a.tap_xy(px, py);
    a.settle();
    assert_eq!(a.route(), "pages/pay/pay", "键盘收起后点到车辆 → 缴费页");
    assert_eq!(a.depth(), 2);
    assert_eq!(a.val("getCurrentPages()[0].data.carTaps"), 1);
    a.assert_no_errors();
}

#[test]
fn 停车场列表_排序_只看有空位_搜索() {
    let _g = serial();
    let mut a = App::open("parking", None, "lots");
    a.tap_settle(".go-lots");
    assert_eq!(a.route(), "pages/lots/lots");
    assert_eq!(ids(&a.data("list")), vec![1, 4, 2, 6, 3, 5], "默认按距离");
    assert_eq!(a.data("list[0].distText"), "400m");
    assert_eq!(a.data("list[2].distText"), "1.2km");
    a.tap(".sort-price");
    assert_eq!(a.data("sort"), "price");
    assert_eq!(ids(&a.data("list")), vec![5, 3, 2, 1, 4, 6], "按首小时价格");
    a.tap(".only-free");
    assert_eq!(a.data("onlyFree"), true);
    assert_eq!(ids(&a.data("list")), vec![5, 3, 1, 4], "去掉已满的 2、6");
    a.type_into(".search", "停车场");
    a.blur();
    assert_eq!(a.data("keyword"), "停车场");
    assert_eq!(ids(&a.data("list")), vec![5, 1]);
    assert_eq!(a.count(".lot"), 2);
    a.tap(".only-free");
    assert_eq!(ids(&a.data("list")), vec![5, 1, 6], "放开空位筛选，已满的万象城回来");
    a.tap(".s-clear");
    assert_eq!(a.data("keyword"), "");
    a.tap(".sort-dist");
    assert_eq!(ids(&a.data("list")), vec![1, 4, 2, 6, 3, 5]);
    a.tap_nth(".lot", 1);
    a.settle();
    assert_eq!(a.route(), "pages/lot/lot");
    assert_eq!(a.data("lot.id"), 4);
    assert_eq!(a.data("free"), 5);
    a.assert_no_errors();
}

#[test]
fn 停车场详情_预约费用实时计算_预约与取消() {
    let _g = serial();
    let mut a = App::open("parking", Some("pages/lot/lot?id=1"), "reserve");
    assert_eq!(a.data("free"), 36);
    assert_eq!(a.data("rules"), serde_json::json!(["15 分钟内免费", "首小时 6 元", "之后每半小时 3 元", "24 小时封顶 50 元"]));
    // 默认 18:00 到达、2 小时：6 + 2×3
    assert_eq!(a.data("durIdx"), 1);
    assert_eq!(a.data("feeText"), "¥12.00");
    assert_eq!(a.data("leaveText"), "20:00");
    pick(&mut a, ".pk-dur");
    row(&mut a, 0, 1, 1);
    confirm(&mut a);
    assert_eq!(a.data("durIdx"), 2);
    assert!(a.data("durIdx").is_number());
    assert_eq!(a.data("feeText"), "¥18.00", "3 小时：6 + 4×3");
    pick(&mut a, ".pk-dur");
    row(&mut a, 0, 1, 2);
    confirm(&mut a);
    assert_eq!(a.data("feeText"), "¥48.00", "8 小时：6 + 14×3");
    pick(&mut a, ".pk-dur");
    row(&mut a, 0, 1, 1);
    confirm(&mut a);
    assert_eq!(a.data("durIdx"), 5);
    assert_eq!(a.data("fee"), 50, "12 小时 72 元，封顶 50");
    pick(&mut a, ".pk-arrive");
    row(&mut a, 0, 2, 2);
    confirm(&mut a);
    assert_eq!(a.data("arrive"), "20:00");
    assert_eq!(a.data("leaveText"), "08:00", "跨零点");
    // 车辆：第二辆
    pick(&mut a, ".pk-car");
    row(&mut a, 0, 1, 1);
    confirm(&mut a);
    assert_eq!(a.data("carIdx"), 1);
    a.tap(".reserve");
    a.frames(3);
    assert!(a.modal_open(), "预约先弹确认");
    a.modal_tap(true);
    assert_eq!(a.route(), "pages/orders/orders");
    assert_eq!(a.data("tab"), 0);
    let first = a.data("list[2]");
    assert_eq!(first["id"], "R1002");
    assert_eq!(first["plate"], "沪AD12345");
    assert_eq!(first["amountText"], "¥50.00");
    assert_eq!(first["statusText"], "已预约");
    assert_eq!(a.data("counts"), serde_json::json!([3, 1]), "进行中：两辆在场 + 一条预约");
    assert_eq!(a.storage("pk_free")["1"], 35, "占掉一个车位");
    // 取消预约
    a.tap(".o-cancel");
    a.frames(3);
    a.modal_tap(true);
    assert_eq!(a.data("counts"), serde_json::json!([2, 2]));
    assert_eq!(a.storage("pk_orders")[0]["statusText"], "已取消");
    assert_eq!(a.storage("pk_free")["1"], 36, "车位还回去");
    a.tap_nth(".tab", 1);
    assert_eq!(a.data("list[0].statusText"), "已取消");
    a.back();
    assert_eq!(a.route(), "pages/lot/lot");
    assert_eq!(a.data("free"), 36, "回到详情 onShow 刷新空位");
    a.assert_no_errors();
}

#[test]
fn 已满的停车场不能预约() {
    let _g = serial();
    let mut a = App::open("parking", Some("pages/lot/lot?id=2"), "full");
    assert_eq!(a.data("free"), 0);
    a.tap(".reserve");
    a.frames(3);
    assert!(!a.modal_open(), "没空位不弹确认");
    assert_eq!(a.data("err"), "暂无空位，无法预约");
    a.assert_no_errors();
}

#[test]
fn 缴费_停车时长与费用_封顶_优惠券() {
    let _g = serial();
    let mut a = App::open("parking", Some("pages/pay/pay?plate=京A12345&now=18:30"), "fee");
    assert_eq!(a.data("plate"), "京A12345");
    assert_eq!(a.data("inTime"), "15:10");
    // 15:10 → 18:30 = 200 分钟：6 + ⌈140/30⌉×3 = 21
    assert_eq!(a.data("minutes"), 200);
    assert_eq!(a.data("durText"), "3小时20分钟");
    assert_eq!(a.data("feeText"), "¥21.00");
    assert_eq!(a.data("payText"), "¥21.00");
    pick(&mut a, ".pk-coupon");
    row(&mut a, 0, 1, 1);
    confirm(&mut a);
    assert_eq!(a.data("couponIdx"), 1);
    assert_eq!(a.data("payText"), "¥18.00", "满 10 减 3");
    pick(&mut a, ".pk-coupon");
    row(&mut a, 0, 1, 2);
    confirm(&mut a);
    assert_eq!(a.data("couponIdx"), 3);
    assert_eq!(a.data("off"), 6, "首小时免费抵 6 元");
    assert_eq!(a.data("payText"), "¥15.00");
    // 换成另一辆：市民中心 09:40 入场，530 分钟 → 4 + 16×2 = 36，封顶 30；首小时抵 4
    a.tap_nth(".car-radio", 1);
    a.frames(3);
    assert_eq!(a.data("plate"), "沪AD12345", "radio-group 的 value 是车牌字符串");
    assert_eq!(a.data("durText"), "8小时50分钟");
    assert_eq!(a.data("fee"), 30);
    assert_eq!(a.data("payText"), "¥26.00");
    a.tap_nth(".car-radio", 0);
    a.frames(3);
    // 入场时间校正到 17:10：80 分钟 → 6 + 3 = 9，满 10 减 3 不满足
    pick(&mut a, ".pk-in");
    row(&mut a, 0, 2, 2);
    confirm(&mut a);
    assert_eq!(a.data("inTime"), "17:10");
    assert_eq!(a.data("fee"), 9);
    assert_eq!(a.data("payText"), "¥3.00");
    pick(&mut a, ".pk-coupon");
    row(&mut a, 0, 1, -2);
    confirm(&mut a);
    assert_eq!(a.data("couponIdx"), 0, "不满足条件的券退回不使用");
    assert_eq!(a.data("payText"), "¥9.00");
    // 18:15 入场：15 分钟内免费
    pick(&mut a, ".pk-in");
    row(&mut a, 0, 2, 1);
    row(&mut a, 1, 2, 2);
    row(&mut a, 1, 2, 2);
    row(&mut a, 1, 2, 1);
    confirm(&mut a);
    assert_eq!(a.data("inTime"), "18:15");
    assert_eq!(a.data("fee"), 0);
    assert_eq!(a.data("tip"), "15 分钟内免费离场");
    a.assert_no_errors();
}

#[test]
fn 缴费_确认支付_结果页_订单记录() {
    let _g = serial();
    let mut a = App::open("parking", Some("pages/pay/pay?plate=京A12345&now=18:30"), "pay");
    pick(&mut a, ".pk-coupon");
    row(&mut a, 0, 1, 1);
    confirm(&mut a);
    // 先取消：不扣费
    a.tap(".pay-btn");
    a.frames(3);
    a.modal_tap(false);
    assert_eq!(a.route(), "pages/pay/pay");
    // 订单是懒初始化的：取消支付后连 pk_orders 都没写过
    assert!(a.storage("pk_orders").as_array().map_or(true, |l| l.iter().all(|o| o["id"] != "P1002")), "取消不生成订单");
    a.tap(".pay-btn");
    a.frames(3);
    a.modal_tap(true);
    a.settle();
    assert_eq!(a.route(), "pages/result/result");
    // redirectTo 应替换缴费页（栈深 1），但栈底页面 redirectTo 时引擎不出栈，见下面 ignore 的用例
    assert_eq!(a.val("getCurrentPages().slice(-1)[0].route"), "pages/result/result");
    assert_eq!(a.data("order.id"), "P1002");
    assert_eq!(a.data("amountText"), "¥18.00");
    assert_eq!(a.data("offText"), "-¥3.00");
    let o = a.storage("pk_orders")[0].clone();
    assert_eq!(o["amount"], 18);
    assert_eq!(o["fee"], 21);
    assert_eq!(o["coupon"], "满 10 减 3");
    assert_eq!(plates(&a.storage("pk_sessions")), vec!["沪AD12345"], "缴完费离场");
    assert_eq!(a.storage("pk_free")["1"], 37);
    a.tap_settle(".go-orders");
    assert_eq!(a.route(), "pages/orders/orders");
    assert_eq!(a.data("tab"), 1);
    assert_eq!(a.data("list[0].id"), "P1002");
    assert_eq!(a.data("list[1].id"), "P1000");
    a.tap_nth(".tab", 0);
    assert_eq!(a.data("list").as_array().unwrap().len(), 1);
    assert_eq!(a.data("list[0].plate"), "沪AD12345");
    // 进行中的车点「去缴费」
    a.tap_settle(".go-pay");
    assert_eq!(a.route(), "pages/pay/pay");
    assert_eq!(a.data("plate"), "沪AD12345");
    // 重进首页：京A12345 已不在场
    a.js("wx.reLaunch({ url: '/pages/index/index' })");
    a.settle();
    assert_eq!(a.data("cars[0].where"), "未在场");
    assert_eq!(a.data("cars[1].where"), "市民中心 P3 停车楼 · 09:40 入场");
    a.assert_no_errors();
}


/// 栈底那一页 `wx.redirectTo`：微信替换当前页（栈深仍是 1），引擎把旧页留在了栈里（栈深 2）。
/// 栈深 ≥2 时 redirectTo 正常替换（首页 navigateTo 进缴费页再 redirectTo，栈是 [index, result]）。
#[test]
fn 回归_栈底页面redirectTo不替换() {
    let _g = serial();
    let mut a = App::open("parking", Some("pages/pay/pay?plate=京A12345&now=18:30"), "redirect");
    assert_eq!(a.depth(), 1);
    a.js("wx.redirectTo({ url: '/pages/result/result?id=P1000' })");
    a.settle();
    assert_eq!(a.route(), "pages/result/result");
    assert_eq!(a.val("getCurrentPages().map(function (p) { return p.route })"), serde_json::json!(["pages/result/result"]));
    a.assert_no_errors();
}
