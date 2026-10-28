//! 移动端 SDK（[`MiniEngine`]）的**指针链路**回归。
//!
//! 为什么单独一套：`tools/sdk-parity.sh` 只比静态页面的像素，`tools/tab-click-check.sh`
//! 与 `tools/interaction-check.sh` 走的是桌面窗体。于是「SDK 的输入退化成简化版」
//! 这类分叉一直没人守 —— 引擎从前 `pointer_down` 直接把位移全给页面滚动：
//! 没有方向锁定、没有嵌套交接、没有 picker 面板、tabBar 只认原生配置。
//!
//! 这些用例只依赖 `sample/` 里固定的示例小程序，不联网、不开窗口，可以进 CI。

use crate::host::MiniEngine;

/// `slot` 用来给每个用例分一个**独立的 storage 目录**。
///
/// 别图省事共用一个：`wx.setStorageSync` 是写穿到磁盘的，测试并行跑时会互相读到
/// 对方写的购物车/设置，页面内容随之变化 —— 表现是「单独跑必过、全量跑偶发失败」。
pub(crate) fn engine_in(route: &str, slot: &str) -> super::LockedEngine {
    let g = super::LockedEngine::lock();
    let root = crate::app_dir::resolve("sample-app");
    let data = crate::data_dir::subdir(&format!("engine-input-tests/{slot}"));
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new(
        &root.to_string_lossy(),
        Some(&data.to_string_lossy()),
        375,
        667,
        2.0,
    )
    .expect("创建引擎");
    e.set_animation_time(2.0);
    e.launch(Some(route)).expect("打开页面");
    e.pump(1);
    super::LockedEngine::new(e, g)
}

/// 一次完整触摸：按下 → 若干步移动 → 抬起（步进给状态机足够的位移判定）
fn swipe(e: &mut MiniEngine, from: (f32, f32), to: (f32, f32), steps: u32) {
    e.pointer_down(from.0, from.1);
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        e.pointer_move(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        e.pump(0);
    }
    e.pointer_up(to.0, to.1);
    e.pump(0);
}

#[test]
fn 纵向拖动能滚起页面() {
    let mut e = engine_in("pages/index/index", "drag-v");
    assert_eq!(e.scroll_position(), 0.0, "初始应在顶部");
    // 手指上滑 240px：页面应该跟着滚下去（不是 0，也不该超过内容上限）
    swipe(&mut e, (180.0, 500.0), (180.0, 260.0), 8);
    let pos = e.scroll_position();
    assert!(pos > 50.0, "纵向拖动应滚动页面，实际 {pos}");
}

#[test]
fn 横向卡片上竖着划交给页面滚动() {
    // 首页那排横滑卡片：竖着划不该被它吃掉（微信/浏览器的方向锁定语义）。
    // 旧的 SDK 实现里 scroll-view 一旦命中就无条件接管，于是这一划什么都不动。
    let mut e = engine_in("pages/index/index", "lock-v");
    // y=300 附近是首页的横向卡片区；竖向位移足够跨过 4px 的方向锁定阈值
    swipe(&mut e, (180.0, 330.0), (180.0, 130.0), 8);
    let pos = e.scroll_position();
    assert!(pos > 20.0, "横滑容器上的竖向拖动应落到页面滚动，实际 {pos}");
}

#[test]
fn 横向拖动不该让页面上下跳() {
    let mut e = engine_in("pages/index/index", "drag-h");
    swipe(&mut e, (300.0, 330.0), (60.0, 330.0), 8);
    let pos = e.scroll_position();
    assert!(pos.abs() < 0.5, "纯横向拖动不该改变页面滚动位置，实际 {pos}");
}

#[test]
fn 惯性滚动中按一下只停住不算点击() {
    let mut e = engine_in("pages/index/index", "fling");
    // 甩一把让页面进入惯性
    swipe(&mut e, (180.0, 560.0), (180.0, 160.0), 4);
    let route_before = e.current_route();
    // 惯性还在跑的时候点一下：应该只是停住，不触发任何跳转
    e.pointer_down(180.0, 300.0);
    e.pointer_up(180.0, 300.0);
    e.pump(0);
    assert_eq!(e.current_route(), route_before, "惯性中的那一下不该当成点击去导航");
}

