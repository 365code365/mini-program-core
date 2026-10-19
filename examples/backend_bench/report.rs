//! 指标口径与汇总（设计 §12.3、§12.5）。**口径只有这一份**：测量端只记录逐帧原始数据，
//! 分位数、峰值、跨次中位数全在这里算 —— 两边各算一遍，迟早出现「同一份数据两个 p99」。
use serde_json::{json, Value};

/// 最近秩法分位数：`sorted[ceil(q·n) − 1]`。空集返回 0
pub fn nearest_rank(sorted: &[f32], q: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = ((q * sorted.len() as f32).ceil() as usize).clamp(1, sorted.len());
    sorted[k - 1]
}

fn sorted(mut v: Vec<f32>) -> Vec<f32> {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v
}

fn f(v: &Value, k: &str) -> f32 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32
}

fn b(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

/// 一次运行的指标（只统计 `phase == "drive"` 的帧）
#[derive(Clone, Debug, Default)]
pub struct RunMetrics {
    pub frames: usize,
    pub slowest: f32,
    pub p99: f32,
    pub p50: f32,
    /// max(interval)，从第 2 个 drive 帧起算（第 1 帧的间隔连着 settle / eval，不属于本段）
    pub interval_peak: f32,
    /// 同上，但排除「本帧或上一帧重画过页面画布」的帧：衡量合成器本身的帧节奏。
    /// 跨条带重画造成的尖峰属于光栅，归 P3 解决
    pub composite_interval_peak: f32,
    pub over_budget: usize,
    pub compose_p99: f32,
    /// 只有 GPU 后端有（提交 → 完成）
    pub gpu_done_p99: Option<f32>,
    pub rerastered: usize,
    pub animating: usize,
    pub modal_visible: usize,
    /// 每帧的 backend 都是这个值；不一致时为 None
    pub backend: Option<String>,
    pub presented: usize,
    /// 最慢那一帧的分段（logic, render, compose）
    pub slowest_parts: (f32, f32, f32),
}

pub fn metrics(run: &Value) -> RunMetrics {
    let budget = f(run, "budget_ms");
    let frames: Vec<&Value> = run["frames"]
        .as_array()
        .map(|a| a.iter().filter(|x| x["phase"] == "drive").collect())
        .unwrap_or_default();
    let mut m = RunMetrics { frames: frames.len(), ..RunMetrics::default() };
    if frames.is_empty() {
        return m;
    }
    let work: Vec<f32> = frames.iter().map(|x| f(x, "work_ms")).collect();
    let ws = sorted(work.clone());
    m.slowest = *ws.last().unwrap();
    m.p99 = nearest_rank(&ws, 0.99);
    m.p50 = nearest_rank(&ws, 0.50);
    m.over_budget = work.iter().filter(|w| **w > budget).count();
    m.compose_p99 = nearest_rank(&sorted(frames.iter().map(|x| f(x, "compose_cpu_ms")).collect()), 0.99);
    let gpu_done: Vec<f32> = frames.iter().filter_map(|x| x["gpu_done_ms"].as_f64().map(|v| v as f32)).collect();
    m.gpu_done_p99 = (!gpu_done.is_empty()).then(|| nearest_rank(&sorted(gpu_done), 0.99));
    for i in 1..frames.len() {
        let gap = f(frames[i], "interval_ms");
        m.interval_peak = m.interval_peak.max(gap);
        if !b(frames[i], "rerastered") && !b(frames[i - 1], "rerastered") {
            m.composite_interval_peak = m.composite_interval_peak.max(gap);
        }
    }
    m.rerastered = frames.iter().filter(|x| b(x, "rerastered")).count();
    m.animating = frames.iter().filter(|x| b(x, "animating")).count();
    m.modal_visible = frames.iter().filter(|x| b(x, "modal_visible")).count();
    m.presented = frames.iter().filter(|x| b(x, "presented")).count();
    let first = frames[0]["backend"].as_str().map(str::to_string);
    m.backend = first.filter(|bk| frames.iter().all(|x| x["backend"].as_str() == Some(bk.as_str())));
    let worst = frames.iter().max_by(|a, b| f(a, "work_ms").partial_cmp(&f(b, "work_ms")).unwrap()).unwrap();
    m.slowest_parts = (f(worst, "logic_ms"), f(worst, "render_ms"), f(worst, "compose_cpu_ms"));
    m
}

fn median(mut v: Vec<f32>) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

/// 一组（场景 × 缩放 × 模式 × 后端）跨多次运行的汇总
#[derive(Clone, Debug)]
pub struct Summary {
    pub scene: String,
    pub scale: f32,
    pub mode: String,
    pub backend: String,
    pub runs: usize,
    pub slowest: f32,
    pub slowest_min: f32,
    pub slowest_max: f32,
    pub p99: f32,
    pub p50: f32,
    pub interval_peak: f32,
    pub composite_interval_peak: f32,
    pub over_budget: f32,
    pub compose_p99: f32,
    pub gpu_done_p99: Option<f32>,
    pub display_hz: Option<f64>,
}

pub fn summarize(scene: &str, scale: f32, mode: &str, backend: &str, runs: &[RunMetrics], display_hz: Option<f64>) -> Summary {
    let col = |g: fn(&RunMetrics) -> f32| median(runs.iter().map(g).collect());
    let slow: Vec<f32> = runs.iter().map(|r| r.slowest).collect();
    let gd: Vec<f32> = runs.iter().filter_map(|r| r.gpu_done_p99).collect();
    Summary {
        scene: scene.into(),
        scale,
        mode: mode.into(),
        backend: backend.into(),
        runs: runs.len(),
        slowest: col(|r| r.slowest),
        slowest_min: slow.iter().cloned().fold(f32::MAX, f32::min),
        slowest_max: slow.iter().cloned().fold(0.0, f32::max),
        p99: col(|r| r.p99),
        p50: col(|r| r.p50),
        interval_peak: col(|r| r.interval_peak),
        composite_interval_peak: col(|r| r.composite_interval_peak),
        over_budget: col(|r| r.over_budget as f32),
        compose_p99: col(|r| r.compose_p99),
        gpu_done_p99: (!gd.is_empty()).then(|| median(gd)),
        display_hz,
    }
}

impl Summary {
    pub fn to_json(&self) -> Value {
        json!({
            "scene": self.scene, "scale": self.scale, "mode": self.mode, "backend": self.backend,
            "runs": self.runs, "slowest_ms": self.slowest, "slowest_min_ms": self.slowest_min,
            "slowest_max_ms": self.slowest_max, "p99_ms": self.p99, "p50_ms": self.p50,
            "interval_peak_ms": self.interval_peak, "composite_interval_peak_ms": self.composite_interval_peak,
            "over_budget_frames": self.over_budget, "compose_p99_ms": self.compose_p99,
            "gpu_done_p99_ms": self.gpu_done_p99, "display_hz": self.display_hz,
        })
    }
}

fn opt(v: Option<f32>) -> String {
    v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into())
}

