//! `wx.showActionSheet` 的底部操作菜单（官方 api/ui/interaction/wx.showActionSheet.html）。
//!
//! 与 picker 面板同一类：宿主画的原生浮层，画在合成那一步（`compositor::cpu`）；GPU 后端只传面板那几行
//! （遮罩在着色器里查表，见 `compositor::footprint`）。
//! - `alertText`（可选）在最上面：56 高、12px、FG-1 灰、最多两行（WeUI 2.x `weui-actionsheet__title`）；
//!   `itemList` 最多 6 项（多了走 fail，与微信一致），每行 56 高、17px，`itemColor` 默认 #000000；
//!   下面隔一道 8px 的 #F7F7F7 灰缝是「取消」（FG-0）；面板上两角圆 12（抗锯齿）；
//! - 点某一项：`success({tapIndex})`（数字）；点「取消」或遮罩：`fail({errMsg: 'showActionSheet:fail cancel'})`；
//! - 入场 / 退场 0.3s 从底边滑入、遮罩淡入（WeUI `transition: transform .3s`）。
//!
//! 状态放在线程局部（一个引擎实例一条线程），SDK 与桌面共用；宿主只在三处接线：
//! 每帧 [`take_requests`]、指针 [`on_press`] / [`on_release`]、合成 [`render`]。

use super::draw::*;
use crate::compositor::footprint::{self, Footprint};
use crate::runtime::MiniApp;
use crate::text::TextRenderer;
use crate::Color;
use serde_json::Value;
use std::cell::RefCell;
use std::time::Instant;

const ITEM_H: f32 = 56.0;
const ALERT_H: f32 = 56.0;
const ALERT_FONT: f32 = 12.0;
const GAP_H: f32 = 8.0;
const RADIUS: f32 = 12.0;
const ANIM_SECS: f32 = 0.3;
const MAX_ITEMS: usize = 6;
/// rgba(0,0,0,0.5) / rgba(0,0,0,0.9) 压在白底上
const FG_1: u32 = 0x808080;
const FG_0: u32 = 0x1A1A1A;

#[derive(Clone, Debug)]
pub struct ActionSheet {
    id: u64,
    alert: String,
    items: Vec<String>,
    color: Color,
    opened: Instant,
    closing: Option<Instant>,
    /// 按住的那一行：0.. 是选项，-1 是「取消」
    pressed: Option<i32>,
    /// 当前这段入场 / 退场的终态已经画出来过（之后不必再续帧）。
    /// 只看时间不够：宿主在动画末尾那一刻恰好没出帧的话，最后合成的是半路的画面
    settled: bool,
}

thread_local! {
    static SHEET: RefCell<Option<ActionSheet>> = const { RefCell::new(None) };
    /// [`reach`] 用的那一刻：紧接着的 [`render`] 按同一刻画（取走即清）。
    /// 动画按墙上时钟走，两次各取 `Instant::now()` 的话面板位置就对不上 reach 算的范围
    static FROZEN: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
    /// 最近一次画出来的面板顶边（逻辑像素；`MINI_PACE` 诊断用）
    static LAST_TOP: std::cell::Cell<Option<f32>> = const { std::cell::Cell::new(None) };
}

/// 最近一次画出来的面板顶边（逻辑像素），没弹着时 None（出帧节拍诊断用）
pub fn last_drawn_top() -> Option<f32> {
    if !visible() {
        return None;
    }
    LAST_TOP.with(|c| c.get())
}

fn with<R>(f: impl FnOnce(&mut Option<ActionSheet>) -> R) -> R {
    SHEET.with(|s| f(&mut s.borrow_mut()))
}

/// 菜单弹着（含退场动画途中）：宿主据此把指针交给它
pub fn visible() -> bool {
    with(|s| s.is_some())
}

/// 入场 / 退场动画在走、或终态还没画出来（宿主据此续帧 / 重新合成）
pub fn animating() -> bool {
    with(|s| s.as_ref().map(|st| !st.settled).unwrap_or(false))
}

/// 换页 / 销毁时收掉（不回调：页面已经没了）
pub fn clear() {
    with(|s| *s = None);
}

fn sheet_height(st: &ActionSheet) -> f32 {
    (if st.alert.is_empty() { 0.0 } else { ALERT_H }) + st.items.len() as f32 * ITEM_H + GAP_H + ITEM_H
}

/// 面板顶边的位移比例：0 = 完全展开，1 = 完全收起（按 `now` 那一刻算）
fn hidden_ratio(st: &ActionSheet, now: Instant) -> f32 {
    let (t0, closing) = match st.closing {
        Some(t) => (t, true),
        None => (st.opened, false),
    };
    let t = (now.saturating_duration_since(t0).as_secs_f32() / ANIM_SECS).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t).powi(3);
    if closing {
        eased
    } else {
        1.0 - eased
    }
}