#[test]
fn 点底部tabbar能切页() {
    // sample-app 用的是自定义 tabBar（页面内组件），旧实现只按 app.json 的 list
    // 平均分栏算下标、完全不看组件的事件绑定 —— 手机上点 tabBar 没反应就是这条。
    let mut e = engine_in("pages/index/index", "tabbar");
    let h = 667.0 - crate::host::tabbar_height() as f32;
    // 第二格（分类）：375/4 = 93.75，中心 ~141
    e.pointer_down(141.0, h + 25.0);
    e.pointer_up(141.0, h + 25.0);
    for _ in 0..4 {
        e.pump(0);
    }
    assert_eq!(
        e.current_route(),
        "pages/category/category",
        "点第二个 tab 应切到分类页"
    );
}

#[test]
fn 点picker会弹出底部面板并能确定() {
    let mut e = engine_in("pages/showcase/showcase", "picker");
    // picker 在内容坐标 y≈834（用 MINI_PICKER_LOG 量过），滚到 300 后落在视口 534
    e.scroll_to(300.0);
    e.pump(0);
    assert!(!e.picker_sheet_open(), "初始不该有面板");
    e.pointer_down(180.0, 540.0);
    e.pointer_up(180.0, 540.0);
    e.pump(0);
    assert!(e.picker_sheet_open(), "点在 <picker> 上应弹出底部面板");
    // 面板弹着时点页面别处不该穿透（面板吃掉事件）
    let route = e.current_route();
    e.pointer_down(180.0, 100.0);
    e.pointer_up(180.0, 100.0);
    e.pump(0);
    assert_eq!(e.current_route(), route, "面板弹着时点击不该穿透到页面");
}

/// 打开 showcase 页那个 selector 的面板，并等入场动画走完（命中坐标随面板位移变化）
fn open_picker(slot: &str) -> super::LockedEngine {
    let mut e = engine_in("pages/showcase/showcase", slot);
    e.scroll_to(300.0);
    e.pump(0);
    e.pointer_down(180.0, 540.0);
    e.pointer_up(180.0, 540.0);
    e.pump(0);
    assert!(e.picker_sheet_open(), "点在 <picker> 上应弹出底部面板");
    // 入场位移按墙上时钟走 0.22s
    std::thread::sleep(std::time::Duration::from_millis(300));
    e.pump(0);
    e
}

/// 面板第一列：(选中下标, 项数)
fn picker_col0(e: &MiniEngine) -> (usize, usize) {
    let s = e.picker_sheet.as_ref().expect("面板应该还在");
    (s.selected[0], s.columns[0].len())
}

#[test]
fn picker面板上下拖动能换选项且不漏给页面() {
    // 375×667 视口：面板顶 667-265=402，滚轮从 447 开始，中心行 535..579
    let mut e = open_picker("picker-drag");
    let (before, n) = picker_col0(&e);
    assert!(n >= 2, "这个 picker 至少要有两项才测得了拖动");
    let scroll_before = e.scroll_position();
    // 往还有项的那一头拖一行：手指上推 = 往后面的项走
    let dir: f32 = if before + 1 < n { -1.0 } else { 1.0 };
    let y0 = 557.0;
    let row = crate::host::picker_sheet::ITEM_H;
    let slop = crate::host::touch::TOUCH_SLOP;
    e.pointer_down(180.0, y0);
    // 第一步跨过拖动阈值（滚轮在这里锚定），之后匀速走满一行
    let anchor = y0 + dir * (slop + 1.0);
    e.pointer_move(180.0, anchor);
    for i in 1..=4 {
        e.pointer_move(180.0, anchor + dir * row * i as f32 / 4.0);
        e.pump(0);
    }
    assert!(e.picker_sheet.as_ref().unwrap().wheel_dragging(), "滚轮应进入拖动");
    // 停住再松手：速度窗口（100ms）里没有位移，不带惯性，正好落一行
    std::thread::sleep(std::time::Duration::from_millis(150));
    e.pointer_up(180.0, anchor + dir * row);
    e.pump(0);
    let (after, _) = picker_col0(&e);
    let expect = if dir < 0.0 { before + 1 } else { before - 1 };
    assert_eq!(after, expect, "拖一行应换到相邻项（之前 {before}，共 {n} 项）");
    assert!(e.picker_sheet_open(), "拖动松手不该关掉面板");
    assert_eq!(e.scroll_position(), scroll_before, "面板上的拖动不该滚动底下的页面");
}

