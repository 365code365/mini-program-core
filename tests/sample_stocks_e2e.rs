//! 自选股（sample/stocks）：每页能打开；setInterval 每秒跳行情（涨红跌绿与涨跌一致）、表头排序与升降序、
//! 搜索加自选（catchtap 不进详情）、长按删自选、详情页分时 / 日 K 切换与五档盘口、
//! 交易页价格步进 ±0.01 / 数量 100 股一档 / slider / 资金与可卖校验 / 下单确认（弹窗不穿透）、
//! 持仓市值与浮动盈亏随行情变化。行情是随机游走：断言只比「变没变、顺序对不对、算得对不对」，不写死价格。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::Value;

const APP: &str = "stocks";

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("不是数字：{v}"))
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn list_codes(a: &App) -> Vec<String> {
    a.data("list").as_array().unwrap().iter().map(|q| q["code"].as_str().unwrap().to_string()).collect()
}

/// 当前页 list 里每只的 `key`
fn col(a: &App, key: &str) -> Vec<f64> {
    a.data("list").as_array().unwrap().iter().map(|q| f(&q[key])).collect()
}

fn sheet(a: &mut App, i: usize, n: usize) {
    a.sheet_tap(i, n);
    a.wait_ms(400);
}

/// 行情模块里的现价（与页面同一份状态）
fn price(a: &App, code: &str) -> f64 {
    f(&a.val(&format!("require('utils/market.js').quote('{code}').price")))
}

#[test]
fn 自选股_每页都能打开() {
    let _g = serial();
    smoke_all_pages(APP, &[("pages/detail/detail", "code=600519"), ("pages/trade/trade", "code=601318&side=buy")]);
}

#[test]
fn 自选股_每秒跳行情_涨红跌绿() {
    let _g = serial();
    let mut a = App::open(APP, None, "tick");
    let t0 = f(&a.data("tickNo"));
    let p0 = col(&a, "price");
    a.wait_ms(2600);
    let t1 = f(&a.data("tickNo"));
    assert!(t1 >= t0 + 2.0, "2.6 秒至少跳两次：{t0} → {t1}");
    let p1 = col(&a, "price");
    assert_ne!(p0, p1, "跳了之后价格变了");
    for q in a.data("list").as_array().unwrap() {
        let chg = f(&q["chg"]);
        let want = if chg > 0.0 { "up" } else if chg < 0.0 { "down" } else { "flat" };
        assert_eq!(q["dir"], want, "{} 涨跌 {chg} 对应颜色", q["name"]);
        let pct = q["pctText"].as_str().unwrap();
        assert_eq!(pct.starts_with('+'), chg > 0.0, "{pct}");
        assert!((f(&q["pct"]) - r2((f(&q["price"]) - f(&q["prev"])) / f(&q["prev"]) * 100.0)).abs() < 0.011);
    }
    assert_eq!(f(&a.data("upCount")) + f(&a.data("downCount")) <= 6.0, true);
    a.assert_no_errors();
}

#[test]
fn 自选股_表头排序_切换升降序_跳行情后仍有序() {
    let _g = serial();
    let mut a = App::open(APP, None, "sort");
    let orig = list_codes(&a);
    assert_eq!(orig, ["600519", "601318", "300750", "000001", "600036", "002594"]);
    a.tap(".th-pct");
    a.frames(2);
    assert_eq!(a.data("sortKey"), "pct");
    assert_eq!(a.data("sortAsc"), false);
    let v = col(&a, "pct");
    assert!(v.windows(2).all(|w| w[0] >= w[1]), "涨跌幅降序：{v:?}");
    a.tap(".th-pct");
    a.frames(2);
    assert_eq!(a.data("sortAsc"), true);
    let v = col(&a, "pct");
    assert!(v.windows(2).all(|w| w[0] <= w[1]), "再点一次升序：{v:?}");
    a.tap(".th-price");
    a.frames(2);
    assert_eq!(a.data("sortKey"), "price");
    assert_eq!(a.data("sortAsc"), false, "换一列先降序");
    assert_eq!(list_codes(&a)[0], "600519", "茅台价格最高");
    assert_eq!(list_codes(&a)[5], "000001", "平安银行价格最低");
    a.wait_ms(1500);
    let v = col(&a, "price");
    assert!(v.windows(2).all(|w| w[0] >= w[1]), "跳行情后按新价格重排：{v:?}");
    a.tap(".th-name");
    a.frames(2);
    assert_eq!(a.data("sortKey"), "");
    assert_eq!(list_codes(&a), orig, "点名称恢复自选顺序");
    a.assert_no_errors();
}

