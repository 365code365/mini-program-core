//! 日程本（sample/calendar）：月视图 6×7 格子（上月 / 下月补位、周起始日）、左右按钮与滑动切月、
//! 重复规则在月视图展开、新建日程的校验与各类 picker 的 e.detail 类型、详情的编辑 / 删除、
//! 列表视图的嵌套分组、storage 持久化。「今天」一律用 `?today=2026-03-15` 注入（2026-03-15 是周日）。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::{json, Value};

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

const HOME: &str = "pages/month/month?today=2026-03-15";

/// 月视图里某个日期格的下标
fn cell_index(a: &App, date: &str) -> usize {
    let cells = a.data("cells");
    cells.as_array().unwrap().iter().position(|c| c["date"] == date).unwrap_or_else(|| panic!("月视图里没有 {date}"))
}

fn dots(a: &App, date: &str) -> Vec<String> {
    let i = cell_index(a, date);
    a.data(&format!("cells[{i}].dots")).as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect()
}

fn titles(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|e| e["title"].as_str().unwrap().to_string()).collect()
}

#[test]
fn 日程本_每页都能打开() {
    let _g = serial();
    smoke_all_pages("calendar", &[("pages/month/month", "today=2026-03-15"), ("pages/detail/detail", "id=3"), ("pages/edit/edit", "id=3")]);
}

#[test]
fn 月视图_格子补位_今天_按钮切月() {
    let _g = serial();
    let mut a = App::open("calendar", Some(HOME), "grid");
    assert_eq!(a.data("title"), "2026年3月");
    assert_eq!(a.data("heads"), json!(["一", "二", "三", "四", "五", "六", "日"]), "默认周一开头");
    assert_eq!(a.data("cells").as_array().unwrap().len(), 42);
    // 3 月 1 日是周日：周一开头时前面补 6 格上月（2-23 ~ 2-28），末尾补到 4-05
    assert_eq!(a.data("cells[0].date"), "2026-02-23");
    assert_eq!(a.data("cells[0].cur"), false);
    assert_eq!(a.data("cells[6].date"), "2026-03-01");
    assert_eq!(a.data("cells[6].cur"), true);
    assert_eq!(a.data("cells[41].date"), "2026-04-05");
    assert_eq!(a.data("cells[20].isToday"), true, "03-15 在第 21 格");
    assert_eq!(a.data("cells[20].sel"), true);
    assert_eq!(a.data("selDate"), "2026-03-15");
    // 当天两条：按开始时间排序
    assert_eq!(titles(&a.data("selEvents")), vec!["项目评审", "牙医复诊"]);
    assert_eq!(a.data("selEvents[0].time"), "09:30 - 11:30");
    assert_eq!(a.count(".day"), 42);

    a.tap(".next");
    assert_eq!(a.data("title"), "2026年4月");
    // 4 月 1 日周三：前面补 3-30、3-31
    assert_eq!(a.data("cells[0].date"), "2026-03-30");
    assert_eq!(a.data("cells[2].date"), "2026-04-01");
    assert_eq!(a.data("selDate"), "2026-04-01", "切月后选中 1 号");
    a.tap(".prev");
    a.tap(".prev");
    assert_eq!(a.data("title"), "2026年2月");
    // 2 月 1 日周日：补 6 格 1 月
    assert_eq!(a.data("cells[0].date"), "2026-01-26");
    a.tap(".prev");
    a.tap(".prev");
    assert_eq!(a.data("title"), "2025年12月", "跨年");
    a.tap(".today-btn");
    assert_eq!(a.data("title"), "2026年3月");
    assert_eq!(a.data("selDate"), "2026-03-15");
    a.assert_no_errors();
}

#[test]
fn 月视图_左右滑动切月_点补位格跳月() {
    let _g = serial();
    let mut a = App::open("calendar", Some(HOME), "swipe");
    let (x, y, w, h) = a.rect(".grid");
    let cy = y + h / 2.0;
    // 手指往左滑 → 下个月
    a.drag(x + w - 30.0, cy, -200.0, 6.0, 10);
    a.frames(3);
    assert_eq!(a.data("title"), "2026年4月", "左滑到下个月");
    a.drag(x + 30.0, cy, 200.0, -6.0, 10);
    a.drag(x + 30.0, cy, 200.0, 0.0, 10);
    a.frames(3);
    assert_eq!(a.data("title"), "2026年2月", "右滑两次回到 2 月");
    // 位移不够 60px 不切月，也不该当成点日期
    a.drag(x + 100.0, cy, -30.0, 0.0, 6);
    assert_eq!(a.data("title"), "2026年2月");
    a.tap(".today-btn");
    // 点 3 月里的 3-16（周一）
    let i = cell_index(&a, "2026-03-16");
    a.tap_nth(".day", i);
    assert_eq!(a.data("selDate"), "2026-03-16");
    assert_eq!(a.data(&format!("cells[{i}].sel")), true);
    assert_eq!(titles(&a.data("selEvents")), vec!["产品周会"]);
    assert_eq!(a.data("selEvents[0].repeat"), "每周");
    // 点第一格（上月补位 2-23）：跳到 2 月并选中它
    a.tap_nth(".day", 0);
    assert_eq!(a.data("title"), "2026年2月");
    assert_eq!(a.data("selDate"), "2026-02-23");
    assert_eq!(titles(&a.data("selEvents")), vec!["产品周会"], "周会从 2-23 起");
    a.assert_no_errors();
}

