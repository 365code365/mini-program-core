//! 移动端 SDK（[`MiniEngine`]）的**出帧**回归：页面画布的尺寸与滚动时的重绘策略。
//!
//! 与桌面窗体不同，这里没有窗口截图可看，问题只会在真机上表现为「滚到下面一片空白」
//! 或「滑动掉帧」，所以直接检查引擎内部的页面画布与合成结果。

use crate::host::MiniEngine;

fn engine_in(route: &str, slot: &str) -> super::LockedEngine {
    let g = super::LockedEngine::lock();
    let root = crate::app::dir::resolve("sample-app");
    let data = crate::sys::data_dir::subdir(&format!("engine-render-tests/{slot}"));
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
    let root = crate::app::dir::resolve("sample-app");
    let data = crate::sys::data_dir::subdir("engine-render-tests/sq-wake");
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

// ───────────── WP-E1：canvas（官方 component/canvas、api/canvas/*；2D 行为按 HTML Canvas）─────────────

/// 只有逻辑层的 MiniApp（不开页面）：2d 节点用 `__canvas2dNode` 直接拿，像素用 `getImageData` 同步读
fn js_app() -> crate::runtime::MiniApp {
    let mut a = crate::runtime::MiniApp::new(375, 667).expect("create MiniApp");
    a.init().expect("init");
    a
}

/// 跑 update 直到 `cond`（JS 表达式）为真（定时器 / 下载回调要几次 update）
fn until(a: &mut crate::runtime::MiniApp, cond: &str) -> bool {
    for _ in 0..100 {
        a.update().ok();
        if a.eval(cond).unwrap_or_default() == "true" {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    false
}

/// 临时 PNG：左半红、右半蓝（`w × h`）
fn red_blue_png(name: &str, w: u32, h: u32) -> String {
    let dir = std::env::temp_dir().join("mini-wpe-canvas");
    std::fs::create_dir_all(&dir).ok();
    let p = dir.join(format!("{name}.png"));
    image::RgbaImage::from_fn(w, h, |x, _| if x < w / 2 { image::Rgba([255, 0, 0, 255]) } else { image::Rgba([0, 0, 255, 255]) })
        .save(&p)
        .unwrap();
    p.to_string_lossy().into_owned()
}

const PX_JS: &str = "function __px(ctx, x, y) { return Array.prototype.slice.call(ctx.getImageData(x, y, 1, 1).data).join(','); }";

#[test]
fn e1_drawimage_三参数按原图尺寸_五参数缩放_九参数裁源() {
    let src = red_blue_png("e1-draw", 4, 2);
    let a = js_app();
    a.eval(PX_JS).unwrap();
    let out = a
        .eval(&format!(
            r#"(function() {{
  var n = __canvas2dNode({{ id: 'e1-draw', width: 100, height: 50 }}), ctx = n.getContext('2d'), s = {src:?};
  ctx.drawImage(s, 0, 0);
  ctx.drawImage(s, 10, 0, 8, 4);
  ctx.drawImage(s, 2, 0, 2, 2, 30, 0, 4, 4);
  return [__px(ctx, 0, 0), __px(ctx, 3, 1), __px(ctx, 4, 0), __px(ctx, 0, 2), __px(ctx, 17, 3), __px(ctx, 18, 0), __px(ctx, 30, 0), __px(ctx, 33, 3), __px(ctx, 34, 0)].join(' | ');
}})()"#
        ))
        .unwrap();
    let px: Vec<&str> = out.split(" | ").collect();
    assert_eq!(px[0], "255,0,0,255", "3 参数：左上是红");
    assert_eq!(px[1], "0,0,255,255", "3 参数：(3,1) 是右半的蓝");
    assert_eq!(px[2], "0,0,0,0", "3 参数按原图 4×2 画，不是从前的 100×100");
    assert_eq!(px[3], "0,0,0,0", "原图只有 2 高");
    assert_eq!(px[4], "0,0,255,255", "5 参数缩放到 8×4");
    assert_eq!(px[5], "0,0,0,0", "5 参数目标右边界 18");
    assert_eq!((px[6], px[7], px[8]), ("0,0,255,255", "0,0,255,255", "0,0,0,0"), "9 参数只取源右半（蓝），画成 4×4");
}

