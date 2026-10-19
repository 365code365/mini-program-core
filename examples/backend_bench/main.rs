//! 渲染后端基准的编排端（设计 §12；v2 修订 R11、R12、#23）。**所有性能结论只引用它的输出。**
//!
//! ```bash
//! cargo build --release && cargo build --release --examples
//! cargo run --release --example backend_bench -- \
//!     [--runs 5] [--backends cpu,gpu] [--mode headless|windowed|both] [--scenes all|S1,R1,…] [--out target/_bench]
//! cargo run --release --example backend_bench -- --gen-only target/_bench/apps   # 只生成夹具
//! ```
//!
//! 每个「场景 × 缩放 × 模式 × 后端 × 第几次」都是一个独立子进程（`mini-app-window --bench`）：
//! 先 `rm -rf target/mini-storage`（无头点击会把数据落盘，下次读回来页面就变了），
//! 固定 `MINI_FAKE_NOW`，以 `MINI_BACKEND=<后端>` 拉起，60s 超时。
//!
//! 退出码：0 全部通过；1 有场景失败（子进程失败 / 超时 / 守卫不过）；
//! 3 有一部分没测（要求了 GPU 但不可用；窗口模式的显示器不是 144Hz）—— 「没测」不能算通过。
mod report;
mod scenes;

use report::{RunMetrics, Summary};
use scenes::{Drive, Guard, Scene};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_BIN: &str = "target/release/mini-app-window";

/// 被测二进制。`MINI_BENCH_BIN=<路径>` 可以换成别的构建做 A/B：
/// 同一台机器、同一时段交替跑改动前后两个二进制，才排除得了「机器当时热不热」的漂移
/// （实测同一二进制隔半小时再跑，S1 的 p50 能差 4 倍）
fn bin() -> String {
    std::env::var("MINI_BENCH_BIN").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_BIN.into())
}
const TIMEOUT: Duration = Duration::from_secs(60);
const EXIT_FAILED: i32 = 1;
const EXIT_UNTESTED: i32 = 3;
/// 窗口判据要求的显示器刷新率（R12）
const WINDOW_HZ: f64 = 144.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Headless,
    Windowed,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Headless => "无头",
            Mode::Windowed => "窗口",
        }
    }
}

struct Args {
    runs: u32,
    backends: Vec<String>,
    modes: Vec<Mode>,
    scenes: Option<Vec<String>>,
    out: PathBuf,
    gen_only: Option<PathBuf>,
}

fn usage_exit(msg: &str) -> ! {
    eprintln!("❌ {msg}\n用法：backend_bench [--runs N] [--backends cpu,gpu] [--mode headless|windowed|both] [--scenes all|S1,R1] [--out DIR] [--gen-only DIR]");
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut a = Args {
        runs: 5,
        backends: vec!["cpu".into()],
        modes: vec![Mode::Headless],
        scenes: None,
        out: PathBuf::from("target/_bench"),
        gen_only: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| usage_exit(&format!("{k} 缺值")));
        match k.as_str() {
            "--runs" => a.runs = val().parse().ok().filter(|n| *n > 0).unwrap_or_else(|| usage_exit("--runs 要是正整数")),
            "--backends" => {
                a.backends = val().split(',').map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()).collect();
                if a.backends.iter().any(|b| b != "cpu" && b != "gpu") {
                    usage_exit("--backends 只认 cpu / gpu");
                }
            }
            "--mode" => {
                a.modes = match val().as_str() {
                    "headless" => vec![Mode::Headless],
                    "windowed" => vec![Mode::Windowed],
                    "both" => vec![Mode::Headless, Mode::Windowed],
                    other => usage_exit(&format!("--mode 不认识 {other}")),
                }
            }
            "--scenes" => {
                let v = val();
                a.scenes = (v != "all").then(|| v.split(',').map(|s| s.trim().to_ascii_uppercase()).collect());
            }
            "--out" => a.out = PathBuf::from(val()),
            "--gen-only" => a.gen_only = Some(PathBuf::from(val())),
            other => usage_exit(&format!("不认识的参数 {other}")),
        }
    }
    a
}

/// 一次子进程运行的参数
struct RunSpec<'a> {
    scene: &'a Scene,
    scale: f32,
    backend: &'a str,
    json: PathBuf,
    shot: Option<PathBuf>,
    snapshot: Option<PathBuf>,
    log: PathBuf,
}

