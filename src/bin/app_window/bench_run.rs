//! `--bench <out.json>` / `--hz <f>`：逐帧记录的性能基准（设计 §12.1、§12.3；v2 修订 R11、R12）。
//!
//! 所有性能结论只引用这里写出的 JSON（经 `examples/backend_bench` 汇总）。这一层只**记录**，
//! 不判定、不汇总：分位数、防空转守卫、跨次中位数都在编排端，口径只有一份 ——
//! 两边各算一遍的话，迟早会出现「同一份数据两个 p99」。
//!
//! 两种模式（由有没有 `--snapshot` 决定，与快照 / 交互窗体的分法一致）：
//! - **无头**：`--drag` / `--frames` 的每一帧在 `pump_one_frame` 之后**额外合成一次**
//!   （CPU：`cpu::compose` 进复用的 `Vec<u32>`）。无头本来只在截图那一帧合成，
//!   不补这一步的话测到的只有「逻辑 + 光栅」，合成器的成本恒为 0 —— 而 GPU 后端要比的
//!   恰恰是合成那一步。只记 drive 阶段：settle 与 eval 照快照原样跑，不额外合成。
//! - **窗口**：正常开窗，每次 `RedrawRequested` 里推进脚本（settle → eval → drive，
//!   见 [`super::bench_drive`]），帧照常走 softbuffer 上屏。这是唯一测得到「帧间隔峰值」
//!   与真实上屏（含 macOS 合成器那一侧）的方式。三个阶段的帧都记，用 `phase` 区分。
//!
//! 节拍：两种模式都按 `--hz`（默认 144）走 [`super::pacer`]，与交互窗体同一个节拍器。
//! 不带 `--bench` 时这里一行都不走（窗体的 `bench` 字段为 `None`）。
use super::pacer::Pacer;
use super::scroll_bench::DragSpec;
use mini_render::compositor::{diag, BackendStatus};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const DEFAULT_HZ: f32 = 144.0;
pub const HZ_MIN: f32 = 30.0;
pub const HZ_MAX: f32 = 360.0;
/// `--hz` 越界 / 不是数字、`--bench` 缺路径（与 `--backend` 的值不对同一个码）
pub const EXIT_BAD_ARG: i32 = 2;
/// `--bench` 的目录建不出来 / JSON 写不进去（设计 §9）
pub const EXIT_WRITE_FAILED: i32 = 1;
/// JSON 的格式版本：字段改名或改口径时 +1，编排端据此拒收旧格式
pub const SCHEMA: &str = "mini-bench/1";

/// 解析 `--hz` 的值：`f32`，范围 [30, 360]（设计 §10）
pub fn parse_hz(raw: Option<&str>) -> Result<f32, String> {
    let v = raw.ok_or_else(|| "❌ --hz 后面要跟刷新率（30~360）".to_string())?;
    let hz: f32 = v.trim().parse().map_err(|_| format!("❌ --hz 要是数字（30~360），收到 {v:?}"))?;
    // NaN 不落在任何区间里，contains 对它返回 false，不用单独判
    if !(HZ_MIN..=HZ_MAX).contains(&hz) {
        return Err(format!("❌ --hz 超出范围 [30, 360]：{hz}"));
    }
    Ok(hz)
}

/// `--hz`：不合法就以退出码 2 结束。拼错的刷新率不能被静默当成 144 ——
/// 节拍一错，整份数据的「超预算」口径就全错了
pub fn hz_arg_or_exit(v: Option<String>) -> f32 {
    parse_hz(v.as_deref()).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(EXIT_BAD_ARG);
    })
}

/// `--bench <path>`：缺路径退出码 2；父目录建不出来退出码 1。
/// 开跑前就建目录，免得跑完一整轮才发现写不进去
pub fn out_arg_or_exit(v: Option<String>) -> PathBuf {
    let Some(p) = v.map(PathBuf::from) else {
        eprintln!("❌ --bench 后面要跟输出 JSON 的路径");
        std::process::exit(EXIT_BAD_ARG);
    };
    if let Some(dir) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("❌ --bench 输出目录 {} 建不出来：{e}", dir.display());
            std::process::exit(EXIT_WRITE_FAILED);
        }
    }
    p
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Headless,
    Windowed,
}

