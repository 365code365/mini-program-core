//! 聊天页（sample/chat `pages/chat/chat`）的键盘：输入框在页面底部的输入条里，键盘弹起时要像原生聊天那样
//! **消息列表变矮、输入条贴着键盘、最新消息仍在眼前**，而不是整页被顶上去、上面的聊天记录被推出屏幕。
//!
//! 微信的写法：输入框 `adjust-position="{{false}}"` + `bindkeyboardheightchange`，页面按键盘高度改 scroll-view 的高。
//! 判据：
//! ① 宿主推 `keyboardHeightChange` 时，聚焦输入框的 `bindkeyboardheightchange` 收到 `{height, duration}`（number）；
//! ② `adjust-position=false` 的输入框，`inputFocus` 里 `adjustPosition` 为 false（宿主据此不顶画面）；
//! ③ 输入条跟着键盘上移之后（输入框没有 `id`，组件 id 含坐标、位置变了），焦点仍在它身上：
//!    打字照样进 `bindinput`、光标画在新位置；
//! ④ 键盘收起（高度 0）时页面也收到一次，列表恢复原高；
//! ⑤ 宿主送 `escape`（用户按返回键 / 下滑收起键盘）：输入框失焦、发 `inputBlur`、光标消失；
//! ⑥ 列表变矮后 `scrollToEnd`（`intoView` 先设 '' 再设回最后一条，同一帧）照样滚到底；
//! ⑦ 上屏与发送键同一拍到（中文输入法点「发送」）：发送后输入框清空，不留上一句。
use mini_render::host::{EngineOptions, MiniEngine};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

const W: u32 = 375;
const H: u32 = 667;
const DPR: f32 = 2.0;

fn engine(tag: &str) -> MiniEngine {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/chat");
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("chat-kb-{tag}"));
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), W, H, DPR, Default::default(), EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(Some("pages/chat/chat")).expect("打开聊天页");
    e
}

fn pump(e: &mut MiniEngine, t: &mut u64, n: u32) {
    for _ in 0..n {
        *t += 16;
        std::thread::sleep(Duration::from_millis(4));
        e.pump(*t);
    }
}

fn tap(e: &mut MiniEngine, t: &mut u64, (x, y): (f32, f32)) {
    e.pointer_down(x, y);
    pump(e, t, 1);
    e.pointer_up(x, y);
    pump(e, t, 3);
}

fn events(e: &mut MiniEngine) -> Vec<Value> {
    std::iter::from_fn(|| e.poll_event()).filter_map(|s| serde_json::from_str(&s).ok()).collect()
}

fn data(e: &mut MiniEngine, key: &str) -> Value {
    let s = e.eval(&format!("JSON.stringify(getCurrentPages().slice(-1)[0].data.{key})")).unwrap_or_default();
    serde_json::from_str(s.trim()).unwrap_or(Value::Null)
}

fn num(e: &mut MiniEngine, key: &str) -> f32 {
    data(e, key).as_f64().unwrap_or(-1.0) as f32
}

/// 输入框所在那一条（逻辑像素 y 范围）里有没有光标色（#07C160 一类的绿）
fn caret_rows(e: &mut MiniEngine) -> Vec<u32> {
    let px = e.rgba().to_vec();
    let (pw, ph) = e.pixel_size();
    let mut rows = Vec::new();
    for y in 0..ph {
        let lit = (0..pw / 2).any(|x| {
            let i = ((y * pw + x) * 4) as usize;
            px[i] < 40 && px[i + 1] > 160 && px[i + 2] < 120
        });
        if lit {
            rows.push(y);
        }
    }
    rows
}

/// 等光标亮的那一拍（500ms 闪一次），返回亮着的设备像素行
fn wait_caret(e: &mut MiniEngine, t: &mut u64) -> Vec<u32> {
    for _ in 0..80 {
        pump(e, t, 1);
        let r = caret_rows(e);
        if !r.is_empty() {
            return r;
        }
    }
    Vec::new()
}

/// 消息列表（页面上唯一的 scroll-view）的滚动偏移
fn list_offset(e: &MiniEngine) -> f32 {
    let v = e.scroll_view_offsets();
    assert_eq!(v.len(), 1, "只该有一个 scroll-view：{v:?}");
    v[0].1
}

fn focus_input(e: &mut MiniEngine, t: &mut u64) -> Value {
    pump(e, t, 8);
    let (scroll_h, bar_h) = (num(e, "scrollH"), num(e, "barH"));
    assert!(scroll_h > 300.0 && bar_h > 40.0, "scrollH {scroll_h} barH {bar_h}");
    events(e);
    tap(e, t, (80.0, scroll_h + bar_h / 2.0));
    events(e).into_iter().find(|v| v["type"] == "inputFocus").expect("点输入条应聚焦输入框")
}

