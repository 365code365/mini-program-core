// 钢琴音色与音频时钟。全部走 wx.createWebAudioContext：
// - 琴音：JS 里按「多谐波 + 两段指数衰减包络」合成一段 PCM，编成 16 位 WAV 交给 decodeAudioData 解成 AudioBuffer，
//   之后每次按键只建一个 BufferSource 播（现建 Oscillator 在真机上一次 ~25ms，见 play-snake 的 README）；
//   解码是异步的：还没解完就先用同一段 PCM 直接 createBuffer + copyToChannel 顶上，听感一样。
// - 提示音 / 失败音：OscillatorNode + GainNode 包络（setValueAtTime / exponentialRampToValueAtTime），
//   用 start(when) / stop(when) 排在音频时钟上（倒计时四拍一次排好，拍点与画面都对着同一个 currentTime）。
// - 伴奏：BufferSource.start(when) 提前 0.3s 排进音频时钟，不跟着 requestAnimationFrame 抖。
// 没有 WebAudio 时整套静音，游戏照玩（时钟退回帧时间，见 game.js 的 Clock）。
var SR = 22050;                       // 合成采样率：琴音最高到 A5 的第 6 次谐波 ~5.3kHz，22.05k 足够；解码时引擎重采样到上下文的采样率
var TABLE = 4096, SINE = new Float32Array(TABLE + 1);
for (var i = 0; i <= TABLE; i++) SINE[i] = Math.sin(i / TABLE * Math.PI * 2);
var HARM = [1, 0.62, 0.36, 0.24, 0.13, 0.09, 0.05];   // 各次谐波的起始幅度：低次强、高次弱，钢琴的「亮」来自 2~4 次

function midiOf(name) {
  var m = /^([A-G])(#|b)?(-?\d)$/.exec(name);
  if (!m) return -1;
  var base = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 }[m[1]];
  return (Number(m[3]) + 1) * 12 + base + (m[2] === '#' ? 1 : m[2] === 'b' ? -1 : 0);
}
function freqOf(midi) { return 440 * Math.pow(2, (midi - 69) / 12); }

// 一个音的 PCM：每次谐波一个相位累加器查正弦表；高次谐波衰减得更快（真钢琴的音色越弹越「闷」）；
// 弦的非谐性让 n 次谐波略高于 n×f；起音 4ms 线性上升，之后「快衰减 + 慢衰减」两段叠加
function render(midi) {
  var f = freqOf(midi);
  var dur = Math.max(0.7, Math.min(1.8, 1.8 - (midi - 45) * 0.035));
  var len = Math.floor(SR * dur), out = new Float32Array(len);
  var tau = 0.55 * Math.pow(261.6 / f, 0.35) * dur;
  var k = 0, incs = [], amps = [], muls = [], phs = [];
  for (var h = 0; h < HARM.length; h++) {
    var n = h + 1, fh = f * n * Math.sqrt(1 + 0.0004 * n * n);
    if (fh > SR * 0.45) break;
    incs.push(fh / SR * TABLE); amps.push(HARM[h]); phs.push(0);
    muls.push(Math.exp(-1 / (SR * tau / (1 + 0.7 * h))));
    k++;
  }
  var atk = Math.floor(SR * 0.004), fast = Math.exp(-1 / (SR * 0.09)), fe = 1, peak = 0;
  for (var i = 0; i < len; i++) {
    var s = 0;
    for (var j = 0; j < k; j++) {
      var p = phs[j] + incs[j]; if (p >= TABLE) p -= TABLE; phs[j] = p;
      s += SINE[p | 0] * amps[j]; amps[j] *= muls[j];
    }
    fe *= fast;
    var env = (i < atk ? i / atk : 1) * (0.45 + 0.55 * fe);
    if (i > len - 600) env *= (len - i) / 600;          // 尾巴 27ms 收到 0，不留爆音
    s *= env; out[i] = s;
    if (s > peak) peak = s; else if (-s > peak) peak = -s;
  }
  var g = peak > 0 ? 0.8 / peak : 1;
  for (var q = 0; q < len; q++) out[q] *= g;
  return out;
}
function wav(pcm) {
  var ab = new ArrayBuffer(44 + pcm.length * 2), v = new DataView(ab);
  function str(o, s) { for (var i = 0; i < s.length; i++) v.setUint8(o + i, s.charCodeAt(i)); }
  str(0, 'RIFF'); v.setUint32(4, 36 + pcm.length * 2, true); str(8, 'WAVE');
  str(12, 'fmt '); v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, 1, true);
  v.setUint32(24, SR, true); v.setUint32(28, SR * 2, true); v.setUint16(32, 2, true); v.setUint16(34, 16, true);
  str(36, 'data'); v.setUint32(40, pcm.length * 2, true);
  for (var i = 0; i < pcm.length; i++) { var x = pcm[i]; v.setInt16(44 + i * 2, (x < -1 ? -1 : x > 1 ? 1 : x) * 32767, true); }
  return ab;
}

