//! 页面转场：`navigateTo` 新页从右边滑进来盖住旧页，返回时当前页向右滑出、露出上一页。
//!
//! 微信（Skyline 默认路由同样如此）的 navigateTo / navigateBack 都有这一下，整页（导航栏 +
//! 内容）一起动；引擎从前「一换就到」，真机上和微信并排一看就不对。宿主打开
//! `routeAnimation` 选项才有（无头测试与老宿主照旧一换就到）。
//!
//! ## 怎么画
//! 与侧滑返回是同一个动作（`edge_back::compose` 的视差、压暗、阴影），只是位移由时钟驱动：
//! - 进入：缓冲里是**新页**（照常出帧，onLoad 里的 setData、动画都是活的），被盖住的旧页
//!   用它离开时留下的图（`back_shots` 最后一张）垫在下面；
//! - 返回：先留一张当前画面（要离开的那页），再照常退栈 —— 缓冲里是**恢复出来的上一页**，
//!   离开的那页只剩那张图，盖在上面往右滑走（`edge_back::compose_shot_over`）。
//!
//! 页面状态在转场开始那一刻就已经切好（与微信一致：生命周期先派发、动画在渲染层跑），
//! 动画只管合成。
//!
//! ## 时钟：每帧最多走一个半周期，起步那一步最多一个周期
//! 换页那一帧要整页建树、排版、画、整张上传（一加 PLY110 上 20~22ms，两个 vsync），从前的时钟起点
//! 是这一帧的 vsync 时刻：第二帧一来已经过了 22~33ms，减速曲线起手又最快 —— 新页一下子冲进来
//! 两成屏宽（220px），之后才每帧 8%。这就是「点进页面一顿」。下一帧还常常被上一帧那 20MB 上传
//! 拖住（提交 12ms），再跳一次。
//!
//! 现在按「出了几帧」推进：头两帧都是 t=0（新页还在屏幕外 / 离开的那页还盖满屏，看起来就是原画面），
//! 从第三帧起每帧走实际流逝的时间，但最多 1.5 个刷新周期。第二帧不走：它与第一帧之间隔的是建页的时间
//! （屏上什么都没动），按周期估计补一步也不行 —— 页面静止时 ColorOS 把屏降到 60Hz，估计还是 17ms，
//! 实测起步那一步仍是后面的 1.55 倍。代价是晚一帧起步（11ms），看不出来。
//! 起步后的头 [`STARTUP_STEPS`] 步再收紧到一个周期：建页那一帧传了 20MB（新页 + 垫底的旧页留图），
//! 紧跟着的那次提交常被拖住 8~13ms，下一帧晚到一个周期 —— 按 1.5 倍放行的话起步那一步又是 1.7~1.95 倍
//! （一加真机 4 次转场里有 1 次）。
//! 偶尔一帧慢了（新页 onReady 里 setData 整页重画），动画慢一点走完，不往前跳。
//! 周期取引擎的 pump 间隔估计（`engine_resample.rs`），不认识时按 120Hz。
//!
//! 转场期间不接受触摸（微信同样在转场中忽略点击）：按下就整串吞掉。
use crate::compositor::EdgeBackFrame;
use crate::host::edge_back::PageShot;

/// 转场时长（毫秒）。Skyline 自定义路由的 `transitionDuration` 缺省也是 300
pub const DURATION_MS: f32 = 300.0;

enum Kind {
    /// 新页从右边滑进来（垫在下面的是 `back_shots` 最后一张）
    Push,
    /// 离开的那页（这张图）向右滑走
    Pop(PageShot),
}

struct Anim {
    kind: Kind,
    clock: StepClock,
    /// 0~1
    t: f32,
}

/// 起步后按一个周期封顶的步数（见文件头「时钟」）
const STARTUP_STEPS: u32 = 3;

/// 按帧推进的动画钟（见文件头「时钟」）
#[derive(Default)]
pub(crate) struct StepClock {
    /// 上一帧的宿主时刻；None = 还没出过第一帧
    last_ms: Option<u64>,
    /// 动画已走的毫秒
    elapsed: f32,
    steps: u32,
    /// 最近一步的实际间隔与封顶（诊断用）
    last_dt: f32,
    last_cap: f32,
}

