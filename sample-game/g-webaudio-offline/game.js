// g-webaudio-offline（game-doc/05 GT-AU4）：同一张 WebAudio 图在引擎（离线渲染、MINI_AUDIO_DUMP）与 Chrome OfflineAudioContext
// 里各渲染 2.048 秒，tools/game-audio-compare.py 逐样本比。图在加载时一次建好（两边的 0 时刻都是渲染起点），
// 只用确定性的东西（噪声用 T.rand）。分 8 段、每段 12288 帧（0.256s，正好 96 个渲染量子）只开一类节点，误差落在哪一段
// 就是哪类节点的差别。段边界对齐量子是有意的：Chrome 的振荡器在「开始于量子中间」的那个量子里，按量子开头读频率参数
// （输出第 offset+j 帧用的是第 j 帧的频率），规范与引擎按帧自己的时刻读；对齐之后两边不受这个差别影响（验收文档记下）。
//   0 正弦 + 增益    1 方波 + 直线渐变        2 锯齿 + 低通 + 指数渐变   3 三角 + Panner（声像 / 距离）
//   4 噪声缓冲 + 倍速循环 + 高通             5 FM（振荡器接频率参数）+ ConstantSource + WaveShaper
//   6 立体声缓冲 → 拆分 / 交叉合并 → Delay + setTarget                 7 噪声 → IIR + setValueCurve
var T = require('_lib/t.js');
var ctx = wx.createWebAudioContext();
var R = ctx.sampleRate, S = 12288 / R, out = ctx.destination;
function seg(i) { return i * S; }
function osc(type, f, i) { var o = ctx.createOscillator(); o.type = type; o.frequency.value = f; o.start(seg(i)); o.stop(seg(i + 1)); return o; }
function gain(v) { var g = ctx.createGain(); g.gain.value = v; return g; }
function noise(ch, sec) {
  var b = ctx.createBuffer(ch, Math.round(sec * R), R);
  for (var c = 0; c < ch; c++) { var d = b.getChannelData(c); for (var k = 0; k < d.length; k++) d[k] = T.rand() * 2 - 1; }
  return b;
}
// 0
osc('sine', 440, 0).connect(gain(0.5)).connect(out);
// 1
var g1 = gain(0); g1.gain.setValueAtTime(0, seg(1)); g1.gain.linearRampToValueAtTime(0.4, seg(2));
osc('square', 220, 1).connect(g1).connect(out);
// 2
var lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 800; lp.Q.value = 5;
var g2 = gain(0.01); g2.gain.setValueAtTime(0.01, seg(2)); g2.gain.exponentialRampToValueAtTime(0.5, seg(3));
osc('sawtooth', 330, 2).connect(lp).connect(g2).connect(out);
// 3
var pn = ctx.createPanner(); pn.positionX.value = 2; pn.positionZ.value = -1; pn.refDistance = 1; pn.rolloffFactor = 0.5;
osc('triangle', 550, 3).connect(pn).connect(gain(0.6)).connect(out);
// 4
var hp = ctx.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = 2000; hp.Q.value = 1;
var s4 = ctx.createBufferSource(); s4.buffer = noise(1, 0.05); s4.loop = true; s4.playbackRate.value = 1.5;
s4.connect(hp).connect(gain(0.3)).connect(out); s4.start(seg(4)); s4.stop(seg(5));
// 5
var car = osc('sine', 600, 5), mod = osc('sine', 90, 5), depth = gain(200);
mod.connect(depth).connect(car.frequency);
var ws = ctx.createWaveShaper(), curve = new Float32Array(257);
for (var k = 0; k < curve.length; k++) { var x = k / 128 - 1; curve[k] = Math.tanh(3 * x); }
ws.curve = curve;
var cs = ctx.createConstantSource(); cs.offset.value = 0.1; cs.start(seg(5)); cs.stop(seg(6));
car.connect(gain(0.4)).connect(ws); cs.connect(ws); ws.connect(gain(0.5)).connect(out);
// 6
var s6 = ctx.createBufferSource(); s6.buffer = noise(2, S);
var sp = ctx.createChannelSplitter(2), mg = ctx.createChannelMerger(2), dl = ctx.createDelay(0.1);
dl.delayTime.value = 0.01;
var g6 = gain(0.5); g6.gain.setTargetAtTime(0.05, seg(6), 0.05);
s6.connect(sp); sp.connect(mg, 0, 1); sp.connect(mg, 1, 0); mg.connect(dl).connect(g6).connect(out);
s6.start(seg(6));
// 拿住引用：拆分 / 合并节点若被 GC，原生会在上游播完后拿掉它们（Chrome 也是 JS 对象没了才关输出），Delay 的尾巴就从立体声
// 变成单声道。哪一边什么时候 GC 不确定，对拍不能依赖它
GameGlobal.__keep6 = [s6, sp, mg, dl, g6];
// 7
var iir = ctx.createIIRFilter([0.1, 0.2, 0.1], [1, -0.9, 0.3]);
var g7 = gain(0); g7.gain.setValueCurveAtTime([0, 0.5, 0.2, 0.6, 0], seg(7), S);
var s7 = ctx.createBufferSource(); s7.buffer = noise(1, S);
s7.connect(iir).connect(g7).connect(out); s7.start(seg(7));
T.ready('g-webaudio-offline');
