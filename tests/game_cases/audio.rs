//! GT-AU1 / AU2 / AU3 / AU9（game-doc/05 §3）：无头离线模式（固定时钟 → 不开声卡，混音器按帧时钟渲染），
//! `MINI_AUDIO_DUMP` 把混音输出写成小端 f32（48k 立体声交错），逐样本对期望。
use super::*;
use mini_render::audio_ctx::decode::make_wav;

/// 夹具写的 440Hz WAV（make_wav：幅度 8000/32768）的第 i 个样本
fn wav_sample(i: usize, rate: u32) -> f32 {
    (((i as f32 / rate as f32 * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16) as f32 / 32768.0
}

fn read_dump(p: &std::path::Path) -> Vec<f32> {
    std::fs::read(p).unwrap_or_default().chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn open_audio(name: &str, js: &str, dump: &std::path::Path) -> GameHarness {
    std::env::set_var("MINI_AUDIO_DUMP", dump);
    let h = GameHarness::open_inline(name, js, Opts::default());
    std::env::remove_var("MINI_AUDIO_DUMP");
    h
}

fn put_wavs(h: &GameHarness) {
    std::fs::write(h.dir.join("a.wav"), make_wav(48_000, 1, 0.5)).unwrap();
    std::fs::write(h.dir.join("short.wav"), make_wav(48_000, 1, 0.1)).unwrap();
}

/// AU1：事件序列 canplay → play → timeUpdate… → ended，currentTime 单调；AU2：音量 0.5 的输出逐样本 = 源 × 0.5
#[test]
fn au1_au2_事件序列与逐样本输出() {
    let _g = serial();
    let dump = std::env::temp_dir().join(format!("au12-{}.f32", std::process::id()));
    let js = r#"
var T = require('_lib/t.js');
globalThis.go = function () {
  var a = wx.createInnerAudioContext(); a.volume = 0.5;
  ['Canplay', 'Play', 'Pause', 'Stop', 'Ended', 'TimeUpdate', 'Error', 'Waiting'].forEach(function (e) {
    a['on' + e](function () { T.ev('a', { e: e, t: a.currentTime, d: a.duration, p: a.paused }); });
  });
  a.src = 'a.wav'; a.play();
};
T.ready('au');
"#;
    let mut h = open_audio("au12", js, &dump);
    put_wavs(&h);
    h.engine.eval("go()").unwrap();
    h.until("\"e\":\"Ended\"", 200);
    h.assert_no_fail();
    let ev = h.events("a");
    let names: Vec<&str> = ev.iter().map(|e| e["e"].as_str().unwrap()).collect();
    assert_eq!(&names[..3], ["Waiting", "Canplay", "Play"], "{names:?}");
    assert_eq!(names.last(), Some(&"Ended"));
    let tu: Vec<f64> = ev.iter().filter(|e| e["e"] == "TimeUpdate").map(|e| e["t"].as_f64().unwrap()).collect();
    assert!(tu.len() >= 2 && tu.windows(2).all(|w| w[0] <= w[1]), "timeUpdate 的 currentTime 单调：{tu:?}");
    let end = ev.last().unwrap();
    assert_eq!((end["t"].as_f64(), end["p"].as_bool()), (Some(0.5), Some(true)));
    assert!((end["d"].as_f64().unwrap() - 0.5).abs() < 1e-6);
    drop(h);
    let out = read_dump(&dump);
    let s0 = out.chunks_exact(2).position(|f| f[0] != 0.0).expect("没出声");
    let n = 24_000;
    let mut worst = 0f32;
    for i in 0..n {
        let want = wav_sample(i + 1, 48_000) * 0.5; // s0 是源的第 1 个样本（第 0 个是 sin 0 = 0）
        let (l, r) = (out[(s0 + i) * 2], out[(s0 + i) * 2 + 1]);
        worst = worst.max((l - want).abs()).max((r - want).abs());
    }
    assert!(worst <= 1e-3, "与源 × 0.5 的最大误差 {worst}（判据 1e-3）");
    assert!(out[(s0 + n - 1) * 2..].iter().take(4800).all(|&v| v == 0.0), "0.5s 播完之后静音");
    std::fs::remove_file(&dump).ok();
}

/// AU2：loop 与 playbackRate=2（2 倍速时第 i 个输出样本 = 源的第 2i 个）
#[test]
fn au2_倍速与循环() {
    let _g = serial();
    let dump = std::env::temp_dir().join(format!("au2-{}.f32", std::process::id()));
    let js = r#"
var T = require('_lib/t.js');
globalThis.go = function () {
  var a = wx.createInnerAudioContext(); a.playbackRate = 2; a.loop = true; a.src = 'short.wav';
  a.onCanplay(function () { a.play(); });
  var n = 0; a.onTimeUpdate(function () { if (++n === 3) T.ev('loops', { t: a.currentTime }); });
  a.onEnded(function () { T.ev('ended-should-not', {}); });
};
T.ready('au');
"#;
    let mut h = open_audio("au2", js, &dump);
    put_wavs(&h);
    h.engine.eval("go()").unwrap();
    h.vsyncs(40);
    assert!(h.events("ended-should-not").is_empty(), "loop 不发 ended");
    drop(h);
    let out = read_dump(&dump);
    let s0 = out.chunks_exact(2).position(|f| f[0] != 0.0).unwrap();
    // s0 是源的第 2 个样本（源第 0 个是 0）；之后每个输出样本前进 2 个源样本
    let base = 2;
    let mut worst = 0f32;
    for i in 0..10_000 {
        let src = (base + 2 * i) % 4800;
        worst = worst.max((out[(s0 + i) * 2] - wav_sample(src, 48_000)).abs());
    }
    assert!(worst <= 1e-3, "2 倍速 + 循环的最大误差 {worst}");
}

/// AU3：32 路同时发声，第 33 路抢占最早开始的那个（被抢的收到 stop）；AU9：设备打不开 → 静音模式，事件照常
#[test]
fn au3_au9_抢占与静音模式() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
globalThis.go = function () {
  var pool = [];
  for (var i = 0; i < 33; i++) (function (k) {
    var a = wx.createInnerAudioContext(); a.src = 'a.wav'; a.loop = k === 0;
    a.onStop(function () { T.ev('stop', { k: k }); }); pool.push(a);
  })(i);
  pool[0].onCanplay(function () { pool.forEach(function (a) { a.play(); }); T.ev('played', {}); });
  pool[32].onPlay(function () { T.ev('p32', {}); });
};
T.ready('au');
"#;
    std::env::set_var("MINI_AUDIO_FAULT", "open");
    let mut h = GameHarness::open_inline("au3", js, Opts::default());
    std::env::remove_var("MINI_AUDIO_FAULT");
    put_wavs(&h);
    h.engine.eval("go()").unwrap();
    h.until("T:EV p32", 100);
    h.vsyncs(2);
    let stops: Vec<i64> = h.events("stop").iter().map(|e| e["k"].as_i64().unwrap()).collect();
    assert_eq!(stops, vec![1], "第 33 路抢最早开始的非循环那个（0 号在循环，所以是 1 号）");
    assert!(h.has("被抢占"), "抢占要打日志");
    assert!(!h.engine.audio_stats().5, "固定时钟下不开声卡");
}