/// 帧属于脚本的哪一段（v2 修订 R11）
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    Settle,
    Eval,
    #[default]
    Drive,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Phase::Settle => "settle",
            Phase::Eval => "eval",
            Phase::Drive => "drive",
        }
    }
}

/// 场景参数：原样写进 JSON 顶层，拿到 JSON 就能复现这次运行
#[derive(Clone, Debug, Default)]
pub struct BenchScene {
    pub app: String,
    pub route: Option<String>,
    pub settle: Option<f32>,
    pub drag: Option<DragSpec>,
    pub frames: u32,
    pub evals: Vec<String>,
    /// 无头取 `--scale`；窗口取窗口的 `scale_factor`（开窗后才知道）
    pub scale: f64,
}

impl BenchScene {
    /// drive 阶段要跑几帧：拖动按拖动帧数，否则按 `--frames`
    pub fn drive_frames(&self) -> u32 {
        self.drag.map(|d| d.frames).unwrap_or(self.frames)
    }
}

/// 一帧的记录（单位 ms）。字段口径见设计 §12.3
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameSample {
    pub phase: Phase,
    /// 帧开始 → 合成完成（无头：缓冲写完；窗口：`buffer.present()` 返回）
    pub work_ms: f32,
    pub logic_ms: f32,
    pub render_ms: f32,
    /// 渲染内部（页面, fixed 覆盖层, tabBar）
    pub render_parts: (f32, f32, f32),
    /// 合成这一步占主线程的时间
    pub compose_ms: f32,
    /// 与上一帧开始的间隔；第一帧没有
    pub interval_ms: Option<f32>,
    /// 这一帧写过页面画布（`PageCanvas` 的游标变了）
    pub rerastered: bool,
    pub presented: bool,
    // ── 守卫要用的状态（帧末取）──
    pub animating: bool,
    pub modal_visible: bool,
    /// 页面滚动位置（逻辑像素）
    pub scroll_y: f32,
}

/// 滚动位置快照：drive 开始与结束各取一次，守卫据此算「实际滚了多远」
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollProbe {
    pub page: f32,
    pub page_max: f32,
    /// 各 scroll-view：(id, 位置, 最大滚动)
    pub areas: Vec<(String, f32, f32)>,
}

/// 实际滚动距离：页面那份 + 动得最多的那个 scroll-view（嵌套传递时两者先后各滚一段）。
/// `at_end`：滚到底了（拖不动不是空转，守卫放行）
pub fn scroll_summary(start: &ScrollProbe, end: &ScrollProbe) -> (f32, f32, bool) {
    let page_delta = (end.page - start.page).abs();
    let mut area_delta = 0.0f32;
    let mut area_at_end = false;
    for (id, pos, max) in &end.areas {
        let before = start.areas.iter().find(|a| &a.0 == id).map(|a| a.1).unwrap_or(0.0);
        let d = (pos - before).abs();
        area_delta = area_delta.max(d);
        if d > 0.5 && *max > 0.0 && *pos >= max - 0.5 {
            area_at_end = true;
        }
    }
    let page_at_end = end.page_max > 0.0 && end.page >= end.page_max - 0.5;
    (page_delta, area_delta, page_at_end || area_at_end)
}

/// 窗口模式的脚本进度
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    /// 等到这个时点（第一次出帧时才定）
    Settle(Option<Instant>),
    Eval,
    /// 已经跑完的 drive 帧数
    Drive(u32),
    Done,
}

