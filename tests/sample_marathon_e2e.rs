//! 城市马拉松报名（sample/marathon）：首页报名截止倒计时（`?now=` 注入、setInterval 往下走、截止后关闭）、
//! 组别名额 progress、四步报名向导（每步校验失败停在本步、年龄按比赛日校验、身份证校验码与自动带出生日 /
//! 性别、上一步回退数据保留）、确认页「修改」跳回对应步骤、提交 showModal → 参赛证（报名号）→ 缴费、
//! 我的报名取消与 storage 持久化。比赛日 2027-04-12，截止 2027-03-31 23:59:59。
//! 文本节点 mustache 里的 `<` 等引擎问题（已修）的回归用例在 tests/sample_calendar_e2e.rs 末尾。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::json;

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

/// 17 位本体 + GB 11643 校验码
fn id_card(body17: &str) -> String {
    const W: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    let sum: u32 = body17.chars().zip(W).map(|(c, w)| c.to_digit(10).unwrap() * w).sum();
    format!("{body17}{}", "10X98765432".chars().nth((sum % 11) as usize).unwrap())
}

/// 2008-06-15 出生的男性（第 17 位 3 是奇数）
fn good_id() -> String {
    id_card("11010520080615123")
}

/// 点首页第 i 张组别卡片：不在视口里就先把页面往上滚
fn tap_card(a: &mut App, i: usize) {
    for _ in 0..4 {
        let (_, y, _, h) = a.rects(".g-card")[i];
        if y + h / 2.0 < H - 40.0 && y + h / 2.0 > 0.0 {
            a.tap_nth(".g-card", i);
            a.settle();
            return;
        }
        a.scroll_by(W / 2.0, 500.0, 200.0);
    }
    panic!("组别卡片 {i} 滚不进视口");
}

fn next(a: &mut App) {
    a.tap(".next");
    a.frames(4);
}

fn step(a: &App) -> i64 {
    a.data("step").as_i64().unwrap()
}

/// 从首页点组别进向导，按身份证一路填到确认页
fn fill_to_confirm(a: &mut App, group: usize) {
    tap_card(a, group);
    assert_eq!(a.route(), "pages/signup/signup");
    next(a);
    a.type_into(".ipt-name", "李雷");
    a.type_into(".ipt-idno", &good_id());
    a.type_into(".ipt-phone", "13812345678");
    a.blur();
    pick(a, ".pk-region");
    confirm(a);
    next(a);
    assert_eq!(step(a), 2, "个人信息通过：{}", a.data("err"));
    a.type_into(".ipt-ecname", "韩梅梅");
    a.type_into(".ipt-ecphone", "13900001111");
    a.blur();
    a.tap_nth(".size-radio", 2);
    next(a);
    a.tap(".agree-box");
    next(a);
    a.settle();
    assert_eq!(a.route(), "pages/confirm/confirm");
}

#[test]
fn 马拉松_每页都能打开() {
    let _g = serial();
    smoke_all_pages("marathon", &[("pages/index/index", "now=2027-03-15T08:00:00"), ("pages/signup/signup", "group=1"), ("pages/success/success", "no=H10001")]);
}

