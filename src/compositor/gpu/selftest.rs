//! 启动自检（设计 §6.3；v2 修订 R7）：GPU 设备建好后、第一帧之前，用一张**固定测试帧**跑一遍
//! GPU 合成并读回，与 [`cpu::compose`](crate::compositor::cpu::compose) **逐字节**比较。
//!
//! 测试帧（确定性 LCG 生成，尺寸 = 宿主实际输出尺寸）专挑容易写错的边界：
//! 画布高跨环形纹理的层边界、滚动让视口跨过画布底边（「画布外填底色」）、带 tabBar、
//! fixed 层同时有 a=0 / a=255 / 半透明像素、带一条下拉指示器；再加一帧只放 Modal 的直通帧
//! （不放随时间变化的 Toast / Loading，否则 CPU 与 GPU 两次合成之间相位会变）。
//!
//! Auto 还要量一次稳态成本：预热 2 次、采样 7 次取中位数。GPU 主线程时间（编码 + 提交）
//! ≤ 1ms，且 GPU 完成时间 < 4ms（120Hz 一帧的一半）才用 GPU。弱显卡会卡在完成时间上。
//!
//! **为什么不和 CPU 合成时间比**：最早的规则是「GPU 主线程 ≤ 0.8 × CPU 合成」，但自检里
//! CPU 那一侧只是合成进内存（约 0.5ms），量不到 softbuffer 上屏 —— 窗口模式下 CPU 每帧
//! 上屏实测 3~8ms（`MINI_FPS` 的「上屏」段），这才是 GPU 省掉的大头。拿不含上屏的 CPU
//! 去比，恰好在 GPU 最有用的机器上把它判掉：M3 Pro 窗口启动时测得 GPU 主线程 0.467ms、
//! CPU 0.473ms，被 0.8 倍那条拒了，而同一台机器窗口基准里 GPU p50 0.37ms、CPU 3.06ms。
//! CPU 样本仍然采、仍然写进 `MINI_GPU_LOG`，只是不再参与判定。
use super::readback::{cpu_rgba, first_mismatch};
use super::GpuCompositor;
use crate::compositor::{cpu, FrameLayers, LayerCanvas, OverlayRefs, PageCanvas, PullState};
use crate::host::ui_overlay::ModalState;
use crate::{Canvas, Color};
use std::time::Instant;

/// 自检逻辑的版本：改了自检就 +1，判定缓存里旧的「自检不一致」记录随之失效（v2 修订 R15）
pub const SELFTEST_REV: u32 = 2;

const WARMUP: usize = 2;
const SAMPLES: usize = 7;
/// Auto 要求 GPU 主线程时间（编码 + 提交）≤ 这个值（ms）：144Hz 一帧 6.94ms 的约 1/7
pub const AUTO_MAX_MAIN_MS: f32 = 1.0;
/// Auto 要求 GPU 完成时间 < 这个值（ms）
pub const AUTO_MAX_DONE_MS: f32 = 4.0;

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub mismatch: Option<String>,
    /// 各次采样：(GPU 主线程, GPU 完成, CPU 合成)，ms
    pub samples: Vec<(f32, f32, f32)>,
}

fn median(mut v: Vec<f32>) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v[v.len() / 2]
}

impl Report {
    pub fn gpu_main_ms(&self) -> f32 {
        median(self.samples.iter().map(|s| s.0).collect())
    }
    pub fn gpu_done_ms(&self) -> f32 {
        median(self.samples.iter().map(|s| s.1).collect())
    }
    pub fn cpu_ms(&self) -> f32 {
        median(self.samples.iter().map(|s| s.2).collect())
    }

    /// Auto 的性能结论：`Some(原因)` 表示不该用 GPU
    pub fn auto_rejection(&self) -> Option<String> {
        let (g, d) = (self.gpu_main_ms(), self.gpu_done_ms());
        if g > AUTO_MAX_MAIN_MS {
            return Some(format!("自检：GPU 主线程 {g:.3}ms > {AUTO_MAX_MAIN_MS}ms"));
        }
        if d >= AUTO_MAX_DONE_MS {
            return Some(format!("自检：GPU 完成 {d:.3}ms ≥ {AUTO_MAX_DONE_MS}ms"));
        }
        None
    }

    pub fn describe(&self) -> String {
        let s: Vec<String> = self.samples.iter().map(|(g, d, c)| format!("{g:.3}/{d:.3}/{c:.3}")).collect();
        format!(
            "{}；中位数 GPU 主线程 {:.3}ms、GPU 完成 {:.3}ms、CPU {:.3}ms；样本（主线程/完成/CPU）{}",
            self.mismatch.as_deref().unwrap_or("逐字节一致"),
            self.gpu_main_ms(),
            self.gpu_done_ms(),
            self.cpu_ms(),
            s.join(" ")
        )
    }
}

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u8 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 24) as u8
    }
}

