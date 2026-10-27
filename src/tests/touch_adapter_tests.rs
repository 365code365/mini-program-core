//! GT-T9：小程序实例走「每根指针一次」的 `mr_app_touch`（host/touch_adapter.rs），结果与老接口逐字节相同。
//!
//! 两个引擎跑同一个手势：A 调老接口（pointer_* / pointer2，与 Android MiniTouch.kt 的调用序列相同），
//! B 只调适配层（宿主指针 id 故意不是 0/1）。帧时刻、事件时间两边一模一样（pump 给显式的毫秒），
//! 比路由、面板状态与整帧像素。
//!
//! 比像素的用例只用**没有墙上时钟驱动内容**的页面（cart / category）：首页有 200ms 后弹出的优惠券浮层与每秒走的倒计时，
//! showcase 有 GIF 与定时器 —— 全量并行跑时机器一慢，两个引擎落在不同的秒上，整帧像素自然不同
//! （实测首页两条路差 94% 像素，单独跑却全绿）。picker 只在 showcase 上有，那一例只比状态。
use super::engine_input_tests::engine_in;
use crate::host::touch_adapter::{TouchAdapter, PHASE_CANCEL, PHASE_DOWN, PHASE_MOVE, PHASE_UP};
use crate::host::MiniEngine;

#[derive(Clone, Copy)]
enum Op {
    /// 第一根 / 第二根手指（适配层那边的宿主 id 是 11 / 23）
    Down(u8, f32, f32),
    Move(u8, f32, f32, f64),
    Up(u8, f32, f32),
    Cancel,
    Pump(u64),
}

/// 老接口：按 MiniTouch.kt 的规则手写调用序列（第一根先抬时报第二根抬起、不报第一根，第二根接班）
fn legacy(e: &mut MiniEngine, ops: &[Op]) -> Vec<u32> {
    let mut trace = Vec::new();
    // 当前谁是「第一根」（0 / 1）、第二根按着没有与它最后的位置
    let (mut primary, mut second, mut pos2) = (0u8, false, (0.0f32, 0.0f32));
    for op in ops {
        match *op {
            Op::Down(f, x, y) if f == primary && !second => e.pointer_down(x, y),
            Op::Down(_, x, y) => {
                second = true;
                pos2 = (x, y);
                e.pointer2(0, x, y)
            }
            Op::Move(f, x, y, t) if f == primary => e.pointer_move_at(x, y, t),
            Op::Move(_, x, y, _) => {
                pos2 = (x, y);
                e.pointer2(1, x, y)
            }
            Op::Up(f, x, y) => {
                if f != primary {
                    second = false;
                    e.pointer2(2, x, y);
                } else if second {
                    e.pointer2(2, pos2.0, pos2.1);
                    second = false;
                    primary = 1 - primary;
                } else {
                    e.pointer_up(x, y);
                    primary = 0;
                }
            }
            Op::Cancel => {
                if second {
                    e.pointer2(3, 0.0, 0.0);
                }
                second = false;
                primary = 0;
                e.pointer_cancel();
            }
            Op::Pump(t) => {
                e.pump(t);
                trace.push(e.scroll_position().to_bits());
            }
        }
    }
    trace
}

fn adapted(e: &mut MiniEngine, ops: &[Op]) -> Vec<u32> {
    let mut trace = Vec::new();
    let mut a = TouchAdapter::default();
    let id = |f: u8| if f == 0 { 11 } else { 23 };
    for op in ops {
        match *op {
            Op::Down(f, x, y) => a.apply(e, PHASE_DOWN, id(f), x, y, 0.0),
            Op::Move(f, x, y, t) => a.apply(e, PHASE_MOVE, id(f), x, y, t),
            Op::Up(f, x, y) => a.apply(e, PHASE_UP, id(f), x, y, 0.0),
            Op::Cancel => a.apply(e, PHASE_CANCEL, 0, 0.0, 0.0, 0.0),
            Op::Pump(t) => {
                e.pump(t);
                trace.push(e.scroll_position().to_bits());
            }
        }
    }
    trace
}

