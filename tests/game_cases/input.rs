//! GT-T 触摸与键鼠（game-doc/05 §3）。挂在 game_e2e.rs 下（同一个测试二进制，steering 30）。
use super::*;
use mini_render::game::touch::{PHASE_CANCEL, PHASE_DOWN, PHASE_MOVE, PHASE_UP};

/// 每个触摸事件记一行：类型、touches / changedTouches 的 identifier、第一个变化点的坐标、timeStamp；
/// rAF 里记「此刻游戏看到的最后坐标」（T7）
const TOUCH_JS: &str = r#"
var T = require('_lib/t.js');
var lastX = -1;
function log(kind) {
  return function (e) {
    var c = e.changedTouches[0] || {};
    lastX = c.clientX;
    T.ev(kind, { t: e.touches.map(function (t) { return t.identifier; }), c: e.changedTouches.map(function (t) { return t.identifier; }),
      x: c.clientX, y: c.clientY, px: c.pageX, sx: c.screenX, f: c.force, ts: e.timeStamp });
  };
}
wx.onTouchStart(log('start')); wx.onTouchMove(log('move')); wx.onTouchEnd(log('end')); wx.onTouchCancel(log('cancel'));
var n = 0;
(function tick() { n++; T.ev('raf', { n: n, x: lastX }); requestAnimationFrame(tick); })();
T.ready('touch');
"#;

fn open() -> GameHarness {
    let mut h = GameHarness::open_inline("touch", TOUCH_JS, Opts::default());
    h.until("T:READY touch", 10);
    h.vsyncs(1);
    h
}

fn ids(v: &serde_json::Value, k: &str) -> Vec<i64> {
    v[k].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect()
}

/// T1 / T2 / T5（默认字段规则）：单指与双指交错，identifier 稳定，touches 长度对，三组坐标相等，timeStamp 在游戏时间轴上
#[test]
fn t1_t2_单指与双指交错() {
    let _g = serial();
    let mut h = open();
    let t = h.now_us();
    h.engine.touch(PHASE_DOWN, 5, 10.0, 20.0, 0.5, t);
    h.engine.touch(PHASE_MOVE, 5, 12.0, 21.0, 0.5, t + 1000);
    h.engine.touch(PHASE_DOWN, 8, 100.0, 200.0, 0.0, t + 2000);
    h.engine.touch(PHASE_UP, 5, 13.0, 22.0, 0.0, t + 3000);
    h.engine.touch(PHASE_UP, 8, 101.0, 201.0, 0.0, t + 4000);
    h.vsyncs(1);
    h.assert_no_fail();
    let kinds: Vec<_> = ["start", "move", "end"].iter().map(|k| h.events(k).len()).collect();
    assert_eq!(kinds, [2, 1, 2]);
    let s = h.events("start");
    assert_eq!((ids(&s[0], "t"), ids(&s[0], "c")), (vec![0], vec![0]));
    assert_eq!((ids(&s[1], "t"), ids(&s[1], "c")), (vec![0, 1], vec![1]));
    let e = h.events("end");
    assert_eq!((ids(&e[0], "t"), ids(&e[0], "c")), (vec![1], vec![0]), "先抬的是 identifier 0");
    assert_eq!((ids(&e[1], "t"), ids(&e[1], "c")), (vec![], vec![1]));
    assert_eq!(e[0]["x"], 13.0, "抬起点坐标");
    assert_eq!((s[0]["px"].clone(), s[0]["sx"].clone()), (s[0]["x"].clone(), s[0]["x"].clone()), "page / screen / client 相等");
    assert_eq!(s[0]["f"], 0.5);
    assert_eq!(s[0]["ts"].as_f64().unwrap(), t as f64 / 1000.0, "timeStamp = 宿主时刻（固定时钟下就是游戏时刻）");
}

/// T7：这一帧到的触摸，这一帧的 rAF 就看得到（输入在 rAF 之前派发）
#[test]
fn t7_输入在同帧_raf_可见() {
    let _g = serial();
    let mut h = open();
    h.engine.touch(PHASE_DOWN, 0, 1.0, 1.0, 0.0, 0);
    for k in 0..10 {
        h.engine.touch(PHASE_MOVE, 0, 100.0 + k as f32, 1.0, 0.0, 0);
        h.vsyncs(1);
        let raf = h.events("raf");
        assert_eq!(raf.last().unwrap()["x"], 100.0 + k as f64, "第 {k} 帧的 rAF 没看到这一帧的坐标");
    }
}

