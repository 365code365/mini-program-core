//! 移动端 SDK（[`MiniEngine`]）的**指针链路**回归。
//!
//! 为什么单独一套：`tools/sdk-parity.sh` 只比静态页面的像素，`tools/tab-click-check.sh`
//! 与 `tools/interaction-check.sh` 走的是桌面窗体。于是「SDK 的输入退化成简化版」
//! 这类分叉一直没人守 —— 引擎从前 `pointer_down` 直接把位移全给页面滚动：
//! 没有方向锁定、没有嵌套交接、没有 picker 面板、tabBar 只认原生配置。
//!
//! 这些用例只依赖 `sample/` 里固定的示例小程序，不联网、不开窗口，可以进 CI。

use crate::host::MiniEngine;

fn engine(route: &str) -> MiniEngine {
    let root = crate::app_dir::resolve("sample-app");
    let data = crate::data_dir::subdir("engine-input-tests");
    // 每个用例一个实例：storage 落在专用目录里，互不影响
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
    e
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
    let mut e = engine("pages/index/index");
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
    let mut e = engine("pages/index/index");
    // y=300 附近是首页的横向卡片区；竖向位移足够跨过 4px 的方向锁定阈值
    swipe(&mut e, (180.0, 330.0), (180.0, 130.0), 8);
    let pos = e.scroll_position();
    assert!(pos > 20.0, "横滑容器上的竖向拖动应落到页面滚动，实际 {pos}");
}

#[test]
fn 横向拖动不该让页面上下跳() {
    let mut e = engine("pages/index/index");
    swipe(&mut e, (300.0, 330.0), (60.0, 330.0), 8);
    let pos = e.scroll_position();
    assert!(pos.abs() < 0.5, "纯横向拖动不该改变页面滚动位置，实际 {pos}");
}

#[test]
fn 惯性滚动中按一下只停住不算点击() {
    let mut e = engine("pages/index/index");
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
    let mut e = engine("pages/index/index");
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
    let mut e = engine("pages/showcase/showcase");
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

#[test]
fn 弹窗按钮按下与抬手落在同一个按钮才回调() {
    let mut e = engine("pages/profile/profile");
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