#[test]
fn 首页_倒计时按注入时间走_组别名额() {
    let _g = serial();
    let mut a = App::open("marathon", Some("pages/index/index?now=2027-03-15T08:00:00"), "countdown");
    // 03-15 08:00:00 → 03-31 23:59:59：16 天 15 时 59 分 59 秒
    assert_eq!(a.data("closed"), false);
    assert_eq!(a.data("cd.d"), 16);
    assert_eq!(a.data("cd.h"), 15);
    assert_eq!(a.data("cd.m"), 59);
    let s0 = a.data("cd.s").as_i64().unwrap();
    assert!((58..=59).contains(&s0), "刚打开：{s0}");
    a.wait_ms(2200);
    let s1 = a.data("cd.s").as_i64().unwrap();
    assert!(s0 - s1 >= 2 && s0 - s1 <= 3, "setInterval 每秒走一格：{s0} → {s1}");
    assert_eq!(a.data("cd.text"), format!("16天15时59分{s1}秒"));
    // 名额进度
    let pct: Vec<i64> = a.data("groups").as_array().unwrap().iter().map(|g| g["percent"].as_i64().unwrap()).collect();
    assert_eq!(pct, vec![85, 99, 40, 100]);
    assert_eq!(a.data("groups[3].full"), true);
    assert_eq!(a.data("groups[1].left"), 120);
    assert_eq!(a.count(".g-prog"), 4);
    // 迷你跑已满：点了不进向导
    tap_card(&mut a, 3);
    assert_eq!(a.route(), "pages/index/index");
    tap_card(&mut a, 1);
    assert_eq!(a.route(), "pages/signup/signup");
    assert_eq!(a.data("form.group"), 1, "带着组别进来（number）");
    a.assert_no_errors();
}

#[test]
fn 首页_截止之后不能报名() {
    let _g = serial();
    let mut a = App::open("marathon", Some("pages/index/index?now=2027-04-01T00:00:00"), "closed");
    assert_eq!(a.data("closed"), true);
    assert_eq!(a.data("cd.d"), 0);
    a.tap_nth(".g-card", 0);
    a.settle();
    assert_eq!(a.route(), "pages/index/index");
    a.assert_no_errors();
}

