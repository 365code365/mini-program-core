//! 图层画布：让「这一帧改写了哪块像素」由类型记录，而不是在宿主各处手动打标记。
//!
//! 合成后端（以后的 GPU 合成器）只镜像 CPU 画布、不画内容。它要做到「只传变化的
//! 部分」又不出错，前提是**任何一次写入都留了记录**。所以宿主手里的画布字段换成这两个
//! 类型：拿可变引用只能经过 `write*()`，忘了声明在编译期就不成立。
//!
//! - [`LayerCanvas`]（tabBar、fixed 覆盖层）：拿一次可变引用就换一个代数，读者看到代数
//!   变了就整层重传。这两层每次都是「整层清空再画」，按层记账就够了。
//! - [`PageCanvas`]（页面画布，可上万行高）：写入必须声明条带或矩形，留一份**只读可查**
//!   的写入日志。不是「取走即清空」的队列：合成层只拿 `&PageCanvas`，读者自己记住看到
//!   哪儿了（游标 `(gen, seq)`），不需要可变借用、也不需要内部可变性。
//!
//! CPU 合成器不读这些记账，所以换上这两个类型之后 CPU 路径的像素逐字节不变。
use crate::{pixel_rect, Canvas, Color, IRect, Rect};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

/// 进程级的代数发号器。每次新建、`replace`、`LayerCanvas::write` 都取一个新值：
/// 读者用「代数相等」判断「还是我上次看到的那块画布」，各画布各自从 0 数的话，
/// 重新分配出来的新画布可能恰好撞上旧代数，读者就会把旧镜像当成新内容。
static NEXT_GEN: AtomicU64 = AtomicU64::new(1);

fn next_gen() -> u64 {
    // 只要求唯一，不与其它内存操作排序
    NEXT_GEN.fetch_add(1, Ordering::Relaxed)
}

/// 写入日志最多留这么多条。CPU 模式下没人读，也不会无限涨；读者落后超过这么多次写入
/// （比如中间有一段没合成），[`PageCanvas::ops_since`] 返回 `None`，读者整体作废重来。
pub const PAGE_LOG_CAP: usize = 32;

/// 整层记账的画布（tabBar、fixed 覆盖层）
pub struct LayerCanvas {
    canvas: Canvas,
    gen: u64,
}

impl LayerCanvas {
    pub fn new(canvas: Canvas) -> Self {
        Self { canvas, gen: next_gen() }
    }

    pub fn read(&self) -> &Canvas {
        &self.canvas
    }

    /// 把全是 0 的那几行（`[0, y0)` 与 `[y1, 高)`，调用方保证每个字节都是 0）的页还给系统。
    /// 内容不变（还回去的页再读还是 0），所以不换代数、GPU 不用重传
    pub fn release_zero_rows_outside(&mut self, y0: u32, y1: u32) -> usize {
        let h = self.canvas.height();
        self.canvas.release_rows(0, y0) + self.canvas.release_rows(y1, h)
    }

    /// 拿可变引用即视为整层变了（不管实际画了多少）
    pub fn write(&mut self) -> &mut Canvas {
        self.gen = next_gen();
        &mut self.canvas
    }

    pub fn gen(&self) -> u64 {
        self.gen
    }
}

/// 页面画布的一次写入声明
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirtyOp {
    /// 整宽条带 `[y0, y1)`（画布行，未夹紧，读者自己夹到画布内）：清条带 + 整帧重画
    Band(i32, i32),
    /// 损伤区（已按 [`pixel_rect`] 取整并夹到画布内）：清矩形 + 带裁剪的局部重画
    Rect(IRect),
}

/// 写入要声明范围、并留写入日志的页面画布
pub struct PageCanvas {
    canvas: Canvas,
    /// 新建 / 重新分配时换新值
    gen: u64,
    /// 每次写入 +1（跨 `replace` 继续往上数）
    seq: u64,
    /// 最近 [`PAGE_LOG_CAP`] 次写入，`(这次写入后的 seq, 写了哪块)`，seq 连续、末尾是 `self.seq`
    log: VecDeque<(u64, DirtyOp)>,
}

