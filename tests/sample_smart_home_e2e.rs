//! 智慧家（sample/smart-home）：每页能打开；房间 tab 与 swiper 双向联动；卡片开关（catchtap 不跳详情、离线不可控、
//! storage 持久化）；空调温度 16~30 边界 / 模式 radio / 风速 slider / 定时 picker；灯光亮度色温 slider 与颜色块；
//! 一键场景（loading → 批量改设备 → toast）；自定义场景的输入校验、checkbox 勾选、保存、执行、长按删除；
//! 能耗周 / 月切换与条形宽度。原生 tabBar 一律点屏幕底部 50px 那一条。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::{json, Value};

const APP: &str = "smart-home";

/// 点原生 tabBar 第 `i` 个（共 3 个，底部 50px 等分）
fn tab_bar(a: &mut App, i: usize) {
    let w = W / 3.0;
    a.tap_xy(w * i as f32 + w / 2.0, H - 25.0);
    a.settle();
}

/// storage 里某台设备
fn dev(a: &App, id: &str) -> Value {
    a.val(&format!("(wx.getStorageSync('sh_devices') || []).filter(function (d) {{ return d.id === '{id}'; }})[0]"))
}

/// 把 slider 从轨道左端附近拖到 `to_x`（视口 x）
fn slide_to(a: &mut App, sel: &str, to_x: f32) {
    let (x, y, w, h) = a.rect(sel);
    let from = x + w * 0.5;
    a.drag(from, y + h / 2.0, to_x - from, 0.0, 10);
    a.frames(4);
}

fn slide_max(a: &mut App, sel: &str) {
    let (x, _, w, _) = a.rect(sel);
    slide_to(a, sel, x + w + 30.0);
}

fn slide_min(a: &mut App, sel: &str) {
    let (x, _, _, _) = a.rect(sel);
    slide_to(a, sel, (x - 30.0).max(1.0));
}

/// 当前 swiper 页左上第一张卡的图标颜色（卡片左上 + 18px 处）
fn icon_px(a: &mut App) -> [u8; 3] {
    let (_, y, _, _) = a.rect("#dev-ac1"); // 纵坐标与页无关
    a.frames(2);
    let px = a.e.rgba().to_vec();
    let (x, y) = ((30.0 * DPR) as usize, ((y + 30.0) * DPR) as usize);
    let i = (y * (W * DPR) as usize + x) * 4;
    [px[i], px[i + 1], px[i + 2]]
}

fn backspace(a: &mut App, n: usize) {
    for _ in 0..n {
        a.key("backspace");
    }
}

#[test]
fn 智慧家_每页都能打开() {
    let _g = serial();
    smoke_all_pages(APP, &[("pages/ac/ac", "id=ac1"), ("pages/light/light", "id=light1")]);
}

#[test]
fn 智慧家_房间tab点击与swiper滑动双向联动() {
    let _g = serial();
    let mut a = App::open(APP, None, "rooms");
    assert_eq!(a.route(), "pages/home/home");
    assert_eq!(a.data("current"), 0);
    assert_eq!(a.data("pages.length"), 4);
    assert_eq!(a.data("pages[0].length"), 4, "客厅 4 台设备");
    // 点「厨房」tab → swiper 跟过去
    a.tap_nth(".tab", 2);
    a.wait_ms(400);
    assert_eq!(a.data("current"), 2);
    let on = a.rect(".tab.on");
    let kitchen = a.rects(".tab")[2];
    assert_eq!(on, kitchen, "高亮落在厨房 tab 上");
    // 画面跟过去：左上第一张卡的图标是冰箱（开着，#06d6a0），不是客厅空调（关着，灰）
    assert_eq!(icon_px(&mut a), [6, 214, 160], "swiper 切到厨房页");
    // 手指往左滑 swiper → 书房，bindchange 同步 tab
    let (sx, sy, sw, sh) = a.rect(".rooms");
    a.drag(sx + sw * 0.8, sy + sh * 0.3, -sw * 0.6, 0.0, 10);
    a.wait_ms(500);
    assert_eq!(a.data("current"), 3, "滑到书房");
    assert_eq!(icon_px(&mut a), [0xd9, 0xde, 0xe7], "书房第一台（台灯）关着：灰图标");
    assert!(a.data("current").is_u64(), "e.detail.current 是 number");
    assert_eq!(a.rect(".tab.on"), a.rects(".tab")[3]);
    // 往右滑回厨房
    a.drag(sx + sw * 0.2, sy + sh * 0.3, sw * 0.6, 0.0, 10);
    a.wait_ms(500);
    assert_eq!(a.data("current"), 2);
    a.assert_no_errors();
}