fn run_child(apps: &Path, spec: &RunSpec<'_>) -> Result<Value, String> {
    let _ = std::fs::remove_dir_all("target/mini-storage");
    let s = spec.scene;
    let mut cmd = Command::new(bin());
    cmd.arg(s.app_path(apps))
        .args(["--route", s.route, "--settle", &s.settle.to_string(), "--scale", &spec.scale.to_string()])
        .args(s.drive.args())
        .arg("--bench")
        .arg(&spec.json)
        .env("MINI_FAKE_NOW", "2024-03-15")
        .env("MINI_BACKEND", spec.backend);
    for e in s.evals {
        cmd.args(["--eval", e]);
    }
    if let Some(dir) = &spec.snapshot {
        cmd.arg("--snapshot").arg(dir);
    }
    if let Some(p) = &spec.shot {
        cmd.arg("--bench-shot").arg(p);
    }
    let log = std::fs::File::create(&spec.log).map_err(|e| format!("建日志 {} 失败：{e}", spec.log.display()))?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    let mut child = cmd.stdout(Stdio::from(log)).stderr(Stdio::from(err)).spawn().map_err(|e| format!("拉起 {} 失败：{e}", bin()))?;
    let t0 = Instant::now();
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        if t0.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("超时（>{}s），日志 {}", TIMEOUT.as_secs(), spec.log.display()));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    if !status.success() {
        return Err(format!("退出码 {:?}，日志 {}", status.code(), spec.log.display()));
    }
    let text = std::fs::read_to_string(&spec.json).map_err(|e| format!("读 {} 失败：{e}", spec.json.display()))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("JSON 解析失败：{e}"))?;
    if v["schema"] != "mini-bench/1" {
        return Err(format!("JSON schema 不是 mini-bench/1：{}", v["schema"]));
    }
    Ok(v)
}

/// 防空转守卫（设计 §12.4）。`Err` 是不通过的原因
fn guard(scene: &Scene, backend: &str, run: &Value, m: &RunMetrics) -> Result<(), String> {
    let want = match scene.drive {
        Drive::Drag { frames, .. } | Drive::Frames(frames) => frames as usize,
    };
    if m.frames < want {
        return Err(format!("drive 帧 {} < 请求 {want}", m.frames));
    }
    if m.backend.as_deref() != Some(backend) {
        return Err(format!("后端没生效：要 {backend}，帧里是 {:?}（{}）", m.backend, run["backend_reason"]));
    }
    let pct = |n: usize| n as f32 / m.frames.max(1) as f32;
    match scene.guard {
        Guard::Scrolled => {
            let sc = &run["scroll"];
            let req = sc["requested_px"].as_f64().unwrap_or(0.0);
            let got = sc["page_delta_px"].as_f64().unwrap_or(0.0) + sc["area_delta_px"].as_f64().unwrap_or(0.0);
            let at_end = sc["at_end"].as_bool().unwrap_or(false);
            if got < 0.9 * req && !at_end {
                return Err(format!("只滚了 {got:.0}px / 请求 {req:.0}px（没到底）"));
            }
        }
        Guard::Animating if pct(m.animating) < 0.9 => return Err(format!("动画帧占比 {:.0}% < 90%", pct(m.animating) * 100.0)),
        Guard::Rerastered if pct(m.rerastered) < 0.3 => return Err(format!("重画帧占比 {:.0}% < 30%", pct(m.rerastered) * 100.0)),
        Guard::ModalVisible if m.modal_visible < m.frames => return Err(format!("Modal 只在 {}/{} 帧可见", m.modal_visible, m.frames)),
        _ => {}
    }
    Ok(())
}

/// `--backend-probe` 的退出码：0 GPU 可用 / 3 不可用 / 4 没编进来
fn probe_gpu() -> i32 {
    Command::new(bin()).arg("--backend-probe").stdout(Stdio::null()).stderr(Stdio::null()).status().ok().and_then(|s| s.code()).unwrap_or(4)
}

fn machine() -> String {
    let cpu = Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).filter(|s| !s.is_empty());
    cpu.unwrap_or_else(|| std::env::consts::ARCH.to_string())
}

