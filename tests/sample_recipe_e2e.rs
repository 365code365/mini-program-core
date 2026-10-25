//! 厨房小当家（sample/recipe）：每页能打开；横向分类 + 触底分页、搜索（历史 storage / 热门 / 空态）、
//! 份量 stepper 联动食材用量、checkbox-group 勾选计数、swiper 步骤 + 计时器、收藏长按 actionSheet 置顶 / 删除、
//! 打卡与「我的」统计，fixed tabBar 与操作菜单不穿透。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::{json, Value};

/// 点自定义 tabBar 的第 `i` 个（共 3 个）
fn tab(a: &mut App, i: usize) {
    let r = a.rects(".tab-item");
    if r.len() == 3 {
        let (x, y, w, h) = r[i];
        a.tap_xy(x + w / 2.0, y + h / 2.0);
    } else {
        a.tap_xy(W * (i as f32 + 0.5) / 3.0, H - 25.0);
    }
    a.settle();
}

fn hook_toast(a: &App) {
    a.js("globalThis.__toasts = []; (function () { var o = wx.showToast; wx.showToast = function (opt) { __toasts.push(opt && opt.title); return o.apply(this, arguments); }; })()");
}

fn last_toast(a: &App) -> String {
    a.val("(globalThis.__toasts || []).slice(-1)[0] || ''").as_str().unwrap_or("").to_string()
}

fn len(v: Value) -> usize {
    v.as_array().map(|a| a.len()).unwrap_or(0)
}

fn ids(v: Value) -> Vec<i64> {
    v.as_array().unwrap().iter().map(|x| x["id"].as_i64().unwrap()).collect()
}

/// 先滚到 tabBar（50px）上方再点
fn tap_visible(a: &mut App, sel: &str, i: usize) {
    for _ in 0..8 {
        let (_, y, _, h) = a.rects(sel)[i];
        if y + h / 2.0 < H - 80.0 && y + h / 2.0 > 20.0 {
            break;
        }
        a.scroll_by(W / 2.0, H / 2.0, (y + h / 2.0 - H / 2.0).clamp(-300.0, 300.0));
    }
    a.tap_nth(sel, i);
}

#[test]
fn 菜谱_每页都能打开() {
    let _g = serial();
    smoke_all_pages("recipe", &[("pages/detail/detail", "id=2")]);
}

#[test]
fn 菜谱_触底分页追加_横向分类切换() {
    let _g = serial();
    let mut a = App::open("recipe", None, "page");
    assert_eq!(len(a.data("list")), 6, "首屏 6 道");
    assert_eq!(a.data("total"), 22);
    assert_eq!(a.data("noMore"), false);
    let mut seen = vec![6];
    for _ in 0..12 {
        a.scroll_by(W / 2.0, H * 0.7, 500.0);
        a.wait_ms(450);
        let n = len(a.data("list"));
        if *seen.last().unwrap() != n {
            seen.push(n);
        }
        if a.data("noMore") == true {
            break;
        }
    }
    assert_eq!(seen, vec![6, 12, 18, 22], "onReachBottom 每次追加一页，不重复加载");
    assert_eq!(a.data("loading"), false);
    let all = ids(a.data("list"));
    let mut uniq = all.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), 22, "追加的没有重复");
    // 横向分类：最后一个「甜品」一开始在屏幕外，横着拖过去再点
    a.js("wx.pageScrollTo({ scrollTop: 0, duration: 0 })");
    a.frames(4);
    let (cx, cy, cw, ch) = a.rect(".cats");
    let (lx, _, lw, _) = a.rects(".cat")[7];
    assert!(lx + lw > W, "「甜品」在屏幕右侧之外：{lx}");
    a.drag(cx + cw * 0.8, cy + ch / 2.0, -cw * 0.7, 0.0, 14);
    a.wait_ms(600);
    let (lx2, _, lw2, _) = a.rects(".cat")[7];
    assert!(lx2 + lw2 <= W + 1.0, "横着拖过来了：{lx} → {lx2}");
    a.tap_nth(".cat", 7);
    assert_eq!(a.data("cat"), 7);
    assert_eq!(a.data("total"), 3);
    assert_eq!(ids(a.data("list")), vec![15, 16, 22]);
    assert_eq!(a.data("noMore"), true, "一页就装下了");
    a.tap_settle(".rc");
    assert_eq!(a.route(), "pages/detail/detail");
    assert_eq!(a.data("r.name"), "芒果班戟");
    a.assert_no_errors();
}

