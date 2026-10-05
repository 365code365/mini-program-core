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
    let canvas = &e.canvas;
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
