//! 小窗（`picture-in-picture-mode`，目前只有同层渲染的 `<live-pusher>`）：离开页面时（push：navigateTo 盖住它；
//! pop：navigateBack 关掉它）组件不停，画面缩成右下角一个浮窗，画在合成那一步、所有页面之上（与全屏视频同一层次，
//! `compositor::cpu::compose_rows`），宿主那边的控件一直活着（`sameLayerViews` 里照报它，带 `pip: true`）。
//!
//! - 点小窗：push 进的小窗回到原来那页（`wx.navigateBack` 到它那一层），回到之后小窗收起、组件照常画在页面里；
//!   pop 进的小窗原页已经关了，点了不跳（官方没写这种情形回哪，待验）；
//! - 点右上角的 ×：关掉小窗（组件跟着放掉：推流停）。
//!
//! 状态是线程级的（一个线程一个引擎，同 `video::fullscreen`）。引擎胶水在 host/engine_pip.rs。
use crate::Rect;
use std::cell::RefCell;

/// 怎么进的小窗
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipDir {
    /// navigateTo 盖住了原页（原页还在栈里）
    Push,
    /// navigateBack 关掉了原页
    Pop,
}

#[derive(Debug, Clone)]
pub struct Pip {
    /// 同层组件的键（host_view::key）与标签、属性（照原样报给宿主，宿主的控件不重建）
    pub key: String,
    pub tag: String,
    pub attrs: std::collections::HashMap<String, String>,
    /// 原页里的组件 id（派 enter / leave 事件）
    pub component_id: String,
    pub dir: PipDir,
    /// 原页在栈里第几层（1 起）与路由：回到它就收起小窗
    pub origin_depth: usize,
    pub origin_route: String,
    /// 原来的框的宽高比（高 / 宽）
    pub aspect: f32,
    /// 小窗在窗口里的位置（逻辑 px，引擎每帧按窗口尺寸 / 安全区摆）
    pub rect: Rect,
    /// 上次合成时画的是宿主的第几帧（`host_view::seq_of`）：变了重新合成
    pub drawn_seq: Option<u32>,
}

thread_local! {
    static PIP: RefCell<Option<Pip>> = const { RefCell::new(None) };
    /// 按在小窗上的那一下（按下时记，抬起时在同一处才算）
    static PRESS: RefCell<Option<PipHit>> = const { RefCell::new(None) };
}

/// 点在小窗的哪儿
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipHit {
    Body,
    Close,
}

/// 关闭键的半径与离边的距离（逻辑 px）
const CLOSE_R: f32 = 10.0;
const CLOSE_INSET: f32 = 4.0;
/// 圆角（逻辑 px）
const RADIUS: f32 = 8.0;

pub fn enter(p: Pip) {
    PIP.with(|s| *s.borrow_mut() = Some(p));
}

/// 收起，返回收起的那个
pub fn exit() -> Option<Pip> {
    PRESS.with(|p| *p.borrow_mut() = None);
    PIP.with(|s| s.borrow_mut().take())
}

pub fn current() -> Option<Pip> {
    PIP.with(|s| s.borrow().clone())
}

pub fn visible() -> bool {
    PIP.with(|s| s.borrow().is_some())
}

pub fn with_mut<R>(f: impl FnOnce(&mut Pip) -> R) -> Option<R> {
    PIP.with(|s| s.borrow_mut().as_mut().map(f))
}

/// 小窗的位置：宽是窗口宽的 32%（90~160 px），高按原框比例（夹在 1/2~2 倍宽之间），贴右下角，离右边 12、离底 `bottom_inset + 12`
pub fn place(win_w: f32, win_h: f32, bottom_inset: f32, aspect: f32) -> Rect {
    let w = (win_w * 0.32).clamp(90.0, 160.0);
    let h = (w * aspect).clamp(w * 0.5, w * 2.0);
    Rect::new(win_w - w - 12.0, (win_h - bottom_inset - 12.0 - h).max(0.0), w, h)
}