impl PageCanvas {
    pub fn new(canvas: Canvas) -> Self {
        Self { canvas, gen: next_gen(), seq: 0, log: VecDeque::with_capacity(PAGE_LOG_CAP) }
    }

    pub fn read(&self) -> &Canvas {
        &self.canvas
    }

    /// 只保留 `[y0, y1)` 这几行，其余的页还给系统（`Canvas::release_rows`）。**不**记写入日志：
    /// 条带以外的行内容本来就不算数（读之前一定先清再画，engine_frame.rs），GPU 镜像不用重传
    pub fn release_outside(&mut self, y0: u32, y1: u32) -> usize {
        let h = self.canvas.height();
        self.canvas.release_rows(0, y0) + self.canvas.release_rows(y1, h)
    }

    /// 声明要写整宽条带 `[y0, y1)`，返回可变画布
    pub fn write_band(&mut self, y0: i32, y1: i32) -> &mut Canvas {
        self.record(DirtyOp::Band(y0, y1));
        &mut self.canvas
    }

    /// 声明要写矩形 `r`（画布像素坐标），返回可变画布。
    ///
    /// 取整用 [`pixel_rect`]，与 `Canvas::clear_area` 清掉的范围是同一个函数算的。
    pub fn write_rect(&mut self, r: Rect) -> &mut Canvas {
        let ir = pixel_rect(r, self.canvas.width(), self.canvas.height());
        self.record(DirtyOp::Rect(ir));
        &mut self.canvas
    }

    /// 换一块新画布（高度变了要重新分配）：换代数、清空日志。旧游标从此拿到 `None`。
    pub fn replace(&mut self, canvas: Canvas) {
        self.canvas = canvas;
        self.gen = next_gen();
        self.log.clear();
    }

    pub fn gen(&self) -> u64 {
        self.gen
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// 读者的游标：记下它，下次用 [`ops_since`](Self::ops_since) 问「之后写了哪些」
    pub fn cursor(&self) -> (u64, u64) {
        (self.gen, self.seq)
    }

    /// 游标 `seen = (gen, seq)` 之后的写入，按写入顺序。
    ///
    /// 返回 `None`（调用方按「全部失效」处理）的情形：代数不同（画布换过了），
    /// 或日志已经丢掉了其中一部分（落后超过 [`PAGE_LOG_CAP`] 次写入），
    /// 或游标比现在还新（不是这块画布给出的游标）。
    pub fn ops_since(&self, seen: (u64, u64)) -> Option<impl Iterator<Item = &DirtyOp> + '_> {
        let (gen, seen_seq) = seen;
        if gen != self.gen || seen_seq > self.seq {
            return None;
        }
        let behind = self.seq - seen_seq;
        let held = self.log.len() as u64;
        if behind > held {
            return None;
        }
        Some(self.log.iter().skip((held - behind) as usize).map(|(_, op)| op))
    }

    fn record(&mut self, op: DirtyOp) {
        self.seq += 1;
        if self.log.len() == PAGE_LOG_CAP {
            self.log.pop_front();
        }
        self.log.push_back((self.seq, op));
    }
}

// `color_bytes` 的前提：`Color` 恰好是 4 个字节、没有对齐要求（`#[repr(C)]` 的 4×u8）
const _: () = assert!(std::mem::size_of::<Color>() == 4 && std::mem::align_of::<Color>() == 1);

