//! GT-AU4 的引擎侧部分与 WebAudio 行为（game-doc/05 §3）：无头离线渲染，`MINI_AUDIO_DUMP` 逐样本对期望。
//! 上下文在 game.js 加载时建：它的 0 时刻就是 dump 的第 0 帧（第一次 pump 时开始渲染）。
use super::*;
use mini_render::audio_ctx::decode::make_wav;

fn read_dump(p: &std::path::Path) -> Vec<f32> {
    std::fs::read(p).unwrap_or_default().chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn run(name: &str, js: &str, until: &str, frames: u32, prep: impl FnOnce(&std::path::Path)) -> (Vec<f32>, Vec<serde_json::Value>) {
    let _g = serial();
    let dump = std::env::temp_dir().join(format!("{name}-{}.f32", std::process::id()));
    let dir = std::env::temp_dir().join(format!("{name}-pre-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    prep(&dir);
    std::env::set_var("MINI_AUDIO_DUMP", &dump);
    std::env::set_var("WA_PRE", &dir);
    let mut h = GameHarness::open_inline(name, js, Opts::default());
    std::env::remove_var("MINI_AUDIO_DUMP");
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        std::fs::copy(e.path(), h.dir.join(e.file_name())).unwrap();
    }
    h.engine.eval("typeof go === 'function' && go()").unwrap();
    h.until(until, frames);
    h.assert_no_fail();
    let ev = h.events("w");
    drop(h);
    let out = read_dump(&dump);
    std::fs::remove_file(&dump).ok();
    std::fs::remove_dir_all(&dir).ok();
    (out, ev)
}

/// 振荡器 → 增益（0.5 起直线降到 0）→ destination；stop(1) 之后 onended；ended 时 currentTime ≥ 1
#[test]
fn wa1_振荡器_增益自动化_逐样本_ended() {
    let js = r#"
var T = require('_lib/t.js');
var ctx = wx.createWebAudioContext();
var o = ctx.createOscillator(), g = ctx.createGain();
o.frequency.value = 1000;
g.gain.setValueAtTime(0.5, 0); g.gain.linearRampToValueAtTime(0, 0.5);
o.connect(g); g.connect(ctx.destination);
o.start(0); o.stop(1);
o.onended = function (e) { T.ev('w', { e: 'ended', t: ctx.currentTime, same: e.target === o, rate: ctx.sampleRate }); };
T.ready('wa1');
"#;
    let (out, ev) = run("wa1", js, "\"e\":\"ended\"", 200, |_| {});
    assert_eq!(ev.len(), 1, "{ev:?}");
    assert!(ev[0]["t"].as_f64().unwrap() >= 1.0 && ev[0]["same"] == true && ev[0]["rate"] == 48000.0, "{ev:?}");
    let mut worst = 0f64;
    for k in 0..24_000 {
        let g = 0.5 * (1.0 - k as f64 / 24_000.0);
        let want = (2.0 * std::f64::consts::PI * 1000.0 * k as f64 / 48_000.0).sin() * g;
        worst = worst.max((out[2 * k] as f64 - want).abs()).max((out[2 * k + 1] as f64 - want).abs());
    }
    assert!(worst < 1e-5, "与理想值的最大误差 {worst}");
    assert!(out[2 * 24_000..2 * 48_000].iter().all(|v| v.abs() < 1e-6), "增益降到 0 之后静音");
    assert!(out[2 * 48_000..].iter().all(|&v| v == 0.0), "stop 之后静音");
}

/// createBuffer + getChannelData 写数据 → BufferSource → 拆分 → 交叉合并：左右声道互换
#[test]
fn wa2_缓冲上传_拆分合并换声道() {
    let js = r#"
var T = require('_lib/t.js');
var ctx = wx.createWebAudioContext();
var b = ctx.createBuffer(2, 4800, 48000);
b.getChannelData(0).fill(0.25); b.getChannelData(1).fill(-0.5);
var s = ctx.createBufferSource(); s.buffer = b;
var sp = ctx.createChannelSplitter(2), m = ctx.createChannelMerger(2);
s.connect(sp); sp.connect(m, 0, 1); sp.connect(m, 1, 0); m.connect(ctx.destination);
s.start(0.01);
s.onended = function () { T.ev('w', { e: 'ended', dur: b.duration, ch: m.channelCount }); };
T.ready('wa2');
"#;
    let (out, ev) = run("wa2", js, "\"e\":\"ended\"", 100, |_| {});
    assert_eq!(ev[0]["dur"], 0.1);
    assert_eq!((out[2 * 479], out[2 * 479 + 1]), (0.0, 0.0), "0.01s = 第 480 帧开始");
    assert_eq!((out[2 * 480], out[2 * 480 + 1]), (-0.5, 0.25));
    assert_eq!((out[2 * 5279], out[2 * 5279 + 1]), (-0.5, 0.25));
    assert_eq!(out[2 * 5280], 0.0, "4800 帧之后结束");
}

/// decodeAudioData（WAV 字节）→ 回调与 Promise 都给 AudioBuffer；getChannelData 与源样本一致；放出来逐样本一致
#[test]
fn wa3_解码_取数据_播放() {
    let js = r#"
var T = require('_lib/t.js');
var ctx = wx.createWebAudioContext();
var fs = wx.getFileSystemManager();
globalThis.go = function () {
  var data = fs.readFileSync('a.wav');
  var cb = 0;
  ctx.decodeAudioData(data, function () { cb++; }).then(function (b) {
    var d = b.getChannelData(0);
    T.ev('w', { e: 'decoded', ch: b.numberOfChannels, len: b.length, rate: b.sampleRate, cb: cb, s100: d[100] });
    var s = ctx.createBufferSource(); s.buffer = b; s.connect(ctx.destination);
    s.onended = function () { T.ev('w', { e: 'ended' }); };
    s.start(1);
  });
  ctx.decodeAudioData(new ArrayBuffer(16)).catch(function (e) { T.ev('w', { e: 'bad', msg: e.errMsg }); });
};
T.ready('wa3');
"#;
    let (out, ev) = run("wa3", js, "\"e\":\"ended\"", 200, |d| std::fs::write(d.join("a.wav"), make_wav(48_000, 1, 0.25)).unwrap());
    let dec = ev.iter().find(|e| e["e"] == "decoded").expect("decoded");
    assert_eq!((dec["ch"].as_u64(), dec["len"].as_u64(), dec["rate"].as_f64(), dec["cb"].as_u64()), (Some(1), Some(12_000), Some(48_000.0), Some(1)));
    let want100 = (((100.0f32 / 48_000.0 * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16) as f32 / 32768.0;
    assert!((dec["s100"].as_f64().unwrap() as f32 - want100).abs() < 1e-6);
    let bad = ev.iter().find(|e| e["e"] == "bad").expect("坏数据要失败");
    assert!(bad["msg"].as_str().unwrap().starts_with("decodeAudioData:fail"));
    assert!((out[2 * (48_000 + 100)] - want100).abs() < 1e-6, "1s 处开始放");
}

/// 一次性音效的寿命：200 个「源 → 匿名 gain → destination」播完之后，原生的节点与槽位回到只剩 destination
#[test]
fn wa4_一次性音效播完全部回收_异常名() {
    let js = r#"
var T = require('_lib/t.js');
var ctx = wx.createWebAudioContext();
var b = ctx.createBuffer(1, 480, 48000);
var ended = 0;
for (var i = 0; i < 200; i++) {
  var s = ctx.createBufferSource(); s.buffer = b;
  s.connect(ctx.createGain()).connect(ctx.destination);
  s.onended = function () { if (++ended === 200) done(); };
  s.start(0.05 + i * 0.001);
}
var errs = [];
try { s.start(); } catch (e) { errs.push(e.name); }
try { ctx.createGain().gain.exponentialRampToValueAtTime(0, 1); } catch (e) { errs.push(e.name); }
try { ctx.createDelay(200); } catch (e) { errs.push(e.name); }
try { ctx.createAnalyser().fftSize = 1000; } catch (e) { errs.push(e.name); }
function done() {
  s = null;
  setTimeout(function () {
    wx.triggerGC();
    setTimeout(function () {
      var st = JSON.parse(__game_webaudio('stats', '{}')[0]);
      T.ev('w', { e: 'done', slots: st.slots, nodes: st.nodes, bufs: st.bufs, errs: errs });
    }, 50);
  }, 50);
}
T.ready('wa4');
"#;
    let (_, ev) = run("wa4", js, "\"e\":\"done\"", 300, |_| {});
    let d = &ev[0];
    assert_eq!(d["errs"], serde_json::json!(["InvalidStateError", "RangeError", "NotSupportedError", "IndexSizeError"]), "{d}");
    // 播完的源、它们下游没人引用的 gain、抛异常那几行建的节点（GC 之后）全部拿掉：只剩 destination
    assert_eq!((d["slots"].as_u64(), d["nodes"].as_u64()), (Some(1), Some(1)), "{d}");
}

/// AnalyserNode：1500Hz 正弦的频谱峰值在第 64 个频点（fftSize 2048、48k），时域数据就是输入
#[test]
fn wa5_分析器() {
    let js = r#"
var T = require('_lib/t.js');
var ctx = wx.createWebAudioContext();
var o = ctx.createOscillator(), an = ctx.createAnalyser();
o.frequency.value = 1500; an.smoothingTimeConstant = 0;
o.connect(an); o.start(0);
var n = 0;
(function tick() {
  if (++n === 10) {
    var f = new Float32Array(an.frequencyBinCount); an.getFloatFrequencyData(f);
    var best = 0; for (var i = 1; i < f.length; i++) if (f[i] > f[best]) best = i;
    var t = new Float32Array(2048); an.getFloatTimeDomainData(t);
    var mx = 0; for (var k = 0; k < t.length; k++) mx = Math.max(mx, Math.abs(t[k]));
    var by = new Uint8Array(32); an.getByteFrequencyData(by);
    T.ev('w', { e: 'an', peak: best, db: f[best], max: mx, bins: an.frequencyBinCount });
    return;
  }
  requestAnimationFrame(tick);
})();
T.ready('wa5');
"#;
    let (_, ev) = run("wa5", js, "\"e\":\"an\"", 60, |_| {});
    let a = &ev[0];
    assert_eq!((a["peak"].as_u64(), a["bins"].as_u64()), (Some(64), Some(1024)), "{a}");
    // 幅度 0.5 × Blackman 相干增益 0.42 ≈ 0.21 → -13.6dB
    assert!((a["db"].as_f64().unwrap() + 13.6).abs() < 0.3, "{a}");
    assert!((a["max"].as_f64().unwrap() - 1.0).abs() < 1e-3, "{a}");
}
