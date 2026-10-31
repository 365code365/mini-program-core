//! 慢帧归因（`MINI_GAME_SLOW_MS=<毫秒>`，iOS 的 `-gameStats 1` 会设成 25）：一次 pump 超过阈值就打一行，
//! 说清这一帧的时间花在哪 —— 输入 / 定时器回调 / rAF 回调（JS）/ 帧末收尾，以及 WebGL 调用里最贵的几种
//! （次数与耗时，`linkProgram` / `compileShader` / `texImage2D` 是常见的元凶）。
//!
//! 为什么要它：真机上的「卡」几乎都是零星的长帧（稳态 16.7ms，偶尔一帧 40~90ms，tools/ios-game-sweep.sh 实测），
//! 平均值、分位数都看不出是谁。Android 的 `🩹 >8ms` 是同一个思路。
//! 不开时每个 GL 调用只多一次已缓存的布尔判断，pump 里一次 Instant::now
use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::Instant;

/// 阈值（毫秒）；None = 不开
pub fn threshold_ms() -> Option<f64> {
    static T: OnceLock<Option<f64>> = OnceLock::new();
    *T.get_or_init(|| std::env::var("MINI_GAME_SLOW_MS").ok().and_then(|v| v.parse::<f64>().ok()).filter(|v| *v > 0.0))
}

#[inline]
pub fn enabled() -> bool {
    threshold_ms().is_some()
}

#[derive(Default)]
struct Acc {
    /// 每种 GL op：(次数, 纳秒)
    gl: Vec<(u32, u64)>,
    /// 每种 2D 画布 op（canvas/natives.rs 的编号）：(次数, 纳秒)
    d2: Vec<(u32, u64)>,
    phases: Vec<(&'static str, u64)>,
    mark: Option<Instant>,
    /// 画布 → 纹理的增量上传（webgl/upload_cache.rs）：整张次数、只传几行的次数、跳过次数、传了的行 / 总行
    up: [u64; 5],
}

thread_local! {
    static ACC: RefCell<Acc> = RefCell::new(Acc::default());
}

/// 一次 GL 调用花了多久（natives.rs 的 dispatch 里调）
pub fn note_gl(op: usize, ns: u64) {
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        if a.gl.len() <= op {
            a.gl.resize(op + 1, (0, 0));
        }
        a.gl[op].0 += 1;
        a.gl[op].1 += ns;
    });
}

/// 2D 画布 op 的名字（编号见 canvas/natives.rs 文件头）
const NAMES_2D: [&str; 49] = [
    "?", "fillRect", "strokeRect", "clearRect", "beginPath", "closePath", "moveTo", "lineTo", "arc", "rect",
    "quadraticCurveTo", "bezierCurveTo", "arcTo", "ellipse", "fill", "stroke", "clip", "save", "restore",
    "translate", "rotate", "scale", "setTransform", "transform", "resetTransform", "fillText", "strokeText", "measureText",
    "〔裁剪拷贝〕", "〔fx蒙版〕", "fillStyle", "strokeStyle", "lineWidth", "globalAlpha", "font", "textAlign", "textBaseline",
    "lineCap", "lineJoin", "miterLimit", "setLineDash", "lineDashOffset", "shadow", "composite", "gradient",
    "getImageData", "putImageData", "drawImage", "pattern",
];
// 28 / 29 不是 JS 的 op：是上面那些绘制里「裁剪区快照 + 混合」与「fx 蒙版」各花了多少（已含在所属 op 里）
const SUB_CLIP: usize = 28;
const SUB_FX: usize = 29;

/// 绘制内部的一段（裁剪拷贝 / fx 蒙版）的耗时，记在 28 / 29 号
pub fn note_sub(fx: bool, ns: u64) {
    let op = if fx { SUB_FX } else { SUB_CLIP };
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        if a.d2.len() <= op {
            a.d2.resize(op + 1, (0, 0));
        }
        a.d2[op].0 += 1;
        a.d2[op].1 += ns;
    });
}

/// 一次 2D 调用的计时（离开作用域时记账）
pub struct Timer2d(i32, Instant);

impl Timer2d {
    pub fn start(op: i32) -> Self {
        Self(op, Instant::now())
    }
}