#[test]
fn 月视图_重复规则展开() {
    let _g = serial();
    let a = App::open("calendar", Some(HOME), "repeat");
    let blue = "#3a86ff".to_string();
    // 每周一的产品周会（2-23 起）：3 月每个周一都有蓝点
    for d in ["2026-02-23", "2026-03-02", "2026-03-09", "2026-03-16", "2026-03-23", "2026-03-30"] {
        assert!(dots(&a, d).contains(&blue), "{d} 应有每周例会的圆点：{:?}", dots(&a, d));
    }
    for d in ["2026-03-03", "2026-03-10", "2026-03-17", "2026-03-29"] {
        assert!(!dots(&a, d).contains(&blue), "{d} 不是周一");
    }
    // 每月 5 号交房租（琥珀色），1 月起
    assert_eq!(dots(&a, "2026-03-05"), vec!["#ffa41b"]);
    assert_eq!(dots(&a, "2026-04-05"), vec!["#ffa41b"], "补位格里的下月 5 号也展开");
    // 今天两条、03-19 生日、03-24 读书会
    assert_eq!(dots(&a, "2026-03-15").len(), 2);
    assert_eq!(dots(&a, "2026-03-19"), vec!["#ff6b5b"]);
    assert_eq!(dots(&a, "2026-03-24"), vec!["#19b37d"]);
    assert_eq!(dots(&a, "2026-03-04"), Vec::<String>::new());
    // 本月有安排的天：周一 5 天 + 5 号 + 15/19/24 = 9
    assert_eq!(a.data("monthCount"), 9);
    a.assert_no_errors();
}