#[test]
fn e1_create_image_真加载_宽高_onerror_svg() {
    let src = red_blue_png("e1-img", 6, 3);
    let mut a = js_app();
    a.eval(&format!(
        r#"var __r = {{}};
var n = __canvas2dNode({{ id: 'e1-img', width: 50, height: 50 }});
var i1 = n.createImage(); i1.onload = function() {{ __r.ok = [i1.width, i1.height, i1.complete]; }}; i1.onerror = function() {{ __r.ok = 'err'; }};
i1.src = {src:?};
var i2 = n.createImage(); i2.onload = function() {{ __r.bad = 'load'; }}; i2.onerror = function(e) {{ __r.bad = 'error:' + typeof e.errMsg; }};
i2.src = '/no/such/file.png';
var i3 = n.createImage(); i3.onload = function() {{ __r.svg = [i3.width, i3.height]; }}; i3.onerror = function() {{ __r.svg = 'err'; }};
i3.src = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="12" height="7"><rect width="12" height="7" fill="red"/></svg>');
var i4 = n.createImage(); i4.onerror = function() {{ __r.cloud = 'error'; }}; i4.src = 'cloud://env/a.png';"#
    ))
    .unwrap();
    assert!(until(&mut a, "!!(__r.ok && __r.bad && __r.svg && __r.cloud)"), "四张图都应有结论：{}", a.eval("JSON.stringify(__r)").unwrap_or_default());
    assert_eq!(a.eval("JSON.stringify(__r.ok)").unwrap(), "[6,3,true]", "onload 时 width / height 是原图尺寸（number）");
    assert_eq!(a.eval("__r.bad").unwrap(), "error:string", "读不到发 onerror");
    let svg = a.eval("JSON.stringify(__r.svg)").unwrap();
    if cfg!(feature = "svg") {
        assert_eq!(svg, "[12,7]", "SVG 的固有尺寸");
    }
    assert_eq!(a.eval("__r.cloud").unwrap(), "error", "cloud:// 走 onerror");
}

#[test]
fn e1_roundrect_evenodd_ispointinpath_gettransform() {
    let a = js_app();
    a.eval(PX_JS).unwrap();
    let out = a
        .eval(
            r#"(function() {
  var n = __canvas2dNode({ id: 'e1-path', width: 100, height: 100 }), ctx = n.getContext('2d'), r = [];
  ctx.fillStyle = '#ff0000';
  ctx.beginPath(); ctx.roundRect(0, 0, 40, 40, 12); ctx.fill();
  r.push(__px(ctx, 1, 1), __px(ctx, 20, 1));
  // 同向的两个矩形：nonzero 填满，evenodd 中间是洞
  ctx.beginPath(); ctx.rect(50, 0, 40, 40); ctx.rect(60, 10, 20, 20); ctx.fill('evenodd');
  r.push(__px(ctx, 70, 20), __px(ctx, 52, 2));
  ctx.beginPath(); ctx.rect(50, 50, 40, 40); ctx.rect(60, 60, 20, 20);
  r.push(ctx.isPointInPath(70, 70), ctx.isPointInPath(70, 70, 'evenodd'), ctx.isPointInPath(5, 5));
  var p = n.createPath2D(); p.roundRect(0, 50, 30, 30, [5]);
  r.push(ctx.isPointInPath(p, 15, 65), ctx.isPointInStroke(p, 0, 65));
  ctx.save(); ctx.translate(10, 20); ctx.scale(2, 3); var m = ctx.getTransform(); ctx.restore();
  var m2 = ctx.getTransform();
  r.push([m.a, m.d, m.e, m.f].join('/'), m2.isIdentity);
  var bad = 'ok'; try { ctx.roundRect(0, 0, 1, 1, [1, 2, 3, 4, 5]); } catch (e) { bad = e.name; }
  r.push(bad);
  return r.join(' | ');
})()"#,
        )
        .unwrap();
    let r: Vec<&str> = out.split(" | ").collect();
    assert_eq!(r[0], "0,0,0,0", "圆角 12：左上角 (1,1) 在圆角外");
    assert_eq!(r[1], "255,0,0,255", "上边中间照填");
    assert_eq!(r[2], "0,0,0,0", "fill('evenodd')：环绕数 2 的中间是洞");
    assert_eq!(r[3], "255,0,0,255");
    assert_eq!((r[4], r[5], r[6]), ("true", "false", "false"), "isPointInPath：nonzero 在里、evenodd 在洞里、远处不在");
    assert_eq!((r[7], r[8]), ("true", "true"), "Path2D.roundRect 也能问；左边线上在描边里");
    assert_eq!(r[9], "2/3/10/20", "getTransform 跟着 translate + scale");
    assert_eq!(r[10], "true", "restore 之后回到单位矩阵");
    assert_eq!(r[11], "RangeError", "radii 超过 4 个按标准抛 RangeError");
}