#[test]
fn 向导_每步校验停在本步_年龄按比赛日_回退保留() {
    let _g = serial();
    let mut a = App::open("marathon", Some("pages/signup/signup?group=0"), "wizard");
    assert_eq!(step(&a), 0);
    assert_eq!(a.count(".step"), 4);
    assert!(!a.exists(".prev"), "第 1 步没有上一步");
    next(&mut a);
    assert_eq!(step(&a), 1);
    next(&mut a);
    assert_eq!(step(&a), 1, "姓名没填停在第 2 步");
    assert_eq!(a.data("err"), "请填写真实姓名（2~20 字）");
    a.type_into(".ipt-name", "李雷");
    pick(&mut a, ".pk-idtype");
    row(&mut a, 0, 1, 1);
    confirm(&mut a);
    assert_eq!(a.data("form.idType"), 1, "护照（selector 下标 number）");
    a.type_into(".ipt-idno", "e1234567");
    a.blur();
    assert_eq!(a.data("form.idNo"), "E1234567");
    next(&mut a);
    assert_eq!(a.data("err"), "请选择性别");
    a.tap_nth(".gd-radio", 1);
    assert_eq!(a.data("form.gender"), "女");
    next(&mut a);
    assert_eq!(a.data("err"), "请选择出生日期");
    // 面板初值 2009-06-15：比赛日 17 岁，全马不够
    pick(&mut a, ".pk-birth");
    confirm(&mut a);
    assert_eq!(a.data("form.birth"), "2009-06-15");
    next(&mut a);
    assert_eq!(step(&a), 1);
    assert_eq!(a.data("err"), "全马要求年满 18 周岁（按比赛日计）");
    pick(&mut a, ".pk-birth");
    row(&mut a, 0, 3, -1);
    confirm(&mut a);
    assert_eq!(a.data("form.birth"), "2008-06-15", "date picker 的值是字符串");
    next(&mut a);
    assert_eq!(a.data("err"), "手机号格式不正确");
    a.type_into(".ipt-phone", "1381234567");
    a.blur();
    next(&mut a);
    assert_eq!(a.data("err"), "手机号格式不正确", "10 位不行");
    a.type_into(".ipt-phone", "8");
    a.blur();
    assert_eq!(a.data("form.phone"), "13812345678");
    next(&mut a);
    assert_eq!(a.data("err"), "请选择所在地区");
    pick(&mut a, ".pk-region");
    row(&mut a, 0, 3, 1);
    confirm(&mut a);
    let region = a.data("form.region");
    let r = region.as_array().expect("region 的值是数组");
    assert_eq!(r.len(), 3);
    assert!(r.iter().all(|v| v.is_string() && !v.as_str().unwrap().is_empty()), "{region}");
    next(&mut a);
    assert_eq!(step(&a), 2, "第 2 步通过");
    // 第 3 步
    next(&mut a);
    assert_eq!(a.data("err"), "请填写紧急联系人姓名");
    a.type_into(".ipt-ecname", "韩梅梅");
    a.type_into(".ipt-ecphone", "13812345678");
    a.blur();
    next(&mut a);
    assert_eq!(a.data("err"), "紧急联系人不能是本人");
    a.tap(".ipt-ecphone");
    a.key("end");
    a.key("backspace");
    a.e.text_input("9");
    a.frames(3);
    a.blur();
    assert_eq!(a.data("form.ecPhone"), "13812345679");
    next(&mut a);
    assert_eq!(a.data("err"), "请选择服装尺码");
    assert_eq!(step(&a), 2);
    a.tap_nth(".size-radio", 3);
    assert_eq!(a.data("form.size"), "L");
    pick(&mut a, ".pk-best");
    row(&mut a, 0, 2, 1);
    confirm(&mut a);
    assert_eq!(a.data("form.best"), "04:30");
    assert_eq!(a.data("bestLabel"), "4 小时 30 分");
    next(&mut a);
    assert_eq!(step(&a), 3);
    next(&mut a);
    assert_eq!(a.data("err"), "请阅读并勾选免责声明");
    assert_eq!(step(&a), 3);
    a.tap(".agree-box");
    assert_eq!(a.data("form.agree"), true);
    // 上一步两次回到个人信息：填过的都在，输入框里也是
    a.tap(".prev");
    a.tap(".prev");
    a.frames(3);
    assert_eq!(step(&a), 1);
    assert_eq!(a.data("form.name"), "李雷");
    assert_eq!(a.data("form.birth"), "2008-06-15");
    assert_eq!(a.data("form.size"), "L");
    assert!(a.exists(".ipt-name"));
    // 点步骤条的第 1 步回去也行
    a.tap_nth(".step", 0);
    assert_eq!(step(&a), 0);
    for _ in 0..4 {
        next(&mut a);
    }
    a.settle();
    assert_eq!(a.route(), "pages/confirm/confirm", "四步都通过进确认页");
    assert_eq!(a.data("sum.age"), 18);
    assert_eq!(a.data("sum.idType"), "护照");
    a.assert_no_errors();
}

#[test]
fn 向导_身份证校验码_自动带出生日与性别() {
    let _g = serial();
    let mut a = App::open("marathon", Some("pages/signup/signup?group=1"), "idcard");
    next(&mut a);
    a.type_into(".ipt-name", "王小明");
    let id = good_id();
    // 校验码改错一位
    let bad_last = if id.ends_with('0') { '1' } else { '0' };
    a.type_into(".ipt-idno", &format!("{}{bad_last}", &id[..17]));
    a.blur();
    assert_eq!(a.data("form.birth"), "", "校验不过不带出生日");
    next(&mut a);
    assert_eq!(a.data("err"), "身份证号校验不通过");
    assert_eq!(step(&a), 1);
    a.tap(".ipt-idno");
    a.key("end");
    a.key("backspace");
    a.e.text_input(&id[17..]);
    a.frames(3);
    a.blur();
    assert_eq!(a.data("form.idNo"), id);
    assert_eq!(a.data("form.birth"), "2008-06-15", "从身份证带出");
    assert_eq!(a.data("form.gender"), "男");
    // 手动改生日：与身份证不一致
    pick(&mut a, ".pk-birth");
    row(&mut a, 2, 3, 1);
    confirm(&mut a);
    assert_eq!(a.data("form.birth"), "2008-06-16");
    a.type_into(".ipt-phone", "13812345678");
    a.blur();
    pick(&mut a, ".pk-region");
    confirm(&mut a);
    next(&mut a);
    assert_eq!(a.data("err"), "出生日期与身份证不一致");
    assert_eq!(step(&a), 1);
    pick(&mut a, ".pk-birth");
    row(&mut a, 2, 3, -1);
    confirm(&mut a);
    next(&mut a);
    assert_eq!(step(&a), 2, "半马 16 岁起，18 岁可报：{}", a.data("err"));
    a.assert_no_errors();
}