#[test]
fn 新建日程_校验_各类picker_持久化() {
    let _g = serial();
    let mut a = App::open("calendar", Some(HOME), "create");
    a.tap_settle(".add");
    assert_eq!(a.route(), "pages/edit/edit");
    assert_eq!(a.depth(), 2);
    assert_eq!(a.data("form.date"), "2026-03-15", "带着选中日期进来");
    assert_eq!(a.data("isNew"), true);
    // 标题必填
    a.tap(".save");
    a.frames(3);
    assert_eq!(a.data("err"), "请填写日程标题");
    assert_eq!(a.route(), "pages/edit/edit", "校验失败停在本页");
    a.type_into(".title-input", "瑜伽课");
    a.blur();
    assert_eq!(a.data("form.title"), "瑜伽课");
    assert_eq!(a.data("err"), "");
    // 结束时间往前拨到 08:00（早于开始 09:00）：提示且不接受
    pick(&mut a, ".pk-end");
    row(&mut a, 0, 2, -2);
    confirm(&mut a);
    assert_eq!(a.data("err"), "结束时间不能早于开始时间");
    assert_eq!(a.data("form.end"), "10:00", "被拒的选择不落到表单里");
    // 开始拨到 18:00 → 结束自动顺延到 19:00
    pick(&mut a, ".pk-start");
    for _ in 0..4 {
        row(&mut a, 0, 2, 2);
    }
    row(&mut a, 0, 2, 1);
    confirm(&mut a);
    assert_eq!(a.data("form.start"), "18:00", "time picker 的值是字符串");
    assert_eq!(a.data("form.end"), "19:00");
    pick(&mut a, ".pk-end");
    row(&mut a, 1, 2, 2);
    confirm(&mut a);
    assert_eq!(a.data("form.end"), "19:02");
    // 日期 +2 天 → 03-17（周二）
    pick(&mut a, ".pk-date");
    row(&mut a, 2, 3, 2);
    confirm(&mut a);
    assert_eq!(a.data("form.date"), "2026-03-17");
    // 重复：每周（下标 2，number）
    pick(&mut a, ".pk-repeat");
    row(&mut a, 0, 1, 2);
    confirm(&mut a);
    assert_eq!(a.data("form.repeat"), 2);
    assert!(a.data("form.repeat").is_number(), "selector 的 value 是 number");
    pick(&mut a, ".pk-remind");
    row(&mut a, 0, 1, 2);
    confirm(&mut a);
    assert_eq!(a.data("form.remind"), 3, "默认「准时」(1) 往下两项 → 提前 15 分钟");
    // 颜色：第 4 个（琥珀）
    a.tap_nth(".color-radio", 3);
    assert_eq!(a.data("form.color"), 3);
    a.tap_settle(".save");
    a.wait_ms(300);
    assert_eq!(a.route(), "pages/month/month", "保存后回到月视图");
    assert_eq!(a.depth(), 1);
    let saved = a.storage("cal_events");
    let ev = saved.as_array().unwrap().iter().find(|e| e["title"] == "瑜伽课").expect("存进了 storage").clone();
    assert_eq!(ev["id"], 7);
    assert_eq!(ev["date"], "2026-03-17");
    assert_eq!(ev["start"], "18:00");
    assert_eq!(ev["end"], "19:02");
    assert_eq!(ev["repeat"], 2);
    assert_eq!(ev["allDay"], false);
    // 月视图回来就刷新：周二 17/24/31 有琥珀点，10 号没有（早于起始日）
    for d in ["2026-03-17", "2026-03-24", "2026-03-31"] {
        assert!(dots(&a, d).contains(&"#ffa41b".to_string()), "{d}：{:?}", dots(&a, d));
    }
    assert!(dots(&a, "2026-03-10").is_empty());
    // 重进小程序：storage 里还在（reLaunch 的 query 引擎会丢，「今天」靠 globalData 里已注入的值）
    a.js("wx.reLaunch({ url: '/pages/month/month?today=2026-03-15' })");
    a.settle();
    let i = cell_index(&a, "2026-03-24");
    a.tap_nth(".day", i);
    assert_eq!(a.data("selDate"), "2026-03-24");
    assert_eq!(titles(&a.data("selEvents")), vec!["瑜伽课", "读书会"], "18:00 在 19:30 前");
    a.assert_no_errors();
}

#[test]
fn 全天日程_隐藏时间选择() {
    let _g = serial();
    let mut a = App::open("calendar", Some("pages/edit/edit?date=2026-03-20"), "allday");
    assert!(a.exists(".pk-start"));
    a.tap(".allday");
    a.frames(3);
    assert_eq!(a.data("form.allDay"), true, "switch 的 e.detail.value 是 boolean");
    assert!(!a.exists(".pk-start"), "全天不显示开始 / 结束");
    a.type_into(".title-input", "春分出游");
    a.blur();
    a.tap_settle(".save");
    let list = a.storage("cal_events");
    let ev = list.as_array().unwrap().iter().find(|e| e["title"] == "春分出游").unwrap().clone();
    assert_eq!(ev["allDay"], true);
    assert_eq!(ev["date"], "2026-03-20");
    a.assert_no_errors();
}

#[test]
fn 详情_编辑后刷新_删除走确认弹窗() {
    let _g = serial();
    let mut a = App::open("calendar", Some(HOME), "detail");
    // 今天第二条：牙医复诊
    a.tap_nth(".ev", 1);
    a.settle();
    assert_eq!(a.route(), "pages/detail/detail");
    assert_eq!(a.data("ev.title"), "牙医复诊");
    assert_eq!(a.data("info.time"), "14:00 - 15:00");
    assert_eq!(a.data("info.remind"), "提前 1 小时");
    // 编辑：改标题、改成每天重复
    a.tap_settle(".edit");
    assert_eq!(a.route(), "pages/edit/edit");
    assert_eq!(a.data("isNew"), false);
    assert_eq!(a.data("form.title"), "牙医复诊");
    a.type_into(".title-input", "（改约）");
    a.blur();
    pick(&mut a, ".pk-repeat");
    row(&mut a, 0, 1, 1);
    confirm(&mut a);
    assert_eq!(a.data("form.repeat"), 1);
    a.tap_settle(".save");
    assert_eq!(a.route(), "pages/detail/detail", "回到详情");
    assert_eq!(a.data("ev.title"), "牙医复诊（改约）", "onShow 重新读了 storage");
    assert_eq!(a.data("info.repeat"), "每天");
    let n0 = a.storage("cal_events").as_array().unwrap().len();
    // 删除：先取消，再确定
    a.tap(".btn-del");
    a.frames(3);
    a.modal_tap(false);
    assert_eq!(a.route(), "pages/detail/detail", "取消不删");
    assert_eq!(a.storage("cal_events").as_array().unwrap().len(), n0);
    a.tap(".btn-del");
    a.frames(3);
    a.modal_tap(true);
    a.wait_ms(200);
    assert_eq!(a.route(), "pages/month/month");
    let left = a.storage("cal_events");
    assert_eq!(left.as_array().unwrap().len(), n0 - 1);
    assert!(left.as_array().unwrap().iter().all(|e| e["id"] != 3), "id=3 被删掉");
    assert_eq!(titles(&a.data("selEvents")), vec!["项目评审"], "月视图回来就少了一条");
    // 每天重复删掉之后，明天也没有它
    assert_eq!(dots(&a, "2026-03-17").len(), 0);
    a.assert_no_errors();
}