/// 固定测试帧的画布：页面（跨层边界的高度）、tabBar、fixed（三种 alpha 都有）
pub fn test_canvases(w: u32, h: u32) -> (PageCanvas, LayerCanvas, LayerCanvas, u32) {
    let mut rng = Lcg(0x5EED_1234);
    let ch = 2100.max(h + 1024);
    let mut page = Canvas::new(w, ch);
    for p in page.pixels_mut() {
        *p = Color::new(rng.next(), rng.next(), rng.next(), rng.next());
    }
    let tab_h = (h / 13).max(1);
    let mut tab = Canvas::new(w, tab_h);
    for p in tab.pixels_mut() {
        *p = Color::new(rng.next(), rng.next(), rng.next(), 255);
    }
    let mut fixed = Canvas::new(w, h);
    for (i, p) in fixed.pixels_mut().iter_mut().enumerate() {
        let a = match i % 3 {
            0 => 0,
            1 => 255,
            _ => rng.next(),
        };
        *p = Color::new(rng.next(), rng.next(), rng.next(), a);
    }
    (PageCanvas::new(page), LayerCanvas::new(tab), LayerCanvas::new(fixed), tab_h)
}

fn layers<'a>(
    w: u32,
    h: u32,
    page: &'a PageCanvas,
    tab: &'a LayerCanvas,
    fixed: &'a LayerCanvas,
    tab_h: u32,
    overlays: OverlayRefs<'a>,
) -> FrameLayers<'a> {
    let ch = page.read().height() as i32;
    FrameLayers {
        width: w,
        height: h,
        scale: 2.0,
        page,
        // 视口跨过画布底边：下半截要填底色
        scroll_px: ch - (h as i32) / 2,
        page_bg: 0x12_34_56,
        has_tabbar: true,
        tabbar: Some(tab),
        tabbar_px: tab_h,
        fixed: Some(fixed),
        fixed_rows: Some((h / 4, h * 3 / 4)),
        pull: Some(PullState { gap_logical: 40.0, refreshing: false, progress: 0.8, phase: 0.25 }),
        overlays,
        edge_back: None,
    }
}

/// 一帧：GPU 离屏合成并读回、CPU 合成，比较。返回 (不一致的描述, GPU 主线程, GPU 完成, CPU)
fn one(comp: &mut GpuCompositor, l: &FrameLayers<'_>, check: bool) -> Result<(Option<String>, f32, f32, f32), String> {
    let st = comp.compose_offscreen(l, true).map_err(|e| format!("自检合成失败：{e}"))?;
    let mut buf = vec![0u32; (l.width * l.height) as usize];
    let t = Instant::now();
    cpu::compose(&mut buf, l);
    let cpu_ms = t.elapsed().as_secs_f32() * 1000.0;
    let mut bad = None;
    if check {
        let gpu = comp.read_back().map_err(|e| format!("自检读回失败：{e}"))?;
        if let Some((x, y, c, g)) = first_mismatch(&cpu_rgba(&buf), &gpu, l.width) {
            bad = Some(format!("自检像素不一致（首个差异 @{x},{y}：CPU #{c:06x} / GPU #{g:06x}）"));
        }
    }
    Ok((bad, st.encode_ms, st.gpu_done_ms.unwrap_or(0.0), cpu_ms))
}

pub fn run(comp: &mut GpuCompositor, w: u32, h: u32, force_mismatch: bool) -> Report {
    comp.selftesting = true;
    let mut report = Report::default();
    let (page, tab, fixed, tab_h) = test_canvases(w, h);
    let modal = Some(ModalState::new("自检", "直通帧"));
    let result = (|| -> Result<(), String> {
        let plain = layers(w, h, &page, &tab, &fixed, tab_h, OverlayRefs::none());
        for i in 0..WARMUP + SAMPLES {
            // 第一次（首次上传）与最后一次（稳态、没有新上传）都比像素
            let check = i == 0 || i == WARMUP + SAMPLES - 1;
            let (bad, g, d, c) = one(comp, &plain, check)?;
            if let Some(b) = bad {
                return Err(b);
            }
            if i >= WARMUP {
                report.samples.push((g, d, c));
            }
        }
        let mut with_modal = layers(w, h, &page, &tab, &fixed, tab_h, OverlayRefs::none());
        with_modal.overlays.modal = &modal;
        let (bad, ..) = one(comp, &with_modal, true)?;
        bad.map_or(Ok(()), Err)
    })();
    if let Err(e) = result {
        report.mismatch = Some(e);
    } else if force_mismatch {
        report.mismatch = Some("自检像素不一致（注入故障 selftest）".into());
    }
    comp.selftesting = false;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(samples: &[(f32, f32, f32)]) -> Report {
        Report { mismatch: None, samples: samples.to_vec() }
    }

    /// 回归：M3 Pro 窗口启动时的实测样本（GPU 主线程 ≈ CPU 合成）不能再被判掉
    #[test]
    fn gpu_about_as_fast_as_in_memory_cpu_compose_is_accepted() {
        let r = report(&[(0.467, 0.9, 0.473); 7]);
        assert_eq!(r.auto_rejection(), None);
    }

    #[test]
    fn slow_main_thread_or_slow_gpu_is_rejected() {
        let r = report(&[(1.2, 0.9, 5.0); 7]);
        assert!(r.auto_rejection().unwrap().contains("主线程"), "主线程超 1ms 要拒（不管 CPU 多慢）");
        let r = report(&[(0.2, 4.5, 0.5); 7]);
        assert!(r.auto_rejection().unwrap().contains("完成"), "GPU 完成 ≥ 4ms 要拒（弱显卡）");
    }

    #[test]
    fn decision_uses_the_median_not_one_outlier() {
        let mut s = vec![(0.2, 0.6, 0.5); 6];
        s.push((9.0, 30.0, 0.5));
        assert_eq!(report(&s).auto_rejection(), None);
    }
}