#[test]
fn 智慧家_卡片开关_不跳详情_离线不可控_持久化() {
    let _g = serial();
    let mut a = App::open(APP, None, "switch");
    assert_eq!(a.data("running"), 2, "初始：吸顶灯与冰箱在运行");
    assert_eq!(a.data("online"), 10);
    assert_eq!(a.data("total"), 12);
    // 客厅第 3 张卡（智能电视）的开关
    a.tap_nth(".sw", 2);
    a.frames(3);
    assert_eq!(a.depth(), 1, "点开关不跳详情（外层 catchtap）");
    assert_eq!(a.data("pages[0][2].on"), true);
    assert_eq!(a.data("pages[0][2].status"), "运行中");
    assert_eq!(a.data("running"), 3);
    assert_eq!(dev(&a, "tv1")["on"], true, "落盘");
    // 离线的电动窗帘：开关 disabled，点了不变
    a.tap_nth(".sw", 3);
    a.frames(3);
    assert_eq!(dev(&a, "curtain1")["on"], false, "离线设备点开关无效");
    assert_eq!(a.data("pages[0][3].status"), "离线");
    // 点离线卡片本体：只提示，不跳
    a.tap("#dev-curtain1 .dev-name");
    a.frames(3);
    assert_eq!(a.depth(), 1);
    // 关吸顶灯，再「全关」客厅
    a.tap_nth(".sw", 0); // 空调打开
    assert_eq!(dev(&a, "ac1")["on"], true);
    a.tap(".room-off");
    assert_eq!(dev(&a, "ac1")["on"], false, "全关客厅");
    assert_eq!(dev(&a, "light1")["on"], false);
    assert_eq!(dev(&a, "tv1")["on"], false);
    assert_eq!(dev(&a, "fridge1")["on"], true, "别的房间不动");
    assert_eq!(a.data("running"), 1);
    a.wait_ms(1600);
    // 重启后读回来，再拨一次电视
    a.js("wx.reLaunch({ url: '/pages/home/home' })");
    a.settle();
    assert_eq!(a.data("pages[0][2].on"), false, "重启后电视仍关着");
    assert_eq!(a.data("pages[0][1].on"), false, "吸顶灯仍关着");
    assert_eq!(a.data("running"), 1);
    a.tap_nth(".sw", 2);
    assert_eq!(dev(&a, "tv1")["on"], true, "重启后开关照常可拨");
    assert_eq!(a.data("running"), 2);
    a.assert_no_errors();
}