#[test]
fn 列表视图_按日期分组的嵌套列表() {
    let _g = serial();
    let mut a = App::open("calendar", Some(HOME), "list");
    a.tap_settle(".to-list");
    assert_eq!(a.route(), "pages/list/list");
    // 03-15 ~ 04-13：周一 ×5、04-05 房租、今天 2、03-19、03-24
    assert_eq!(a.data("total"), 10);
    assert_eq!(a.data("range"), "03-15 ~ 04-13");
    let groups = a.data("groups");
    let dates: Vec<&str> = groups.as_array().unwrap().iter().map(|g| g["date"].as_str().unwrap()).collect();
    assert_eq!(dates, vec!["2026-03-15", "2026-03-16", "2026-03-19", "2026-03-23", "2026-03-24", "2026-03-30", "2026-04-05", "2026-04-06", "2026-04-13"]);
    assert_eq!(a.data("groups[0].label"), "今天");
    assert_eq!(a.data("groups[1].label"), "明天");
    assert_eq!(a.data("groups[2].label"), "03-19 周四");
    assert_eq!(titles(&a.data("groups[0].items")), vec!["项目评审", "牙医复诊"]);
    assert_eq!(a.count(".group"), 9);
    assert_eq!(a.count(".item"), 10, "内层 wx:for 全部渲染");
    // 点「明天」那组的周会进详情
    a.tap_nth(".item", 2);
    a.settle();
    assert_eq!(a.route(), "pages/detail/detail");
    assert_eq!(a.data("ev.title"), "产品周会");
    assert_eq!(a.data("info.repeat"), "每周");
    a.assert_no_errors();
}

#[test]
fn 设置_周起始日切到周日_月视图跟着变() {
    let _g = serial();
    let mut a = App::open("calendar", Some(HOME), "weekstart");
    a.tap_settle(".to-settings");
    assert_eq!(a.route(), "pages/settings/settings");
    assert_eq!(a.data("wsIdx"), 0);
    pick(&mut a, ".pk-ws");
    row(&mut a, 0, 1, 1);
    confirm(&mut a);
    assert_eq!(a.data("wsIdx"), 1);
    assert_eq!(a.storage("cal_settings")["weekStart"], 0);
    a.back();
    assert_eq!(a.route(), "pages/month/month");
    assert_eq!(a.data("heads"), json!(["日", "一", "二", "三", "四", "五", "六"]));
    // 周日开头：3 月 1 日（周日）正好是第一格，不需要补位
    assert_eq!(a.data("cells[0].date"), "2026-03-01");
    assert_eq!(a.data("cells[0].cur"), true);
    assert_eq!(a.data("cells[14].date"), "2026-03-15");
    assert_eq!(a.data("cells[14].isToday"), true);
    // 周一在第二列：3-02 / 3-09 的周会圆点
    assert!(dots(&a, "2026-03-02").contains(&"#3a86ff".to_string()));
    assert_eq!(cell_index(&a, "2026-03-02") % 7, 1);
    // 4 月：4-01 周三，前面补 3-29 ~ 3-31
    a.tap(".next");
    assert_eq!(a.data("cells[0].date"), "2026-03-29");
    // 设置重进后仍是周日
    a.js("wx.reLaunch({ url: '/pages/month/month?today=2026-03-15' })");
    a.settle();
    assert_eq!(a.data("heads[0]"), "日");
    a.assert_no_errors();
}

// ───────── 写这三个示例时撞到的引擎问题（已修）：最小复现，留作回归（独立小程序，不依赖示例） ─────────

/// 最小复现：mustache 表达式里的小于号 `<`。微信里 `{{index < step ? 'a' : 'b'}}` 是合法表达式。
fn open_inline(wxml: &str, tag: &str) -> Result<(), String> {
    render_inline(wxml, "", "", tag, |_| ())
}