#[test]
fn e1_filltext_maxwidth_压缩_stroketext_描轮廓() {
    let a = js_app();
    let out = a
        .eval(
            r#"(function() {
  var n = __canvas2dNode({ id: 'e1-text', width: 300, height: 250 }), ctx = n.getContext('2d');
  function ink(x0, y0, x1, y1) { var d = ctx.getImageData(x0, y0, x1 - x0, y1 - y0).data, k = 0, right = -1;
    for (var i = 0; i < d.length; i += 4) if (d[i + 3] > 40) { k++; right = Math.max(right, (i / 4) % (x1 - x0)); } return [k, right]; }
  ctx.font = '40px sans-serif'; ctx.textBaseline = 'top';
  ctx.fillText('WWWWWW', 0, 0);
  var free = ink(0, 0, 300, 50);
  ctx.fillText('WWWWWW', 0, 60, 60);
  var squeezed = ink(0, 60, 300, 110);
  ctx.font = '80px sans-serif'; ctx.lineWidth = 1; ctx.strokeText('HH', 0, 120);
  var stroked = ink(0, 120, 140, 220);
  ctx.fillText('HH', 150, 120);
  var filled = ink(150, 120, 290, 220);
  return JSON.stringify({ free: free, squeezed: squeezed, stroked: stroked, filled: filled });
})()"#,
        )
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let n = |k: &str, i: usize| v[k][i].as_i64().unwrap();
    assert!(n("free", 1) > 100, "不压缩时六个 W 远超 60px：{out}");
    assert!(n("squeezed", 1) <= 62 && n("squeezed", 0) > 50, "maxWidth 60：墨迹右端不超过 60（+抗锯齿 2px）：{out}");
    assert!(n("stroked", 0) > 0 && n("stroked", 0) < n("filled", 0) * 4 / 5, "strokeText 只描轮廓，墨比填充少：{out}");
}