#[test]
fn 智慧家_空调_温度边界_模式_风速slider_定时picker() {
    let _g = serial();
    let mut a = App::open(APP, None, "ac");
    a.tap_settle("#dev-ac1 .dev-name");
    assert_eq!(a.route(), "pages/ac/ac");
    assert_eq!(a.depth(), 2);
    assert_eq!(a.data("dev.id"), "ac1");
    // 关着时调温：不变
    a.tap(".plus");
    assert_eq!(a.data("dev.temp"), 26);
    a.tap(".power-btn");
    assert_eq!(a.data("dev.on"), true);
    for _ in 0..4 {
        a.tap(".plus");
    }
    assert_eq!(a.data("dev.temp"), 30);
    a.tap(".plus");
    assert_eq!(a.data("dev.temp"), 30, "最高 30");
    assert!(a.exists(".plus.disabled"));
    for _ in 0..15 {
        a.tap(".minus");
    }
    assert_eq!(a.data("dev.temp"), 16, "最低 16");
    assert!(a.exists(".minus.disabled"));
    assert_eq!(dev(&a, "ac1")["temp"], 16);
    // 模式 radio
    a.tap_nth(".mode-radio", 1);
    assert_eq!(a.data("dev.mode"), "heat");
    a.tap_nth(".mode", 3); // 点 label 的文字也能选
    assert_eq!(a.data("dev.mode"), "fan");
    a.tap(".plus");
    assert_eq!(a.data("dev.temp"), 16, "送风不能调温");
    // 风速 slider：拖到最右 5 档、最左 1 档（number）
    slide_max(&mut a, ".wind");
    assert_eq!(a.data("dev.wind"), 5);
    assert!(a.data("dev.wind").is_u64(), "slider 的值是 number");
    assert_eq!(a.data("winds[5]"), "强劲");
    slide_min(&mut a, ".wind");
    assert_eq!(a.data("dev.wind"), 1);
    assert_eq!(dev(&a, "ac1")["wind"], 1);
    // 定时 picker：不定时 → 往下两行「1 小时后关」
    a.picker(".timer");
    a.picker_row(0, 1, 2);
    a.picker_confirm();
    assert_eq!(a.data("dev.timer"), 2);
    assert_eq!(dev(&a, "ac1")["timer"], 2);
    a.wait_ms(1600);
    a.back();
    assert_eq!(a.route(), "pages/home/home");
    assert_eq!(a.data("pages[0][0].status"), "送风 16℃", "首页卡片跟着变");
    a.assert_no_errors();
}

#[test]
fn 智慧家_灯光_亮度色温slider_预设颜色块() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/light/light?id=light2"), "light");
    assert_eq!(a.data("dev.on"), false);
    assert_eq!(a.data("dev.brightness"), 40);
    slide_min(&mut a, ".bright");
    assert_eq!(a.data("dev.brightness"), 1);
    assert_eq!(a.data("dev.on"), true, "调亮度顺手开灯");
    slide_max(&mut a, ".bright");
    assert_eq!(a.data("dev.brightness"), 100);
    slide_max(&mut a, ".ct");
    assert_eq!(a.data("dev.ct"), 6500);
    // 点色温轨道中间：按 100K 一档取整，落在中段
    let (x, y, w, h) = a.rect(".ct");
    a.tap_xy(x + w / 2.0, y + h / 2.0);
    a.frames(3);
    let ct = a.data("dev.ct").as_u64().expect("色温是 number");
    assert!(ct % 100 == 0 && (4200..=5000).contains(&ct), "中间一点 ≈ 4600K：{ct}");
    a.tap_nth(".color", 3);
    assert_eq!(a.data("dev.color"), "#b8e0ff");
    assert_eq!(a.data("colorName"), "冰蓝");
    assert_eq!(a.count(".color.cur"), 1);
    let d = dev(&a, "light2");
    assert_eq!((d["brightness"].clone(), d["ct"].clone(), d["color"].clone(), d["on"].clone()), (json!(100), json!(ct), json!("#b8e0ff"), json!(true)));
    a.tap(".power-btn");
    assert_eq!(dev(&a, "light2")["on"], false);
    a.assert_no_errors();
}