/// 起一个只有一页的小程序，打开后跑几帧，把引擎交给 `f`（读像素等）
fn render_inline<T>(wxml: &str, app_wxss: &str, wxss: &str, tag: &str, f: impl FnOnce(&mut mini_render::host::MiniEngine) -> T) -> Result<T, String> {
    use mini_render::host::MiniEngine;
    let root = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("lt-{tag}"));
    std::fs::remove_dir_all(&root).ok();
    let files = [
        ("app.json", r#"{"pages":["pages/index/index"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", app_wxss),
        ("pages/index/index.json", "{}"),
        ("pages/index/index.wxml", wxml),
        ("pages/index/index.wxss", wxss),
        ("pages/index/index.js", "Page({ data: { a: 1, b: 2 } });"),
    ];
    for (rel, text) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let data = root.join("_data");
    let r = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0).and_then(|mut e| {
        e.set_animation_clock(0.0);
        e.launch(Some("pages/index/index"))?;
        for i in 0..6 {
            e.pump(1000 + i * 16);
        }
        Ok(f(&mut e))
    });
    std::fs::remove_dir_all(&root).ok();
    r.map_err(|e| e.to_string())
}

/// 在另一个线程里打开，最多等 `secs` 秒（解析卡死时测试本身不跟着卡死）
fn open_inline_timeout(wxml: &'static str, tag: &'static str, secs: u64) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        tx.send(open_inline(wxml, tag)).ok();
    });
    rx.recv_timeout(std::time::Duration::from_secs(secs)).unwrap_or_else(|_| Err(format!("{secs} 秒内没打开（解析卡死）")))
}

/// 属性里的 `{{a < b ? ...}}` 能解析（对照组，正常）
#[test]
fn 对照_属性里的小于号能解析() {
    let _g = serial();
    assert_eq!(open_inline_timeout(r#"<view class="{{a < b ? 'x' : 'y'}}">hi</view>"#, "attr", 20), Ok(()));
}

/// 文本节点里的 `{{a < b ? 'x' : 'y'}}`：微信正常显示 x，引擎整页解析失败「Empty tag name」
#[test]
fn 回归_文本节点里的小于号_解析失败() {
    let _g = serial();
    assert_eq!(open_inline_timeout(r#"<view>{{a < b ? 'x' : 'y'}}</view>"#, "text", 20), Ok(()));
}

/// 文本节点里不带空格的 `{{a<b ? 'x' : 'y'}}`：引擎打开页面卡死不返回（实测 30 分钟无结果）
#[test]
fn 回归_文本节点里紧挨着的小于号_卡死() {
    let _g = serial();
    assert_eq!(open_inline_timeout(r#"<view>{{a<b ? 'x' : 'y'}}</view>"#, "text2", 10), Ok(()));
}

/// 逻辑像素 (x, y) 处的 RGB
fn pixel(e: &mut mini_render::host::MiniEngine, x: f32, y: f32) -> [u8; 3] {
    let px = e.rgba();
    let i = ((y * DPR) as usize * (W * DPR) as usize + (x * DPR) as usize) * 4;
    [px[i], px[i + 1], px[i + 2]]
}

/// 对照：inline `style="display:none"` 生效（红块不画）
#[test]
fn 对照_行内display_none生效() {
    let _g = serial();
    let got = render_inline(r#"<view style="display:none;height:100px;background:#ff0000"></view>"#, "", "", "dn-inline", |e| pixel(e, 20.0, 20.0));
    assert_ne!(got, Ok([255, 0, 0]));
}

/// class 里的 `display: none`（页面 wxss 与 app.wxss 都一样）：微信不显示，引擎照样布局、照样画
#[test]
fn 回归_class里的display_none不生效() {
    let _g = serial();
    let got = render_inline(r#"<view class="gone" style="height:100px;background:#ff0000"></view>"#, "", ".gone { display: none; }", "dn-class", |e| pixel(e, 20.0, 20.0));
    assert_ne!(got, Ok([255, 0, 0]), "红块应该被隐藏");
}

/// `<progress activeColor="#ff0000">`：微信进度是红色；引擎里 page 上的 `color` 继承下来盖掉了 activeColor
#[test]
fn 回归_progress的activeColor被继承的color覆盖() {
    let _g = serial();
    let got = render_inline(r##"<progress percent="100" stroke-width="20" activeColor="#ff0000" backgroundColor="#00ff00" />"##, "page { color: #111111; }", "", "pg", |e| pixel(e, 187.0, 12.0));
    assert_eq!(got, Ok([255, 0, 0]));
}
