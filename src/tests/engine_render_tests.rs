//! 移动端 SDK（[`MiniEngine`]）的**出帧**回归：页面画布的尺寸与滚动时的重绘策略。
//!
//! 与桌面窗体不同，这里没有窗口截图可看，问题只会在真机上表现为「滚到下面一片空白」
//! 或「滑动掉帧」，所以直接检查引擎内部的页面画布与合成结果。

use crate::host::MiniEngine;

fn engine_in(route: &str, slot: &str) -> MiniEngine {
    let root = crate::app_dir::resolve("sample-app");
    let data = crate::data_dir::subdir(&format!("engine-render-tests/{slot}"));
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
    e
}

#[test]
fn 长页面滚到下面不该空白() {
    // 首页内容高几千像素。页面画布从前固定 1500 逻辑像素高、从不扩容，
    // 滚过这个高度之后上屏只剩底色。
    let mut e = engine_in("pages/index/index", "long-page");
    e.scroll_to(3000.0);
    e.pump(0);
    let pos = e.scroll_position();
    assert!(pos > 1500.0, "首页应能滚过 1500px，实际 {pos}");

    let dpr = 2.0;
    let vp = e.viewport_height();
    let canvas = e.canvas.read();
    let (y0, y1) = ((pos * dpr) as u32, ((pos + vp) * dpr) as u32);
    assert!(
        canvas.height() >= y1,
        "页面画布应覆盖当前视口：画布高 {}，视口底 {}",
        canvas.height(),
        y1
    );
    let mut colors = std::collections::HashSet::new();
    for y in (y0..y1).step_by(5) {
        for x in (0..canvas.width()).step_by(5) {
            let c = canvas.get_pixel(x, y);
            colors.insert((c.r, c.g, c.b));
        }
    }
    assert!(colors.len() > 20, "视口里应该有内容，实际只有 {} 种颜色", colors.len());
}

#[test]
fn 条带内滚动只换切片且画面跟着走() {
    // 视口还在已绘制条带内时，滚动只重新合成、不重画页面画布。
    // 合成结果必须与「整页往上平移」一致，否则就是只更新了位置、画面没动。
    let mut e = engine_in("pages/showcase/showcase", "band-scroll");
    e.scroll_to(200.0);
    e.pump(0);
    let (w, _) = e.pixel_size();
    let before = e.rgba().to_vec();

    let step = 20.0;
    e.scroll_to(200.0 + step);
    assert!(e.pump(0), "滚动之后应产出新画面");
    let after = e.rgba();

    let row = w as usize * 4;
    let shift = (step * 2.0) as usize;
    // 取视口上半部分比较（避开 tabBar 与可能的底部固定栏）
    for y in 40..400usize {
        assert_eq!(
            &after[y * row..(y + 1) * row],
            &before[(y + shift) * row..(y + shift + 1) * row],
            "第 {y} 行应等于滚动前第 {} 行",
            y + shift
        );
    }
}

/// 照 iOS / Android 宿主的出帧闸门走：先 pump 一次（进页面 / 输入之后宿主总会挂一帧），
/// 之后只在引擎要求续帧（`wants_next_frame`）时接着 pump
fn pump_like_host(e: &mut MiniEngine) {
    for _ in 0..120 {
        e.pump(0);
        if !e.wants_next_frame() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
}

/// `type="2d"` 画布后备缓冲上的一个像素（键 `2d#<id>`）
fn canvas2d_px(key: &str, x: u32, y: u32) -> Option<(u8, u8, u8)> {
    let m = crate::renderer::components::CANVAS_MANAGER.lock().ok()?;
    let c = m.get_existing_context(key)?.get_canvas();
    let p = c.lock().ok()?.get_pixel(x, y);
    Some((p.r, p.g, p.b))
}

#[test]
fn 进页面就发的selector_query与回调里的画布指令不靠输入也能办完() {
    // Flutter demo 的画布页：onReady 里 createSelectorQuery 拿 canvas 节点。进页面那一帧
    // 还没布局，查询留到下一帧；回调里画的指令又要等下一次 update 才执行。从前
    // wants_next_frame 两样都不看，宿主第一帧后就停了 vsync —— 2d 画布一片空白，
    // 「换一组数据」「画个动画」也成了空操作（this.canvas 一直没拿到）。
    //
    // 用 sample-app 的登录页：没有 swiper、定时器、动画。不用临时目录搭页面 ——
    // `MiniEngine::new` 会改进程级的小程序根目录，并行跑的别的用例会找不到自己的页面。
    // 也不断言「办完就停帧」：swiper 状态是进程级的，并行的首页用例会让它一直要续帧
    let root = crate::app_dir::resolve("sample-app");
    let data = crate::data_dir::subdir("engine-render-tests/sq-wake");
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 2.0)
        .expect("创建引擎");
    e.launch(Some("pages/login/login")).expect("打开页面");
    // 还没 pump 过（没布局）就查：与 onReady 里查同一个时机
    e.eval(
        r#"var __sq = 'pending';
wx.createSelectorQuery().selectViewport().scrollOffset().exec(function () {
  __sq = 'done';
  __native_canvas_draw('2d#sq-wake', JSON.stringify([
    { type: '__reset' }, { type: 'setFillStyle', color: '#ff0000' },
    { type: 'fillRect', x: 0, y: 0, width: 20, height: 20 }
  ]));
});"#,
    )
    .expect("发查询");
    pump_like_host(&mut e);
    assert_eq!(e.eval("__sq").unwrap_or_default(), "done", "宿主停帧前 SelectorQuery 应已答复");
    assert_eq!(canvas2d_px("2d#sq-wake", 5, 5), Some((255, 0, 0)), "回调里发的画布指令应已执行");
}

#[test]
fn 待办的宿主活只算会被处理的() {
    // has_pending_host_work 决定宿主要不要续帧：漏算 = 画布空白，多算 = 静止页面空转耗电
    let mut app = crate::runtime::MiniApp::new(375, 667).expect("create MiniApp");
    app.init().expect("init");
    assert!(!app.has_pending_host_work(), "刚起来没有待办");
    // 桥上的事件：下一次 update 才处理
    app.eval("__native_canvas_draw('2d#pending-work', '[]')").ok();
    assert!(app.has_pending_host_work(), "桥上有画布指令");
    app.update().ok();
    assert!(!app.has_pending_host_work(), "update 处理完就没了");
    // SelectorQuery：update 后进宿主请求队列，等宿主答
    app.eval("wx.createSelectorQuery().selectViewport().scrollOffset().exec(function(){})").ok();
    app.update().ok();
    assert!(app.has_pending_host_work(), "没答的 SelectorQuery 要续帧");
    assert_eq!(app.take_host_requests("selectorQuery").len(), 1);
    assert!(!app.has_pending_host_work(), "答完就没了");
    // 别的种类不算：没人取的话会让宿主一直空转
    app.push_host_request("no-such-kind", serde_json::json!({}));
    assert!(!app.has_pending_host_work(), "没人处理的请求种类不该让宿主续帧");
}
