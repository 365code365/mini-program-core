//! 2048（sample/game-2048）：每页能打开；固定种子下真手势滑动后棋盘逐格确定；合并规则、撤销、
//! 胜利 / 失败弹窗、排行榜清空、设置页 picker / switch / radio 回到游戏生效、最高分持久化。
//!
//! 种子：`?seed=7` 的 LCG（utils/rng.js）开局是
//! `[[0,0,0,4],[0,0,0,0],[0,0,4,0],[0,0,0,0]]`，左滑 → `[[4,2,0,0],[0,0,0,0],[4,0,0,0],[0,0,0,0]]`，
//! 再下滑 → 两个 4 合成 8 得 8 分：`[[0,0,0,0],[0,2,0,0],[0,0,0,0],[8,2,0,0]]`（node 按同一份 utils 推演）。
#[path = "sample_harness/mod.rs"]
mod harness;
use harness::*;
use serde_json::{json, Value};

const APP: &str = "game-2048";

/// 在棋盘中心往 (dx, dy) 滑一下（真指针：touchstart → touchmove → touchend）
fn swipe(a: &mut App, dx: f32, dy: f32) {
    let (x, y, w, h) = a.rect(".board");
    a.drag(x + w / 2.0, y + h / 2.0, dx, dy, 8);
    a.frames(4);
}

fn left(a: &mut App) {
    swipe(a, -120.0, 0.0)
}
fn right(a: &mut App) {
    swipe(a, 120.0, 0.0)
}
fn down(a: &mut App) {
    swipe(a, 0.0, 120.0)
}

/// 直接摆一个棋盘（测试布局用；之后的移动照常走手势与种子）
fn set_board(a: &mut App, board: Value, score: u32) {
    a.js(&format!(
        "var p = getCurrentPages().slice(-1)[0]; p.setData({{ score: {score} }}); p.render({board}, -1, -1);"
    ));
    a.frames(3);
}

fn nonzero(b: &Value) -> usize {
    b.as_array().unwrap().iter().flat_map(|r| r.as_array().unwrap().iter()).filter(|v| v.as_u64().unwrap_or(0) > 0).count()
}

#[test]
fn g2048_每页都能打开() {
    let _g = serial();
    smoke_all_pages(APP, &[("pages/game/game", "seed=7")]);
}

#[test]
fn g2048_固定种子_左滑再下滑_棋盘逐格确定() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/game/game?seed=7"), "seed");
    assert_eq!(a.data("board"), json!([[0, 0, 0, 4], [0, 0, 0, 0], [0, 0, 4, 0], [0, 0, 0, 0]]), "seed=7 开局");
    assert_eq!(a.data("size"), 4);
    left(&mut a);
    assert_eq!(a.data("board"), json!([[4, 2, 0, 0], [0, 0, 0, 0], [4, 0, 0, 0], [0, 0, 0, 0]]), "左滑后");
    assert_eq!(a.data("score"), 0, "左滑没有合并");
    assert_eq!(a.data("moves"), 1);
    down(&mut a);
    assert_eq!(a.data("board"), json!([[0, 0, 0, 0], [0, 2, 0, 0], [0, 0, 0, 0], [8, 2, 0, 0]]), "下滑后两个 4 合成 8");
    assert_eq!(a.data("score"), 8);
    assert!(a.data("score").is_u64(), "分数是 number");
    assert_eq!(a.data("gained"), 8);
    // 渲染层跟着变：8 那一格是 t8 色块
    assert_eq!(a.data("rows[3][0].cls"), "tile t8");
    // 点一下（位移 < 24px）不算滑动
    let (x, y, w, h) = a.rect(".board");
    a.tap_xy(x + w / 2.0, y + h / 2.0);
    assert_eq!(a.data("moves"), 2, "点击棋盘不移动");
    a.assert_no_errors();
}