#[test]
fn 向导_已满组别不能选() {
    let _g = serial();
    let mut a = App::open("marathon", Some("pages/signup/signup?group=3"), "full");
    assert_eq!(a.data("full"), json!([false, false, false, true]));
    next(&mut a);
    assert_eq!(step(&a), 0);
    assert_eq!(a.data("err"), "迷你跑名额已满，请选择其他组别");
    a.tap_nth(".g-radio", 2);
    assert_eq!(a.data("form.group"), 2, "radio-group 的值转成 number");
    next(&mut a);
    assert_eq!(step(&a), 1);
    a.assert_no_errors();
}

#[test]
fn 确认页_修改跳回对应步骤_提交_参赛证_缴费() {
    let _g = serial();
    let mut a = App::open("marathon", Some("pages/index/index?now=2027-03-15T08:00:00"), "submit");
    fill_to_confirm(&mut a, 1);
    assert_eq!(a.depth(), 3);
    assert_eq!(a.data("sum.group"), "半程马拉松 · 21.0975 公里");
    assert_eq!(a.data("sum.fee"), 150);
    assert_eq!(a.data("form.size"), "M");
    assert_eq!(a.data("sum.idMasked"), format!("1101**********{}", &good_id()[14..]));
    // 修改尺码：跳回第 3 步，别的都还在
    a.tap_settle(".edit-2");
    assert_eq!(a.route(), "pages/signup/signup");
    assert_eq!(step(&a), 2);
    assert_eq!(a.data("form.ecName"), "韩梅梅");
    a.tap_nth(".size-radio", 4);
    next(&mut a);
    assert_eq!(step(&a), 3);
    assert_eq!(a.data("form.agree"), true, "勾过的免责声明保留");
    next(&mut a);
    a.settle();
    assert_eq!(a.route(), "pages/confirm/confirm");
    assert_eq!(a.data("form.size"), "XL");
    // 修改组别：跳回第 1 步
    a.tap_settle(".edit-0");
    assert_eq!(step(&a), 0);
    assert_eq!(a.data("form.group"), 1);
    for _ in 0..4 {
        next(&mut a);
    }
    a.settle();
    assert_eq!(a.route(), "pages/confirm/confirm");
    // 提交：先取消
    a.tap(".submit");
    a.frames(3);
    a.modal_tap(false);
    assert_eq!(a.route(), "pages/confirm/confirm");
    assert!(a.storage("mr_entries").is_null() || a.storage("mr_entries") == json!([]) || a.storage("mr_entries") == json!(""));
    a.tap(".submit");
    a.frames(3);
    a.modal_tap(true);
    a.settle();
    assert_eq!(a.route(), "pages/success/success");
    assert_eq!(a.data("e.no"), "H10001", "半马前缀 H + 序号");
    assert_eq!(a.data("e.statusText"), "待缴费");
    assert_eq!(a.data("e.size"), "XL");
    let saved = a.storage("mr_entries");
    assert_eq!(saved[0]["no"], "H10001");
    assert_eq!(saved[0]["group"], 1);
    assert_eq!(saved[0]["birth"], "2008-06-15");
    assert_eq!(saved[0]["region"].as_array().map(|r| r.len()), Some(3));
    a.tap(".pay");
    a.frames(3);
    a.modal_tap(true);
    assert_eq!(a.data("e.status"), "paid");
    assert_eq!(a.data("e.statusText"), "报名成功");
    assert_eq!(a.storage("mr_entries")[0]["status"], "paid");
    a.tap_settle(".go-mine");
    assert_eq!(a.route(), "pages/mine/mine");
    assert_eq!(a.data("list[0].no"), "H10001");
    assert_eq!(a.data("activeCount"), 1);
    // 回首页（栈 [index, signup, success, mine]，退 3 层；reLaunch 带的 query 引擎会丢，见 ignore 的用例）：
    // 半马已报 +1；同一证件再报被拦
    a.js("wx.navigateBack({ delta: 3 })");
    a.settle();
    assert_eq!(a.route(), "pages/index/index");
    assert_eq!(a.data("groups[1].taken"), 11881);
    assert_eq!(a.data("myCount"), 1);
    fill_to_confirm(&mut a, 2);
    a.tap(".submit");
    a.frames(3);
    assert!(!a.modal_open(), "重复报名不弹提交确认");
    assert_eq!(a.data("err"), "该证件已报名本届赛事");
    a.assert_no_errors();
}

