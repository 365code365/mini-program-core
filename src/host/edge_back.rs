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
fn dim(color: u32, factor: f32) -> u32 {
    let f = factor.clamp(0.0, 1.0);
    let r = (((color >> 16) & 0xFF) as f32 * f) as u32;
    let g = (((color >> 8) & 0xFF) as f32 * f) as u32;
    let b = ((color & 0xFF) as f32 * f) as u32;
    (r << 16) | (g << 8) | b
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
    let dim_factor = 1.0 - 0.22 * (1.0 - progress.clamp(0.0, 1.0));
    let shadow = (SHADOW_WIDTH as usize).min(off);
    for y in 0..height as usize {
        let row_start = y * w;
        if row_start + w > buffer.len() {
            break;
        }
        let row = &mut buffer[row_start..row_start + w];
        // 当前页右移（从右往左搬，copy_within 内部是 memmove，重叠也安全）
        row.copy_within(0..w - off, off);
        // 左侧露出的一条：上一页
        for x in 0..off {
            let c = match shot {
                Some(s) => {
                    let sx = (x + par).min(w - 1);
                    s.pixels[row_start + sx]
                }
                None => 0x00_00_00,
            };
            row[x] = dim(c, dim_factor);
        }
        // 交界处的投影：当前页的左边缘压在上一页上
        for i in 0..shadow {
            let x = off - 1 - i;
            let t = 1.0 - i as f32 / shadow as f32; // 贴着边缘最深
            row[x] = dim(row[x], 1.0 - 0.45 * t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        PageShot { width, height, pixels: vec![color; (width * height) as usize] }
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