#[test]
fn g2048_合并规则_每格一次_得分累加() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/game/game?seed=11"), "merge");
    set_board(&mut a, json!([[2, 2, 2, 2], [4, 4, 8, 0], [2, 0, 0, 2], [16, 8, 8, 16]]), 0);
    right(&mut a);
    let b = a.data("board");
    // 不计新出的方块：逐行比
    assert_eq!(b[0].as_array().unwrap()[2..], [json!(4), json!(4)], "2 2 2 2 → _ _ 4 4，不是 8");
    assert_eq!(b[1].as_array().unwrap()[2..], [json!(8), json!(8)], "4 4 8 _ → _ _ 8 8");
    assert_eq!(b[2][3], 4, "2 _ _ 2 → 4");
    assert_eq!(b[3].as_array().unwrap()[1..], [json!(16), json!(16), json!(16)], "16 8 8 16 → _ 16 16 16");
    assert_eq!(a.data("score"), 4 + 4 + 8 + 4 + 16, "得分 = 合成的新数值之和");
    // 合并后 8 个方块 + 新出 1 个
    assert_eq!(nonzero(&b), 9);
    a.assert_no_errors();
}

#[test]
fn g2048_撤销一步_只能撤一次() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/game/game?seed=7"), "undo");
    left(&mut a);
    let after_left = a.data("board");
    down(&mut a);
    assert_eq!(a.data("score"), 8);
    assert_eq!(a.data("canUndo"), true);
    a.tap(".btn-undo");
    assert_eq!(a.data("board"), after_left, "撤销回到上一步棋盘");
    assert_eq!(a.data("score"), 0, "分数也回去");
    assert_eq!(a.data("moves"), 1);
    assert_eq!(a.data("canUndo"), false);
    a.tap(".btn-undo");
    a.frames(3);
    assert_eq!(a.data("board"), after_left, "第二次撤销不生效");
    a.wait_ms(1600); // 等「只能撤销一步」的 toast 收起
    a.assert_no_errors();
}

#[test]
fn g2048_合成2048_胜利弹窗_再来一局() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/game/game?seed=3"), "win");
    set_board(&mut a, json!([[1024, 1024, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 2]]), 1200);
    left(&mut a);
    assert_eq!(a.data("board[0][0]"), 2048);
    assert_eq!(a.data("won"), true);
    assert_eq!(a.data("score"), 3248);
    assert_eq!(a.storage("g2048_best_4"), 3248, "最高分落盘");
    assert!(a.modal_open(), "胜利弹窗");
    a.modal_tap(true);
    assert_eq!(a.data("score"), 0, "再来一局");
    assert_eq!(a.data("won"), false);
    assert_eq!(nonzero(&a.data("board")), 2, "新开局两块");
    let hist = a.storage("g2048_history");
    assert_eq!(hist[0]["score"], 3248, "上一局记进历史");
    assert_eq!(hist[0]["maxTile"], 2048);
    a.assert_no_errors();
}

#[test]
fn g2048_无路可走_失败弹窗_排行榜显示并清空() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/game/game?seed=5"), "lose");
    let stuck = json!([[2, 4, 2, 4], [4, 2, 4, 2], [2, 4, 2, 4], [4, 2, 4, 2]]);
    set_board(&mut a, stuck.clone(), 520);
    left(&mut a);
    assert_eq!(a.data("over"), true, "无法移动 → 结束");
    assert_eq!(a.data("board"), stuck, "死局滑动不改棋盘");
    assert!(a.modal_open(), "失败弹窗");
    a.modal_tap(true); // 只有「再来一局」
    assert_eq!(a.data("over"), false);
    assert_eq!(a.data("score"), 0);
    // 再打一局 0 分以外的：新游戏按钮要确认
    set_board(&mut a, json!([[8, 8, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]), 900);
    left(&mut a);
    assert_eq!(a.data("score"), 916);
    a.tap(".btn-new");
    assert!(a.modal_open(), "有分数时新游戏要确认");
    a.modal_tap(false);
    assert_eq!(a.data("score"), 916, "取消 → 不重开");
    a.tap(".btn-new");
    a.modal_tap(true);
    assert_eq!(a.data("score"), 0);
    // 排行榜：两局按分数从高到低
    a.tap_settle(".btn-rank");
    assert_eq!(a.route(), "pages/rank/rank");
    assert_eq!(a.depth(), 2);
    assert_eq!(a.data("list.length"), 2);
    assert_eq!(a.data("list[0].score"), 916);
    assert_eq!(a.data("list[1].score"), 520);
    assert_eq!(a.data("list[0].rank"), 1);
    assert_eq!(a.count(".item"), 2);
    // 统计页同一份数据
    a.back();
    a.tap_settle(".btn-stats");
    assert_eq!(a.data("games"), 2);
    assert_eq!(a.data("top"), 916);
    assert_eq!(a.data("avg"), 718);
    a.back();
    a.tap_settle(".btn-rank");
    // 清空：先取消、再确认
    a.tap(".clear");
    a.modal_tap(false);
    assert_eq!(a.data("list.length"), 2, "取消不清");
    a.tap(".clear");
    a.modal_tap(true);
    assert_eq!(a.data("list.length"), 0);
    assert!(a.storage("g2048_history").is_null() || a.storage("g2048_history") == json!(""), "存储也清掉");
    assert!(a.exists(".empty"));
    a.wait_ms(1600);
    a.assert_no_errors();
}