#[test]
fn 我的报名_取消报名_持久化() {
    let _g = serial();
    let mut a = App::open("marathon", Some("pages/index/index?now=2027-03-15T08:00:00"), "cancel");
    fill_to_confirm(&mut a, 2);
    a.tap(".submit");
    a.frames(3);
    a.modal_tap(true);
    a.settle();
    assert_eq!(a.data("e.no"), "T10001");
    a.tap_settle(".go-mine");
    assert_eq!(a.data("list").as_array().unwrap().len(), 1);
    assert_eq!(a.data("list[0].statusText"), "待缴费");
    // 取消报名：先「再想想」
    a.tap(".cancel");
    a.frames(3);
    a.modal_tap(false);
    assert_eq!(a.data("list[0].status"), "unpaid");
    assert_eq!(a.route(), "pages/mine/mine", "取消键是 catchtap，不会点进参赛证");
    a.tap(".cancel");
    a.frames(3);
    a.modal_tap(true);
    assert_eq!(a.data("list[0].statusText"), "已取消");
    assert_eq!(a.data("activeCount"), 0);
    assert_eq!(a.count(".cancel"), 0, "已取消的不再显示取消键");
    assert_eq!(a.storage("mr_entries")[0]["status"], "canceled");
    // 重进：状态还在；首页名额释放
    a.js("wx.reLaunch({ url: '/pages/mine/mine' })");
    a.settle();
    assert_eq!(a.data("list[0].statusText"), "已取消");
    a.tap(".item");
    a.settle();
    assert_eq!(a.route(), "pages/success/success");
    assert_eq!(a.data("e.no"), "T10001");
    assert!(!a.exists(".pay"), "已取消不能缴费");
    // 这里只看名额，不看倒计时（reLaunch 的 query 引擎会丢）
    a.js("wx.reLaunch({ url: '/pages/index/index' })");
    a.settle();
    assert_eq!(a.data("groups[2].taken"), 3200);
    assert_eq!(a.data("myCount"), 0);
    a.assert_no_errors();
}

/// `wx.reLaunch({ url: '/pages/x?k=v' })`：微信把 `{k:'v'}` 传给新页面的 onLoad，引擎传的是空对象
/// （`engine_nav.rs` 的 ReLaunch 分支 `let (p, _) = parse_url(&url)` 丢了 query）
#[test]
fn 回归_reLaunch丢query() {
    let _g = serial();
    let mut a = App::open("marathon", None, "relaunch");
    a.js("wx.reLaunch({ url: '/pages/signup/signup?group=2' })");
    a.settle();
    assert_eq!(a.route(), "pages/signup/signup");
    assert_eq!(a.data("form.group"), 2, "onLoad 应收到 group=2");
}