/// 一次基准运行的全部状态
pub struct BenchDriver {
    pub out: PathBuf,
    /// 只存第一帧 drive 的合成结果（无头）。写 PNG 要几十毫秒，会顶高下一帧的间隔，
    /// 所以编排端用**单独一次不计时**的运行来存图
    pub shot: Option<PathBuf>,
    pub hz: f32,
    pub mode: Mode,
    pub scene: BenchScene,
    /// 窗口模式从 winit monitor 取（R12）；无头为 `None`
    pub display_hz: Option<f64>,
    pub frames: Vec<FrameSample>,
    /// 无头用；窗口模式用窗体自己的节拍器（同一个类型）
    pub pacer: Pacer,
    /// 无头逐帧合成的输出缓冲：复用，不在计时区里逐帧分配
    pub buf: Vec<u32>,
    pub stage: Stage,
    /// 当前帧的阶段（窗口模式在帧开头定）
    pub current: Phase,
    /// 帧开头的页面画布游标（帧末比较得出 `rerastered`）
    pub frame_cursor: Option<(u64, u64)>,
    /// `present()` 留下的（合成耗时, 是否上屏成功）
    pub present_note: Option<(f32, bool)>,
    prev_begin: Option<Instant>,
    scroll_start: Option<ScrollProbe>,
    scroll_end: Option<ScrollProbe>,
}

impl BenchDriver {
    pub fn new(out: PathBuf, shot: Option<PathBuf>, hz: f32, mode: Mode, scene: BenchScene) -> Self {
        let cap = scene.drive_frames() as usize + 256;
        Self {
            out,
            shot,
            hz,
            mode,
            scene,
            display_hz: None,
            frames: Vec::with_capacity(cap),
            pacer: Pacer::new(Instant::now()),
            buf: Vec::new(),
            stage: Stage::Settle(None),
            current: Phase::Settle,
            frame_cursor: None,
            present_note: None,
            prev_begin: None,
            scroll_start: None,
            scroll_end: None,
        }
    }

    /// 一帧的预算（刷新周期）
    pub fn interval(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.hz as f64)
    }

    /// 与上一帧开始的间隔（ms），并把 `begin` 记为「上一帧开始」
    pub fn interval_since(&mut self, begin: Instant) -> Option<f32> {
        let gap = self.prev_begin.map(|p| (begin - p).as_secs_f32() * 1000.0);
        self.prev_begin = Some(begin);
        gap
    }

    pub fn push(&mut self, s: FrameSample) {
        self.frames.push(s);
    }

    pub fn begin_drive(&mut self, p: ScrollProbe) {
        self.scroll_start = Some(p);
    }

    pub fn end_drive(&mut self, p: ScrollProbe) {
        self.scroll_end = Some(p);
    }

    pub fn to_json(&self, backend: &BackendStatus, commit: Option<String>, dirty: Option<bool>) -> Value {
        let label = backend.active.label().to_ascii_lowercase();
        let frames: Vec<Value> = self
            .frames
            .iter()
            .enumerate()
            .map(|(i, f)| frame_json(i, f, &label))
            .collect();
        let requested = self.scene.drag.map(|d| d.dy.abs() * d.frames as f32);
        let scroll = match (&self.scroll_start, &self.scroll_end) {
            (Some(s), Some(e)) => {
                let (page, area, at_end) = scroll_summary(s, e);
                json!({ "requested_px": requested, "page_delta_px": page,
                        "area_delta_px": area, "at_end": at_end })
            }
            _ => Value::Null,
        };
        json!({
            "schema": SCHEMA,
            "mode": match self.mode { Mode::Headless => "headless", Mode::Windowed => "windowed" },
            "backend": label,
            "backend_tag": diag::backend_tag(backend),
            "backend_reason": backend.reason,
            "hz": self.hz,
            "budget_ms": 1000.0 / self.hz,
            "display_hz": self.display_hz,
            "scale": self.scene.scale,
            "app": self.scene.app,
            "route": self.scene.route,
            "settle": self.scene.settle,
            "drag": self.scene.drag.map(|d| json!({ "dy": d.dy, "frames": d.frames })),
            "frames_requested": self.scene.drive_frames(),
            "evals": self.scene.evals,
            "commit": commit,
            "dirty": dirty,
            "scroll": scroll,
            "frames": frames,
        })
    }

    /// 写 JSON；失败以退出码 1 结束（设计 §9）。成功时打一行摘要给人看
    pub fn finish_or_exit(&self, backend: &BackendStatus) {
        let (commit, dirty) = git_commit();
        let v = self.to_json(backend, commit, dirty);
        let text = serde_json::to_string_pretty(&v).unwrap_or_default();
        if let Err(e) = std::fs::write(&self.out, text) {
            eprintln!("❌ --bench 写 {} 失败：{e}", self.out.display());
            std::process::exit(EXIT_WRITE_FAILED);
        }
        let drive: Vec<f32> =
            self.frames.iter().filter(|f| f.phase == Phase::Drive).map(|f| f.work_ms).collect();
        let slowest = drive.iter().cloned().fold(0.0f32, f32::max);
        println!(
            "📏 {} 基准：{} 帧（drive {}）最慢 {:.2}ms @{}Hz -> {}",
            diag::backend_tag(backend),
            self.frames.len(),
            drive.len(),
            slowest,
            self.hz,
            self.out.display()
        );
    }
}

