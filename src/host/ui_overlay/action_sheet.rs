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
use super::sheet_raster::{Placement, RasterKey, SheetRaster};
use crate::compositor::footprint::{self, Footprint};
use crate::runtime::MiniApp;
use crate::text::TextRenderer;
use crate::Color;
use serde_json::Value;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
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
    /// 这串触摸是在菜单弹出**之后**按下的。长按里弹的菜单：按下那根手指还在屏上，
    /// 它抬起时不能当成「点了遮罩」—— 从前菜单刚弹出就以 fail cancel 关掉（轻聊会话列表）
    armed: bool,
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
    /// 面板缓存（sheet_raster.rs）。它画在一块整屏画布上，换下一张时画布接着用（按一下选项就要重画一次）
    static RASTER: RefCell<Option<SheetRaster>> = const { RefCell::new(None) };
    /// [`reach`] 算出的这一帧的贴法（GPU 据此把面板缓存当纹理挪，见 [`gpu_layer`]）
    static PLACED: std::cell::Cell<Option<Placement>> = const { std::cell::Cell::new(None) };
    /// 面板正文这一帧由 GPU 从缓存纹理贴（[`set_panel_on_gpu`]）：CPU 只画 `rows` 里的圆角行
    static PANEL_ON_GPU: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 面板缓存的编号（`SheetRaster::id`）：GPU 合成器认这个号决定要不要重传纹理，所以必须**进程里唯一**。
///
/// 从前它是线程局部的、每条线程都从 1 数：GPU 自检在预热线程（`mini-gpu-warm`）上画了一张 1 号缓存
/// （两项、没有文字），合成器搬到引擎线程后，第一次真弹出来的菜单也是 1 号 —— 合成器以为纹理就是它、
/// 不传，屏上贴的是自检那张：面板白板没字，底下一截黑（自检的面板矮，越界取样是 0）。
/// 直到按一下选项（换一张缓存）才变对
static NEXT_RASTER: AtomicU64 = AtomicU64::new(1);

/// GPU 直通帧把面板缓存当纹理挪时，合成前设 true、合成后设 false：这期间 [`render`] 只写 `rows`
/// 那几行（圆角），面板其余行由 GPU 贴。平时面板整块画（`rows` 只限定压暗与底图）
pub fn set_panel_on_gpu(on: bool) {
    PANEL_ON_GPU.with(|c| c.set(on));
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
    RASTER.with(|r| *r.borrow_mut() = None);
    PLACED.with(|p| p.set(None));
    // CPU 合成器留着的底图（整屏 10MB）：GPU 上屏时 CPU 合成器可能好久都不再被叫到，这里一起放掉
    crate::compositor::sheet_base::release();
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
        with(|s| *s = Some(ActionSheet { id, alert, items, color, opened: crate::frame_clock::now(), closing: None, pressed: None, armed: false, settled: false }));
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
        st.armed = true;
        true
    })
}

