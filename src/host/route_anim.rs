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
//! 动画只管合成。时钟从**第一帧出完之后**才开始走：换页那一帧要整页建树排版（几十毫秒），
//! 从按下那一刻算的话动画开头那一截直接跳过去。
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
    /// 时钟起点（毫秒）。None = 还没出过第一帧
    start_ms: Option<u64>,
    /// 0~1
    t: f32,
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

    /// 新页刚入栈
    pub fn start_push(&mut self) {
        self.anim = Some(Anim { kind: Kind::Push, start_ms: None, t: 0.0 });
    }

    /// 刚退栈。`leaving` 是退栈之前那一帧的画面（要离开的那页）
    pub fn start_pop(&mut self, leaving: PageShot) {
        self.anim = Some(Anim { kind: Kind::Pop(leaving), start_ms: None, t: 0.0 });
    }

    /// 立刻收手（又来了一次跳转、尺寸变了）
    pub fn finish(&mut self) {
        self.anim = None;
    }

    /// 按宿主时钟推进
    pub fn tick(&mut self, now_ms: u64) -> Tick {
        let Some(a) = self.anim.as_mut() else { return Tick::Idle };
        let start = *a.start_ms.get_or_insert(now_ms);
        a.t = (now_ms.saturating_sub(start) as f32 / DURATION_MS).min(1.0);
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
        assert_eq!(r.tick(1000), Tick::Running, "第一帧：起点");
        let shots = [shot(1)];
        assert_eq!(r.frame(&shots, 400).unwrap().offset_px, 400, "第一帧新页整个在右边外面");
        assert_eq!(r.tick(1150), Tick::Running);
        let mid = r.frame(&shots, 400).unwrap().offset_px;
        assert!(mid > 0 && mid < 100, "减速曲线：一半时间已走完大半（{mid}）");
        assert_eq!(r.tick(1300), Tick::Done);
        assert!(r.frame(&shots, 400).is_none());
        assert_eq!(r.tick(1316), Tick::Idle);
    }

    #[test]
    fn 返回是图在上_从零滑到整屏() {
        let mut r = RouteState::default();
        r.start_pop(shot(7));
        r.tick(0);
        let f = r.frame(&[], 400).unwrap();
        assert!(f.shot_on_top && f.offset_px == 0 && f.shot.is_some(), "第一帧整屏还是离开的那页");
        r.tick(299);
        assert!(r.frame(&[], 400).unwrap().offset_px > 390);
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