/// 每帧：取走 `wx.showActionSheet` 的请求；退场动画走完就丢掉。返回是否要重新合成
pub fn take_requests(app: &mut MiniApp) -> bool {
    let mut changed = false;
    for req in app.take_host_requests("actionSheet") {
        let id = req.get("id").and_then(Value::as_u64).unwrap_or(0);
        let items: Vec<String> = req
            .get("itemList")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).collect())
            .unwrap_or_default();
        if items.is_empty() || items.len() > MAX_ITEMS {
            let msg = if items.is_empty() { "parameter error: itemList should not be empty" } else { "parameter error: itemList should not be large than 6" };
            resolve(app, id, None, Some(msg));
            continue;
        }
        // 已经有一个弹着：旧的按取消收尾（微信里新的会顶掉旧的）
        if let Some(old) = with(|s| s.take()) {
            resolve(app, old.id, None, Some("cancel"));
        }
        let color = req
            .get("itemColor")
            .and_then(Value::as_str)
            .and_then(crate::host::tabbar::parse_color)
            .unwrap_or(Color::BLACK);
        let alert = req.get("alertText").and_then(Value::as_str).unwrap_or("").to_string();
        with(|s| *s = Some(ActionSheet { id, alert, items, color, opened: crate::frame_clock::now(), closing: None, pressed: None, settled: false }));
        changed = true;
    }
    let now = crate::frame_clock::now();
    let done = with(|s| s.as_ref().and_then(|st| st.closing).map(|t| now.saturating_duration_since(t).as_secs_f32() >= ANIM_SECS).unwrap_or(false));
    if done {
        clear();
        changed = true;
    }
    changed || animating()
}

fn resolve(app: &mut MiniApp, id: u64, tap: Option<usize>, err: Option<&str>) {
    let js = match (tap, err) {
        (Some(i), _) => format!("__resolveActionSheet({id}, {i}, '')"),
        (None, e) => format!("__resolveActionSheet({id}, -1, {})", serde_json::to_string(e.unwrap_or("cancel")).unwrap_or_default()),
    };
    app.eval(&js).ok();
}

/// 逻辑坐标 → 命中：Some(行) 0.. 是选项、-1 是取消；None 是遮罩
fn hit(st: &ActionSheet, vp: (f32, f32), y: f32) -> Option<i32> {
    let h = sheet_height(st);
    let top = vp.1 - h + h * hidden_ratio(st, Instant::now());
    if y < top {
        return None;
    }
    let mut cur = top + if st.alert.is_empty() { 0.0 } else { ALERT_H };
    if y < cur {
        // 标题那一行不可点：按在上面什么也不选（不关闭）
        return Some(i32::MIN);
    }
    for i in 0..st.items.len() {
        if y < cur + ITEM_H {
            return Some(i as i32);
        }
        cur += ITEM_H;
    }
    if y < cur + GAP_H {
        return Some(i32::MIN);
    }
    Some(-1)
}

/// 按下：菜单弹着就吃掉（并给按住的那一行一个按压态）。返回是否消费
pub fn on_press(vp: (f32, f32), _x: f32, y: f32) -> bool {
    with(|s| {
        let Some(st) = s.as_mut() else { return false };
        st.pressed = if st.closing.is_some() { None } else { hit(st, vp, y).filter(|r| *r != i32::MIN) };
        true
    })
}

/// 抬手：点选项回 tapIndex，点取消 / 遮罩回 fail cancel，然后退场。返回是否消费
pub fn on_release(app: &mut MiniApp, vp: (f32, f32), _x: f32, y: f32) -> bool {
    let outcome = with(|s| {
        let st = s.as_mut()?;
        st.pressed = None;
        if st.closing.is_some() {
            return Some(None);
        }
        let r = hit(st, vp, y);
        if r == Some(i32::MIN) {
            return Some(None);
        }
        st.closing = Some(Instant::now());
        st.settled = false;
        Some(Some((st.id, r)))
    });
    match outcome {
        None => false,
        Some(None) => true,
        Some(Some((id, r))) => {
            match r {
                Some(i) if i >= 0 => resolve(app, id, Some(i as usize), None),
                _ => resolve(app, id, None, Some("cancel")),
            }
            true
        }
    }
}