/// 抬手：点选项回 tapIndex，点取消 / 遮罩回 fail cancel，然后退场。返回是否消费
pub fn on_release(app: &mut MiniApp, vp: (f32, f32), _x: f32, y: f32) -> bool {
    let outcome = with(|s| {
        let st = s.as_mut()?;
        st.pressed = None;
        // 弹出前就按下的那根手指：抬起交还页面（它要收 touchend），菜单留着
        if !st.armed {
            return None;
        }
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
    render_impl(buf, None, w, h, sf, text, rows)
}

/// [`render`]，但 `rows` 那几行的底图从 `base`（同样大小的缓冲）读：边读边压暗写进 `buf`，再贴面板。
/// CPU 合成器把底图留着一份时用（`compositor::sheet_base`），结果与「先把底图拼进 `buf` 再 [`render`]」逐字节相同
pub fn render_over(buf: &mut [u32], base: &[u32], w: u32, h: u32, sf: f32, text: Option<&TextRenderer>, rows: (u32, u32)) -> Footprint {
    render_impl(buf, Some(base), w, h, sf, text, rows)
}

fn render_impl(buf: &mut [u32], base: Option<&[u32]>, w: u32, h: u32, sf: f32, text: Option<&TextRenderer>, rows: (u32, u32)) -> Footprint {
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
        // 按表压暗（与逐像素 `(c as f32 * keep) as u32` 逐字节相同；表写得成乘法时按乘法算，见
        // `Footprint::apply_lut`）：遮罩每帧要盖满面板以上的整片屏，两百万像素上下
        let lut = footprint::lut_trunc(1.0 - mask);
        match base {
            Some(src) => Footprint::apply_lut_from(src, buf, w, rows, &lut, 0),
            None => Footprint::apply_lut(buf, w, rows, &lut, 0),
        }
        fp.dim(&lut);
    } else if let Some(src) = base {
        // 遮罩淡到看不出来了：底图原样拷过去
        let n = src.len().min(buf.len());
        let (a, b) = ((rows.0 as usize * w as usize).min(n), (rows.1 as usize * w as usize).min(n));
        if a < b {
            buf[a..b].copy_from_slice(&src[a..b]);
        }
    }
    let sheet_h = sheet_height(&st);
    let top = h as f32 / sf - sheet_h + sheet_h * hidden;
    LAST_TOP.with(|c| c.set(Some(top)));
    // 面板画一次、按整像素挪着贴（sheet_raster.rs）。下面每一笔都落在面板顶边以下（面板一直伸到屏幕底）
    let panel_rows = if PANEL_ON_GPU.with(|c| c.get()) { rows } else { (0, h) };
    let placed = with_raster(&st, (w, h), sf, text, top, |r, p| r.blit(buf, w, h, p, panel_rows));
    if let Some(p) = placed {
        fp.touch(p.top_row as f32, h as f32, h);
    }
    fp
}

/// 面板缓存（状态变了就重画），交给 `f` 用；返回这一帧顶边 `top` 时的贴法
fn with_raster<R>(
    st: &ActionSheet,
    (w, h): (u32, u32),
    sf: f32,
    text: Option<&TextRenderer>,
    top: f32,
    f: impl FnOnce(&SheetRaster, Placement) -> R,
) -> Option<Placement> {
    let key = RasterKey { sheet: st.id, pressed: st.pressed, size: (w, h), sf_bits: sf.to_bits(), has_text: text.is_some() };
    RASTER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().map(|r| &r.key) != Some(&key) {
            let t0 = Instant::now();
            let id = NEXT_RASTER.fetch_add(1, Ordering::Relaxed);
            // 按**完全展开**的位置画：入场第一帧面板还整个在屏幕下面，按那一刻画的话缓存是空的。
            // 展开后贴的位置与画的位置相同，静止的面板与直接画逐字节一致
            let sheet_h = sheet_height(st);
            let open_top = h as f32 / sf - sheet_h;
            let rect = (0.0, open_top * sf, w as f32, (sheet_h + RADIUS) * sf);
            // 旧缓存的画布接着用（尺寸不对 build 会重新分配）
            let canvas = slot.take().map(SheetRaster::into_canvas).unwrap_or_default();
            *slot = Some(SheetRaster::build(id, key, (w, h), sf, open_top, rect, RADIUS * sf, canvas, |b| {
                draw_panel(b, w, h, sf, text, st, open_top)
            }));
            // 弹出的第一帧就在画它：`MINI_DAMAGE_LOG` 下记一笔（真机上曾经一张 7.5~10ms）
            if std::env::var_os("MINI_DAMAGE_LOG").is_some() {
                crate::compositor::diag::emit(&format!("🧾 操作菜单面板缓存 #{id} 画了 {:.1}ms", t0.elapsed().as_secs_f32() * 1000.0));
            }
        }
        let r = slot.as_ref()?;
        let p = r.place(top, sf);
        f(r, p);
        Some(p)
    })
}