#[test]
fn g2048_设置_尺寸picker_动画switch_主题radio_回游戏生效() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/game/game?seed=9"), "settings");
    a.tap_settle(".btn-settings");
    assert_eq!(a.route(), "pages/settings/settings");
    assert_eq!(a.data("sizeIndex"), 1);
    a.picker(".size-picker");
    a.picker_row(0, 1, 1); // 4×4 往下一行 → 5×5
    a.picker_confirm();
    assert_eq!(a.data("sizeIndex"), 2, "picker 下标是 number");
    assert!(a.data("sizeIndex").is_u64());
    assert_eq!(a.storage("g2048_settings")["size"], 5);
    a.tap(".anim-switch");
    assert_eq!(a.data("anim"), false, "switch 关掉动画");
    a.tap_nth(".theme-radio", 1);
    assert_eq!(a.data("theme"), "ocean");
    let s = a.storage("g2048_settings");
    assert_eq!(s, json!({ "size": 5, "anim": false, "theme": "ocean" }));
    a.wait_ms(1600);
    a.back();
    assert_eq!(a.route(), "pages/game/game");
    assert_eq!(a.data("size"), 5, "回到游戏按 5×5 重开");
    assert_eq!(a.data("board.length"), 5);
    assert_eq!(a.data("board[0].length"), 5);
    assert_eq!(a.data("theme.key"), "ocean");
    assert_eq!(a.data("anim"), false);
    assert_eq!(a.count(".cell"), 25);
    // 关了动画：没有 pop
    let rows = a.data("rows").to_string();
    assert!(!rows.contains("pop"), "关动画后新方块不带 pop");
    // 5×5 上照样能滑
    let before = a.data("board");
    left(&mut a);
    right(&mut a);
    assert_ne!(a.data("board"), before);
    a.assert_no_errors();
}

#[test]
fn g2048_最高分持久化_重启后还在_规则页() {
    let _g = serial();
    let mut a = App::open(APP, Some("pages/game/game?seed=7"), "best");
    left(&mut a);
    down(&mut a);
    assert_eq!(a.data("best"), 8);
    assert_eq!(a.storage("g2048_best_4"), 8);
    a.js("wx.reLaunch({ url: '/pages/game/game' })");
    a.settle();
    assert_eq!(a.depth(), 1);
    assert_eq!(a.data("score"), 0);
    assert_eq!(a.data("best"), 8, "重启后最高分还在");
    a.tap_settle(".btn-rules");
    assert_eq!(a.route(), "pages/rules/rules");
    assert_eq!(a.data("open"), 0);
    assert_eq!(a.count(".sec-body"), 1);
    a.tap_nth(".sec", 2);
    assert_eq!(a.data("open"), 2);
    a.tap_nth(".sec", 2);
    assert_eq!(a.data("open"), -1, "再点收起");
    assert_eq!(a.count(".sec-body"), 0);
    a.tap_settle(".play");
    assert_eq!(a.route(), "pages/game/game");
    a.assert_no_errors();
}