#[test]
fn 自选股_搜索添加自选_加号不进详情() {
    let _g = serial();
    let mut a = App::open(APP, None, "search");
    a.tap_settle(".go-search");
    assert_eq!(a.route(), "pages/search/search");
    assert_eq!(a.data("results").as_array().unwrap().len(), 14);
    a.type_into(".s-input", "中");
    let names: Vec<String> = a.data("results").as_array().unwrap().iter().map(|r| r["name"].as_str().unwrap().to_string()).collect();
    assert_eq!(names, ["中国平安", "中国中免", "中芯国际"]);
    assert_eq!(a.data("results[0].watched"), true);
    assert_eq!(a.count(".s-add"), 2, "已在自选里的没有加号");
    a.blur();
    a.tap_nth(".s-add", 0); // 中国中免
    a.frames(3);
    assert_eq!(a.route(), "pages/search/search", "加号是 catchtap");
    assert_eq!(a.data("results[1].watched"), true);
    assert_eq!(a.count(".s-add"), 1);
    let w = a.storage("stk_watch");
    assert_eq!(w.as_array().unwrap().last().unwrap(), "601888");
    // 按代码搜
    a.tap(".s-clear");
    a.frames(2);
    a.type_into(".s-input", "688");
    assert_eq!(a.data("results").as_array().unwrap().len(), 1);
    assert_eq!(a.data("results[0].name"), "中芯国际");
    a.e.text_input("x");
    a.frames(3);
    assert!(a.exists(".s-empty"), "搜不到显示空态");
    a.blur();
    a.back();
    assert_eq!(a.data("list").as_array().unwrap().len(), 7);
    assert_eq!(list_codes(&a)[6], "601888");
    a.assert_no_errors();
}

#[test]
fn 自选股_长按删除自选_重启后仍在() {
    let _g = serial();
    let mut a = App::open(APP, None, "long");
    a.long_press(".q-row");
    assert_eq!(a.route(), "pages/watch/watch", "长按不进详情");
    sheet(&mut a, 0, 2);
    assert_eq!(a.route(), "pages/watch/watch", "删除后还在自选页（depth={}）", a.depth());
    assert_eq!(list_codes(&a), ["601318", "300750", "000001", "600036", "002594"]);
    a.js("wx.reLaunch({url:'/pages/watch/watch'})");
    a.settle();
    assert_eq!(a.data("list").as_array().unwrap().len(), 5);
    assert!(!a.storage("stk_watch").as_array().unwrap().iter().any(|c| c == "600519"));
    a.assert_no_errors();
}

#[test]
fn 自选股_详情_分时日k切换_五档盘口_删自选() {
    let _g = serial();
    let mut a = App::open(APP, None, "detail");
    a.tap_settle(".q-row"); // 贵州茅台
    assert_eq!(a.route(), "pages/detail/detail");
    assert_eq!(a.data("code"), "600519");
    assert_eq!(a.count(".ts-bar"), 48, "分时 48 个点");
    assert_eq!(a.count(".k-col"), 0);
    a.tap_nth(".tab", 1);
    a.frames(3);
    assert_eq!(a.data("tab"), 1);
    assert_eq!(a.count(".k-col"), 30, "日 K 30 根");
    assert_eq!(a.count(".ts-bar"), 0);
    let (_, _, kw, kh) = a.rect(".k-body");
    assert!(kw > 2.0 && kh > 0.0, "K 线实体画出来了：{kw}×{kh}");
    a.wait_ms(1300);
    let p = f(&a.data("q.price"));
    assert_eq!(a.data("book.sells").as_array().unwrap().len(), 5);
    assert_eq!(a.data("book.sells[4].label"), "卖1");
    assert_eq!(a.data("book.sells[4].price").as_str().unwrap(), format!("{:.2}", p + 0.01), "卖一 = 现价 + 0.01");
    assert_eq!(a.data("book.buys[0].price").as_str().unwrap(), format!("{p:.2}"), "买一 = 现价");
    assert_eq!(p, price(&a, "600519"), "详情跟着行情跳");
    a.tap_nth(".tab", 0);
    a.frames(3);
    assert_eq!(a.count(".ts-bar"), 48);
    assert_eq!(a.data("watched"), true);
    a.tap(".btn-watch");
    a.frames(3);
    assert_eq!(a.data("watched"), false);
    a.back();
    assert_eq!(a.data("list").as_array().unwrap().len(), 5);
    a.assert_no_errors();
}

