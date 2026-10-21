//! swiper 状态机（自动播放、衔接、拖动、受控 current、事件）与三条绘制路径的分页。
//!
//! 依据官方 [component/swiper.html](https://developers.weixin.qq.com/miniprogram/dev/component/swiper.html)。
//! 端到端（`MiniEngine` 真指针拖动、事件到页面）在集成测试 `tests/swiper_e2e.rs`。
//!
//! 状态机用例直接建 `SwiperStateManager`、自己给时间，不碰全局表（全局表是进程级的，
//! 与别的用例并行时会互相影响）。

use crate::renderer::components::{SwiperConfig, SwiperEasing as Easing, SwiperStateManager};
use serde_json::{json, Value};

fn cfg(total: usize) -> SwiperConfig {
    SwiperConfig {
        total,
        per_view: 1,
        circular: false,
        vertical: false,
        autoplay: false,
        interval: 5000,
        duration: 500,
        easing: Easing::Default,
        current_attr: None,
        direction: 0,
        scroll_with_animation: true,
    }
}

fn names(ev: &[(String, &'static str, Value)]) -> Vec<String> {
    ev.iter().map(|(_, n, d)| format!("{n}:{}", d.get("current").map(|c| c.to_string()).unwrap_or_default())).collect()
}

#[test]
fn 自动播放_非衔接到末页后回第一页_事件带_autoplay() {
    let mut m = SwiperStateManager::new();
    let c = SwiperConfig { autoplay: true, interval: 1000, ..cfg(3) };
    m.sync("s", &c, 375.0, 0);
    assert!(!m.advance(999), "间隔没到不翻");
    assert!(m.advance(1000));
    let ev = m.take_events();
    assert_eq!(ev[0].1, "change");
    assert_eq!(ev[0].2, json!({ "current": 1, "source": "autoplay" }), "current 是数字");
    // duration 默认 500：动画中间位置在 0 与 1 之间，走完发 animationfinish
    m.advance(1250);
    let (pos, active, _) = m.sync("s", &c, 375.0, 1250);
    assert!(active && pos > 0.0 && pos < 1.0, "半路：{pos}");
    m.advance(1500);
    assert!(names(&m.take_events()).contains(&"animationfinish:1".to_string()));
    // 间隔从动画结束算起：1500 + 1000 翻到 2，再 1000 回到 0（往回滑过去）
    m.advance(2500);
    m.advance(3000);
    m.advance(4000);
    let ev = names(&m.take_events());
    assert!(ev.contains(&"change:2".to_string()) && ev.contains(&"change:0".to_string()), "{ev:?}");
    m.advance(4250);
    let (pos, _, cur) = m.sync("s", &c, 375.0, 4250);
    assert_eq!(cur, 0);
    assert!(pos > 0.0 && pos < 2.0, "非衔接：从 2 往回滑向 0：{pos}");
}

#[test]
fn 衔接模式自动播放一直往前_末项到首项是连续的() {
    let mut m = SwiperStateManager::new();
    let c = SwiperConfig { autoplay: true, interval: 1000, circular: true, ..cfg(3) };
    m.sync("s", &c, 300.0, 0);
    let mut t = 0;
    for _ in 0..2 {
        t += 1000;
        m.advance(t);
        t += 600;
        m.advance(t);
    }
    // 现在在第 2 页（末页）；下一次往前滑到「第 3 页」= 第 0 页
    t += 1000;
    m.advance(t);
    m.advance(t + 250);
    let (pos, _, cur) = m.sync("s", &c, 300.0, t + 250);
    assert_eq!(cur, 0);
    assert!(pos > 2.0 && pos < 3.0, "衔接：往前走过末页：{pos}");
    m.advance(t + 600);
    let (pos, active, _) = m.sync("s", &c, 300.0, t + 600);
    assert!(!active && pos.abs() < 1e-4, "落定后归一化到 0：{pos}");
}

#[test]
fn 拖动_快甩翻一页_慢拖不过半吸回_事件带_touch() {
    let mut m = SwiperStateManager::new();
    let c = cfg(3);
    m.sync("s", &c, 300.0, 0);
    // 慢慢往左拖 100px（不过半）再松手：吸回第 0 页，不发 change
    m.begin_drag("s", 200.0, 0);
    m.update_drag("s", 150.0, 200);
    m.update_drag("s", 100.0, 400);
    let (pos, active, _) = m.sync("s", &c, 300.0, 400);
    assert!(active && (pos - 1.0 / 3.0).abs() < 0.01, "跟手：{pos}");
    m.end_drag("s", 450);
    assert!(!names(&m.take_events()).iter().any(|e| e.starts_with("change")), "没换页不发 change");
    m.advance(1000);
    assert_eq!(m.sync("s", &c, 300.0, 1000).0, 0.0);
    // 快速往左甩 60px：翻到第 1 页，松手立刻发 change(touch)
    m.take_events();
    m.begin_drag("s", 200.0, 2000);
    m.update_drag("s", 170.0, 2030);
    m.update_drag("s", 140.0, 2060);
    m.end_drag("s", 2060);
    let ev = m.take_events();
    let ch = ev.iter().find(|e| e.1 == "change").expect("应发 change");
    assert_eq!(ch.2, json!({ "current": 1, "source": "touch" }));
    assert!(ev.iter().any(|e| e.1 == "transition"), "拖动中有 transition");
    m.advance(3000);
    assert!(names(&m.take_events()).contains(&"animationfinish:1".to_string()));
    // 现在在第 1 页；往右拖两屏：越过第一页的那一屏按阻尼走（非衔接）
    m.begin_drag("s", 100.0, 4000);
    m.update_drag("s", 700.0, 4300);
    let (pos, _, _) = m.sync("s", &c, 300.0, 4300);
    assert!((pos + 0.3).abs() < 0.01, "两端有阻尼（手指多走一屏，内容只多走 0.3 屏）：{pos}");
}

#[test]
fn 受控_current_值变了才动且发_change_空来源() {
    let mut m = SwiperStateManager::new();
    let mut c = SwiperConfig { current_attr: Some(1), ..cfg(4) };
    let (pos, _, cur) = m.sync("s", &c, 300.0, 0);
    assert_eq!((pos, cur), (1.0, 1), "首帧直接落位");
    assert!(m.take_events().is_empty(), "首帧不发事件");
    m.sync("s", &c, 300.0, 100);
    assert!(m.take_events().is_empty(), "值没变不动");
    c.current_attr = Some(3);
    m.sync("s", &c, 300.0, 200);
    assert_eq!(m.take_events()[0].2, json!({ "current": 3, "source": "" }));
    m.advance(800);
    assert_eq!(m.sync("s", &c, 300.0, 800).0, 3.0);
    // 拖动换页后页面把 current 设成同一个值（bindchange 里 setData 的常见写法）：不再动、不再发
    c.current_attr = Some(3);
    m.sync("s", &c, 300.0, 900);
    assert!(m.take_events().iter().all(|e| e.1 != "change"));
}

#[test]
fn 一屏多项的页数与_duration_easing() {
    let mut m = SwiperStateManager::new();
    let c = SwiperConfig { per_view: 2, autoplay: true, interval: 100, duration: 0, easing: Easing::Linear, ..cfg(4) };
    m.sync("s", &c, 150.0, 0);
    let mut seen = Vec::new();
    for i in 1..=4 {
        m.advance(i * 100);
        seen.push(m.sync("s", &c, 150.0, i * 100).2);
    }
    assert_eq!(seen, vec![1, 2, 0, 1], "4 项一屏 2 项：只有 3 页；duration 0 直接跳");
    assert_eq!(Easing::parse("easeInCubic").apply(0.5), 0.125);
    assert_eq!(Easing::parse("linear").apply(0.3), 0.3);
}

// ============ 绘制：三条路径同一套分页（G-13） ============

use crate::parser::wxml::WxmlParser;
use crate::parser::wxss::WxssParser;
use crate::renderer::wxml_renderer::WxmlRenderer;
use crate::Canvas;

fn px(c: &Canvas, x: u32, y: u32) -> (u8, u8, u8) {
    let p = c.to_rgba();
    let i = ((y * c.width() + x) * 4) as usize;
    (p[i], p[i + 1], p[i + 2])
}

const SW_CSS: &str = ".sw{width:200px;height:100px}.a{background-color:#ff0000;height:100px}.b{background-color:#00ff00;height:100px}.c{background-color:#0000ff;height:100px}";

fn swiper_page(attrs: &str) -> String {
    format!(r#"<swiper class="sw" {attrs}><swiper-item><view class="a"></view></swiper-item><swiper-item><view class="b"></view></swiper-item><swiper-item><view class="c"></view></swiper-item></swiper>"#)
}

#[test]
fn 无交互的_render_也分页_只画当前页_有指示点() {
    let ss = WxssParser::new(SW_CSS).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 200.0, 1.0);
    let mut c = Canvas::new(375, 200);
    c.clear(crate::Color::WHITE);
    let nodes = WxmlParser::new(&swiper_page(r#"id="g13" current="1" indicator-dots"#)).parse().unwrap();
    r.render(&mut c, &nodes, &json!({}));
    assert_eq!(px(&c, 100, 30), (0, 255, 0), "current=1：画第二项");
    assert_eq!(px(&c, 300, 30), (255, 255, 255), "盒子外不画（从前会把第三项排在右边画出来）");
    // 指示点：三个，当前那个是默认的黑色
    let dot_y = 100 - 8 - 4;
    assert_eq!(px(&c, 100, dot_y), (0, 0, 0), "当前指示点默认 #000000（官方默认值）");
}

#[test]
fn 前后边距露出相邻项_衔接时首项前面是末项() {
    let ss = WxssParser::new(SW_CSS).parse().unwrap_or_default();
    let mut r = WxmlRenderer::new_with_scale(ss, 375.0, 200.0, 1.0);
    let mut c = Canvas::new(375, 200);
    c.clear(crate::Color::WHITE);
    let nodes = WxmlParser::new(&swiper_page(r#"id="mg" previous-margin="30px" next-margin="30px" circular"#)).parse().unwrap();
    r.render(&mut c, &nodes, &json!({}));
    assert_eq!(px(&c, 10, 30), (0, 0, 255), "前边距里露出的是末项（衔接）");
    assert_eq!(px(&c, 100, 30), (255, 0, 0), "中间是当前项");
    assert_eq!(px(&c, 190, 30), (0, 255, 0), "后边距里露出下一项");
}

/// Skyline `direction`：positive 只许往前一页方向划（横向右滑），往左划拖不动、松手不翻页
#[test]
fn direction_positive_blocks_swiping_to_next_page() {
    let mut m = SwiperStateManager::new();
    let c = SwiperConfig { direction: 1, current_attr: Some(1), ..cfg(3) };
    m.sync("d", &c, 100.0, 0);
    m.take_events();
    // 往左划 80px（去下一页的方向）：位置不动
    m.begin_drag("d", 200.0, 0);
    m.update_drag("d", 120.0, 50);
    let (pos, ..) = m.sync("d", &c, 100.0, 50);
    assert_eq!(pos, 1.0, "positive：不许往下一页拖");
    m.end_drag("d", 60);
    assert!(names(&m.take_events()).iter().all(|n| !n.starts_with("change")), "也不翻页");
    // 往右划：照常去上一页
    m.begin_drag("d", 100.0, 1000);
    m.update_drag("d", 180.0, 1050);
    m.end_drag("d", 1060);
    assert!(names(&m.take_events()).contains(&"change:0".to_string()));
}

/// Skyline `scroll-with-animation="{{false}}"`：页面改 current 直接跳过去（change 与 animationfinish 同一帧）
#[test]
fn scroll_with_animation_false_jumps() {
    let mut m = SwiperStateManager::new();
    let mut c = SwiperConfig { scroll_with_animation: false, current_attr: Some(0), ..cfg(4) };
    m.sync("j", &c, 100.0, 0);
    m.take_events();
    c.current_attr = Some(2);
    let (pos, moving, cur) = m.sync("j", &c, 100.0, 10);
    assert_eq!((pos, moving, cur), (2.0, false, 2));
    assert_eq!(names(&m.take_events()), vec!["change:2", "animationfinish:2"]);
}
