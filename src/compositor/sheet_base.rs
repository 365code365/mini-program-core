//! CPU 合成器：只弹着操作菜单时，把拼好的底图（页面 → tabBar → fixed → 导航栏）留一份。
//!
//! 弹出 / 收起那 0.3s 里页面一个像素不动，变的只有遮罩深浅和面板位置。从前每一帧都把各层重新拼一遍、
//! 再原地整片压暗：一加 PLY110 的 CPU 后端上拼 1.6~2ms、压 1.7~2.4ms（CPU 刚从空闲里起来、还没升频的
//! 那几帧两样都翻两三倍），加上写进 Surface 的 4ms 就过了 90Hz 的 11.1ms。
//! 现在下一帧各层都没变（[`BaseKey`]：代数、写入序号、滚动位置、行区间都对得上）就不再拼，
//! 直接从这份底图边读边压暗写进输出缓冲（`action_sheet::render_over`），两遍变一遍。
//!
//! - 只缓存要底图的那几行（面板白底整行盖死的行不要，见 `cpu::sheet_cover_from`）。入场时这一截越来越短，
//!   拼过的行一直有效；收起时越来越长，只补拼新露出来的那几行；
//! - 哪一层变了（setData 之后的局部重画、导航栏换色……）整份作废重拼，与不缓存那一帧同样的开销；
//! - 菜单收起、或者这一帧还有别的覆盖层，[`release`] 放掉（整屏 10MB）。
//!
//! 逐字节：补拼的那几行先从输出缓冲拷进来再拼，与直接拼在输出缓冲上同一个初值，所以拼出来的就是
//! `compose_page` 在那几行上的结果；之后各层没变，它就没变。唯一的出入在「叠在缓冲原值上」的像素
//! （导航栏里不透明度不到 255 的像素）：直接拼时原值是上一帧压暗过的结果、这一帧又压一次（越压越黑，
//! 那是直接拼的毛病），这里一直是第一次的原值。引擎画的导航栏底色不透明，碰不到。
use super::{cpu, FrameLayers, Footprint, LayerCanvas};
use crate::host::ui_overlay::action_sheet;
use std::cell::RefCell;

/// 底图由哪些东西决定：任何一样变了，缓存就作废
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BaseKey {
    size: (u32, u32),
    /// 页面画布的 (代数, 写入序号)：重新分配换代数，任何一次写入序号 +1
    page: (u64, u64),
    scroll_px: i32,
    page_bg: u32,
    has_tabbar: bool,
    tabbar: Option<u64>,
    tabbar_px: u32,
    fixed: Option<u64>,
    fixed_rows: Option<(u32, u32)>,
    top: Option<(u32, u64)>,
}

impl BaseKey {
    fn of(l: &FrameLayers<'_>) -> Self {
        Self {
            size: (l.width, l.height),
            page: l.page.cursor(),
            scroll_px: l.scroll_px,
            page_bg: l.page_bg,
            has_tabbar: l.has_tabbar,
            tabbar: l.tabbar.map(LayerCanvas::gen),
            tabbar_px: l.tabbar_px,
            fixed: l.fixed.map(LayerCanvas::gen),
            fixed_rows: l.fixed_rows,
            top: l.top.as_ref().map(|t| (t.rows, t.canvas.gen())),
        }
    }
}

struct Base {
    key: BaseKey,
    /// [0, rows_end) 这几行是拼好的
    rows_end: u32,
    px: Vec<u32>,
}

thread_local! {
    static BASE: RefCell<Option<Base>> = const { RefCell::new(None) };
}

/// 只弹着操作菜单的一帧（`cpu::sheet_cover_from` 给出 `end`：这一行往下被面板整行盖死）：
/// 底图从缓存来（缺的行现拼），边读边压暗进 `buf`，再贴面板。返回与 `cpu::compose_rows(buf, layers, (0, end))` 同一个足迹
pub(super) fn compose(buf: &mut [u32], layers: &FrameLayers<'_>, end: u32) -> Footprint {
    let (w, h) = (layers.width, layers.height);
    let n = (w * h) as usize;
    let key = BaseKey::of(layers);
    BASE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let base = match slot.take() {
            Some(b) if b.key == key && b.px.len() == n => b,
            // 换了：画布留着接着用（尺寸对得上的话），行全部作废
            old => Base { key, rows_end: 0, px: old.map(|b| b.px).filter(|p| p.len() == n).unwrap_or_else(|| vec![0; n]) },
        };
        let base = slot.insert(base);
        #[cfg(test)]
        ROWS.with(|r| r.set((r.get().0 + end.saturating_sub(base.rows_end), r.get().1 + end)));
        if base.rows_end < end {
            let (a, b) = (base.rows_end as usize * w as usize, end as usize * w as usize);
            // 从输出缓冲接着拼：与直接拼在输出缓冲上同一个初值（见文件头「逐字节」）
            base.px[a..b].copy_from_slice(&buf[a..b]);
            cpu::compose_page(&mut base.px, layers, (base.rows_end, end));
            base.rows_end = end;
        }
        let o = &layers.overlays;
        action_sheet::render_over(buf, &base.px, w, h, layers.scale, o.text, (0, end))
    })
}