fn frame_json(i: usize, f: &FrameSample, backend: &str) -> Value {
    let (rp, rf, rt) = f.render_parts;
    json!({
        "i": i,
        "phase": f.phase.as_str(),
        "backend": backend,
        // 从选定 GPU 那一帧起计，与 MINI_GPU_FAULT 的 N 同一个计数器；CPU 没有
        "gpu_frame": Value::Null,
        "presented": f.presented,
        "work_ms": f.work_ms,
        "logic_ms": f.logic_ms,
        "render_ms": f.render_ms,
        "render_page_ms": rp,
        "render_fixed_ms": rf,
        "render_tabbar_ms": rt,
        "compose_cpu_ms": f.compose_ms,
        // 以下三项只有 GPU 后端有：CPU 不上传纹理、不等 GPU、不等垂直同步
        "gpu_done_ms": Value::Null,
        "vsync_wait_ms": Value::Null,
        "upload_bytes": Value::Null,
        "passthrough": Value::Null,
        "interval_ms": f.interval_ms,
        "rerastered": f.rerastered,
        "animating": f.animating,
        "modal_visible": f.modal_visible,
        "scroll_y": f.scroll_y,
    })
}

/// 提交号与「工作区有没有改动」：能取到就取（不在 git 仓库里、没装 git 都返回 None）。
/// 在跑完之后才取，不进计时区
fn git_commit() -> (Option<String>, Option<bool>) {
    let git = |args: &[&str]| -> Option<String> {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let commit = git(&["rev-parse", "--short", "HEAD"]);
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).map(|s| !s.is_empty());
    (commit, dirty)
}