impl Drop for Timer2d {
    fn drop(&mut self) {
        let (op, ns) = (self.0.clamp(0, 48) as usize, self.1.elapsed().as_nanos() as u64);
        ACC.with(|a| {
            let mut a = a.borrow_mut();
            if a.d2.len() <= op {
                a.d2.resize(op + 1, (0, 0));
            }
            a.d2[op].0 += 1;
            a.d2[op].1 += ns;
        });
    }
}

/// 一次画布 → 纹理上传：`kind` 0 整张 / 1 部分 / 2 跳过，传了 `rows` 行（共 `total` 行）
pub fn note_upload(kind: usize, rows: usize, total: usize) {
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        a.up[kind.min(2)] += 1;
        a.up[3] += rows as u64;
        a.up[4] += total as u64;
    });
}

/// 一帧开始
pub fn begin() {
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        a.gl.iter_mut().for_each(|e| *e = (0, 0));
        a.d2.iter_mut().for_each(|e| *e = (0, 0));
        a.phases.clear();
        a.up = [0; 5];
        a.mark = Some(Instant::now());
    });
}

/// 上一段到此为止，记作 `name`
pub fn phase(name: &'static str) {
    ACC.with(|a| {
        let mut a = a.borrow_mut();
        let now = Instant::now();
        if let Some(m) = a.mark.replace(now) {
            a.phases.push((name, (now - m).as_nanos() as u64));
        }
    });
}

/// 一帧结束：超过阈值就返回一行说明（`names` 是 GL op 的名字表）
pub fn end(total_ms: f64, names: &[&str]) -> Option<String> {
    let t = threshold_ms()?;
    if total_ms < t {
        return None;
    }
    ACC.with(|a| {
        let a = a.borrow();
        let ms = |ns: u64| ns as f64 / 1e6;
        let phases: Vec<String> = a.phases.iter().filter(|p| p.1 >= 500_000).map(|(n, ns)| format!("{n} {:.1}", ms(*ns))).collect();
        let mut gl: Vec<(usize, (u32, u64))> = a.gl.iter().copied().enumerate().filter(|(_, e)| e.0 > 0).collect();
        let gl_total: u64 = gl.iter().map(|(_, e)| e.1).sum();
        let gl_calls: u32 = gl.iter().map(|(_, e)| e.0).sum();
        gl.sort_by(|x, y| y.1 .1.cmp(&x.1 .1));
        let top: Vec<String> = gl.iter().take(5).filter(|(_, e)| e.1 >= 200_000)
            .map(|(i, e)| format!("{}×{} {:.1}", names.get(*i).copied().unwrap_or("?"), e.0, ms(e.1))).collect();
        let mut d2: Vec<(usize, (u32, u64))> = a.d2.iter().copied().enumerate().filter(|(_, e)| e.0 > 0).collect();
        let sub = |i: &usize| *i == SUB_CLIP || *i == SUB_FX;
        let d2_total: u64 = d2.iter().filter(|(i, _)| !sub(i)).map(|(_, e)| e.1).sum();
        let d2_calls: u32 = d2.iter().filter(|(i, _)| !sub(i)).map(|(_, e)| e.0).sum();
        d2.sort_by(|x, y| y.1 .1.cmp(&x.1 .1));
        let top2: Vec<String> = d2.iter().take(7).filter(|(_, e)| e.1 >= 200_000)
            .map(|(i, e)| format!("{}×{} {:.1}", NAMES_2D[*i], e.0, ms(e.1))).collect();
        let up = if a.up[4] > 0 {
            format!("；画布传 整{}/部分{}/跳过{} {}/{}行", a.up[0], a.up[1], a.up[2], a.up[3], a.up[4])
        } else {
            String::new()
        };
        let list = |v: &[String]| if v.is_empty() { String::new() } else { format!("（{}）", v.join("、")) };
        Some(format!(
            // 画布上传那段放前面：NSLog 一行超过 1KB 会被截掉，末尾的看不见
            "🐢 慢帧 {total_ms:.1}ms：{}{up}；GL {gl_calls} 次 {:.1}ms{}；2D {d2_calls} 次 {:.1}ms{}",
            phases.join(" / "), ms(gl_total), list(&top), ms(d2_total), list(&top2)
        ))
    })
}
