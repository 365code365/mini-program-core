//! 左边缘侧滑返回：从屏幕最左侧往右拖，跟手把当前页推出去，松手按距离/速度判定返回。
//!
//! 这是原生宿主（iOS 的 `UINavigationController` / 微信客户端）提供的手势，
//! 页面自己写不出来 —— 引擎里缺了它，用户在二级页只能找页面上的返回按钮，
//! 而真机上每个人都是随手往右一划。三件事必须都对：
//!
//! - **跟手**：位移 1:1 跟着手指，上一页在下面按视差露出来，交界处有阴影；
//! - **可取消**：划了一半松手且不够阈值，要滑回去，页面状态一点不变；
//! - **不该触发的地方不触发**：栈底（tab 首页）、弹窗/picker 弹着的时候都没有这个手势。
//!
//! 判定与动画是纯逻辑（[`EdgeBack`]），合成是一段纯像素搬运（[`compose`]），
//! 都能离开窗体单测。

/// 触发区宽度（逻辑像素）：只有从屏幕最左这一条按下才算侧滑返回。
/// 取 20 与 iOS 的边缘手势识别区同量级 —— 再宽会吃掉页面左侧内容的横向滑动。
pub const EDGE_WIDTH: f32 = 20.0;
/// 松手判定「返回」的位移比例（占屏宽）
pub const COMMIT_RATIO: f32 = 0.35;
/// 松手判定「返回」的速度阈值（逻辑像素/秒）：快速甩一下即使没到 35% 也返回
pub const COMMIT_VELOCITY: f32 = 320.0;
/// 速度判定的最小位移：手指几乎没动时的速度噪声不该被当成「甩」
pub const COMMIT_MIN_OFFSET: f32 = 12.0;
/// 松手后的收尾动画时长（秒）
pub const SETTLE_SECS: f32 = 0.22;
/// 交界处阴影宽度（**上屏像素**，`compose` 直接在上屏缓冲上做，不换算缩放）
pub const SHADOW_WIDTH: f32 = 14.0;

/// 一页已经合成好的视口像素（0xRRGGBB，行优先）。
/// 侧滑时下面那一页要显示出来，而它已经不是当前页了，只能靠离开时留下的这张图。
pub struct PageShot {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
    /// 这张图的编号（进程内唯一，[`PageShot::new`] 发）。GPU 合成器按它判断「这张图传过没有」：
    /// 一次转场里同一张图每帧都要用，整张 10MB 只该传一次；按地址判会把新图认成旧图
    /// （上一张释放后，新图可能正好分配在同一个地址）
    pub id: u64,
}

impl PageShot {
    pub fn new(width: u32, height: u32, pixels: Vec<u32>) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self { width, height, pixels, id: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    }

    /// 空图（截不到画面时的占位）
    pub fn empty() -> Self {
        Self::new(0, 0, Vec::new())
    }
}

/// 松手后的收尾动画
#[derive(Debug, Clone, Copy, PartialEq)]
struct Settle {
    from: f32,
    to: f32,
    /// 已经过去的时间（秒）
    t: f32,
    commit: bool,
}

/// 一次侧滑返回手势的状态（纯逻辑）
#[derive(Debug)]
pub struct EdgeBack {
    width: f32,
    offset: f32,
    start_x: f32,
    last_x: f32,
    last_ts: u64,
    /// 水平速度（逻辑像素/秒，向右为正）
    velocity: f32,
    settle: Option<Settle>,
}

impl EdgeBack {
    /// 起点是否落在左边缘触发区
    pub fn at_edge(x: f32) -> bool {
        x <= EDGE_WIDTH
    }

    pub fn new(start_x: f32, ts: u64, width: f32) -> Self {
        Self {
            width: width.max(1.0),
            offset: 0.0,
            start_x,
            last_x: start_x,
            last_ts: ts,
            velocity: 0.0,
            settle: None,
        }
    }