/// 画进输出缓冲（0x00RRGGBB，行宽 `w`）。没有菜单时什么也不写。
/// 返回这一帧写了哪些像素（GPU 只传面板那几行，遮罩在着色器里查表，见 `compositor::footprint`）
///
/// `rows`：只合成缓冲的这几行（遮罩只压这几行；面板总在 [`reach`] 之内）
pub fn render(buf: &mut [u32], w: u32, h: u32, sf: f32, text: Option<&TextRenderer>, rows: (u32, u32)) -> Footprint {
    let mut fp = Footprint::NONE;
    let now = FROZEN.with(|f| f.take()).unwrap_or_else(crate::frame_clock::now);
    let Some(st) = with(|s| s.clone()) else { return fp };
    let hidden = hidden_ratio(&st, now);
    let t0 = st.closing.unwrap_or(st.opened);
    if now.saturating_duration_since(t0).as_secs_f32() >= ANIM_SECS {
        with(|s| if let Some(x) = s.as_mut() { x.settled = true });
    }
    // 遮罩：0.6 黑，随面板淡入淡出
    let mask = 0.6 * (1.0 - hidden);
    if mask > 0.002 {
        // 查表压暗（与逐像素 `(c as f32 * keep) as u32` 逐字节相同）：遮罩每帧要盖满整屏，
        // 1080×2354 两百多万像素，逐像素三次浮点乘法在一加 PLY110 上一帧要好几毫秒
        let lut = footprint::lut_trunc(1.0 - mask);
        Footprint::apply_lut(buf, w, rows, &lut, 0);
        fp.dim(&lut);
    }
    let sheet_h = sheet_height(&st);
    let top = h as f32 / sf - sheet_h + sheet_h * hidden;
    LAST_TOP.with(|c| c.set(Some(top)));
    // 下面每一笔都落在面板顶边以下（面板一直伸到屏幕底）
    fp.touch(top * sf, h as f32, h);
    draw_panel(buf, w, h, sf, text, &st, top);
    fp
}

/// 画之前就给出这一帧的范围（GPU 直通帧据此只合成那一截）：
/// `(会写到的行, 其中要先有底图的行)`。并把这一刻定下来，紧接着的 [`render`] 按同一刻画。
///
/// 面板是满宽、不透明的白底：顶边圆角那几行以下整行被白底盖掉，那些行不用先合成页面、
/// 也不用压暗 —— 操作菜单弹着时 CPU 只合成顶边那三四十行，其余是面板自己
pub fn reach(h: u32, sf: f32) -> (Footprint, Footprint) {
    let (mut rows, mut base) = (Footprint::NONE, Footprint::NONE);
    if let Some(st) = with(|s| s.clone()) {
        let now = crate::frame_clock::now();
        FROZEN.with(|f| f.set(Some(now)));
        let sheet_h = sheet_height(&st);
        let top = h as f32 / sf - sheet_h + sheet_h * hidden_ratio(&st, now);
        rows.touch(top * sf, h as f32, h);
        // 与 fill_rrect_aa 的「整行」判定对齐（覆盖率 1、在圆角之下），再多留一行
        base.touch(top * sf, (top + RADIUS) * sf + 1.0, h);
    }
    (rows, base)
}

fn draw_panel(buf: &mut [u32], w: u32, h: u32, sf: f32, text: Option<&TextRenderer>, st: &ActionSheet, top: f32) {
    let px = |v: f32| (v * sf).round() as i32;
    let sheet_h = sheet_height(st);
    let (bw, bh) = (w as i32, h as i32);
    // 白底（上两角圆、抗锯齿），灰缝，取消行
    let sheet_rect = (0.0, top * sf, w as f32, (sheet_h + RADIUS) * sf);
    fill_rrect_aa(buf, w, h, sheet_rect, RADIUS * sf, 0xFFFFFF, 1.0, [true, true, false, false]);
    let gap_top = top + sheet_h - ITEM_H - GAP_H;
    fill(buf, w, h, 0, px(gap_top), bw, px(GAP_H), 0xF7F7F7);
    let mut y = top;
    let Some(tr) = text else { return };
    let font = 17.0 * sf;
    let center = |t: &str, size: f32| ((w as f32 - tr.measure_text(t, size)) / 2.0) as i32;
    if !st.alert.is_empty() {
        // 左右各留 24，最多两行（`.ellipsisLn(2)`），整块在 56 高里垂直居中
        let small = ALERT_FONT * sf;
        let line_h = ALERT_FONT * 1.4;
        let lines = wrap_lines(Some(tr), &st.alert, ALERT_FONT, w as f32 / sf - 48.0, 2, false);
        let block_top = y + (ALERT_H - lines.len() as f32 * line_h) / 2.0;
        for (i, l) in lines.iter().enumerate() {
            let baseline = (block_top + i as f32 * line_h + line_h / 2.0) * sf + small * 0.34;
            draw_text_line(buf, w, h, tr, l, (center(l, small) as f32, baseline), small, FG_1, false, sf);
        }
        y += ALERT_H;
        fill(buf, w, h, 0, px(y) - 1, bw, 1, 0xE5E5E5);
    }
    for (i, item) in st.items.iter().enumerate() {
        if st.pressed == Some(i as i32) {
            fill(buf, w, h, 0, px(y), bw, px(ITEM_H), 0xECECEC);
        }
        draw_text_direct(buf, w, h, tr, item, center(item, font), px(y + (ITEM_H - 17.0 * 1.2) / 2.0), font, st.color);
        y += ITEM_H;
        if i + 1 < st.items.len() {
            fill(buf, w, h, 0, px(y) - 1, bw, 1, 0xE5E5E5);
        }
    }
    let cancel_y = y + GAP_H;
    if st.pressed == Some(-1) {
        fill(buf, w, h, 0, px(cancel_y), bw, px(ITEM_H), 0xECECEC);
    }
    draw_text_direct(buf, w, h, tr, "取消", center("取消", font), px(cancel_y + (ITEM_H - 17.0 * 1.2) / 2.0), font, Color::from_hex(FG_0));
    let _ = bh;
}