#[test]
fn picker面板快速一甩会继续滑并吸附到整行() {
    let mut e = open_picker("picker-fling");
    let (before, n) = picker_col0(&e);
    // 往还有余量的那一头甩
    let dir: f32 = if before + 1 < n { -1.0 } else { 1.0 };
    let y0 = 557.0;
    e.pointer_down(180.0, y0);
    for i in 1..=6 {
        e.pointer_move(180.0, y0 + dir * 20.0 * i as f32);
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
    e.pointer_up(180.0, y0 + dir * 120.0);
    e.pump(0);
    let (after, _) = picker_col0(&e);
    if dir < 0.0 {
        assert!(after > before, "上推应往后面的项走：{before} -> {after}");
    } else {
        assert!(after < before, "下拉应往前面的项走：{before} -> {after}");
    }
    // 回正动画要续帧，走完停在整行上
    assert!(e.picker_sheet.as_ref().unwrap().animating(), "松手后应有回正动画在走");
    std::thread::sleep(std::time::Duration::from_millis(900));
    e.pump(0);
    let s = e.picker_sheet.as_ref().unwrap();
    assert_eq!(s.column_pos(0, std::time::Instant::now()), s.selected[0] as f32);
}

#[test]
fn 弹窗按钮按下与抬手落在同一个按钮才回调() {
    let mut e = engine_in("pages/profile/profile", "modal");
    // 先声明再赋值：QuickJS 里给未声明的变量赋值会 ReferenceError，
    // 回调里那一行会整条中断（表现就是「回调没跑」）
    e.eval("var __modalResult = 'none';").ok();
    e.eval("wx.showModal({title:'t',content:'c',success(r){ __modalResult = r.confirm; }})")
        .ok();
    e.pump(0);
    assert!(e.modal_open(), "showModal 之后应有弹窗");
    // 按下确定（右半边）后滑到取消再松手：不该回调，也不该关闭
    e.pointer_down(250.0, 385.0);
    e.pointer_up(120.0, 385.0);
    e.pump(0);
    assert!(e.modal_open(), "按下与抬手不在同一个按钮上时不该关闭弹窗");
    // 规规矩矩点确定
    e.pointer_down(250.0, 385.0);
    e.pointer_up(250.0, 385.0);
    e.pump(0);
    assert!(!e.modal_open(), "点确定应关闭弹窗");
    let got = e.eval("String(__modalResult)").unwrap_or_default();
    assert!(
        got.contains("true"),
        "点确定应触发 success(confirm:true)，实际 __modalResult = {got:?}"
    );
}

// ───────────────────────────── 键盘：textarea 换行与快捷键 ─────────────────────────────
//
// 「SDK 里 textarea 按回车出豆腐块」就是这条链路：宿主把回车作为 `text_input("\n")` 送进来，
// 引擎原样塞进值里、单行画出。这里直接驱动 `MiniEngine`，与 Flutter / 原生宿主走同一个入口。

/// 取空事件队列（JSON）
fn drain_events(e: &mut MiniEngine) -> Vec<serde_json::Value> {
    std::iter::from_fn(|| e.poll_event()).filter_map(|s| serde_json::from_str(&s).ok()).collect()
}

/// 滚到 showcase 页的 textarea（「备注」）并点进去，清空原有内容
fn focus_textarea(slot: &str) -> super::LockedEngine {
    use crate::ui::interaction::InteractionType;
    let mut e = engine_in("pages/showcase/showcase", slot);
    for step in 0..16 {
        e.scroll_to(step as f32 * 150.0);
        e.pump(0);
        let pos = e.scroll_position();
        let r = e.renderer.as_ref();
        let hit = e.interaction.elements().iter().find(|el| {
            el.interaction_type == InteractionType::Input
                && !el.is_fixed
                && r.and_then(|r| r.input_region(&el.id)).is_some_and(|r| r.tag == "textarea")
                && el.bounds.y - pos > 40.0
                && el.bounds.y + el.bounds.height - pos < 540.0
        });
        if let Some(b) = hit.map(|el| el.bounds) {
            let (x, y) = (b.x + 20.0, b.y - pos + b.height / 2.0);
            e.pointer_down(x, y);
            e.pointer_up(x, y);
            e.pump(0);
            let f = e.interaction.focused_input.as_ref().expect("点 textarea 应聚焦");
            assert!(f.multiline, "textarea 聚焦后应按多行编辑");
            assert_eq!(f.confirm_type, "return", "textarea 默认 confirm-type 是 return");
            e.key("selectall");
            e.key("backspace");
            return e;
        }
    }
    panic!("showcase 页上没找到 textarea");
}

#[test]
fn sdk里textarea回车是换行而且不收键盘() {
    let mut e = focus_textarea("ta-newline");
    let focus = drain_events(&mut e).into_iter().find(|v| v["type"] == "inputFocus").expect("应发 inputFocus");
    assert_eq!(focus["tag"], "textarea");
    assert_eq!(focus["confirmType"], "return", "宿主据此把右下角设成换行键");
    assert!(focus["cursor"].is_u64(), "inputFocus 要带光标位置：{focus}");

    e.text_input("ab");
    e.text_input("\n"); // 宿主的回车
    e.text_input("cd");
    e.pump(0);
    assert_eq!(e.interaction.focused_input.as_ref().unwrap().value, "ab\ncd");
    assert!(
        !drain_events(&mut e).iter().any(|v| v["type"] == "inputBlur"),
        "textarea 换行不该收键盘"
    );
    let note = e.eval("JSON.stringify(getCurrentPages()[0].data.note)").unwrap_or_default();
    assert_eq!(note, r#""ab\ncd""#, "bindinput 拿到的值里有真正的换行");
}

#[test]
fn sdk的快捷键按名字都能用() {
    let mut e = focus_textarea("ta-keys");
    e.text_input("one\ntwo");
    e.key("up");
    e.key("home");
    e.key("shift+end");
    e.key("cut"); // 剪掉第一行
    assert_eq!(e.interaction.focused_input.as_ref().unwrap().value, "\ntwo");
    let clip = drain_events(&mut e).into_iter().find(|v| v["type"] == "clipboard");
    assert_eq!(clip.map(|v| v["data"].clone()), Some(serde_json::json!("one")), "剪切要让宿主写系统剪贴板");
    let note = e.eval("getCurrentPages()[0].data.note").unwrap_or_default();
    assert_eq!(note, "\ntwo", "剪切也是一次 bindinput");
    e.key("undo");
    assert_eq!(e.interaction.focused_input.as_ref().unwrap().value, "one\ntwo");
    e.key("doc-end");
    e.key("delete-word-back");
    assert_eq!(e.interaction.focused_input.as_ref().unwrap().value, "one\n");
    e.key("escape");
    assert!(e.interaction.focused_input.is_none());
}

/// 慢拖之后松手：移动事件几个一批地到（经 Dart → 平台线程 → 引擎线程转过来），松手惯性的速度
/// 要按**事件时间**算。从前按到达时刻：一批事件挤在 1ms 里，速度算成真实值的十几倍，
/// 一加真机慢拖每帧 2.3px、松手第一帧却跳了 44.5px（engine_resample.rs）
#[test]
fn 带时间戳的慢拖_事件一批到_松手惯性按真实速度() {
    let mut e = engine_in("pages/detail/detail", "drag-burst");
    let mut t = 1_000u64;
    e.pump(t);
    // 手指每 8ms 上移 2px（250px/s）。前 30 帧事件按时到；之后主线程卡了 120ms，
    // 这段时间里的 15 个事件在松手前一刻一起到（到达时刻挤在同一毫秒里）
    e.pointer_down(180.0, 500.0);
    let mut y = 500.0f32;
    let mut ev = t as f64;
    for _ in 0..30 {
        ev += 8.0;
        y -= 2.0;
        e.pointer_move_at(180.0, y, ev);
        t += 8;
        e.pump(t);
    }
    for _ in 0..15 {
        t += 8;
        e.pump(t);
    }
    // 屏上最后一帧的位置（之后的事件还没出帧）
    let shown = e.scroll_position();
    for _ in 0..15 {
        ev += 8.0;
        y -= 2.0;
        e.pointer_move_at(180.0, y, ev);
        // 一批事件实际铺在一两毫秒里（真机上的样子；全挤在同一毫秒的话旧算法算出 0 速度，测不出来）
        std::thread::sleep(std::time::Duration::from_micros(150));
    }
    e.pointer_up(180.0, y);
    t += 8;
    e.pump(t);
    let first = e.scroll_position() - shown;
    // 250px/s × 0.8（release 系数）≈ 200px/s，一帧 8ms 约 1.6px；按到达时刻算会是几十 px
    assert!(first.abs() < 6.0, "松手第一帧走了 {first:.1}px（应与拖动时的每帧位移同一量级）");
}