    /// 手指移动：位移 1:1 跟手（不做阻尼 —— 侧滑返回是「把页面推走」，不是拉橡皮筋）
    pub fn on_move(&mut self, x: f32, ts: u64) -> f32 {
        if self.settle.is_some() {
            return self.offset;
        }
        let dt = ts.saturating_sub(self.last_ts) as f32 / 1000.0;
        if dt > 0.0005 {
            let v = (x - self.last_x) / dt;
            // 低通：单帧采样抖动很大，直接拿它判「甩」会误触发
            self.velocity = v * 0.6 + self.velocity * 0.4;
            self.last_ts = ts;
        }
        self.last_x = x;
        self.offset = (x - self.start_x).clamp(0.0, self.width);
        self.offset
    }

    /// 松手时是否判定为「返回」
    pub fn should_commit(&self) -> bool {
        self.offset >= self.width * COMMIT_RATIO
            || (self.velocity >= COMMIT_VELOCITY && self.offset >= COMMIT_MIN_OFFSET)
    }

    /// 松手：进入收尾动画（推到底 = 返回，滑回去 = 取消）
    pub fn release(&mut self) {
        if self.settle.is_some() {
            return;
        }
        let commit = self.should_commit();
        self.settle = Some(Settle {
            from: self.offset,
            to: if commit { self.width } else { 0.0 },
            t: 0.0,
            commit,
        });
    }

    /// 推进收尾动画。返回 `Some(是否返回)` 表示动画刚刚结束，宿主该收手了。
    pub fn tick(&mut self, dt: f32) -> Option<bool> {
        let s = self.settle.as_mut()?;
        s.t += dt;
        if s.t >= SETTLE_SECS {
            self.offset = s.to;
            return Some(s.commit);
        }
        let p = s.t / SETTLE_SECS;
        // ease-out cubic：起手快、收尾软，和 iOS 的转场同一个观感
        let ease = 1.0 - (1.0 - p).powi(3);
        self.offset = s.from + (s.to - s.from) * ease;
        None
    }

    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// 0~1 的推出进度，合成时用来算视差与压暗
    pub fn progress(&self) -> f32 {
        (self.offset / self.width).clamp(0.0, 1.0)
    }

    pub fn is_settling(&self) -> bool {
        self.settle.is_some()
    }

    #[cfg(test)]
    fn velocity(&self) -> f32 {
        self.velocity
    }
}

/// 上一页的视差位移（逻辑/物理同单位）：进度 0 时躲在左边 1/3 屏外，进度 1 时归位。
/// 与 iOS 一致 —— 两页同向移动但下面那页慢一截，才有「层」的感觉。
fn parallax(width: f32, progress: f32) -> f32 {
    (1.0 - progress) * width / 3.0
}

/// 把 0xRRGGBB 按 `factor`（0~1，越小越暗）压暗
#[cfg(test)]
fn dim(color: u32, factor: f32) -> u32 {
    DimLut::new(factor).apply(color)
}

/// 一个压暗系数下 0..=255 每个分量的结果（与逐像素 `(c as f32 * f) as u32` **逐字节相同**）。
///
/// 转场与侧滑每帧要压暗大半屏（1080 宽的屏上一百多万像素），逐像素三次浮点乘法在 120Hz 的
/// 中端机上就占掉好几毫秒；系数一帧内是常数，查表只剩三次取下标。GPU 合成器的着色器按同一个
/// 浮点算式出图，CPU 这边结果一个字节都不能变（`compositor::gpu` 的逐字节对拍）
struct DimLut([u8; 256]);

impl DimLut {
    fn new(factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        let mut t = [0u8; 256];
        for (c, v) in t.iter_mut().enumerate() {
            *v = (c as f32 * f) as u32 as u8;
        }
        Self(t)
    }

    #[inline]
    fn apply(&self, color: u32) -> u32 {
        let t = &self.0;
        ((t[((color >> 16) & 0xFF) as usize] as u32) << 16)
            | ((t[((color >> 8) & 0xFF) as usize] as u32) << 8)
            | t[(color & 0xFF) as usize] as u32
    }
}

/// 交界阴影那几列各自的压暗表（贴着边缘最深）
fn shadow_luts(shadow: usize) -> Vec<DimLut> {
    (0..shadow).map(|i| DimLut::new(shadow_factor(i, shadow))).collect()
}

