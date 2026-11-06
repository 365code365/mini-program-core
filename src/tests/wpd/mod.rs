//! 组件对齐 WP-D（`.agents/tasks/component-gap-2026-11-03/plan.md`）：滚动、吸顶、轮播。
//! 现场写小程序（自定义导航栏、没有 tabBar：页面从屏幕第 0 行起），`MiniEngine` 跑起来按可判定的判据断言。
//! 吸顶类用例一律开两台引擎（局部重画 / `MINI_NO_DAMAGE` 整帧），同一串操作之后逐字节比画面 —— 只重画那一块不许画丢。
use crate::compositor::HostEnv;
use crate::host::engine_damage::FrameKind;
use crate::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;

mod scroll_2d;
mod scroll_clip;
mod scroll_ctx;
mod sticky;
mod swiper;

/// 单页小程序（pages/p/p），`extra` 可再加文件
pub(super) fn fixture(name: &str, wxml: &str, wxss: &str, js: &str, extra: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join("mini-wpd-tests").join(name);
    std::fs::remove_dir_all(&root).ok();
    let base: Vec<(&str, &str)> = vec![
        ("app.json", r#"{"pages":["pages/p/p"],"window":{"navigationStyle":"custom"}}"#),
        ("app.js", "App({ log: [] });"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", wxml),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", js),
        ("pages/p/p.json", "{}"),
    ];
    for (rel, text) in base.iter().chain(extra.iter()) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

/// 包根是进程级的：拿包根写锁（与 wpc / wpb 互斥）
pub(super) struct Pair {
    /// 局部重画
    pub a: MiniEngine,
    /// 参照：一律整帧
    pub b: MiniEngine,
    pub t: u64,
    pub kinds: Vec<FrameKind>,
    _g: std::sync::RwLockWriteGuard<'static, ()>,
}

fn make(root: &std::path::Path, no_damage: bool) -> MiniEngine {
    let data = root.join(if no_damage { "_data_b" } else { "_data_a" });
    let env = HostEnv { no_damage, damage_log: std::env::var_os("MINI_DAMAGE_LOG").is_some() && !no_damage, ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 1.0, env, EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    // 引擎时钟停在每次 pump 给的时刻（engine_life.rs）：拖动按事件时间重采样，事件时刻从前取墙上时钟，
    // 两台先后收到同一下 move 差几十微秒，拖出来的位置就差零点几像素（橡皮筋那段尤其明显），一比画面就「不一致」——
    // 那是量具的毛病，不是局部重画画丢了（全量并行时 10 次里撞 1~2 次）
    e.freeze_clock_for_test();
    e.launch(None).expect("打开首页");
    e
}

/// 只开一台（局部重画）：swiper 的状态与事件队列是进程级的（`SWIPER_MANAGER`），两台同一个 swiper id 的事件
/// 谁先 pump 谁取走 —— 断言事件的用例用这个
pub(super) fn single(root: &std::path::Path) -> (MiniEngine, std::sync::RwLockWriteGuard<'static, ()>) {
    let g = crate::sys::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
    let mut e = make(root, false);
    for t in 1..=3 {
        e.pump(t * 16);
    }
    (e, g)
}

impl Pair {
    pub fn new(root: &std::path::Path) -> Self {
        let g = crate::sys::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner());
        let mut p = Pair { a: make(root, false), b: make(root, true), t: 0, kinds: Vec::new(), _g: g };
        for _ in 0..3 {
            p.step("首帧");
        }
        p
    }

    /// 两台各出一帧，画面必须逐字节相同
    pub fn step(&mut self, what: &str) {
        self.t += 16;
        let t = self.t;
        self.a.set_animation_clock(t as f32 / 1000.0);
        self.b.set_animation_clock(t as f32 / 1000.0);
        self.a.pump(t);
        self.b.pump(t);
        self.kinds.push(self.a.last_frame_kind());
        assert_eq!(self.a.scroll_position(), self.b.scroll_position(), "{what}：两台的滚动位置不同");
        let (pa, pb) = (self.a.rgba().to_vec(), self.b.rgba().to_vec());
        if pa != pb {
            let (w, _) = self.a.pixel_size();
            let i = pa.iter().zip(&pb).position(|(x, y)| x != y).unwrap() / 4;
            // 不一致的包围盒与第一处两边的颜色：分得清是「该画没画」还是「画错了位置」
            let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
            for (k, (p, q)) in pa.chunks(4).zip(pb.chunks(4)).enumerate() {
                if p != q {
                    let (x, y) = (k as u32 % w, k as u32 / w);
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                }
            }
            panic!(
                "{what}：局部重画与整帧不一致，第一处在 ({}, {})（局部 {:?} / 整帧 {:?}），包围盒 x[{x0},{x1}] y[{y0},{y1}]，sv {:?}/{:?}，帧 {:?}",
                i as u32 % w,
                i as u32 / w,
                &pa[i * 4..i * 4 + 4],
                &pb[i * 4..i * 4 + 4],
                self.a.scroll_view_offset("sv"),
                self.b.scroll_view_offset("sv"),
                self.a.last_frame_kind()
            );
        }
    }

    pub fn both(&mut self, mut f: impl FnMut(&mut MiniEngine)) {
        f(&mut self.a);
        f(&mut self.b);
    }

    /// 页面滚到 y（两台），出一帧
    pub fn scroll(&mut self, y: f32) {
        self.both(|e| e.scroll_to(y));
        self.step(&format!("滚到 {y}"));
    }

    pub fn set(&mut self, data: &str) {
        self.both(|e| {
            e.eval(&format!("getCurrentPages()[0].setData({data})")).unwrap();
        });
        self.step(&format!("setData {data}"));
    }

    /// 两台同一串拖动：按下 → `steps` 步移动（每步一帧）→ 抬起 → 再走 `settle` 帧（惯性 / 回弹）
    pub fn drag(&mut self, x: f32, y0: f32, y1: f32, steps: u32, settle: u32) {
        self.both(|e| e.pointer_down(x, y0));
        self.step("按下");
        for i in 1..=steps {
            let y = y0 + (y1 - y0) * i as f32 / steps as f32;
            self.both(|e| e.pointer_move(x, y));
            self.step(&format!("拖到 {y}"));
        }
        self.both(|e| e.pointer_up(x, y1));
        for i in 0..=settle {
            self.step(&format!("松手后第 {i} 帧"));
        }
    }

    pub fn rgb(&mut self, x: u32, y: u32) -> [u8; 3] {
        let (w, _) = self.a.pixel_size();
        let px = self.a.rgba();
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    }

    pub fn log(&self) -> Vec<String> {
        serde_json::from_str(&self.a.eval("JSON.stringify(getApp().log)").unwrap_or_default()).unwrap_or_default()
    }

    /// 有没有整帧（`FrameKind::Full`）：吸顶滚动不许逼出整帧
    pub fn fulls(&self) -> usize {
        self.kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count()
    }

    pub fn count(&self, k: FrameKind) -> usize {
        self.kinds.iter().filter(|x| **x == k).count()
    }
}