#[test]
fn e1_旧接口_draw_reserve_语义_属性式写法_回调在画完之后() {
    let mut a = js_app();
    a.eval(
        r#"var __cb = null;
var c = wx.createCanvasContext('e1-old');
c.setFillStyle('red'); c.fillRect(0, 0, 10, 10); c.draw();
c.fillRect(20, 0, 10, 10); c.draw(true);
c.fillStyle = '#00ff00'; c.fillRect(40, 0, 10, 10);
c.draw(true, function() { __cb = Array.prototype.slice.call(new Uint8Array(__b64decode(__native_canvas_read('e1-old', '45', '5', '1', '1', '[1,1]')))).join(','); });"#,
    )
    .unwrap();
    assert!(until(&mut a, "__cb !== null"), "draw 的回调要来");
    assert_eq!(a.eval("__cb").unwrap(), "0,255,0,255", "回调时这一批已经画完（属性式 fillStyle 生效）");
    let rgba = |x: u32| {
        let m = crate::renderer::components::CANVAS_MANAGER.lock().unwrap();
        let c = m.get_existing_context("e1-old").unwrap().get_canvas();
        let p = c.lock().unwrap().get_pixel(x, 5);
        (p.r, p.g, p.b, p.a)
    };
    assert_eq!(rgba(5), (255, 0, 0, 255), "draw(true) 保留上一次的内容");
    assert_eq!(rgba(25), (0, 0, 0, 255), "官方示例：第二次 draw 之前状态回到默认，fillStyle 变回黑");
    a.eval("c.fillRect(60, 0, 10, 10); c.draw();").unwrap();
    a.update().ok();
    assert_eq!((rgba(5).3, rgba(45).3), (0, 0), "draw() / draw(false) 先清空画布");
    assert_eq!(rgba(65), (0, 0, 0, 255));
    assert_eq!(a.eval("c.fillStyle").unwrap(), "#000000", "draw 之后属性也回到默认");
}

#[test]
fn e1_canvas_默认_300x150_disable_scroll_锁滚动() {
    use crate::parser::wxml::WxmlParser;
    let mut r = crate::renderer::WxmlRenderer::new_with_scale(Default::default(), 375.0, 667.0, 2.0);
    let wxml = r#"<view><canvas class="d" type="2d" id="e1-def"></canvas><canvas class="w" canvas-id="e1-w" style="width:100px"></canvas>
<canvas class="ds" canvas-id="e1-ds" style="width:100px;height:100px" disable-scroll="true" bindtouchmove="mv"></canvas></view>"#;
    let nodes = WxmlParser::new(wxml).parse().unwrap_or_default();
    let mut c = crate::Canvas::new(750, 1334);
    let mut im = crate::ui::interaction::InteractionManager::new();
    r.render_with_interaction(&mut c, &nodes, &serde_json::json!({}), &mut im);
    assert_eq!(r.node_size_by_class("d"), Some((600.0, 300.0)), "没写尺寸：300×150（物理像素 ×2）");
    assert_eq!(r.node_size_by_class("w"), Some((200.0, 300.0)), "只写宽：高仍是 150");
    // 第三个 canvas 在 y = 150 + 150 = 300 逻辑 px 处
    assert!(r.has_catch_for(50.0, 350.0, "touchmove"), "disable-scroll + 绑了 touchmove：锁住页面滚动");
    assert!(!r.has_catch_for(50.0, 100.0, "touchmove"), "别的 canvas 不锁");
}

#[test]
fn e1_canvas_id_重复_第二个起派_binderror_一次() {
    let root = std::env::temp_dir().join("mini-e1-canvas-dup");
    std::fs::remove_dir_all(&root).ok();
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", r#"<canvas canvas-id="c1" binderror="onErr" data-n="a"></canvas><canvas canvas-id="c1" binderror="onErr" data-n="b"></canvas><canvas canvas-id="c2" binderror="onErr" data-n="c"></canvas>"#),
        ("pages/p/p.wxss", ""),
        ("pages/p/p.js", "Page({ data: { errs: [] }, onErr(e) { this.data.errs.push(e.currentTarget.dataset.n + ':' + (typeof e.detail.errMsg)); } });"),
        ("pages/p/p.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let _g = crate::sys::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&root.join("_data").to_string_lossy()), 375, 667, 1.0).expect("创建引擎");
    e.launch(None).expect("打开首页");
    for t in 1..=4 {
        e.pump(t * 16);
    }
    assert_eq!(e.eval("JSON.stringify(getCurrentPages()[0].data.errs)").unwrap(), r#"["b:string"]"#, "只有重复的第二个报，报一次");
}