/// 离交界第 `i` 列（共 `shadow` 列）的压暗系数
fn shadow_factor(i: usize, shadow: usize) -> f32 {
    let t = 1.0 - i as f32 / shadow as f32;
    (1.0 - 0.45 * t).clamp(0.0, 1.0)
}

/// GPU 合成器复刻 [`compose`] / [`compose_shot_over`] 要的参数（与这两个函数**同一套算式**）：
/// `(off, par, dim, shadow, 各列阴影系数)`，系数都已夹到 0~1。着色器按 `u32(f32(c) * f)` 压暗，
/// 与 [`DimLut`] 的 `(c as f32 * f) as u32` 逐字节相同
pub fn gpu_params(width: u32, offset_px: u32, progress: f32) -> (u32, u32, f32, u32, [f32; 16]) {
    let off = offset_px.min(width);
    let par = parallax(width as f32, progress) as usize as u32;
    let dim = (1.0 - 0.22 * (1.0 - progress.clamp(0.0, 1.0))).clamp(0.0, 1.0);
    let shadow = (SHADOW_WIDTH as u32).min(off);
    let mut f = [1.0f32; 16];
    for (i, v) in f.iter_mut().enumerate().take(shadow as usize) {
        *v = shadow_factor(i, shadow as usize);
    }
    (off, par, dim, shadow, f)
}

/// 把已经合成好的这一帧按侧滑位移重排：当前页整体右移 `offset_px`，
/// 左边露出来的部分画上一页（视差 + 压暗），交界处补一道阴影。
///
/// 直接在上屏缓冲上原地搬运，不额外开一张全屏图 —— 侧滑是每帧都要做的事。
pub fn compose(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    offset_px: u32,
    prev: Option<&PageShot>,
    progress: f32,
) {
    let w = width as usize;
    let off = (offset_px as usize).min(w);
    if off == 0 || w == 0 {
        return;
    }
    // 上一页尺寸不一致（换了缩放因子）就不画它，只压暗 —— 错位比暗一点难看得多
    let shot = prev.filter(|s| s.width == width && s.height == height);
    let par = parallax(width as f32, progress) as usize;
    // 压暗随进度收回：刚开始推的时候下面那页最暗，推到底时恢复原色
    let lut = DimLut::new(1.0 - 0.22 * (1.0 - progress.clamp(0.0, 1.0)));
    let shadow = (SHADOW_WIDTH as usize).min(off);
    let shadow_lut = shadow_luts(shadow);
    for y in 0..height as usize {
        let row_start = y * w;
        if row_start + w > buffer.len() {
            break;
        }
        let row = &mut buffer[row_start..row_start + w];
        // 当前页右移（从右往左搬，copy_within 内部是 memmove，重叠也安全）
        row.copy_within(0..w - off, off);
        // 左侧露出的一条：上一页（视差：取它 [par, par + off) 那一段，越界的钉在最后一列）
        match shot {
            Some(s) => {
                let src = &s.pixels[row_start..row_start + w];
                for (x, d) in row[..off].iter_mut().enumerate() {
                    *d = lut.apply(src[(x + par).min(w - 1)]);
                }
            }
            None => row[..off].fill(lut.apply(0)),
        }
        // 交界处的投影：当前页的左边缘压在上一页上
        for (i, l) in shadow_lut.iter().enumerate() {
            let x = off - 1 - i;
            row[x] = l.apply(row[x]);
        }
    }
}