#[test]
fn 菜谱_搜索_历史持久化_热门标签_无结果空态() {
    let _g = serial();
    let mut a = App::open("recipe", None, "search");
    a.tap_settle(".search");
    assert_eq!(a.route(), "pages/search/search");
    assert!(!a.exists(".hist"), "一开始没有历史");
    hook_toast(&a);
    a.tap(".go");
    assert_eq!(last_toast(&a), "请输入关键词");
    a.type_into(".ipt", "番茄");
    a.blur();
    a.tap(".go");
    assert_eq!(a.data("searched"), "番茄");
    assert_eq!(ids(a.data("results")), vec![1, 8], "菜名 / 标签 / 食材里带番茄的");
    assert_eq!(a.storage("rc_history"), json!(["番茄"]));
    a.tap(".clear");
    assert_eq!(a.data("keyword"), "");
    assert_eq!(a.count(".hist"), 1);
    a.tap_nth(".hot", 3); // 热门第 4 个：豆腐
    assert_eq!(a.data("keyword"), "豆腐");
    assert_eq!(ids(a.data("results")), vec![3, 7, 8, 11]);
    assert_eq!(a.storage("rc_history"), json!(["豆腐", "番茄"]));
    a.tap(".clear");
    a.tap_nth(".hist", 1); // 历史里点「番茄」：去重并提到最前
    assert_eq!(a.storage("rc_history"), json!(["番茄", "豆腐"]));
    a.tap(".clear");
    a.type_into(".ipt", "火星菜");
    a.blur();
    a.tap(".go");
    assert_eq!(len(a.data("results")), 0);
    assert!(a.exists(".no-result"), "无结果空态");
    assert_eq!(a.count(".result"), 0);
    // 历史跨页面保留：回首页再进来
    a.back();
    a.tap_settle(".search");
    assert_eq!(a.data("history"), json!(["火星菜", "番茄", "豆腐"]));
    a.tap(".clear-history");
    a.modal_tap(false);
    assert_eq!(a.count(".hist"), 3, "取消不清");
    a.tap(".clear-history");
    a.modal_tap(true);
    assert_eq!(a.count(".hist"), 0);
    assert_eq!(a.storage("rc_history"), Value::String(String::new()));
    a.tap_nth(".hot", 1); // 鸡蛋
    tap_visible(&mut a, ".result", 0);
    a.settle();
    assert_eq!(a.route(), "pages/detail/detail");
    assert_eq!(a.data("r.id"), 1);
    a.assert_no_errors();
}

#[test]
fn 菜谱_份量加减联动用量_食材勾选计数() {
    let _g = serial();
    let mut a = App::open("recipe", Some("pages/detail/detail?id=1"), "servings");
    hook_toast(&a);
    assert_eq!(a.data("servings"), 2);
    let amounts = |a: &App| -> Vec<String> { a.data("ingredients").as_array().unwrap().iter().map(|x| x["amount"].as_str().unwrap().to_string()).collect() };
    assert_eq!(amounts(&a), vec!["2", "3", "5", "2", "5"]);
    a.tap(".plus");
    assert_eq!(a.data("servings"), 3);
    assert_eq!(amounts(&a), vec!["3", "4.5", "7.5", "3", "7.5"], "按 3/2 等比换算");
    a.tap(".minus");
    a.tap(".minus");
    assert_eq!(a.data("servings"), 1);
    assert_eq!(amounts(&a), vec!["1", "1.5", "2.5", "1", "2.5"]);
    a.tap(".minus");
    assert_eq!(a.data("servings"), 1, "不能少于 1 人份");
    assert_eq!(last_toast(&a), "至少 1 人份");
    for _ in 0..12 {
        a.tap(".plus");
    }
    assert_eq!(a.data("servings"), 10);
    assert_eq!(last_toast(&a), "最多 10 人份");
    assert_eq!(a.data("ingredients[1].amount"), "15");
    // checkbox-group：勾第 1、3 样
    tap_visible(&mut a, ".ing", 0);
    tap_visible(&mut a, ".ing", 2);
    assert_eq!(a.data("ready"), 2);
    assert_eq!(a.data("ingredients[0].checked"), true);
    assert_eq!(a.data("ingredients[1].checked"), false);
    // 改份量不丢勾选
    let (_, y, _, _) = a.rect(".plus");
    if y < 0.0 {
        a.js("wx.pageScrollTo({ scrollTop: 0, duration: 0 })");
        a.frames(4);
    }
    a.tap(".minus");
    assert_eq!(a.data("ready"), 2);
    assert_eq!(a.data("ingredients[2].checked"), true);
    tap_visible(&mut a, ".ing", 0);
    assert_eq!(a.data("ready"), 1, "再点一次取消勾选");
    for i in [0, 1, 3, 4] {
        tap_visible(&mut a, ".ing", i);
    }
    assert_eq!(a.data("ready"), 5);
    assert_eq!(last_toast(&a), "食材备齐，开做吧");
    a.assert_no_errors();
}