fn fill(buf: &mut [u32], w: u32, h: u32, x: i32, y: i32, rw: i32, rh: i32, c: u32) {
    for yy in y.max(0)..(y + rh).min(h as i32) {
        for xx in x.max(0)..(x + rw).min(w as i32) {
            buf[(yy as u32 * w + xx as u32) as usize] = c;
        }
    }
}

/// 测试用：直接摆一个 `ago_ms` 毫秒前弹出（`closing` 时是那时开始收起）的菜单，第 1 项按着
#[cfg(test)]
pub(crate) fn open_at(ago_ms: u64, closing: bool) {
    let t = Instant::now().checked_sub(std::time::Duration::from_millis(ago_ms)).unwrap();
    let st = ActionSheet {
        id: 1,
        alert: "提示".into(),
        items: vec!["a".into(), "b".into(), "c".into()],
        color: Color::BLACK,
        opened: t,
        closing: closing.then_some(t),
        pressed: Some(1),
        settled: false,
    };
    with(|s| *s = Some(st));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GPU 直通帧先按 `reach` 只合成那一截：入场 / 退场任何时刻画出来的都不能超出它，
    /// 而且分窗画出来的那一截要与整帧画的逐字节相同（遮罩只压了窗口里的行）
    #[test]
    fn 面板总在_reach_之内_分窗与整帧逐字节相同() {
        let (w, h, sf) = (90u32, 500u32, 1.5f32);
        let base: Vec<u32> = (0..w * h).map(|i| i.wrapping_mul(2_654_435_761) & 0xFF_FFFF).collect();
        for (ago, closing) in [(0, false), (40, false), (120, false), (500, false), (60, true), (250, true)] {
            open_at(ago, closing);
            let (rows, base_rows) = reach(h, sf);
            let at = FROZEN.with(|f| f.get()).expect("reach 定住了这一刻");
            let win = rows.rows.expect("弹着就有范围");
            let b = base_rows.rows.unwrap_or((win.0, win.0));
            assert!(b.0 == win.0 && b.1 <= win.1, "{ago}ms：底图行 {b:?} 应在范围 {win:?} 的开头");
            if win.1 - win.0 > 100 {
                assert!(b.1 - b.0 < 40, "{ago}ms：白底整行盖住的那些行不该要底图 {b:?} / {win:?}");
            }
            // 分窗：只给顶边那几行合成底图、压暗，其余行靠白底整行盖掉；未合成的行先填一个怪色
            let mut part: Vec<u32> = base.iter().enumerate().map(|(i, &p)| if (i as u32 / w) >= b.0 && (i as u32 / w) < b.1 { p } else { 0x12_3456 }).collect();
            let fp = render(&mut part, w, h, sf, None, b);
            assert_eq!(fp.rows, Some(win), "{ago}ms：同一刻画出来的范围就是 reach 给的");
            // 整帧（同一刻）
            FROZEN.with(|f| f.set(Some(at)));
            let mut full = base.clone();
            render(&mut full, w, h, sf, None, (0, h));
            let (s, e) = ((win.0 * w) as usize, (win.1 * w) as usize);
            assert!(part[s..e] == full[s..e], "{ago}ms：分窗与整帧在覆盖层范围里不一致");
        }
        clear();
    }
}