/// [`compose`] 的另一半：缓冲里是**下面**那一页（已经合成好的这一帧），`top` 是盖在上面、
/// 正在右移 `offset_px` 退场的那一页的图。返回转场用：上一页已经恢复成当前页、照常出帧，
/// 刚离开的那页只剩一张图。视差、压暗、阴影与 [`compose`] 同一套，两种转场看起来是同一个动作。
///
/// 没有图（尺寸不一致）时上面那一截填暗底 —— 与 [`compose`] 没有上一页时一致。
pub fn compose_shot_over(
    buffer: &mut [u32],
    width: u32,
    height: u32,
    offset_px: u32,
    top: Option<&PageShot>,
    progress: f32,
) {
    let w = width as usize;
    let off = (offset_px as usize).min(w);
    if w == 0 {
        return;
    }
    let shot = top.filter(|s| s.width == width && s.height == height);
    let par = parallax(width as f32, progress) as usize;
    let lut = DimLut::new(1.0 - 0.22 * (1.0 - progress.clamp(0.0, 1.0)));
    let shadow = (SHADOW_WIDTH as usize).min(off);
    let shadow_lut = shadow_luts(shadow);
    for y in 0..height as usize {
        let row_start = y * w;
        if row_start + w > buffer.len() {
            break;
        }
        let row = &mut buffer[row_start..row_start + w];
        // 左边露出的一条：下面那一页按视差左移、压暗。从左往右写，读的下标 x + par ≥ x
        // 还没被写过（par ≥ 0），所以能原地做
        for x in 0..off {
            row[x] = lut.apply(row[(x + par).min(w - 1)]);
        }
        // 右边：上面那一页的图整体右移 off（整段拷贝）
        match shot {
            Some(s) => row[off..].copy_from_slice(&s.pixels[row_start..row_start + w - off]),
            None => row[off..].fill(0x00_00_00),
        }
        for (i, l) in shadow_lut.iter().enumerate() {
            let x = off - 1 - i;
            row[x] = l.apply(row[x]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 返回转场：上面那页的图右移、左边是缓冲里的下一页（视差 + 压暗），与 compose 互为镜像
    #[test]
    fn compose_shot_over_slides_shot_right_over_live_page() {
        let (w, h) = (32u32, 2u32);
        let mut buf = vec![0x00FF00u32; (w * h) as usize];
        let top: Vec<u32> = (0..(w * h)).map(|i| 0x010000 * (i % w + 1)).collect();
        let shot = PageShot::new(w, h, top.clone());
        compose_shot_over(&mut buf, w, h, 20, Some(&shot), 1.0);
        assert_eq!(buf[20], top[0], "图右移 20 列");
        assert_eq!(buf[31], top[11]);
        assert_eq!(buf[0], 0x00FF00, "进度 1 时下面那页不压暗、不视差");
        assert!(buf[19] & 0xFF00 < 0xFF00, "交界处有阴影");
        // 位移 0：整帧就是图（转场第一帧）
        let mut buf = vec![0x00FF00u32; (w * h) as usize];
        compose_shot_over(&mut buf, w, h, 0, Some(&shot), 0.0);
        assert_eq!(buf, top);
    }

    #[test]
    fn only_left_edge_starts_the_gesture() {
        assert!(EdgeBack::at_edge(0.0));
        assert!(EdgeBack::at_edge(20.0));
        assert!(!EdgeBack::at_edge(21.0));
        assert!(!EdgeBack::at_edge(200.0));
    }

    /// 跟手：位移就是手指位移，且夹在 0~屏宽
    #[test]
    fn offset_follows_finger_and_clamps() {
        let mut g = EdgeBack::new(5.0, 0, 375.0);
        assert_eq!(g.on_move(55.0, 16), 50.0);
        assert_eq!(g.on_move(200.0, 32), 195.0);
        // 往回划回到起点之前也不该出现负位移
        assert_eq!(g.on_move(0.0, 48), 0.0);
        assert_eq!(g.on_move(999.0, 64), 375.0);
    }

    /// 过了 35% 屏宽就返回
    #[test]
    fn past_ratio_commits() {
        let mut g = EdgeBack::new(0.0, 0, 375.0);
        g.on_move(100.0, 300); // 26%，慢慢划
        assert!(!g.should_commit());
        g.on_move(140.0, 600); // 37%
        assert!(g.should_commit());
    }

    /// 快速甩一下：位移不够 35% 也返回
    #[test]
    fn fast_flick_commits_without_distance() {
        let mut g = EdgeBack::new(0.0, 0, 375.0);
        g.on_move(40.0, 16); // 40px / 16ms = 2500px/s
        g.on_move(70.0, 32);
        assert!(g.velocity() > COMMIT_VELOCITY);
        assert!(g.offset() < 375.0 * COMMIT_RATIO);
        assert!(g.should_commit());
    }

    /// 慢慢划一点点就松手：取消，且位移动画回到 0
    #[test]
    fn short_slow_drag_cancels_and_slides_back() {
        let mut g = EdgeBack::new(0.0, 0, 375.0);
        g.on_move(30.0, 500);
        assert!(!g.should_commit());
        g.release();
        assert!(g.is_settling());
        let mut guard = 0;
        loop {
            match g.tick(0.016) {
                Some(commit) => {
                    assert!(!commit);
                    break;
                }
                None => {
                    guard += 1;
                    assert!(guard < 200, "收尾动画没有结束");
                }
            }
        }
        assert_eq!(g.offset(), 0.0);
    }

    /// 判定返回时，收尾动画把页面推到整屏之外
    #[test]
    fn commit_settles_to_full_width() {
        let mut g = EdgeBack::new(0.0, 0, 375.0);
        g.on_move(200.0, 400);
        g.release();
        let mut last = g.offset();
        loop {
            let done = g.tick(0.016);
            assert!(g.offset() >= last - 0.001, "位移不该往回走");
            last = g.offset();
            if let Some(commit) = done {
                assert!(commit);
                break;
            }
        }
        assert_eq!(g.offset(), 375.0);
    }

    /// 松手后手指的移动不再影响位移（收尾动画期间手势已经交出去了）
    #[test]
    fn moves_after_release_are_ignored() {
        let mut g = EdgeBack::new(0.0, 0, 375.0);
        g.on_move(200.0, 400);
        g.release();
        let before = g.offset();
        assert_eq!(g.on_move(10.0, 500), before);
    }

    fn shot(width: u32, height: u32, color: u32) -> PageShot {
        PageShot::new(width, height, vec![color; (width * height) as usize])
    }

    /// 合成：当前页整体右移，左边是上一页
    #[test]
    fn compose_shifts_current_page_right() {
        let (w, h) = (32u32, 2u32);
        let mut buf: Vec<u32> = (0..(w * h)).map(|i| 0x010000 * (i % w + 1)).collect();
        let before = buf.clone();
        let prev = shot(w, h, 0x00FF00);
        compose(&mut buf, w, h, 20, Some(&prev), 1.0);
        // 右移 20 列：新的第 20 列应该是原来的第 0 列
        assert_eq!(buf[20], before[0]);
        assert_eq!(buf[(w + 20) as usize], before[w as usize]);
        // 左边露出的一条来自上一页（进度 1.0 时不压暗），阴影只占贴着交界的那几列
        assert_eq!(buf[0], 0x00FF00);
        // 交界前一列被投影压暗
        assert!(buf[19] & 0xFF00 < 0xFF00);
        assert_eq!(buf[19] & 0xFF00FF, 0);
    }

    /// 没有上一页快照时不越界、不 panic，露出的是暗底
    #[test]
    fn compose_without_shot_fills_dark() {
        let (w, h) = (6u32, 2u32);
        let mut buf = vec![0xFFFFFFu32; (w * h) as usize];
        compose(&mut buf, w, h, 2, None, 0.5);
        assert_eq!(buf[0], 0);
        assert_eq!(buf[1], 0);
        assert_eq!(buf[2], 0xFFFFFF);
    }

    /// 尺寸不一致的快照不参与合成（错位比压暗难看得多）
    #[test]
    fn compose_ignores_mismatched_shot() {
        let (w, h) = (6u32, 2u32);
        let mut buf = vec![0xFFFFFFu32; (w * h) as usize];
        let prev = shot(4, 4, 0x00FF00);
        compose(&mut buf, w, h, 2, Some(&prev), 1.0);
        assert_eq!(buf[0], 0);
    }

    /// 位移为 0 时一个像素都不该动
    #[test]
    fn compose_zero_offset_is_noop() {
        let (w, h) = (4u32, 2u32);
        let mut buf: Vec<u32> = (0..(w * h)).map(|i| i as u32).collect();
        let before = buf.clone();
        compose(&mut buf, w, h, 0, None, 0.0);
        assert_eq!(buf, before);
    }

    /// 视差：进度 0 时上一页在左边 1/3 屏外，进度 1 时归位
    #[test]
    fn parallax_moves_previous_page_into_place() {
        assert_eq!(parallax(375.0, 0.0), 125.0);
        assert_eq!(parallax(375.0, 1.0), 0.0);
    }
}