#[test]
fn 菜谱_步骤swiper滑动_计时器开始暂停重置() {
    let _g = serial();
    let mut a = App::open("recipe", Some("pages/detail/detail?id=1"), "timer");
    hook_toast(&a);
    // 番茄炒蛋 4 步：30 / 60 / 120 / 30 秒
    assert_eq!(a.data("step"), 0);
    assert_eq!(a.data("remainText"), "00:30");
    a.js("wx.pageScrollTo({ scrollTop: 400, duration: 0 })");
    a.frames(6);
    a.tap(".timer-btn");
    assert_eq!(a.data("running"), true);
    a.wait_ms(2400);
    let r1 = a.data("remain").as_i64().unwrap();
    assert!((26..=28).contains(&r1), "每秒减 1：{r1}");
    a.tap(".timer-btn");
    assert_eq!(a.data("running"), false, "暂停");
    a.wait_ms(1300);
    assert_eq!(a.data("remain").as_i64().unwrap(), r1, "暂停后不走");
    a.tap(".reset-btn");
    assert_eq!(a.data("remainText"), "00:30");
    // 手指左滑 swiper：换到第 2 步，计时器换成 60 秒
    let (sx, sy, sw, sh) = a.rect(".steps");
    assert!(sy > 0.0 && sy + sh < H - 50.0, "swiper 在视口里：{sy}");
    a.tap(".timer-btn");
    a.drag(sx + sw * 0.85, sy + sh / 2.0, -sw * 0.6, 0.0, 10);
    a.wait_ms(600);
    let step = a.data("step");
    assert_eq!(step, 1, "bindchange 的 current 是 number：{step}");
    assert_eq!(a.data("running"), false, "换步骤停表");
    assert_eq!(a.data("remainText"), "01:00");
    a.tap(".next-step");
    a.wait_ms(500);
    assert_eq!(a.data("step"), 2);
    assert_eq!(a.data("remainText"), "02:00");
    a.tap(".next-step");
    a.tap(".next-step");
    assert_eq!(a.data("step"), 3, "最后一步再点下一步不动");
    a.tap(".prev-step");
    a.wait_ms(500);
    assert_eq!(a.data("step"), 2);
    // 受控 current 换了之后，再手滑一次从第 3 步滑到第 4 步
    a.drag(sx + sw * 0.85, sy + sh / 2.0, -sw * 0.6, 0.0, 10);
    a.wait_ms(600);
    assert_eq!(a.data("step"), 3, "从 current=2 接着滑");
    // 倒计时走完：停表并提示
    a.js("getCurrentPages().slice(-1)[0].setData({ remain: 2, remainText: '00:02' })");
    a.tap(".timer-btn");
    a.wait_ms(2600);
    assert_eq!(a.data("remain"), 0);
    assert_eq!(a.data("running"), false);
    assert_eq!(last_toast(&a), "第 4 步时间到");
    a.tap(".timer-btn");
    assert_eq!(a.data("remainText"), "00:30", "走完再点开始：从头计时");
    a.tap(".timer-btn");
    a.assert_no_errors();
}