#[test]
fn 自选股_买入_价格步进_数量步进_确认弹窗不穿透_持仓更新() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/detail/detail?code=601318"), "buy");
    a.tap_settle(".btn-buy");
    assert_eq!(a.route(), "pages/trade/trade");
    assert_eq!(a.data("side"), "buy");
    let p0 = f(&a.data("price"));
    a.tap(".p-plus");
    a.tap(".p-plus");
    a.tap(".p-minus");
    let p = f(&a.data("price"));
    assert_eq!(p, r2(p0 + 0.01), "＋＋－ 之后是 +0.01：{p0} → {p}");
    assert_eq!(a.data("priceText").as_str().unwrap(), format!("{p:.2}"));
    a.wait_ms(1200);
    assert_eq!(f(&a.data("price")), p, "委托价不跟行情跳");
    a.tap(".q-plus");
    a.tap(".q-plus");
    a.tap(".q-minus");
    assert_eq!(a.data("qty"), 200);
    a.tap(".q-minus");
    a.tap(".q-minus");
    assert_eq!(a.data("qty"), 100, "最少一手");
    a.tap(".q-plus");
    let amount = r2(p * 200.0);
    assert_eq!(a.data("amountText").as_str().unwrap(), format!("{amount:.2}"));
    // 下单确认：弹窗弹着时点底下的「卖出」tab 不生效
    let (sx, sy, sw, sh) = a.rects(".side")[1];
    a.tap(".submit");
    assert!(a.modal_open());
    a.tap_xy(sx + sw / 2.0, sy + sh / 2.0);
    assert_eq!(a.data("side"), "buy", "弹窗下面的 tab 点不到");
    if !a.modal_open() {
        panic!("点遮罩不该关掉确认弹窗");
    }
    a.modal_tap(true);
    let pos = a.storage("stk_pos");
    assert_eq!(pos["601318"]["qty"], 700);
    let cost = f(&pos["601318"]["cost"]);
    let want = ((44.1 * 500.0 + amount) / 700.0 * 1000.0).round() / 1000.0;
    assert!((cost - want).abs() < 0.0011, "持仓均价 {cost} ≈ {want}");
    assert_eq!(f(&a.storage("stk_cash")), r2(100000.0 - amount));
    assert_eq!(a.data("last.qty"), 200);
    assert_eq!(a.data("hold"), 700);
    assert_eq!(a.storage("stk_orders").as_array().unwrap().len(), 1);
    a.tap_settle(".go-pos");
    assert_eq!(a.route(), "pages/positions/positions");
    assert_eq!(a.depth(), 2, "redirectTo 换掉交易页");
    assert_eq!(a.data("rows[0].qty"), 700);
    a.assert_no_errors();
}

#[test]
fn 自选股_资金与可卖校验() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/trade/trade?code=600519&side=buy"), "check");
    assert_eq!(a.data("maxQty"), 0, "10 万买不起一手茅台");
    a.tap(".submit");
    assert!(!a.modal_open(), "校验不过不弹确认");
    assert_eq!(a.data("error"), "可用资金不足");
    a.tap_nth(".side", 1);
    a.frames(2);
    assert_eq!(a.data("side"), "sell");
    assert_eq!(a.data("error"), "", "切换买卖清掉错误");
    a.tap(".submit");
    assert_eq!(a.data("error"), "没有可卖的持仓");
    let mut a = App::open(APP, Some("pages/trade/trade?code=601318&side=sell"), "check2");
    assert_eq!(a.data("hold"), 500);
    assert_eq!(a.data("maxQty"), 500);
    for _ in 0..5 {
        a.tap(".q-plus");
    }
    assert_eq!(a.data("qty"), 600);
    a.tap(".submit");
    assert_eq!(a.data("error"), "可卖数量不足");
    assert_eq!(a.storage("stk_pos")["601318"]["qty"], 500);
    a.assert_no_errors();
}

#[test]
fn 自选股_数量_slider_与全部() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/trade/trade?code=000001&side=buy"), "slider");
    let p = f(&a.data("price"));
    let max = f(&a.data("maxQty"));
    assert_eq!(max, ((100000.0 / p / 100.0).floor() * 100.0), "可买 = 资金 / 价格 向下取整到手");
    let (x, y, w, h) = a.rect(".q-slider");
    a.drag(x + 12.0, y + h / 2.0, w / 2.0 - 12.0, 0.0, 12);
    a.frames(3);
    let q = f(&a.data("qty"));
    assert!(q > 100.0 && q < max, "拖到一半附近：{q} / {max}");
    assert_eq!(q % 100.0, 0.0, "slider 按 100 股一档");
    assert!((q / max - 0.5).abs() < 0.2, "大约一半：{q}/{max}");
    a.tap(".q-all");
    assert_eq!(f(&a.data("qty")), max);
    a.tap(".submit");
    a.modal_tap(true);
    let cash = f(&a.storage("stk_cash"));
    assert_eq!(cash, r2(100000.0 - r2(p * max)));
    assert_eq!(a.data("maxQty"), 0, "钱花光了");
    assert_eq!(a.storage("stk_pos")["000001"]["qty"], 1000.0 + max);
    a.assert_no_errors();
}