#[test]
fn 聊天页_键盘弹起列表变矮_输入条贴键盘_焦点跟着输入框走() {
    let (mut e, mut t) = (engine("main"), 1000u64);
    let focus = focus_input(&mut e, &mut t);
    assert_eq!(focus["adjustPosition"], false, "聊天输入框不该让宿主顶画面：{focus}");
    let h0 = num(&mut e, "scrollH");
    // 先聊几句，让消息比键盘弹起后的列表高（对方 0.4~2s 后回复，回复按墙上时钟到）
    for msg in ["在吗", "周末一起吃饭吗", "几点"] {
        e.text_input(msg);
        e.key("enter");
        for _ in 0..70 {
            std::thread::sleep(Duration::from_millis(30));
            t += 30;
            e.pump(t);
        }
    }
    assert!(data(&mut e, "rows").as_array().map(|r| r.len()).unwrap_or(0) >= 10, "回复没到：{}", data(&mut e, "rows"));
    let evs = events(&mut e);
    assert!(!evs.iter().any(|v| v["type"] == "inputBlur"), "confirm-hold 的发送不该失焦：{evs:?}");
    // 发送之后输入框清空：IME 上屏与发送键同一拍到（bindinput 的 setData 还没画就 bindconfirm 清空），
    // 也不能留着上一句（engine_input_queue.rs）
    assert_eq!(data(&mut e, "draft"), "");
    let caret0 = wait_caret(&mut e, &mut t);
    let o0 = list_offset(&e);
    // 11 行消息比 611 高的列表略长：滚到底的偏移是正的（375×667 下实测 28）
    assert!(o0 > 1.0, "聊了几句该已滚到最新（偏移 {o0}）");
    assert!(!caret0.is_empty(), "聚焦后没有光标");

    // ① 键盘弹起 300：页面收到高度，列表变矮
    e.dispatch_event("keyboardHeightChange", r#"{"height":300,"duration":250}"#);
    pump(&mut e, &mut t, 6);
    let h1 = num(&mut e, "scrollH");
    assert!((h0 - h1 - 300.0).abs() < 1.0, "列表该矮 300：{h0} → {h1}");
    assert_eq!(num(&mut e, "kbH"), 300.0, "bindkeyboardheightchange 的 detail.height 要是 number 300");

    // 列表变矮后滚到最新：最后一条贴着列表底（`scroll-into-view-alignment="end"`），不是顶到列表最上面、
    // 下面空一大片（Skyline 默认 start 对齐，真机上键盘一弹只剩最后两条，看着像「聊天记录被顶没了」）
    // scrollToEnd 是 `setData({intoView:''}); setData({intoView:last})`：同一帧里变过又变回原值也要再滚
    // （host/scroll_view.rs `rearm_into_view`）。滚到底 = 偏移比键盘弹起前多出列表变矮的那 300
    pump(&mut e, &mut t, 40);
    let o1 = list_offset(&e);
    assert!((o1 - (o0 + (h0 - h1))).abs() < 2.0, "列表变矮后该滚到最新（底对齐）：偏移 {o0} → {o1}，应为 {}", o0 + (h0 - h1));

    // ③ 输入条上移之后：光标画在新位置（上移 300 逻辑像素 = 600 设备像素），打字照样进 bindinput
    let caret1 = wait_caret(&mut e, &mut t);
    assert!(!caret1.is_empty(), "输入条上移之后光标没了（焦点丢了）");
    let moved = caret0[0] as i64 - caret1[0] as i64;
    assert!((moved - 600).abs() <= 2, "光标该跟着输入条上移 600 设备像素：{} → {}", caret0[0], caret1[0]);
    e.text_input("你好");
    pump(&mut e, &mut t, 3);
    assert_eq!(data(&mut e, "draft"), "你好", "输入条上移后打的字没进页面");
    assert_eq!(data(&mut e, "canSend"), true);

    // 发送：消息进列表、输入框清空、焦点还在（confirm-type=send 回车）
    e.key("enter");
    pump(&mut e, &mut t, 3);
    assert_eq!(data(&mut e, "draft"), "", "回车发送后输入框该清空");

    // ⑤ 用户收起键盘（返回键）：宿主送 escape → 失焦
    events(&mut e);
    e.key("escape");
    pump(&mut e, &mut t, 2);
    assert!(events(&mut e).iter().any(|v| v["type"] == "inputBlur"), "收起键盘该发 inputBlur");
    // ④ 键盘落下：高度 0 也要送到（输入框此时已失焦）
    e.dispatch_event("keyboardHeightChange", r#"{"height":0,"duration":200}"#);
    pump(&mut e, &mut t, 40);
    assert_eq!(num(&mut e, "scrollH"), h0, "键盘收起后列表该恢复原高");
    assert!(caret_rows(&mut e).is_empty(), "失焦后光标还在");
}

/// 表情面板开着时点输入框：面板收起、弹键盘；再点一次（已聚焦）宿主仍收到 inputFocus（宿主据此把被用户收起的键盘再弹出来）
#[test]
fn 聊天页_已聚焦再点输入框_仍发inputFocus() {
    let (mut e, mut t) = (engine("refocus"), 1000u64);
    focus_input(&mut e, &mut t);
    pump(&mut e, &mut t, 3);
    let (scroll_h, bar_h) = (num(&mut e, "scrollH"), num(&mut e, "barH"));
    tap(&mut e, &mut t, (120.0, scroll_h + bar_h / 2.0));
    assert!(events(&mut e).iter().any(|v| v["type"] == "inputFocus"), "再点已聚焦的输入框要再发 inputFocus");
}