/// 这一帧不走缓存（菜单收起了、还有别的覆盖层）、或者菜单收掉了（`action_sheet::clear`）：放掉底图
pub(crate) fn release() {
    BASE.with(|cell| {
        if cell.borrow().is_some() {
            *cell.borrow_mut() = None;
        }
    });
}

#[cfg(test)]
thread_local! {
    /// 测试用：(补拼了多少行, 一共要多少行底图)
    static ROWS: std::cell::Cell<(u32, u32)> = const { std::cell::Cell::new((0, 0)) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::{OverlayRefs, PageCanvas, TopBand};
    use crate::{Canvas, Color, Rect};
    use std::time::{Duration, Instant};

    fn noise(c: &mut Canvas, seed: u32, alpha: impl Fn(usize) -> u8) {
        for (i, p) in c.pixels_mut().iter_mut().enumerate() {
            let v = (i as u32 ^ seed).wrapping_mul(2_654_435_761);
            *p = Color::new((v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8, alpha(i));
        }
    }

    /// 像 SDK 那样一帧接一帧合成进同一块缓冲：入场、半路上页面局部重画（setData）、收起，每一帧都与
    /// 「每帧整帧重拼」逐字节相同；而且确实有帧走了缓存
    #[test]
    fn 底图缓存逐帧与整帧重拼逐字节相同() {
        let (w, h, sf) = (120u32, 640u32, 2.0f32);
        let mut page = Canvas::new(w, 1500);
        noise(&mut page, 0x51, |_| 255);
        let mut tab = Canvas::new(w, 50);
        noise(&mut tab, 0x7A, |_| 255);
        let mut fixed = Canvas::new(w, h);
        noise(&mut fixed, 0x3C, |i| [0, 255, 128][i % 3]);
        // 导航栏不透明（半透明像素叠在缓冲原值上，见文件头「逐字节」）
        let mut band = Canvas::new(w, 64);
        noise(&mut band, 0x9E, |_| 255);
        let (mut page, tab, fixed, band) = (PageCanvas::new(page), LayerCanvas::new(tab), LayerCanvas::new(fixed), LayerCanvas::new(band));
        fn frame<'a>(page: &'a PageCanvas, tab: &'a LayerCanvas, fixed: &'a LayerCanvas, band: &'a LayerCanvas, (w, h, sf): (u32, u32, f32)) -> FrameLayers<'a> {
            FrameLayers {
                width: w,
                height: h,
                scale: sf,
                page,
                scroll_px: 300,
                page_bg: 0x12_34_56,
                has_tabbar: true,
                tabbar: Some(tab),
                tabbar_px: 50,
                fixed: Some(fixed),
                fixed_rows: Some((100, 560)), content_shift: 0,
                pull: None,
                overlays: OverlayRefs::none(),
                edge_back: None,
                top: Some(TopBand { rows: 60, canvas: band }),
            }
        }
        let mut direct = vec![0x00AB_CDEFu32; (w * h) as usize];
        let mut cached = direct.clone();
        ROWS.with(|r| r.set((0, 0)));
        // 入场 30 帧（第 12 帧页面变了），收起 30 帧；帧时钟按 90Hz 走
        let mut t0 = Instant::now();
        action_sheet::open_at(0, false);
        for k in 0..60u64 {
            if k == 12 {
                // 弹着的时候 setData 局部重画了一块（换写入序号：缓存要作废）
                let c = page.write_rect(Rect::new(10.0, 420.0, 60.0, 80.0));
                for p in &mut c.pixels_mut()[420 * w as usize..500 * w as usize] {
                    p.r = p.r.wrapping_add(77);
                }
            }
            if k == 30 {
                t0 = Instant::now();
                action_sheet::open_at(0, true);
            }
            crate::frame_clock::set(Some(t0 + Duration::from_millis((k % 30) * 11)));
            let l = frame(&page, &tab, &fixed, &band, (w, h, sf));
            cpu::compose(&mut cached, &l);
            cpu::compose_rows(&mut direct, &l, (0, h));
            let bad = cached.iter().zip(&direct).position(|(a, b)| a != b);
            assert!(bad.is_none(), "第 {k} 帧：走缓存与整帧重拼不同 @{:?}", bad.map(|i| (i as u32 % w, i as u32 / w)));
        }
        crate::frame_clock::set(None);
        let (filled, needed) = ROWS.with(|r| r.get());
        assert!(needed > 20 * h / 2 && filled * 3 < needed, "要底图 {needed} 行，补拼了 {filled} 行：缓存没起作用");
        action_sheet::clear();
        assert!(BASE.with(|b| b.borrow().is_none()), "菜单收掉了，底图应当放掉");
    }
}