/// 首页点第 `i` 张卡进详情、收藏、返回
fn fav_from_index(a: &mut App, i: usize) -> i64 {
    tap_visible(a, ".rc", i);
    a.settle();
    assert_eq!(a.route(), "pages/detail/detail");
    let id = a.data("r.id").as_i64().unwrap();
    a.tap(".fav");
    assert_eq!(a.data("fav"), true);
    a.back();
    id
}

#[test]
fn 菜谱_收藏_长按菜单置顶与删除_持久化() {
    let _g = serial();
    let mut a = App::open("recipe", None, "favs");
    let f1 = fav_from_index(&mut a, 0);
    let f2 = fav_from_index(&mut a, 1);
    let f3 = fav_from_index(&mut a, 2);
    assert_eq!((f1, f2, f3), (1, 2, 3));
    assert_eq!(len(a.storage("rc_favs")), 3);
    a.js("wx.pageScrollTo({ scrollTop: 0, duration: 0 })");
    a.frames(4);
    tab(&mut a, 1);
    assert_eq!(a.route(), "pages/favs/favs");
    assert_eq!(ids(a.data("list")), vec![3, 2, 1], "新收藏的在前");
    // 长按最后一项「番茄炒蛋」→ 置顶
    a.long_press_nth(".fav-item", 2);
    sheet_pick(&mut a, 0, 2);
    assert_eq!(a.route(), "pages/favs/favs", "长按不触发点击跳转");
    assert_eq!(ids(a.data("list")), vec![1, 3, 2]);
    assert_eq!(a.data("list[0].pinned"), true);
    assert!(a.exists(".pin"));
    // 长按第 2 项（3）→ 删除
    a.long_press_nth(".fav-item", 1);
    sheet_pick(&mut a, 1, 2);
    assert_eq!(ids(a.data("list")), vec![1, 2]);
    let st = a.storage("rc_favs");
    assert_eq!(len(st.clone()), 2);
    assert!(st.as_array().unwrap().iter().any(|f| f["id"] == 1 && f["pinned"] == true));
    // 置顶的再长按：菜单第一项变成「取消置顶」
    a.long_press_nth(".fav-item", 0);
    sheet_pick(&mut a, 0, 2);
    assert_eq!(a.data("list[0].pinned"), false);
    assert_eq!(ids(a.data("list")), vec![2, 1], "取消置顶回到按收藏时间排");
    // 重开收藏页：从 storage 读回
    a.js("wx.reLaunch({ url: '/pages/favs/favs' })");
    a.settle();
    assert_eq!(ids(a.data("list")), vec![2, 1]);
    a.tap_nth(".fav-item", 1);
    a.settle();
    assert_eq!(a.route(), "pages/detail/detail");
    assert_eq!(a.data("fav"), true, "详情页认得已收藏");
    a.tap(".fav");
    a.back();
    assert_eq!(ids(a.data("list")), vec![2], "详情页取消收藏，回来就没了");
    a.assert_no_errors();
}

#[test]
fn 菜谱_打卡与我的页统计_清空历史要确认() {
    let _g = serial();
    let mut a = App::open("recipe", Some("pages/detail/detail?id=5"), "mine");
    tap_visible(&mut a, ".cooked", 0);
    tap_visible(&mut a, ".cooked", 0);
    assert_eq!(a.data("cookedTimes"), 2);
    assert_eq!(a.storage("rc_cooked"), json!({ "5": 2 }));
    tap_visible(&mut a, ".fav", 0);
    a.js("wx.setStorageSync('rc_history', ['鸡蛋', '豆腐'])");
    a.js("wx.reLaunch({ url: '/pages/mine/mine' })");
    a.settle();
    assert_eq!(a.route(), "pages/mine/mine");
    assert_eq!(a.data("cookedTotal"), 2);
    assert_eq!(a.data("favCount"), 1);
    assert_eq!(a.data("historyCount"), 2);
    assert_eq!(a.data("cooked[0].name"), "葱油拌面");
    a.tap(".clear-history");
    assert!(a.modal_open());
    a.modal_tap(false);
    assert_eq!(a.data("historyCount"), 2);
    a.tap(".clear-history");
    a.modal_tap(true);
    assert_eq!(a.data("historyCount"), 0);
    a.tap_settle(".cooked");
    assert_eq!(a.route(), "pages/detail/detail");
    assert_eq!(a.data("cookedTimes"), 2, "详情页读回打卡次数");
    a.assert_no_errors();
}

