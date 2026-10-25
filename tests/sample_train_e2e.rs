//! 畅行火车票（sample/train）：每页能打开；城市选择 + 一键互换、date picker + 只看高铁 switch、
//! 车次筛选（时段多选 / 排序 / 前后一天）、乘车人表单校验（toast）、选座下单（radio / checkbox / 座位按钮 / 联动总价 / showModal）、
//! 订单 storage 持久化与取消，固定底栏不穿透。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::Value;

/// 记下每次 wx.showToast 的标题（toast 只在画面上，逻辑层留个底给断言用）
fn hook_toast(a: &App) {
    a.js("globalThis.__toasts = []; (function () { var o = wx.showToast; wx.showToast = function (opt) { __toasts.push(opt && opt.title); return o.apply(this, arguments); }; })()");
}

fn toasts(a: &App) -> Vec<String> {
    a.val("globalThis.__toasts || []").as_array().map(|v| v.iter().filter_map(|s| s.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

fn last_toast(a: &App) -> String {
    toasts(a).last().cloned().unwrap_or_default()
}

/// 逻辑层的「今天 + n 天」，YYYY-MM-DD（与示例同一套本地时区算法）
fn day(a: &App, n: i32) -> String {
    a.val(&format!(
        "(function () {{ var d = new Date(); d.setDate(d.getDate() + {n}); var p = function (x) {{ return x < 10 ? '0' + x : '' + x; }}; return d.getFullYear() + '-' + p(d.getMonth() + 1) + '-' + p(d.getDate()); }})()"
    ))
    .as_str()
    .unwrap()
    .to_string()
}

fn arr(v: Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

/// 把 `sel` 的第 `i` 个滚到固定底栏（120rpx = 60px）上方再点
fn tap_above_footer(a: &mut App, sel: &str, i: usize) {
    for _ in 0..6 {
        let (_, y, _, h) = a.rects(sel)[i];
        if y + h / 2.0 < H - 90.0 {
            break;
        }
        a.scroll_by(W / 2.0, H / 2.0, (y + h / 2.0 - H / 2.0).min(300.0));
    }
    a.tap_nth(sel, i);
}

/// 不带惯性地把页面往上滚 `dy`：慢慢拖、到位后按住停几帧再抬手（速度归零）
fn scroll_exact(a: &mut App, dy: f32) {
    let (x, y) = (W / 2.0, H / 2.0);
    a.e.pointer_down(x, y);
    a.frames(1);
    for i in 1..=30 {
        a.e.pointer_move(x, y - dy * i as f32 / 30.0);
        a.frames(1);
    }
    a.wait_ms(200);
    a.e.pointer_up(x, y - dy);
    a.wait_ms(300);
}

#[test]
fn 火车票_每页都能打开() {
    let _g = serial();
    smoke_all_pages(
        "train",
        &[("pages/city/city", "field=to&current=上海"), ("pages/list/list", "from=北京&to=上海"), ("pages/book/book", "no=")],
    );
}

#[test]
fn 火车票_选城市_一键互换_同城自动对调() {
    let _g = serial();
    let mut a = App::open("train", None, "city");
    assert_eq!((a.data("from"), a.data("to")), ("北京".into(), "上海".into()));
    a.tap_settle(".city");
    assert_eq!(a.route(), "pages/city/city");
    assert_eq!(a.data("field"), "from");
    a.tap_nth(".chip.hot", 4); // 热门城市第 5 个：杭州
    a.settle();
    assert_eq!(a.route(), "pages/index/index");
    assert_eq!(a.data("from"), "杭州");
    a.tap_settle(".city-right");
    assert_eq!(a.data("field"), "to");
    a.type_into(".search-input", "成");
    a.blur();
    assert_eq!(a.data("results"), serde_json::json!(["成都"]));
    a.tap_settle(".res-item");
    assert_eq!(a.data("to"), "成都");
    a.tap(".swap");
    assert_eq!((a.data("from"), a.data("to")), ("成都".into(), "杭州".into()), "一键互换");
    assert_eq!(a.storage("tr_query")["from"], "成都", "查询条件存起来了");
    // 到达城市选成和出发一样的：两端对调
    a.tap_settle(".city-right");
    a.type_into(".search-input", "成都");
    a.blur();
    a.tap_settle(".res-item");
    assert_eq!((a.data("from"), a.data("to")), ("杭州".into(), "成都".into()));
    a.assert_no_errors();
}

/// 城市页的字母索引：点 X，scroll-view 用 scroll-into-view 滚到 X 组。
/// scroll-view 的高度写的是 `calc(100vh - 104rpx)`（顶部搜索栏 104rpx），引擎把 vh 与 px/rpx 混算的 calc 丢掉了，
/// scroll-view 退化成内容高、整页滚动，于是 scroll-into-view 无处可滚
#[test]
fn 火车票_城市页字母索引_scroll_into_view() {
    let _g = serial();
    let mut a = App::open("train", Some("pages/city/city?field=from"), "city-index");
    let (_, _, _, bh) = a.rect(".body");
    assert!(bh < H, "scroll-view 高度受限在视口内：{bh}");
    let before = a.rect("#gX").1;
    let n = a.count(".idx");
    a.tap_nth(".idx", n - 2);
    a.wait_ms(300);
    assert_eq!(a.data("toView"), "gX");
    let after = a.rect("#gX").1;
    assert!(after < before - 100.0, "X 组滚上来了：{before} → {after}");
    a.assert_no_errors();
}

#[test]
fn 火车票_选日期_只看高铁_查询() {
    let _g = serial();
    let mut a = App::open("train", None, "query");
    let today = day(&a, 0);
    assert_eq!(a.data("date"), today.as_str());
    assert!(a.data("dateText").as_str().unwrap().ends_with("今天"));
    a.picker(".date-picker");
    a.picker_row(2, 3, 1); // 日那一列往下一行
    a.picker_confirm();
    let d = a.data("date");
    assert!(d.is_string(), "date 模式的 e.detail.value 是字符串：{d}");
    let tomorrow = day(&a, 1);
    if tomorrow[..7] == today[..7] {
        assert_eq!(d, tomorrow.as_str());
        assert!(a.data("dateText").as_str().unwrap().ends_with("明天"));
    } else {
        assert_ne!(d, today.as_str(), "月底：日那一列换了一天");
    }
    assert_eq!(a.data("highSpeed"), false);
    a.tap(".hs-switch");
    assert_eq!(a.data("highSpeed"), true, "switch 的 e.detail.value 是布尔");
    a.tap_settle(".search-btn");
    assert_eq!(a.route(), "pages/list/list");
    assert_eq!(a.data("date"), d);
    assert_eq!(a.data("highSpeed"), true);
    let trains = arr(a.data("trains"));
    assert_eq!(trains.len(), 12, "16 趟里 4 趟普速被滤掉");
    assert!(trains.iter().all(|t| t["type"] == "G" || t["type"] == "D"));
    assert_eq!(a.storage("tr_history"), serde_json::json!(["北京-上海"]));
    // 起终点相同：toast 拦下
    a.back();
    hook_toast(&a);
    a.js("getCurrentPages()[0].setData({ to: '北京' })");
    a.tap(".search-btn");
    a.settle();
    assert_eq!(a.route(), "pages/index/index");
    assert_eq!(last_toast(&a), "出发地和目的地不能相同");
    a.assert_no_errors();
}

#[test]
fn 火车票_车次筛选时段多选_排序_前后一天() {
    let _g = serial();
    let mut a = App::open("train", Some("pages/list/list?from=南京&to=武汉"), "filter");
    let today = day(&a, 0);
    assert_eq!(a.data("total"), 16);
    assert_eq!(a.data("canPrev"), false);
    a.tap(".prev-day");
    assert_eq!(a.data("date"), today.as_str(), "今天不能再往前");
    a.tap(".period"); // 00:00-06:00：只有 01:00 那趟普速
    assert_eq!(a.data("periods[0].on"), true);
    assert_eq!(a.data("total"), 1);
    a.tap_nth(".period", 3); // 再加 18:00-24:00
    let deps: Vec<i64> = arr(a.data("trains")).iter().map(|t| t["dep"].as_i64().unwrap()).collect();
    assert_eq!(deps, vec![60, 1080 + 60, 1230, 1320], "两个时段取并集、按出发早排");
    a.tap(".hs-chip");
    assert_eq!(a.data("total"), 2, "再叠只看高铁：19:00 D、22:00 G");
    a.tap(".hs-chip");
    a.tap(".period");
    a.tap_nth(".period", 3);
    assert_eq!(a.data("total"), 16, "取消所有时段 = 不限");
    a.tap_nth(".sort", 1);
    assert_eq!(a.data("sort"), 1);
    let durs: Vec<i64> = arr(a.data("trains")).iter().map(|t| t["dur"].as_i64().unwrap()).collect();
    assert!(durs.windows(2).all(|w| w[0] <= w[1]), "耗时短：{durs:?}");
    a.tap_nth(".sort", 2);
    let prices: Vec<i64> = arr(a.data("trains")).iter().map(|t| t["minPrice"].as_i64().unwrap()).collect();
    assert!(prices.windows(2).all(|w| w[0] <= w[1]), "价格低：{prices:?}");
    // 每趟车都有席别价格与余票文案
    let t0 = &arr(a.data("trains"))[0];
    assert_eq!(t0["seats"].as_array().unwrap().len(), 3);
    assert!(t0["seats"][0]["price"].is_number() && t0["seats"][0]["leftText"].is_string());
    a.tap(".next-day");
    assert_eq!(a.data("date"), day(&a, 1).as_str());
    assert_eq!(a.data("canPrev"), true);
    assert_eq!(a.data("sort"), 2, "换日期保留排序");
    a.tap(".prev-day");
    assert_eq!(a.data("date"), today.as_str());
    a.assert_no_errors();
}

#[test]
fn 火车票_添加乘车人_逐项校验_toast提示() {
    let _g = serial();
    let mut a = App::open("train", Some("pages/book/book"), "passenger");
    assert_eq!(a.data("count"), 1, "默认勾本人");
    a.tap_settle(".add-p");
    assert_eq!(a.route(), "pages/passenger/passenger");
    hook_toast(&a);
    a.tap(".save-btn");
    assert_eq!(last_toast(&a), "请填写姓名");
    assert_eq!(a.data("errField"), "name");
    a.type_into(".name-input", "Tom");
    a.blur();
    a.tap(".save-btn");
    assert_eq!(last_toast(&a), "姓名需为 2~15 个汉字");
    a.js("getCurrentPages().slice(-1)[0].setData({ name: '' })");
    a.type_into(".name-input", "王小明");
    a.blur();
    assert_eq!(a.data("name"), "王小明");
    a.tap(".save-btn");
    assert_eq!(last_toast(&a), "请填写身份证号");
    a.type_into(".id-input", "4403042001081545");
    a.blur();
    a.tap(".save-btn");
    assert_eq!(last_toast(&a), "身份证号应为 18 位");
    assert_eq!(a.data("errField"), "idcard");
    // 点输入框中间光标落在点到的字后面（与真机一致），所以清空后整串重输
    a.js("getCurrentPages().slice(-1)[0].setData({ idcard: '' })");
    a.type_into(".id-input", "440304200108154562");
    a.blur();
    assert_eq!(a.data("idcard"), "440304200108154562");
    a.tap(".save-btn");
    assert_eq!(last_toast(&a), "身份证号校验位不正确");
    a.js("getCurrentPages().slice(-1)[0].setData({ idcard: '440304200108154563' })");
    a.type_into(".phone-input", "1381234");
    a.blur();
    a.tap(".save-btn");
    assert_eq!(last_toast(&a), "手机号格式不正确");
    assert_eq!(a.data("errField"), "phone");
    a.js("getCurrentPages().slice(-1)[0].setData({ phone: '' })");
    a.type_into(".phone-input", "13812345678");
    a.blur();
    assert_eq!(a.data("phone"), "13812345678");
    // 与本人身份证号重复：格式都对了再查重
    a.js("getCurrentPages().slice(-1)[0].setData({ idcard: '330106199003071236' })");
    a.tap(".save-btn");
    assert_eq!(last_toast(&a), "该乘车人已存在");
    a.js("getCurrentPages().slice(-1)[0].setData({ idcard: '440304200108154563' })");
    a.picker(".type-row");
    a.picker_row(0, 1, 1);
    a.picker_confirm();
    assert_eq!(a.data("typeIdx"), 1, "selector 的下标是 number");
    a.tap(".save-btn");
    a.settle();
    assert_eq!(last_toast(&a), "已添加");
    assert_eq!(a.route(), "pages/book/book");
    let ps = arr(a.storage("tr_passengers"));
    assert_eq!(ps.len(), 2);
    assert_eq!(ps[1]["name"], "王小明");
    assert_eq!(ps[1]["type"], "学生票");
    assert_eq!(a.data("count"), 2, "新加的乘车人自动勾上");
    assert_eq!(a.data("passengers[1].idText"), "4403**********4563");
    a.assert_no_errors();
}

const EXTRA_PASSENGERS: &str = "[{ id: 1, name: '李明', idcard: '330106199003071236', phone: '13812345678', self: true }, \
{ id: 2, name: '张三', idcard: '11010519491231002X', phone: '13900001111', self: false }, \
{ id: 3, name: '王小明', idcard: '440304200108154563', phone: '13700002222', self: false }, \
{ id: 4, name: '赵雪', idcard: '11010519491231002X', phone: '13600003333', self: false }]";

/// 从车次列表点进第一趟能选座（非普速）、每个席别都有 ≥2 张余票的车
fn open_book(a: &mut App) -> Value {
    let trains = arr(a.data("trains"));
    let i = trains.iter().position(|t| t["type"] != "K" && t["seats"].as_array().unwrap().iter().all(|s| s["left"].as_i64().unwrap() >= 2)).expect("有余票充足的车次");
    for _ in 0..8 {
        let (_, y, _, h) = a.rects(".train")[i];
        if y + h / 2.0 < H - 40.0 {
            break;
        }
        a.scroll_by(W / 2.0, H / 2.0, 300.0);
    }
    a.tap_nth(".train", i);
    a.settle();
    assert_eq!(a.route(), "pages/book/book");
    trains[i].clone()
}

#[test]
fn 火车票_选席别_勾乘车人_选座_总价联动_确认后下单() {
    let _g = serial();
    let mut a = App::open("train", Some("pages/list/list?from=北京&to=上海"), "book");
    a.js(&format!("wx.setStorageSync('tr_passengers', {EXTRA_PASSENGERS}.slice(0, 2))"));
    let t = open_book(&mut a);
    assert_eq!(a.data("train.no"), t["no"]);
    assert_eq!(a.data("seatIdx"), 0);
    assert_eq!(a.data("count"), 1, "首次进来只勾本人");
    let seats = arr(t["seats"].clone());
    let p0 = seats[0]["price"].as_i64().unwrap();
    assert_eq!(a.data("total"), p0);
    // 席别 radio：换到第 3 个
    a.tap_nth(".seat-opt", 2);
    assert_eq!(a.data("seatIdx"), 2, "radio-group 的值转成 number 下标");
    let p2 = seats[2]["price"].as_i64().unwrap();
    assert_eq!(a.data("price"), p2);
    assert_eq!(a.data("total"), p2);
    // 乘车人 checkbox：勾上张三
    tap_above_footer(&mut a, ".p-item", 1);
    assert_eq!(a.data("passengers[1].checked"), true);
    assert_eq!(a.data("count"), 2);
    assert_eq!(a.data("total"), p2 * 2, "总价 = 单价 × 人数");
    // 选座：两人两个座，第三下顶掉最早选的
    tap_above_footer(&mut a, ".letter", 0);
    a.tap_nth(".letter", 4);
    assert_eq!(a.data("chosen"), serde_json::json!(["A", "F"]));
    a.tap_nth(".letter", 2);
    assert_eq!(a.data("chosen"), serde_json::json!(["F", "C"]));
    a.tap_nth(".letter", 2);
    assert_eq!(a.data("chosen"), serde_json::json!(["F"]), "再点一次取消");
    a.tap_nth(".letter", 2);
    // 取消勾选张三：人数变 1，座位只留第一个
    tap_above_footer(&mut a, ".p-item", 1);
    assert_eq!(a.data("count"), 1);
    assert_eq!(a.data("chosen"), serde_json::json!(["F"]));
    tap_above_footer(&mut a, ".p-item", 1);
    a.tap_nth(".letter", 2);
    assert_eq!(a.data("chosen"), serde_json::json!(["F", "C"]));
    // 提交：先取消，不下单
    a.tap(".submit");
    assert!(a.modal_open(), "提交前 showModal 确认");
    a.modal_tap(false);
    assert_eq!(a.route(), "pages/book/book");
    assert_eq!(a.storage("tr_orders"), Value::String(String::new()));
    a.tap(".submit");
    a.modal_tap(true);
    a.settle();
    assert_eq!(a.route(), "pages/orders/orders", "下单后切到订单 tab");
    assert_eq!(a.depth(), 1, "switchTab 关掉了非 tab 页");
    let orders = arr(a.storage("tr_orders"));
    assert_eq!(orders.len(), 1);
    let o = &orders[0];
    assert_eq!(o["no"], t["no"]);
    assert_eq!(o["seat"], seats[2]["name"]);
    assert_eq!(o["count"], 2);
    assert_eq!(o["total"], p2 * 2);
    assert_eq!(o["names"], serde_json::json!(["李明", "张三"]));
    assert_eq!(o["seats"], serde_json::json!(["F", "C"]));
    assert_eq!(a.data("counts"), serde_json::json!([1, 0]));
    assert_eq!(a.data("list[0].stateText"), "待出行");
    a.assert_no_errors();
}

#[test]
fn 火车票_订单取消要确认_待出行已完成切换_持久化() {
    let _g = serial();
    let mut a = App::open("train", Some("pages/orders/orders"), "orders");
    assert_eq!(a.data("counts"), serde_json::json!([0, 0]));
    assert!(a.exists(".empty"));
    let today = day(&a, 0);
    let past = day(&a, -3);
    a.js(&format!(
        "wx.setStorageSync('tr_orders', [\
{{ id: 'E1', no: 'G101', from: '北京', to: '上海', date: '{today}', depText: '08:00', arrText: '12:30', seat: '二等座', price: 553, count: 1, total: 553, names: ['李明'], seats: ['A'], status: 'paid' }}, \
{{ id: 'E2', no: 'D305', from: '北京', to: '天津', date: '{past}', depText: '09:10', arrText: '09:45', seat: '一等座', price: 88, count: 2, total: 176, names: ['李明', '张三'], seats: [], status: 'paid' }}])"
    ));
    a.js("getCurrentPages()[0].onShow()");
    a.frames(2);
    assert_eq!(a.data("counts"), serde_json::json!([1, 1]), "发车日已过的算已完成");
    assert_eq!(a.data("list[0].no"), "G101");
    a.tap(".cancel-btn");
    assert!(a.modal_open());
    a.modal_tap(false);
    assert_eq!(a.data("counts"), serde_json::json!([1, 1]), "「再想想」不取消");
    a.tap(".cancel-btn");
    a.modal_tap(true);
    assert_eq!(a.data("counts"), serde_json::json!([0, 2]));
    assert_eq!(a.storage("tr_orders")[0]["status"], "cancelled");
    a.tap_nth(".tab", 1);
    assert_eq!(a.data("tab"), 1);
    let states: Vec<Value> = arr(a.data("list")).iter().map(|o| o["stateText"].clone()).collect();
    assert_eq!(states, vec![Value::from("已取消"), Value::from("已出行")]);
    assert_eq!(a.count(".cancel-btn"), 0, "已完成的不能再取消");
    // 重启到订单页：从 storage 读回来
    a.js("wx.reLaunch({ url: '/pages/orders/orders' })");
    a.settle();
    assert_eq!(a.data("tab"), 0);
    assert_eq!(a.data("counts"), serde_json::json!([0, 2]));
    // 「我的」页：累计只算没取消的
    a.js("wx.switchTab({ url: '/pages/mine/mine' })");
    a.settle();
    assert_eq!(a.data("orderCount"), 1);
    assert_eq!(a.data("spent"), 176);
    a.assert_no_errors();
}

#[test]
fn 火车票_固定底栏盖住座位按钮时点底栏不穿透() {
    let _g = serial();
    let mut a = App::open("train", Some("pages/list/list?from=北京&to=上海"), "footer");
    a.js(&format!("wx.setStorageSync('tr_passengers', {EXTRA_PASSENGERS})"));
    open_book(&mut a);
    let (_, fy, _, fh) = a.rect(".footer");
    let fc = fy + fh / 2.0;
    let (_, ly, _, lh) = a.rect(".letter");
    let d = ly + lh / 2.0 - fc;
    assert!(d > -lh / 2.0, "4 位乘车人时座位按钮在底栏或更下面：{d}");
    if d.abs() > 4.0 {
        scroll_exact(&mut a, d);
    }
    let (lx, ly, lw, lh) = a.rect(".letter");
    assert!((ly + lh / 2.0 - fc).abs() < fh / 2.0, "座位按钮 A 压在底栏下：{ly} vs 底栏 {fy}");
    a.tap_xy(lx + lw / 2.0, ly + lh / 2.0);
    assert_eq!(a.data("chosen"), serde_json::json!([]), "点的是底栏，不是下面的座位按钮");
    assert_eq!(a.data("count"), 1);
    // 底栏的提交键照常可点
    a.tap(".submit");
    assert!(a.modal_open());
    a.modal_tap(false);
    a.assert_no_errors();
}