/// T4：进后台时按着的手指收到 touchcancel（changedTouches = 全部，touches = 空）；T6：单帧单指 move ≤ 8
#[test]
fn t4_t6_后台取消与_move_上限() {
    let _g = serial();
    let mut h = open();
    h.engine.touch(PHASE_DOWN, 0, 0.0, 0.0, 0.0, 0);
    for i in 1..=200 {
        h.engine.touch(PHASE_MOVE, 0, i as f32, 0.0, 0.0, 0);
    }
    h.engine.touch(PHASE_DOWN, 1, 50.0, 50.0, 0.0, 0);
    h.vsyncs(1);
    let m = h.events("move");
    assert_eq!(m.len(), 8, "单帧单指最多 8 次 move");
    assert_eq!(m[7]["x"], 200.0, "最后坐标保留");
    h.engine.set_background(true);
    h.vsyncs(1);
    let c = h.events("cancel");
    assert_eq!(c.len(), 1);
    assert_eq!((ids(&c[0], "t"), ids(&c[0], "c")), (vec![], vec![0, 1]));
    // 系统取消单根（phase 3）也走 touchcancel
    h.engine.set_background(false);
    h.engine.touch(PHASE_DOWN, 3, 0.0, 0.0, 0.0, 0);
    h.engine.touch(PHASE_CANCEL, 3, 0.0, 0.0, 0.0, 0);
    h.vsyncs(1);
    assert_eq!(h.events("cancel").len(), 2);
}

/// A3.3 / T7（g-touch-draw）：第 n 帧到的点在第 n 帧画出 —— 日志里 T:FRAME 的帧号就是喂进去的那一帧，像素也在
#[test]
fn a3_3_g_touch_draw_同帧画出() {
    let _g = serial();
    let mut h = GameHarness::open("g-touch-draw", Opts::default());
    h.until("T:READY g-touch-draw", 10);
    h.vsyncs(2);
    let before = h.frames().len();
    // 两根手指：0 号（第一种颜色 红）一路右移，1 号（绿）中途按下
    h.engine.touch(PHASE_DOWN, 4, 50.0, 100.0, 0.0, 0);
    for k in 1..=5u32 {
        h.engine.touch(PHASE_MOVE, 4, 50.0 + 20.0 * k as f32, 100.0, 0.0, 0);
        if k == 3 {
            h.engine.touch(PHASE_DOWN, 6, 200.0, 300.0, 0.0, 0);
        }
        h.vsyncs(1);
        let fr = h.frames();
        let (_, v) = fr.last().unwrap();
        assert_eq!(fr.len(), before + k as usize, "每帧一行 T:FRAME");
        let expect = if k == 1 { 2 } else if k == 3 { 2 } else { 1 };
        assert_eq!(v["drawn"], expect, "第 {k} 次：这一帧到的点都在这一帧画（down+move / move+另一根 down）");
        if k == 1 {
            assert_eq!(fr[fr.len() - 2].1["drawn"], 0, "喂之前那一帧没有点");
        }
    }
    h.assert_no_fail();
    // 主画布 = 逻辑尺寸（375×667），点是 6×6 方块
    assert_eq!(&h.pixel(150, 100)[..3], &[255, 0, 0], "0 号手指最后的位置是红点");
    assert_eq!(&h.pixel(200, 300)[..3], &[0, 255, 0], "1 号手指是绿点");
    assert_eq!(&h.pixel(10, 100)[..3], &[0, 0, 0], "别处是黑底");
}

/// off、回调抛错不打断其余回调、PC 键鼠字段
#[test]
fn t8_t10_off_异常隔离与键鼠() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
var a = function () { T.ev('a', {}); };
wx.onTouchStart(function () { throw new Error('boom-touch'); });
wx.onTouchStart(a);
wx.onTouchStart(a); // 同一个函数不重复登记
wx.onTouchEnd(a); wx.offTouchEnd(a);
wx.onMouseDown(function (e) { T.ev('md', e); });
wx.onMouseMove(function (e) { T.ev('mm', e); });
wx.onWheel(function (e) { T.ev('wh', e); });
wx.onKeyDown(function (e) { T.ev('kd', e); });
T.ready('pc');
"#;
    let mut h = GameHarness::open_inline("pc", js, Opts::default());
    h.until("T:READY pc", 10);
    h.engine.touch(PHASE_DOWN, 0, 0.0, 0.0, 0.0, 0);
    h.engine.touch(PHASE_UP, 0, 0.0, 0.0, 0.0, 0);
    h.engine.mouse(0, 2, 5.0, 6.0, 0);
    h.engine.mouse(1, 0, 8.0, 6.0, 0);
    h.engine.mouse(1, 0, 9.0, 10.0, 0);
    h.engine.wheel(0.0, -120.0, 0.0, 9.0, 10.0, 0);
    h.engine.key(true, "a", "KeyA", 0);
    h.vsyncs(1);
    assert_eq!(h.events("a").len(), 1, "抛错的回调之后的回调照常执行、同一函数只登记一次、off 生效");
    assert!(h.errors.iter().any(|e| e["message"].as_str().unwrap_or("").contains("boom-touch")) || h.has("boom-touch"));
    let md = h.events("md");
    assert_eq!((md[0]["button"].as_i64(), md[0]["x"].as_f64()), (Some(2), Some(5.0)));
    let mm = h.events("mm");
    assert_eq!(mm.len(), 1, "同帧 move 并成一次");
    assert_eq!((mm[0]["movementX"].as_f64(), mm[0]["movementY"].as_f64()), (Some(4.0), Some(4.0)));
    assert_eq!(h.events("wh")[0]["deltaY"], -120.0);
    let kd = h.events("kd");
    assert_eq!((kd[0]["key"].as_str(), kd[0]["code"].as_str()), (Some("a"), Some("KeyA")));
    assert!(kd[0]["timeStamp"].is_number());
}