#[test]
fn 菜谱_tabbar盖着卡片_操作菜单开着_点击都不穿透() {
    let _g = serial();
    let mut a = App::open("recipe", None, "overlay");
    // 首屏底部那张卡被 tabBar 压着：点 tabBar 上「菜谱」不会打开卡片
    let bar_y = H - 25.0;
    let cards = a.rects(".rc");
    assert!(cards.iter().any(|&(_, y, _, h)| y < bar_y && y + h > bar_y), "tabBar 下面压着一张卡");
    a.tap_xy(W / 6.0, bar_y);
    a.settle();
    assert_eq!(a.route(), "pages/index/index");
    assert_eq!(a.depth(), 1, "没有点进详情");
    fav_from_index(&mut a, 0);
    fav_from_index(&mut a, 1);
    tab(&mut a, 1);
    assert_eq!(len(a.data("list")), 2);
    // 菜单开着：点上面那张卡（在遮罩上）只收起菜单
    use mini_render::host::ui_overlay::action_sheet;
    a.long_press_nth(".fav-item", 1);
    a.wait_ms(400);
    assert!(action_sheet::visible(), "长按弹出菜单");
    let (x, y, w, h) = a.rects(".fav-item")[0];
    a.tap_xy(x + w / 2.0, y + h / 2.0);
    wait_sheet_gone(&mut a);
    assert!(!action_sheet::visible(), "点遮罩收起菜单");
    assert_eq!(a.route(), "pages/favs/favs", "遮罩挡住了卡片的点击");
    assert_eq!(len(a.storage("rc_favs")), 2, "也没有选中菜单项");
    // 菜单收起之后正常可点
    a.tap_nth(".fav-item", 0);
    a.settle();
    assert_eq!(a.route(), "pages/detail/detail");
    a.assert_no_errors();
}

/// 操作菜单真弹出来了再点第 `i` 项（共 `n` 项），点完等它退场。
/// 菜单状态是引擎线程上的（host::ui_overlay::action_sheet），同一个用例线程里读得到
fn sheet_pick(a: &mut App, i: usize, n: usize) {
    use mini_render::host::ui_overlay::action_sheet;
    for _ in 0..60 {
        if action_sheet::visible() {
            break;
        }
        a.frames(1);
        std::thread::sleep(std::time::Duration::from_millis(14));
    }
    assert!(action_sheet::visible(), "[recipe] 长按没有弹出操作菜单");
    a.sheet_tap(i, n);
    wait_sheet_gone(a);
}

/// 等操作菜单退场完。退场起点记的是墙上时钟、收尾按帧时钟判（host/ui_overlay/action_sheet.rs），
/// 驱动里帧时钟可能落后墙上时钟最多 250ms（frame_clock::RESYNC_MS），所以按状态等、不按固定时长等
fn wait_sheet_gone(a: &mut App) {
    use mini_render::host::ui_overlay::action_sheet;
    for _ in 0..100 {
        if !action_sheet::visible() {
            break;
        }
        a.frames(1);
        std::thread::sleep(std::time::Duration::from_millis(14));
    }
}

/// 驱动的 `long_press` 只按第一个命中节点，这里补「第 i 个」
trait LongPressNth {
    fn long_press_nth(&mut self, sel: &str, i: usize);
}

impl LongPressNth for App {
    fn long_press_nth(&mut self, sel: &str, i: usize) {
        let (x, y, w, h) = self.rects(sel)[i];
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        self.e.pointer_down(cx, cy);
        for _ in 0..40 {
            self.t += 16;
            std::thread::sleep(std::time::Duration::from_millis(16));
            self.e.pump(self.t);
        }
        self.e.pointer_up(cx, cy);
        self.frames(4);
    }
}