#[test]
fn 智慧家_一键场景_回家离家睡眠_批量改设备() {
    let _g = serial();
    let mut a = App::open(APP, None, "scene");
    tab_bar(&mut a, 1);
    assert_eq!(a.route(), "pages/scene/scene", "点 tabBar 中间一格");
    assert_eq!(a.depth(), 1);
    a.tap(".scene-home");
    assert_eq!(a.data("running"), "home", "执行中");
    a.tap(".scene-away"); // 执行中连点无效
    assert_eq!(a.data("running"), "home");
    a.wait_ms(1300);
    assert_eq!(a.data("running"), "");
    assert_eq!(a.data("lastResult"), "回家：已调整 2 台设备，1 台离线已跳过");
    assert_eq!(dev(&a, "ac1")["on"], true);
    assert_eq!(dev(&a, "tv1")["on"], true);
    assert_eq!(dev(&a, "curtain1")["on"], false, "离线窗帘跳过");
    assert_eq!(a.storage("sh_last_scene"), "home");
    assert_eq!(a.count(".scene.last"), 1);
    a.wait_ms(1600);
    tab_bar(&mut a, 0);
    assert_eq!(a.route(), "pages/home/home");
    assert_eq!(a.data("running"), 4, "首页 onShow 读到场景改过的状态");
    tab_bar(&mut a, 1);
    a.tap(".scene-sleep");
    a.wait_ms(1300);
    let ac2 = dev(&a, "ac2");
    assert_eq!((ac2["on"].clone(), ac2["temp"].clone(), ac2["mode"].clone(), ac2["wind"].clone()), (json!(true), json!(27), json!("dry"), json!(1)));
    assert_eq!(dev(&a, "light1")["on"], false);
    assert_eq!(dev(&a, "humid1")["on"], true);
    a.wait_ms(1600);
    a.tap(".scene-away");
    a.wait_ms(1300);
    let on: Vec<String> = a
        .val("wx.getStorageSync('sh_devices').filter(function (d) { return d.on; }).map(function (d) { return d.id; })")
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(on, vec!["fridge1"], "离家：只剩冰箱");
    a.wait_ms(1600);
    a.assert_no_errors();
}

#[test]
fn 智慧家_自定义场景_校验_勾选_保存_执行_长按删除() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/scene/scene"), "custom");
    assert!(a.exists(".empty"));
    a.tap_settle(".add");
    assert_eq!(a.route(), "pages/scene-edit/scene-edit");
    assert_eq!(a.data("groups.length"), 4);
    a.tap(".save");
    assert_eq!(a.data("error"), "请输入场景名称");
    a.type_into(".name-input", "回家");
    assert_eq!(a.data("name"), "回家");
    assert_eq!(a.data("error"), "", "改名字清掉错误");
    a.blur();
    a.tap(".save");
    assert_eq!(a.data("error"), "已有同名场景");
    a.tap(".name-input");
    backspace(&mut a, 2);
    a.e.text_input("周末清晨起床音乐模式");
    a.frames(3);
    a.blur();
    a.tap(".save");
    assert_eq!(a.data("error"), "名称不超过 8 个字");
    a.tap(".name-input");
    backspace(&mut a, 10);
    a.e.text_input("观影模式");
    a.frames(3);
    a.blur();
    assert_eq!(a.data("name"), "观影模式");
    a.tap(".save");
    assert_eq!(a.data("error"), "至少选择一台设备");
    a.tap("#cb-tv1");
    a.tap("#cb-light1");
    assert_eq!(a.data("selected"), json!(["light1", "tv1"]), "checkbox-group 的 detail.value 按文档顺序");
    assert_eq!(a.data("groups[0].list[1].checked"), true);
    a.tap("#cb-light1");
    assert_eq!(a.data("selected"), json!(["tv1"]), "再点取消勾选");
    a.tap("#cb-light1");
    // 保存键在下面：先滚上去
    a.scroll_by(W / 2.0, 500.0, 400.0);
    a.tap_settle(".save");
    assert_eq!(a.route(), "pages/scene/scene", "保存后返回");
    assert_eq!(a.depth(), 1);
    assert_eq!(a.data("custom.length"), 1);
    assert_eq!(a.data("custom[0].name"), "观影模式");
    assert_eq!(a.data("custom[0].desc"), "打开 客厅吸顶灯、智能电视");
    assert_eq!(a.storage("sh_scenes")[0]["devices"], json!(["light1", "tv1"]));
    // 执行自定义场景
    a.tap(".c-item");
    a.wait_ms(1300);
    assert_eq!(dev(&a, "tv1")["on"], true);
    assert_eq!(a.data("lastResult"), "观影模式：已调整 1 台设备", "吸顶灯本来就开着");
    a.wait_ms(1600);
    // 长按 → 操作菜单 → 删除
    a.long_press(".c-item");
    a.sheet_tap(0, 1);
    assert_eq!(a.data("custom.length"), 0);
    assert_eq!(a.storage("sh_scenes"), json!([]));
    a.wait_ms(1600);
    a.assert_no_errors();
}

