// g-webaudio（game-doc/05 GT-AU7 的 WebAudio 版）：真设备上跑 WebAudio 图，量欠载、currentTime 与帧时钟的漂移、节点回收。
// - 持续声：两个振荡器（一个被 LFO 调频）→ 低通（截止频率被另一个 LFO 扫）→ Panner（左右来回）→ 主增益；
// - 音效：点屏幕（或 query auto=1 时每 120ms 一个）放一个「BufferSource → 匿名 Gain(指数衰减) → 主增益」，播完自动回收；
// - 画面：Analyser 的时域波形与频谱（每帧 getFloat*Data，量 JS 侧开销）；
// 每秒一行 T:EV clock {dtMs, dCtxMs, sfx}：dCtxMs 跟 dtMs 的差就是音频时钟相对帧时钟的漂移。
// 节点回收看宿主的音频统计（Android logcat 的 🔊 行：webNodes / webSlots 稳定说明播完的音效在回收）。
var T = require('_lib/t.js');
var ctx = wx.createWebAudioContext(), R = ctx.sampleRate;
var master = ctx.createGain(); master.gain.value = 0.25;
var an = ctx.createAnalyser(); an.fftSize = 1024;
master.connect(an); an.connect(ctx.destination);

var a = ctx.createOscillator(), b = ctx.createOscillator(), lfo = ctx.createOscillator(), sweep = ctx.createOscillator();
a.type = 'sawtooth'; a.frequency.value = 110; b.type = 'square'; b.frequency.value = 165;
lfo.frequency.value = 5; sweep.frequency.value = 0.2;
var lfoDepth = ctx.createGain(); lfoDepth.gain.value = 3; lfo.connect(lfoDepth).connect(a.frequency);
var lp = ctx.createBiquadFilter(); lp.frequency.value = 900; lp.Q.value = 6;
var sweepDepth = ctx.createGain(); sweepDepth.gain.value = 700; sweep.connect(sweepDepth).connect(lp.frequency);
var pn = ctx.createPanner(); pn.panningModel = 'equalpower';
var bg = ctx.createGain(); bg.gain.value = 0.5;
a.connect(lp); b.connect(lp); lp.connect(pn).connect(bg).connect(master);
[a, b, lfo, sweep].forEach(function (o) { o.start(); });
GameGlobal.__keep = [a, b, lfo, sweep, lfoDepth, sweepDepth, lp, pn, bg, master, an];

var click = ctx.createBuffer(1, Math.round(0.08 * R), R);
(function () { var d = click.getChannelData(0); for (var i = 0; i < d.length; i++) d[i] = Math.sin(2 * Math.PI * 880 * i / R) * (1 - i / d.length); })();
var sfx = 0;
function shot() {
  var s = ctx.createBufferSource(), g = ctx.createGain(), t = ctx.currentTime;
  s.buffer = click; s.playbackRate.value = 0.75 + T.rand();
  g.gain.setValueAtTime(0.8, t); g.gain.exponentialRampToValueAtTime(0.01, t + 0.08);
  s.connect(g).connect(master); s.start(t); sfx++;
}
wx.onTouchStart(shot);
if (T.q.auto) setInterval(shot, 120);

var c = wx.createCanvas(), g2 = c.getContext('2d'), W = c.width, H = c.height;
var td = new Float32Array(an.fftSize), fd = new Float32Array(an.frequencyBinCount);
var n = 0, t0 = 0, c0 = 0, jsMs = 0;
(function tick() {
  n++;
  pn.positionX.value = Math.sin(n / 60);
  var j0 = wx.getPerformance().now();
  an.getFloatTimeDomainData(td); an.getFloatFrequencyData(fd);
  jsMs += wx.getPerformance().now() - j0;
  g2.fillStyle = '#101820'; g2.fillRect(0, 0, W, H);
  g2.fillStyle = '#4fc3f7';
  for (var x = 0; x < W; x += 2) { var v = td[Math.floor(x / W * td.length)]; g2.fillRect(x, H * 0.3 + v * H * 0.2, 2, 2); }
  g2.fillStyle = '#ffb300';
  for (var k = 0; k < 128; k++) { var db = fd[k * 2]; var h = Math.max(0, (db + 100) / 70) * H * 0.3; g2.fillRect(k * W / 128, H * 0.95 - h, W / 128 - 1, h); }
  if (n % 60 === 0) {
    var now = wx.getPerformance().now(), ct = ctx.currentTime;
    if (t0) T.ev('clock', { dtMs: +(now - t0).toFixed(1), dCtxMs: +((ct - c0) * 1000).toFixed(1), ctx: +ct.toFixed(3), sfx: sfx, analyserMs: +(jsMs / 60).toFixed(3) });
    t0 = now; c0 = ct; jsMs = 0;
  }
  requestAnimationFrame(tick);
})();
T.ready('g-webaudio');