#[test]
fn 自选股_持仓_浮动盈亏随行情变_卖出清仓() {
    let _g = serial();
    let mut a = App::open(APP, None, "positions");
    a.tap_settle(".go-pos");
    assert_eq!(a.route(), "pages/positions/positions");
    let rows = a.data("rows");
    assert_eq!(rows.as_array().unwrap().len(), 2);
    let check = |a: &App| {
        let s = a.data("s");
        let rows = a.data("rows");
        let mv: f64 = rows.as_array().unwrap().iter().map(|r| r2(f(&r["price"]) * f(&r["qty"]))).sum();
        let pnl: f64 = rows.as_array().unwrap().iter().map(|r| r2((f(&r["price"]) - f(&r["cost"])) * f(&r["qty"]))).sum();
        assert!((f(&s["mv"]) - mv).abs() < 0.011, "市值 = Σ 现价 × 数量");
        assert!((f(&s["pnl"]) - pnl).abs() < 0.011, "浮动盈亏 = Σ (现价 - 成本) × 数量");
        assert!((f(&s["total"]) - f(&s["mv"]) - f(&s["cash"])).abs() < 0.011);
        (f(&s["mv"]), f(&s["pnl"]))
    };
    let (mv0, pnl0) = check(&a);
    let t0 = f(&a.data("tickNo"));
    a.wait_ms(2200);
    assert!(f(&a.data("tickNo")) > t0);
    let (mv1, pnl1) = check(&a);
    assert!(mv0 != mv1 && pnl0 != pnl1, "跳行情后市值与盈亏变了：{mv0}/{pnl0} → {mv1}/{pnl1}");
    // 卖出平安银行全部 1000 股
    let i = a.data("rows").as_array().unwrap().iter().position(|r| r["code"] == "000001").unwrap();
    a.tap_nth(".pos-sell", i);
    a.settle();
    assert_eq!(a.route(), "pages/trade/trade", "「卖」是 catchtap，进的是交易不是详情");
    assert_eq!(a.data("side"), "sell");
    assert_eq!(a.data("hold"), 1000);
    a.tap(".q-all");
    assert_eq!(a.data("qty"), 1000);
    let p = f(&a.data("price"));
    a.tap(".submit");
    a.modal_tap(true);
    assert_eq!(a.data("hold"), 0);
    assert!(a.storage("stk_pos").get("000001").is_none(), "清仓后持仓里没有了");
    assert_eq!(f(&a.storage("stk_cash")), r2(100000.0 + r2(p * 1000.0)));
    a.tap_settle(".go-pos");
    assert_eq!(a.data("rows").as_array().unwrap().len(), 1);
    a.tap_settle(".go-orders");
    assert_eq!(a.route(), "pages/orders/orders");
    assert_eq!(a.data("list[0].sideText"), "卖出");
    assert_eq!(a.data("list[0].qty"), 1000);
    assert_eq!(a.data("sellTotal").as_str().unwrap(), format!("{:.2}", r2(p * 1000.0)));
    a.assert_no_errors();
}

/// 微信：`data-code="{{item.code}}"` 的 dataset 值保留表达式的类型，字符串 '600519' 还是字符串。
/// 引擎把 dataset 存成字符串表、派发时按字面量还原，像数字的字符串（无前导 0）变成了 number，
/// 而 '000001' 仍是字符串 —— 同一个字段类型随值变化。示例里用 String() 兜住，这里单独记着
#[test]
fn 回归_dataset_保留字符串类型() {
    let _g = serial();
    let mut a = App::open(APP, None, "dataset");
    a.js("globalThis.__ds = []; var pg = getCurrentPages()[0]; pg.onOpen = function (e) { globalThis.__ds.push(e.currentTarget.dataset.code) }");
    a.tap_nth(".q-row", 3); // 000001
    a.tap_nth(".q-row", 0); // 600519
    let ds = a.val("globalThis.__ds");
    assert_eq!(ds[0], "000001");
    assert_eq!(ds[1], "600519", "dataset.code 应是字符串：{ds}");
}