impl StepClock {
    /// 推进到这一帧，返回动画已走的毫秒。`period_ms` 是刷新周期估计
    pub(crate) fn advance(&mut self, now_ms: u64, period_ms: f32) -> f32 {
        let Some(prev) = self.last_ms.replace(now_ms) else { return 0.0 };
        let dt = now_ms.saturating_sub(prev) as f32;
        let cap = match self.steps {
            0 => 0.0,
            n if n <= STARTUP_STEPS => period_ms,
            _ => period_ms * 1.5,
        };
        self.elapsed += dt.min(cap);
        self.steps += 1;
        (self.last_dt, self.last_cap) = (dt, cap);
        self.elapsed
    }
}

/// 转场状态（`MiniEngine::route`）
#[derive(Default)]
pub struct RouteState {
    anim: Option<Anim>,
    /// 转场中按下的这串触摸：整串吞掉，直到抬起 / 取消
    swallow: bool,
}

/// 推进一步的结果
#[derive(Debug, PartialEq, Eq)]
pub enum Tick {
    /// 没有转场
    Idle,
    /// 还在动（要重新合成、要续帧）
    Running,
    /// 刚走完（这一帧按「没有转场」合成）
    Done,
}

/// 减速曲线：起手快、收尾软（与侧滑返回的收尾同一条）
fn ease(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

impl RouteState {
    pub fn active(&self) -> bool {
        self.anim.is_some()
    }

    /// 转场进度 0~1、钟走了几步、这一步的实际间隔与封顶（没有转场给 None；`MINI_DAMAGE_LOG` 用）
    pub fn progress(&self) -> Option<(f32, u32, f32, f32)> {
        self.anim.as_ref().map(|a| (a.t, a.clock.steps, a.clock.last_dt, a.clock.last_cap))
    }

    /// 新页刚入栈
    pub fn start_push(&mut self) {
        self.anim = Some(Anim { kind: Kind::Push, clock: StepClock::default(), t: 0.0 });
    }

    /// 刚退栈。`leaving` 是退栈之前那一帧的画面（要离开的那页）
    pub fn start_pop(&mut self, leaving: PageShot) {
        self.anim = Some(Anim { kind: Kind::Pop(leaving), clock: StepClock::default(), t: 0.0 });
    }

    /// 立刻收手（又来了一次跳转、尺寸变了）
    pub fn finish(&mut self) {
        self.anim = None;
    }

    /// 按宿主时钟推进（`period_ms`：刷新周期估计，见文件头「时钟」）
    pub fn tick(&mut self, now_ms: u64, period_ms: f32) -> Tick {
        let Some(a) = self.anim.as_mut() else { return Tick::Idle };
        a.t = (a.clock.advance(now_ms, period_ms) / DURATION_MS).min(1.0);
        if a.t >= 1.0 {
            self.anim = None;
            return Tick::Done;
        }
        Tick::Running
    }

    /// 这一帧的平移参数（没有转场给 None）。`width_px` 是画面宽（设备像素）
    pub fn frame<'a>(&'a self, back_shots: &'a [PageShot], width_px: u32) -> Option<EdgeBackFrame<'a>> {
        let a = self.anim.as_ref()?;
        let e = ease(a.t);
        // 上面那一页被推开的比例：进入从 1 走到 0，返回从 0 走到 1
        let (frac, shot, shot_on_top) = match &a.kind {
            Kind::Push => (1.0 - e, back_shots.last().filter(|s| !s.pixels.is_empty()), false),
            Kind::Pop(s) => (e, Some(s).filter(|s| !s.pixels.is_empty()), true),
        };
        Some(EdgeBackFrame {
            offset_px: (frac * width_px as f32).round() as u32,
            shot,
            progress: frac,
            shot_on_top,
        })
    }

    /// 这一帧**缓冲里那一页**（活的那页）被挪到了哪：`(水平位移, 可见区右沿)`，逻辑像素，
    /// `width` 是画面逻辑宽。宿主承载的原生组件不在画面里，要跟着这一页挪（`nativeViews`）。
    /// 进入：新页右移、整屏可见；返回：活的是下面那页，按视差左移、右边被离开的那页盖住
    pub fn live_shift(&self, width: f32) -> Option<(f32, f32)> {
        let a = self.anim.as_ref()?;
        let e = ease(a.t);
        Some(match a.kind {
            Kind::Push => ((1.0 - e) * width, width),
            Kind::Pop(_) => (-(1.0 - e) * width / 3.0, e * width),
        })
    }

    // ── 转场中的触摸：按下吞掉整串 ──

    /// 按下：转场中返回 true（这串触摸吞掉）
    pub fn swallow_down(&mut self) -> bool {
        self.swallow = self.active();
        self.swallow
    }

    /// 移动：这串被吞了返回 true
    pub fn swallowing(&self) -> bool {
        self.swallow
    }

    /// 抬起 / 取消：这串被吞了返回 true（并结束这串）
    pub fn swallow_end(&mut self) -> bool {
        std::mem::take(&mut self.swallow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(c: u32) -> PageShot {
        PageShot::new(4, 1, vec![c; 4])
    }

    #[test]
    fn 时钟从第一帧之后才走_300ms_走完() {
        let mut r = RouteState::default();
        r.start_push();
        assert_eq!(r.tick(1000, 10.0), Tick::Running, "第一帧：起点");
        let shots = [shot(1)];
        assert_eq!(r.frame(&shots, 400).unwrap().offset_px, 400, "第一帧新页整个在右边外面");
        let mut t = 1000;
        for _ in 0..15 {
            t += 10;
            assert_eq!(r.tick(t, 10.0), Tick::Running);
        }
        let mid = r.frame(&shots, 400).unwrap().offset_px;
        assert!(mid > 0 && mid < 100, "减速曲线：一半时间已走完大半（{mid}）");
        for _ in 0..14 {
            t += 10;
            assert_eq!(r.tick(t, 10.0), Tick::Running);
        }
        // 第二帧不走（建页那段不算），所以第一帧之后 310ms 走完
        assert_eq!(r.tick(t + 10, 10.0), Tick::Running);
        assert_eq!(r.tick(t + 20, 10.0), Tick::Done, "300ms 走完");
        assert!(r.frame(&shots, 400).is_none());
        assert_eq!(r.tick(t + 30, 10.0), Tick::Idle);
    }

    #[test]
    fn 返回是图在上_从零滑到整屏() {
        let mut r = RouteState::default();
        r.start_pop(shot(7));
        r.tick(0, 10.0);
        let f = r.frame(&[], 400).unwrap();
        assert!(f.shot_on_top && f.offset_px == 0 && f.shot.is_some(), "第一帧整屏还是离开的那页");
        for k in 1..30 {
            r.tick(k * 10, 10.0);
        }
        assert!(r.frame(&[], 400).unwrap().offset_px > 390);
    }
    /// 一加真机：换页那一帧 22ms，下一帧又被上传拖了一个周期。从前第二帧新页一下冲进来两成屏宽
    #[test]
    fn 第一帧慢了_第二帧只走一个周期_之后慢帧也不往前跳() {
        let mut r = RouteState::default();
        r.start_push();
        let shots = [shot(1)];
        let w = 1080;
        r.tick(1000, 11.0);
        // 第一帧花了 22ms（再加派发延迟）：下一个回调在 33ms 之后。这一帧还在原地
        r.tick(1033, 11.0);
        assert_eq!(r.frame(&shots, w).unwrap().offset_px, w, "建页之后那一帧还不动");
        // 之后每帧正常 11ms：起步那一步与后面一步差不多大（减速曲线，起步略大）
        r.tick(1044, 11.0);
        let first_step = w - r.frame(&shots, w).unwrap().offset_px;
        let a = r.frame(&shots, w).unwrap().offset_px;
        r.tick(1055, 11.0);
        let normal_step = a - r.frame(&shots, w).unwrap().offset_px;
        assert!(first_step <= normal_step * 5 / 4 + 1, "起步一步 {first_step}px，下一步 {normal_step}px");
        // 起步第二步被上一帧的大上传拖了一个周期：仍只走一个周期
        let b = r.frame(&shots, w).unwrap().offset_px;
        r.tick(1077, 11.0);
        let late = b - r.frame(&shots, w).unwrap().offset_px;
        assert!(late <= normal_step + 2, "起步阶段晚到一帧跳了 {late}px（正常一步 {normal_step}px）");
        // 中途一帧卡 40ms：只走 1.5 个周期
        let before = r.frame(&shots, w).unwrap().offset_px;
        r.tick(1117, 11.0);
        let jump = before - r.frame(&shots, w).unwrap().offset_px;
        assert!(jump <= normal_step * 2, "慢帧之后跳了 {jump}px（正常一步 {normal_step}px）");
    }

    #[test]
    fn 转场中按下吞掉整串_转场之外不吞() {
        let mut r = RouteState::default();
        assert!(!r.swallow_down());
        r.start_push();
        assert!(r.swallow_down() && r.swallowing());
        r.finish();
        assert!(r.swallowing(), "转场结束了这串照样吞到抬起");
        assert!(r.swallow_end());
        assert!(!r.swallowing() && !r.swallow_end());
    }
}