/// Markdown 表格：每个「场景 × 缩放 × 模式 × 后端」一行；GPU 行最后一列是相对同组 CPU 的 p99 比值
pub fn markdown(rows: &[Summary], header: &str) -> String {
    let mut s = String::new();
    s.push_str(header);
    s.push_str("\n\n| 场景 | 缩放 | 模式 | 后端 | 最慢一帧 ms（min~max） | p99 | p50 | 帧间隔峰值 | 合成帧间隔峰值 | 超预算帧 | 合成 p99 | GPU 完成 p99 | GPU/CPU p99 |\n");
    s.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for r in rows {
        let ratio = (r.backend == "gpu")
            .then(|| {
                rows.iter()
                    .find(|c| c.backend == "cpu" && c.scene == r.scene && c.scale == r.scale && c.mode == r.mode)
                    .filter(|c| c.p99 > 0.0)
                    .map(|c| r.p99 / c.p99)
            })
            .flatten();
        s.push_str(&format!(
            "| {} | {}× | {} | {} | {:.2}（{:.2}~{:.2}） | {:.2} | {:.2} | {:.2} | {:.2} | {:.0} | {:.2} | {} | {} |\n",
            r.scene, r.scale, r.mode, r.backend, r.slowest, r.slowest_min, r.slowest_max, r.p99, r.p50,
            r.interval_peak, r.composite_interval_peak, r.over_budget, r.compose_p99, opt(r.gpu_done_p99),
            ratio.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into()),
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_matches_definition() {
        let v: Vec<f32> = (1..=100).map(|x| x as f32).collect();
        assert_eq!(nearest_rank(&v, 0.99), 99.0);
        assert_eq!(nearest_rank(&v, 0.50), 50.0);
        assert_eq!(nearest_rank(&[3.0], 0.99), 3.0);
        assert_eq!(nearest_rank(&[], 0.5), 0.0);
    }

    #[test]
    fn metrics_skip_non_drive_and_composite_excludes_raster_neighbors() {
        let run = json!({ "budget_ms": 6.94, "frames": [
            { "phase": "settle", "work_ms": 99.0, "interval_ms": 99.0 },
            { "phase": "drive", "backend": "cpu", "work_ms": 1.0, "interval_ms": 50.0, "rerastered": false },
            { "phase": "drive", "backend": "cpu", "work_ms": 8.0, "interval_ms": 9.0, "rerastered": true },
            { "phase": "drive", "backend": "cpu", "work_ms": 1.0, "interval_ms": 8.5, "rerastered": false },
            { "phase": "drive", "backend": "cpu", "work_ms": 1.0, "interval_ms": 7.0, "rerastered": false },
        ]});
        let m = metrics(&run);
        assert_eq!(m.frames, 4);
        assert_eq!(m.slowest, 8.0);
        assert_eq!(m.over_budget, 1);
        // 第 1 个 drive 帧的 50ms 间隔不算
        assert_eq!(m.interval_peak, 9.0);
        // 9.0（本帧重画）与 8.5（上一帧重画）都排除，只剩 7.0
        assert_eq!(m.composite_interval_peak, 7.0);
        assert_eq!(m.backend.as_deref(), Some("cpu"));
    }
}