fn hit_rect(r: &Rect, x: f32, y: f32) -> Option<PipHit> {
    if x < r.x || y < r.y || x > r.x + r.width || y > r.y + r.height {
        return None;
    }
    let (cx, cy) = (r.x + r.width - CLOSE_INSET - CLOSE_R, r.y + CLOSE_INSET + CLOSE_R);
    // 关闭键的命中圈比画出来的大一圈（手指粗）
    if (x - cx).powi(2) + (y - cy).powi(2) <= (CLOSE_R + 6.0).powi(2) {
        return Some(PipHit::Close);
    }
    Some(PipHit::Body)
}

/// 按下（窗口逻辑坐标）：按在小窗上返回 true（这串触摸归它，页面收不到）
pub fn press(x: f32, y: f32) -> bool {
    let Some(r) = PIP.with(|s| s.borrow().as_ref().map(|p| p.rect)) else { return false };
    let h = hit_rect(&r, x, y);
    PRESS.with(|p| *p.borrow_mut() = h);
    h.is_some()
}

/// 这串触摸是不是按在小窗上开始的（移动时吃掉）
pub fn pressing() -> bool {
    PRESS.with(|p| p.borrow().is_some())
}

/// 抬起：按下与抬起落在小窗的同一处才算点了它。返回点了什么（None = 不是这串触摸的事 / 移出去了）
pub fn release(x: f32, y: f32) -> Option<Option<PipHit>> {
    let down = PRESS.with(|p| p.borrow_mut().take())?;
    let r = PIP.with(|s| s.borrow().as_ref().map(|p| p.rect))?;
    Some((hit_rect(&r, x, y) == Some(down)).then_some(down))
}

/// 合成时写到的行（设备像素 [a, b)）
pub fn reach(sf: f32, h: u32) -> Option<(u32, u32)> {
    let r = PIP.with(|s| s.borrow().as_ref().map(|p| p.rect))?;
    let a = (r.y * sf).floor().max(0.0) as u32;
    let b = (((r.y + r.height) * sf).ceil() as u32 + 1).min(h);
    (a < b).then_some((a, b))
}

/// 把小窗画进输出缓冲（0x00RRGGBB，行宽 `w`）。没有小窗时什么也不做；宿主还没交帧时是黑底
pub fn draw(buf: &mut [u32], w: u32, h: u32, sf: f32) {
    let Some(p) = current() else { return };
    let frame = super::host_view::frame(&p.key);
    let seq = super::host_view::seq_of(&p.key);
    with_mut(|q| q.drawn_seq = seq);
    let (x0, y0) = ((p.rect.x * sf).round() as i64, (p.rect.y * sf).round() as i64);
    let (rw, rh) = ((p.rect.width * sf).round() as i64, (p.rect.height * sf).round() as i64);
    if rw <= 0 || rh <= 0 {
        return;
    }
    let rad = RADIUS * sf;
    let (cr, cx, cy) = (CLOSE_R * sf, (rw as f32) - (CLOSE_INSET + CLOSE_R) * sf, (CLOSE_INSET + CLOSE_R) * sf);
    for j in 0..rh {
        let y = y0 + j;
        if y < 0 || y >= h as i64 {
            continue;
        }
        for i in 0..rw {
            let x = x0 + i;
            if x < 0 || x >= w as i64 {
                continue;
            }
            // 圆角：角上按到圆心的距离给覆盖率（一像素宽的过渡）
            let cov = corner_cover(i as f32 + 0.5, j as f32 + 0.5, rw as f32, rh as f32, rad);
            if cov <= 0.0 {
                continue;
            }
            let mut px = match &frame {
                Some(f) if f.width > 0 && f.height > 0 => {
                    let sx = ((i as u64 * f.width as u64) / rw as u64).min(f.width as u64 - 1) as usize;
                    let sy = ((j as u64 * f.height as u64) / rh as u64).min(f.height as u64 - 1) as usize;
                    let k = (sy * f.width as usize + sx) * 4;
                    ((f.rgba[k] as u32) << 16) | ((f.rgba[k + 1] as u32) << 8) | f.rgba[k + 2] as u32
                }
                _ => 0x000000,
            };
            // 关闭键：半透明黑圆 + 白色 ×
            let (dx, dy) = (i as f32 + 0.5 - cx, j as f32 + 0.5 - cy);
            let d = (dx * dx + dy * dy).sqrt();
            if d < cr + 1.0 {
                let a = (cr + 1.0 - d).clamp(0.0, 1.0) * 0.55;
                px = mix(px, 0x000000, a);
                let arm = cr * 0.45;
                let on_x = (dx.abs() <= arm && dy.abs() <= arm) && ((dx - dy).abs() < 1.1 * sf || (dx + dy).abs() < 1.1 * sf);
                if on_x {
                    px = mix(px, 0xFFFFFF, 0.95);
                }
            }
            let at = (y as usize) * w as usize + x as usize;
            buf[at] = if cov >= 1.0 { px } else { mix(buf[at], px, cov) };
        }
    }
}