/// GPU 把面板缓存当纹理挪（`compositor::gpu::passthrough`）：这一帧的
/// `(顶边整像素行, 圆角行数, 缓存行数, 缓存 id)`。只在 [`reach`] 之后、[`render`] 之前有效
pub fn gpu_layer() -> Option<(i32, u32, u32, u64)> {
    let p = PLACED.with(|c| c.get())?;
    RASTER.with(|r| r.borrow().as_ref().map(|r| (p.top_row, p.corner_rows, p.rows, r.id)))
}

/// GPU 自检用：临时摆一个静止的菜单跑 `f`（面板缓存纹理那条路要与 CPU 逐字节一致才用 GPU），
/// 跑完原样放回 —— 宿主正弹着的菜单、缓存都不受影响
#[cfg(feature = "gpu")]
pub(crate) fn with_selftest_sheet<R>(f: impl FnOnce() -> R) -> R {
    let saved = with(|s| s.take());
    let saved_raster = RASTER.with(|r| r.borrow_mut().take());
    let saved_placed = PLACED.with(|p| p.take());
    let t = Instant::now().checked_sub(std::time::Duration::from_secs(5)).unwrap_or_else(Instant::now);
    let st = ActionSheet {
        id: u64::MAX,
        alert: String::new(),
        items: vec!["自检".into(), "面板缓存".into()],
        color: Color::BLACK,
        opened: t,
        closing: None,
        // 按着第一项：按压底盖过圆角，缓存里那几个像素要按内容贴
        pressed: Some(0),
        armed: true,
        settled: true,
    };
    with(|s| *s = Some(st));
    let r = f();
    // 自检拿 CPU 合成当参照时留下的底图（自检可能就跑在引擎线程上）
    crate::compositor::sheet_base::release();
    with(|s| *s = saved);
    RASTER.with(|c| *c.borrow_mut() = saved_raster);
    PLACED.with(|p| p.set(saved_placed));
    FROZEN.with(|f| f.set(None));
    r
}

/// 面板缓存的像素（0x00RRGGBB，`width × rows`，第 0 行是贴法里的 `top_row`）
pub fn with_raster_pixels<R>(f: impl FnOnce(u32, u32, &[u32]) -> R) -> Option<R> {
    RASTER.with(|r| r.borrow().as_ref().map(|r| f(r.width, r.rows, r.pixels())))
}

/// 画之前就给出这一帧的范围（GPU 直通帧据此只合成那一截）：
/// `(会写到的行, 其中要先有底图的行)`。并把这一刻定下来，紧接着的 [`render`] 按同一刻画。
///
/// 面板是满宽、不透明的白底：顶边圆角那几行以下整行被白底盖掉，那些行不用先合成页面、
/// 也不用压暗 —— 操作菜单弹着时 CPU 只合成顶边那三四十行，其余是面板自己
pub fn reach(w: u32, h: u32, sf: f32, text: Option<&TextRenderer>) -> (Footprint, Footprint) {
    let (mut rows, mut base) = (Footprint::NONE, Footprint::NONE);
    PLACED.with(|c| c.set(None));
    if let Some(st) = with(|s| s.clone()) {
        let now = crate::frame_clock::now();
        FROZEN.with(|f| f.set(Some(now)));
        let sheet_h = sheet_height(&st);
        let top = h as f32 / sf - sheet_h + sheet_h * hidden_ratio(&st, now);
        // 面板缓存里实测出来的圆角行：只有这几行与底下的页面有关（sheet_raster.rs）
        if let Some(p) = with_raster(&st, (w, h), sf, text, top, |_, _| ()) {
            PLACED.with(|c| c.set(Some(p)));
            rows.touch(p.top_row as f32, h as f32, h);
            base.touch(p.top_row as f32, (p.top_row + p.corner_rows as i32) as f32, h);
        }
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
        armed: true, // 测试里直接点：当作弹出之后才按下
        pressed: Some(1),
        settled: false,
    };
    with(|s| *s = Some(st));
}

#[cfg(test)]
mod tests;