/// 把 0x00RRGGBB 缓冲存成 PNG（首帧确认图）
pub fn save_png(path: &Path, buf: &[u32], w: u32, h: u32) -> Result<(), String> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("建目录 {} 失败：{e}", dir.display()))?;
    }
    let mut rgba = Vec::with_capacity(buf.len() * 4);
    for px in buf {
        rgba.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8, 255]);
    }
    image::save_buffer(path, &rgba, w, h, image::ColorType::Rgba8)
        .map_err(|e| format!("写 {} 失败：{e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mini_render::compositor::PrefSource;

    #[test]
    fn hz_range_and_garbage() {
        assert_eq!(parse_hz(Some("144")), Ok(144.0));
        assert_eq!(parse_hz(Some(" 30 ")), Ok(30.0));
        assert_eq!(parse_hz(Some("360")), Ok(360.0));
        assert!(parse_hz(Some("29.9")).is_err());
        assert!(parse_hz(Some("361")).is_err());
        assert!(parse_hz(Some("NaN")).is_err());
        assert!(parse_hz(Some("fast")).is_err());
        assert!(parse_hz(None).is_err());
    }

    #[test]
    fn scroll_summary_counts_page_and_busiest_area() {
        let start = ScrollProbe { page: 0.0, page_max: 5000.0, areas: vec![("a".into(), 10.0, 900.0)] };
        let end = ScrollProbe {
            page: 300.0,
            page_max: 5000.0,
            areas: vec![("a".into(), 610.0, 900.0), ("b".into(), 40.0, 100.0)],
        };
        assert_eq!(scroll_summary(&start, &end), (300.0, 600.0, false));
        // 页面到底
        let end = ScrollProbe { page: 4999.8, ..end };
        assert!(scroll_summary(&start, &end).2);
        // scroll-view 动过且到底
        let end = ScrollProbe { page: 0.0, page_max: 0.0, areas: vec![("a".into(), 900.0, 900.0)] };
        assert!(scroll_summary(&start, &end).2);
        // 没动过的 scroll-view 恰好停在底部（内容不足一屏）不算到底
        let end = ScrollProbe { page: 0.0, page_max: 0.0, areas: vec![("a".into(), 10.0, 10.0)] };
        let start = ScrollProbe { areas: vec![("a".into(), 10.0, 10.0)], ..ScrollProbe::default() };
        assert!(!scroll_summary(&start, &end).2);
    }

    #[test]
    fn json_has_the_r11_fields_and_cpu_nulls() {
        let scene = BenchScene {
            app: "x".into(),
            drag: Some(DragSpec { dy: 8.0, frames: 2 }),
            scale: 2.0,
            ..BenchScene::default()
        };
        let mut b = BenchDriver::new(PathBuf::from("/tmp/x.json"), None, 144.0, Mode::Headless, scene);
        let t0 = Instant::now();
        assert_eq!(b.interval_since(t0), None, "第一帧没有间隔");
        let gap = b.interval_since(t0 + Duration::from_millis(7)).unwrap();
        assert!((gap - 7.0).abs() < 0.01);
        b.push(FrameSample { work_ms: 1.5, presented: true, interval_ms: None, ..FrameSample::default() });
        b.push(FrameSample { phase: Phase::Settle, ..FrameSample::default() });
        b.begin_drive(ScrollProbe::default());
        b.end_drive(ScrollProbe { page: 15.0, page_max: 100.0, areas: vec![] });
        let s = BackendStatus::preferred_cpu(PrefSource::Env);
        let v = b.to_json(&s, Some("abc1234".into()), Some(false));
        assert_eq!(v["schema"], SCHEMA);
        assert_eq!(v["backend"], "cpu");
        assert_eq!(v["backend_tag"], "[后端 CPU]");
        assert_eq!(v["display_hz"], Value::Null);
        assert_eq!(v["frames_requested"], 2);
        assert_eq!(v["scroll"]["requested_px"], 16.0);
        assert_eq!(v["scroll"]["page_delta_px"], 15.0);
        let f0 = &v["frames"][0];
        assert_eq!(f0["phase"], "drive");
        assert_eq!(f0["gpu_frame"], Value::Null);
        assert_eq!(f0["presented"], true);
        assert_eq!(f0["interval_ms"], Value::Null);
        assert_eq!(f0["work_ms"], 1.5);
        assert_eq!(v["frames"][1]["phase"], "settle");
        for k in ["logic_ms", "render_ms", "compose_cpu_ms", "rerastered", "upload_bytes", "passthrough"] {
            assert!(f0.get(k).is_some(), "缺字段 {k}");
        }
    }

    #[test]
    fn drive_frames_prefers_drag() {
        let s = BenchScene { drag: Some(DragSpec { dy: 8.0, frames: 120 }), frames: 300, ..Default::default() };
        assert_eq!(s.drive_frames(), 120);
        let s = BenchScene { frames: 300, ..Default::default() };
        assert_eq!(s.drive_frames(), 300);
    }
}