function make() {
  var ac = null;
  try { ac = wx.createWebAudioContext ? wx.createWebAudioContext() : null; } catch (e) { ac = null; }
  var muted = false, notes = {}, queue = [], stats = { decoded: 0, direct: 0, decodeFail: 0, played: 0, scheduled: 0, osc: 0 };
  var mel = null, bass = null, fx = null;
  if (ac) {
    try {
      mel = ac.createGain(); mel.gain.value = 0.9; mel.connect(ac.destination);
      bass = ac.createGain(); bass.gain.value = 0.42; bass.connect(ac.destination);
      fx = ac.createGain(); fx.gain.value = 0.5; fx.connect(ac.destination);
    } catch (e) { ac = null; }
  }

  // 准备一个音：合成 → 先放一份直接建的缓冲（马上能用）→ 交给 decodeAudioData，解完换成解码出来的那份
  function prepare(midi) {
    var n = notes[midi];
    if (n || !ac) return n;
    var pcm = render(midi);
    n = notes[midi] = { buf: null, decoded: false };
    try {
      var b = ac.createBuffer(1, pcm.length, SR);
      if (b.copyToChannel) b.copyToChannel(pcm, 0); else b.getChannelData(0).set(pcm);
      n.buf = b; stats.direct++;
    } catch (e) {}
    if (ac.decodeAudioData) {
      try {
        ac.decodeAudioData(wav(pcm), function (buf) { n.buf = buf; n.decoded = true; stats.decoded++; },
          function () { stats.decodeFail++; });
      } catch (e) { stats.decodeFail++; }
    }
    return n;
  }
  function play(midi, when, toBass) {
    if (!ac || muted || midi < 0) return;
    var n = prepare(midi);
    if (!n || !n.buf) return;
    try {
      var s = ac.createBufferSource();
      s.buffer = n.buf;
      s.connect(toBass ? bass : mel);
      s.start(when || 0);
      if (when) stats.scheduled++; else stats.played++;
    } catch (e) {}
  }
  // 振荡器 + 增益包络：起音 10ms 到 vol，指数衰减到 0.001；when 是音频时钟上的时刻
  function blip(freq, when, dur, type, vol) {
    if (!ac || muted) return;
    try {
      var t = Math.max(when || 0, ac.currentTime);
      var o = ac.createOscillator(), g = ac.createGain();
      o.type = type || 'sine';
      o.frequency.setValueAtTime(freq, t);
      g.gain.setValueAtTime(0.0001, t);
      g.gain.linearRampToValueAtTime(vol || 0.3, t + 0.01);
      g.gain.exponentialRampToValueAtTime(0.001, t + dur);
      o.connect(g); g.connect(fx);
      o.start(t); o.stop(t + dur + 0.02);
      stats.osc++;
    } catch (e) {}
  }
  return {
    ac: ac,
    midiOf: midiOf,
    // 空闲时一帧合成一个音（合成一个 ~5ms，一次全做完那一帧就顿了）
    preload: function (list) { list.forEach(function (m) { if (m >= 0 && !notes[m] && queue.indexOf(m) < 0) queue.push(m); }); },
    pump: function () { if (ac && queue.length) prepare(queue.shift()); },
    pending: function () { return queue.length; },
    play: play, blip: blip,
    now: function () { return ac ? ac.currentTime : 0; },
    running: function () { return !!ac && ac.state === 'running'; },
    suspend: function () { if (ac && ac.suspend) ac.suspend().catch(function () {}); },
    resume: function () { if (ac && ac.resume) ac.resume().catch(function () {}); },
    setMuted: function (m) { muted = m; }, isMuted: function () { return muted; },
    stats: stats
  };
}
module.exports = { make: make, midiOf: midiOf };