fn corner_cover(x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let cx = if x < r { r } else if x > w - r { w - r } else { return 1.0 };
    let cy = if y < r { r } else if y > h - r { h - r } else { return 1.0 };
    let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    (r - d + 0.5).clamp(0.0, 1.0)
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 0xFF) as f32, ((b >> s) & 0xFF) as f32);
        ((x + (y - x) * t).round() as u32).min(255) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// 清掉（换小程序 / 测试）
pub fn clear() {
    exit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 摆在右下角_点关闭与点画面分得开() {
        let r = place(375.0, 667.0, 34.0, 16.0 / 9.0);
        assert_eq!(r.width, 120.0);
        assert!((r.height - 120.0 * 16.0 / 9.0).abs() < 0.01);
        assert_eq!(r.x + r.width, 363.0, "离右边 12");
        assert!((r.y + r.height - (667.0 - 34.0 - 12.0)).abs() < 0.01, "离底安全区 12");
        enter(Pip {
            key: "k".into(),
            tag: "live-pusher".into(),
            attrs: Default::default(),
            component_id: "k".into(),
            dir: PipDir::Push,
            origin_depth: 1,
            origin_route: "pages/a/a".into(),
            aspect: 16.0 / 9.0,
            rect: r,
            drawn_seq: None,
        });
        assert!(!press(10.0, 10.0), "小窗外");
        assert!(press(r.x + 20.0, r.y + r.height - 20.0));
        assert_eq!(release(r.x + 22.0, r.y + r.height - 20.0), Some(Some(PipHit::Body)));
        assert!(press(r.x + r.width - 14.0, r.y + 14.0));
        assert_eq!(release(r.x + r.width - 14.0, r.y + 14.0), Some(Some(PipHit::Close)));
        assert!(press(r.x + 20.0, r.y + 40.0));
        assert_eq!(release(5.0, 5.0), Some(None), "移出去了：不算点");
        assert_eq!(release(5.0, 5.0), None, "没按在小窗上");
        // 黑底 + 圆角：角上那个像素不写
        let (w, h) = (375u32, 667u32);
        let mut buf = vec![0x00FF00u32; (w * h) as usize];
        draw(&mut buf, w, h, 1.0);
        let at = |x: f32, y: f32| buf[y as usize * w as usize + x as usize];
        assert_eq!(at(r.x + 30.0, r.y + 60.0), 0x000000, "还没来帧：黑底");
        assert_eq!(at(r.x, r.y), 0x00FF00, "圆角外露出底下");
        assert_eq!(reach(1.0, h).map(|(a, _)| a), Some(r.y.floor() as u32));
        clear();
        assert!(!visible());
    }
}