fn main() {
    let args = parse_args();
    if let Some(dir) = &args.gen_only {
        if let Err(e) = scenes::generate(dir) {
            usage_exit(&e);
        }
        println!("✅ 夹具 -> {}", dir.display());
        return;
    }
    if !Path::new(&bin()).exists() {
        usage_exit(&format!("{} 不存在：先 cargo build --release", bin()));
    }
    let apps = args.out.join("apps");
    let (runs_dir, shots, tmp) = (args.out.join("runs"), args.out.join("shots"), args.out.join("tmp"));
    for d in [&apps, &runs_dir, &shots, &tmp] {
        std::fs::create_dir_all(d).unwrap_or_else(|e| usage_exit(&format!("建目录 {} 失败：{e}", d.display())));
    }
    scenes::generate(&apps).unwrap_or_else(|e| usage_exit(&e));

    let mut untested: Vec<String> = Vec::new();
    let mut backends = args.backends.clone();
    if backends.iter().any(|b| b == "gpu") {
        let code = probe_gpu();
        if code != 0 {
            let why = if code == 4 { "没编进 gpu feature" } else { "没有可用适配器（软件适配器不算）" };
            println!("⚠️ GPU 未测：{why}（--backend-probe 退出码 {code}）");
            untested.push(format!("GPU：{why}"));
            backends.retain(|b| b != "gpu");
        }
    }
    let selected: Vec<Scene> = scenes::all()
        .into_iter()
        .filter(|s| args.scenes.as_ref().is_none_or(|l| l.iter().any(|x| x == s.id)))
        .collect();

    let mut failures: Vec<String> = Vec::new();
    let mut rows: Vec<Summary> = Vec::new();
    let mut commit: Option<String> = None;
    let mut warned_hz = false;
    for scene in &selected {
        // 首帧确认图：单独一次不计时的无头运行（写 PNG 要几十毫秒，会顶高下一帧的间隔）
        let shot = shots.join(format!("{}_{}.png", scene.id, scene.name));
        let spec = RunSpec {
            scene, scale: 2.0, backend: "cpu",
            json: tmp.join(format!("{}_shot.json", scene.id)), shot: Some(shot.clone()),
            snapshot: Some(tmp.join(format!("{}_shot", scene.id))), log: tmp.join(format!("{}_shot.log", scene.id)),
        };
        if let Err(e) = run_child(&apps, &spec) {
            failures.push(format!("{} 首帧图：{e}", scene.id));
        }
        for &mode in &args.modes {
            // 窗口模式的缩放由系统决定（Retina 是 2×），3× 只在无头里测
            let scales: Vec<f32> = match mode {
                Mode::Headless => scene.scales.to_vec(),
                Mode::Windowed => vec![2.0],
            };
            for scale in scales {
                for backend in &backends {
                    let mut ms: Vec<RunMetrics> = Vec::new();
                    let mut display_hz = None;
                    for i in 1..=args.runs {
                        let tag = format!("{}_{}x_{}_{}_{i}", scene.id, scale, if mode == Mode::Headless { "h" } else { "w" }, backend);
                        let spec = RunSpec {
                            scene, scale, backend,
                            json: runs_dir.join(format!("{tag}.json")), shot: None,
                            snapshot: (mode == Mode::Headless).then(|| tmp.join(&tag)),
                            log: runs_dir.join(format!("{tag}.log")),
                        };
                        let run = match run_child(&apps, &spec) {
                            Ok(v) => v,
                            Err(e) => {
                                failures.push(format!("{tag}：{e}"));
                                continue;
                            }
                        };
                        commit = commit.or_else(|| run["commit"].as_str().map(str::to_string));
                        display_hz = run["display_hz"].as_f64();
                        let m = report::metrics(&run);
                        if let Err(e) = guard(scene, backend, &run, &m) {
                            failures.push(format!("{tag}：守卫不过 —— {e}"));
                            continue;
                        }
                        println!("  {tag:<22} 最慢 {:>6.2}ms  p99 {:>6.2}  p50 {:>5.2}  帧间隔峰值 {:>6.2}", m.slowest, m.p99, m.p50, m.interval_peak);
                        ms.push(m);
                    }
                    if mode == Mode::Windowed {
                        let hz = display_hz.unwrap_or(0.0);
                        if (hz - WINDOW_HZ).abs() > 1.0 && !warned_hz {
                            println!("⚠️ 显示器 {hz:.0}Hz，不是 144Hz，窗口判据未测");
                            untested.push(format!("窗口模式：显示器 {hz:.0}Hz"));
                            warned_hz = true;
                        }
                    }
                    if !ms.is_empty() {
                        rows.push(report::summarize(scene.id, scale, mode.as_str(), backend, &ms, display_hz));
                    }
                }
            }
        }
    }

    let header = format!(
        "# 渲染后端基准\n\n机器：{}；提交：{}；每组 {} 次取中位数（最慢一帧另报 min~max）；只统计 drive 段；节拍 144Hz（预算 6.94ms）。",
        machine(), commit.as_deref().unwrap_or("?"), args.runs
    );
    let legend: String = selected.iter().map(|s| format!("\n- {} `{}`：{}", s.id, s.name, s.measures)).collect();
    let md = report::markdown(&rows, &format!("{header}\n{legend}"));
    let json = serde_json::json!({
        "machine": machine(), "commit": commit, "runs": args.runs,
        "rows": rows.iter().map(Summary::to_json).collect::<Vec<_>>(),
        "failures": failures, "untested": untested,
    });
    let _ = std::fs::write(args.out.join("report.md"), &md);
    let _ = std::fs::write(args.out.join("report.json"), serde_json::to_string_pretty(&json).unwrap_or_default());
    println!("\n{md}");
    println!("首帧图：{}；报告：{}", shots.display(), args.out.join("report.md").display());
    if !failures.is_empty() {
        println!("\n❌ {} 项失败：", failures.len());
        for f in &failures {
            println!("   - {f}");
        }
        std::process::exit(EXIT_FAILED);
    }
    if !untested.is_empty() {
        println!("\n⚠️ 有未测项（不算通过）：{}", untested.join("；"));
        std::process::exit(EXIT_UNTESTED);
    }
    println!("\n✅ 全部场景通过守卫");
}