#[test]
fn 智慧家_能耗_周月切换_条形宽度按比例() {
    let _g = serial();
    let mut a = App::open(APP, None, "energy");
    tab_bar(&mut a, 2);
    assert_eq!(a.route(), "pages/energy/energy");
    assert_eq!(a.data("period"), "week");
    assert_eq!(a.data("total"), 52.6);
    assert_eq!(a.data("cost"), "29.46");
    assert_eq!(a.data("bars[0].id"), "ac1");
    assert_eq!(a.data("bars[0].pct"), 100);
    assert_eq!(a.data("bars[1].pct"), 67);
    assert_eq!(a.count(".b-row"), 5, "默认只显示前 5");
    let bars = a.rects(".b-bar");
    let track = a.rect(".b-track");
    assert!((bars[0].2 - track.2).abs() < 1.0, "100% 条与轨道同宽：{} vs {}", bars[0].2, track.2);
    let ratio = bars[1].2 / bars[0].2;
    assert!((ratio - 0.67).abs() < 0.02, "第二条是第一条的 67%：{ratio}");
    a.tap(".seg-month");
    assert_eq!(a.data("period"), "month");
    assert_eq!(a.data("total"), 229.0);
    assert_eq!(a.data("bars[1].pct"), 69);
    assert_eq!(a.count(".seg-i.on"), 1);
    a.tap(".expand");
    assert_eq!(a.count(".b-row"), 12, "展开全部");
    a.tap(".seg-week");
    assert_eq!(a.data("total"), 52.6);
    a.assert_no_errors();
}

/// 曾经的引擎问题（已修，留作回归）：swiper 切页后，swiper-item 里节点的 boundingClientRect 仍按第 0 页算（没减 swiper 的平移）。
/// 画面是对的（见上面像素断言），查询结果差了 current × 宽度
#[test]
fn 回归_swiper切页后_节点查询坐标未跟着平移() {
    let _g = serial();
    let mut a = App::open(APP, None, "swiper-rect");
    a.tap_nth(".tab", 2);
    a.wait_ms(800);
    assert_eq!(a.data("current"), 2);
    let (fx, _, _, _) = a.rect("#dev-fridge1");
    assert!(fx >= 0.0 && fx < W, "厨房页的冰箱卡已在屏上，查询却给 x={fx}（期望 12）");
}

/// 曾经的引擎问题（已修，留作回归）：`<switch>` 的 change 沿祖先冒泡，被外层 `<swiper bindchange>` 收到（detail = { value: true }）。
/// 微信里除 tap/touch*/longpress 等之外的组件事件都不冒泡
#[test]
fn 回归_switch的change冒泡到外层swiper() {
    let _g = serial();
    let mut a = App::open(APP, None, "bubble");
    a.js("globalThis.__sw = []; var p = getCurrentPages()[0]; var o = p.onSwiper; p.onSwiper = function (e) { __sw.push(e.detail); return o.apply(this, arguments); };");
    a.tap_nth(".sw", 2);
    a.frames(3);
    assert_eq!(dev(&a, "tv1")["on"], true, "开关本身拨动了");
    assert_eq!(a.val("__sw"), json!([]), "swiper 的 bindchange 不该收到 switch 的 change");
}

/// 曾经的引擎问题（已修，留作回归）：`<switch checked="{{x}}">` 被点过之后，逻辑层再 setData 改 checked，组件内部状态不跟着变；
/// 下一次点击按旧的内部状态取反（这里应发 value=true，实际发 false）
#[test]
fn 回归_switch点过后_setData改checked不同步() {
    let _g = serial();
    let mut a = App::open(APP, None, "sw-sync");
    a.tap_nth(".sw", 2); // 电视 开
    assert_eq!(dev(&a, "tv1")["on"], true);
    a.tap(".room-off"); // 逻辑层把电视关了，pages[0][2].on = false
    assert_eq!(a.data("pages[0][2].on"), false);
    a.tap_nth(".sw", 2); // 应当再打开
    assert_eq!(dev(&a, "tv1")["on"], true, "按 checked=false 取反应得 true");
}