fn same(route: &str, slot: &str, prep: impl Fn(&mut MiniEngine), ops: &[Op], pixels: bool) -> super::LockedEngine {
    let mut a = engine_in(route, &format!("{slot}-a"));
    let mut b = engine_in(route, &format!("{slot}-b"));
    prep(&mut a);
    prep(&mut b);
    let (ta, tb) = (legacy(&mut a, ops), adapted(&mut b, ops));
    // 每次 pump 之后的滚动位置都要相同（拖动中按事件时间重采样，只比终点的话 move_at 退化成 move 也看不出来）
    assert_eq!(ta, tb, "{slot}：逐帧滚动位置不同");
    assert_eq!(a.current_route(), b.current_route(), "{slot}：路由不同");
    assert_eq!(a.picker_sheet_open(), b.picker_sheet_open(), "{slot}：面板状态不同");
    assert_eq!(a.scroll_position().to_bits(), b.scroll_position().to_bits(), "{slot}：滚动位置不同");
    if pixels {
        let (pa, pb) = (a.rgba().to_vec(), b.rgba().to_vec());
        let diff = pa.chunks(4).zip(pb.chunks(4)).filter(|(x, y)| x != y).count();
        assert_eq!(diff, 0, "{slot}：两条路的整帧像素差 {diff} 个");
    }
    b
}

/// 帧时刻从 1000ms 起每帧 16ms（两边相同）
fn frames(from: u64, n: u64) -> Vec<Op> {
    (0..n).map(|i| Op::Pump(from + i * 16)).collect()
}

#[test]
fn mr_app_touch_点_tabbar_切页() {
    let h = 667.0 - crate::host::tabbar_height() as f32 + 25.0;
    let mut ops = vec![Op::Pump(1000), Op::Down(0, 141.0, h), Op::Pump(1016), Op::Up(0, 141.0, h)];
    ops.extend(frames(1032, 4));
    // 从购物车点第二格（分类）：两个页面都没有定时器
    let e = same("pages/cart/cart", "touch-tab", |_| {}, &ops, true);
    assert_eq!(e.current_route(), "pages/category/category", "测试本身要真的点到 tab（防空转）");
}

#[test]
fn mr_app_touch_带时间戳的拖动() {
    let mut ops = vec![Op::Pump(1000), Op::Down(0, 180.0, 500.0)];
    for i in 1..=12 {
        let t = 1000.0 + i as f64 * 8.0;
        ops.push(Op::Move(0, 180.0, 500.0 - i as f32 * 25.0, t));
        if i % 2 == 0 {
            ops.push(Op::Pump(1000 + i as u64 * 8));
        }
    }
    // 停住 120ms 再抬手：速度归零，不起惯性。抬手（pointer_up 不带时间）按墙上时钟取时刻，
    // 两条路先后跑、各自隔了多久不一样，带惯性的话终点差几个 ULP（全量并行时撞上过）
    for k in 1..=8u64 {
        ops.push(Op::Move(0, 180.0, 200.0, 1096.0 + k as f64 * 16.0));
        ops.push(Op::Pump(1096 + k * 16));
    }
    ops.push(Op::Up(0, 180.0, 200.0));
    ops.extend(frames(1240, 20));
    let e = same("pages/category/category", "touch-drag", |_| {}, &ops, true);
    assert!(e.scroll_position() > 20.0, "测试本身要真的滚起来（防空转）：{}", e.scroll_position());
}

#[test]
fn mr_app_touch_双指_第一根先抬_与取消() {
    let mut ops = vec![Op::Pump(1000), Op::Down(0, 100.0, 300.0), Op::Down(1, 250.0, 300.0)];
    ops.push(Op::Move(1, 260.0, 320.0, 1010.0));
    ops.push(Op::Up(0, 100.0, 300.0));
    ops.push(Op::Move(1, 260.0, 280.0, 1020.0));
    ops.push(Op::Up(1, 260.0, 280.0));
    ops.extend(frames(1032, 3));
    ops.extend([Op::Down(0, 100.0, 300.0), Op::Down(1, 200.0, 300.0), Op::Cancel]);
    ops.extend(frames(1100, 3));
    same("pages/category/category", "touch-two", |_| {}, &ops, true);
}

#[test]
fn mr_app_touch_点_picker_弹面板() {
    let ops = [Op::Pump(1000), Op::Down(0, 180.0, 540.0), Op::Up(0, 180.0, 540.0), Op::Pump(1016)];
    let e = same("pages/showcase/showcase", "touch-picker", |e| {
        e.scroll_to(300.0);
        e.pump(984);
    }, &ops, false);
    assert!(e.picker_sheet_open(), "测试本身要真的弹出面板（防空转）");
}