/// 像素按 `R,G,B,A` 字节原样看（零拷贝），上传纹理时直接当源数据用
pub fn color_bytes(px: &[Color]) -> &[u8] {
    // SAFETY: Color 是 #[repr(C)] 的 4 个 u8，尺寸 4、对齐 1（上面的 const 断言守着），
    // 没有填充字节；u8 接受任何位模式；长度按 4 倍换算不会越界，生命周期跟随 `px`。
    unsafe { std::slice::from_raw_parts(px.as_ptr().cast::<u8>(), px.len() * 4) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ops(p: &PageCanvas, seen: (u64, u64)) -> Option<Vec<DirtyOp>> {
        p.ops_since(seen).map(|it| it.copied().collect())
    }

    #[test]
    fn layer_write_bumps_gen_read_does_not() {
        let mut l = LayerCanvas::new(Canvas::new(4, 4));
        let g0 = l.gen();
        let _ = l.read();
        assert_eq!(l.gen(), g0);
        l.write().clear(Color::WHITE);
        let g1 = l.gen();
        assert_ne!(g1, g0);
        l.write();
        assert_ne!(l.gen(), g1);
    }

    #[test]
    fn page_records_band_raw_and_rect_through_pixel_rect() {
        let mut p = PageCanvas::new(Canvas::new(10, 10));
        let c0 = p.cursor();
        p.write_band(-5, 30);
        p.write_rect(Rect::new(1.5, 2.2, 3.0, 100.0));
        assert_eq!(
            ops(&p, c0).unwrap(),
            vec![
                DirtyOp::Band(-5, 30),
                DirtyOp::Rect(pixel_rect(Rect::new(1.5, 2.2, 3.0, 100.0), 10, 10)),
            ]
        );
        assert_eq!(p.seq(), 2);
        // 读到最新：之后没有写入
        assert_eq!(ops(&p, p.cursor()).unwrap(), vec![]);
    }

    #[test]
    fn log_keeps_last_32_and_too_far_behind_is_none() {
        let mut p = PageCanvas::new(Canvas::new(4, 4));
        let start = p.cursor();
        for i in 0..40 {
            p.write_band(i, i + 1);
        }
        assert_eq!(p.log.len(), PAGE_LOG_CAP);
        // 落后 40 次 > 32：日志已经丢了一部分
        assert!(ops(&p, start).is_none());
        // 落后恰好 32 次：还全在
        let got = ops(&p, (p.gen(), 8)).unwrap();
        assert_eq!(got.len(), 32);
        assert_eq!(got[0], DirtyOp::Band(8, 9));
        assert_eq!(got[31], DirtyOp::Band(39, 40));
        // 落后 33 次：缺第 8 次
        assert!(ops(&p, (p.gen(), 7)).is_none());
        // 比现在还新的游标不是这块画布给的
        assert!(ops(&p, (p.gen(), p.seq() + 1)).is_none());
    }

    #[test]
    fn replace_changes_gen_clears_log_and_old_cursor_gets_none() {
        let mut p = PageCanvas::new(Canvas::new(4, 4));
        p.write_band(0, 2);
        let old = p.cursor();
        let g0 = p.gen();
        p.replace(Canvas::new(4, 8));
        assert_ne!(p.gen(), g0);
        assert!(p.log.is_empty());
        assert_eq!(p.read().height(), 8);
        // 旧游标（旧代数）：None —— 即使 seq 看起来是最新的
        assert!(ops(&p, old).is_none());
        let fresh = p.cursor();
        assert_eq!(ops(&p, fresh).unwrap(), vec![]);
        p.write_band(1, 3);
        assert_eq!(ops(&p, fresh).unwrap(), vec![DirtyOp::Band(1, 3)]);
    }

    #[test]
    fn two_canvases_never_share_a_gen() {
        let a = PageCanvas::new(Canvas::new(1, 1));
        let b = PageCanvas::new(Canvas::new(1, 1));
        let l = LayerCanvas::new(Canvas::new(1, 1));
        assert_ne!(a.gen(), b.gen());
        assert_ne!(a.gen(), l.gen());
        assert_ne!(b.gen(), l.gen());
        // 另一块画布的游标拿到这块画布上用：代数对不上
        assert!(b.ops_since(a.cursor()).is_none());
    }

    #[test]
    fn color_bytes_is_rgba_in_order() {
        let px = [Color::new(1, 2, 3, 4), Color::new(5, 6, 7, 8)];
        let b = color_bytes(&px);
        assert_eq!(b.len(), 8);
        assert_eq!(b, &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(color_bytes(&[]).is_empty());
    }
}
