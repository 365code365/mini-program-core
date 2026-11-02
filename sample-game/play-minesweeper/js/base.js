// 小底座：从 play-snake 的 js/base.js 拷来（包自包含），去掉了「点按 / 滑动」那套归一化输入 ——
// 扫雷要自己算长按时长、按 e.touches 做双指缩放，归一化之后这些信息就没了（输入在 js/input.js）。
// 只用微信小游戏公开 API：wx.createCanvas / requestAnimationFrame / wx.createWebAudioContext /
// wx.getStorageSync / wx.vibrateShort / wx.onShow / wx.onHide。
// 坐标一律是逻辑像素（windowWidth × windowHeight）：主画布是物理像素，每帧开头按 dpr 设一次变换。
var sys = wx.getSystemInfoSync();
var canvas = GameGlobal.__mainCanvas || (GameGlobal.__mainCanvas = wx.createCanvas());
var ctx = null;
function ctx2d() { return ctx || (ctx = canvas.getContext('2d')); }
var W = sys.windowWidth, H = sys.windowHeight;
var dpr = canvas.width / W;

// 顶部留给状态栏与右上角胶囊按钮
var safe = sys.safeArea || { top: sys.statusBarHeight || 20, left: 0, right: W, bottom: H };
var hudTop = safe.top + 8;
try {
  var menu = wx.getMenuButtonBoundingClientRect && wx.getMenuButtonBoundingClientRect();
  if (menu && menu.bottom > 0 && menu.bottom < H / 3) hudTop = Math.max(hudTop, menu.bottom + 8);
} catch (e) {}

// ───────── 主循环：dt 封顶 50ms，切后台暂停，回前台不跳帧 ─────────
// render 返回 false 表示这一帧没画（画面没变）：主画布保留上一帧的内容，不必每帧重画
var paused = false, hideCbs = [], showCbs = [];
function loop(update, render) {
  var last = 0;
  function tick(ts) {
    var dt = last ? Math.min((ts - last) / 1000, 0.05) : 1 / 60;
    last = ts;
    if (!paused) update(dt);
    render(ctx2d());
    requestAnimationFrame(tick);
  }
  wx.onShow && wx.onShow(function () { paused = false; last = 0; showCbs.forEach(function (f) { f(); }); });
  wx.onHide && wx.onHide(function () { paused = true; hideCbs.forEach(function (f) { f(); }); });
  requestAnimationFrame(tick);
}

// ───────── 音效：WebAudio 播预合成的短音，没有就静音（不影响玩） ─────────
// 为什么预合成成 AudioBuffer、不每次现建 Oscillator：见 play-snake 的 base.js（真机一次 ~25ms，就是一次掉帧）
var ac = null, muted = false, tones = {}, toneCount = 0;
try { ac = wx.createWebAudioContext ? wx.createWebAudioContext() : null; } catch (e) { ac = null; }
function tone(freq, dur, type, vol, slide) {
  var key = type + '|' + freq + '|' + dur + '|' + vol + '|' + (slide || 1);
  var b = tones[key];
  if (b) return b;
  if (toneCount > 200) { tones = {}; toneCount = 0; }
  var sr = 22050, len = Math.max(1, Math.floor(sr * (dur + 0.02)));
  b = ac.createBuffer(1, len, sr);
  var d = new Float32Array(len), ph = 0, f1 = Math.max(30, freq * (slide || 1)), decay = Math.log(0.001 / vol);
  for (var i = 0; i < len; i++) {
    var t = i / sr, k = Math.min(1, t / dur);
    var f = slide ? freq * Math.pow(f1 / freq, k) : freq;
    ph += f / sr; ph -= Math.floor(ph);
    var w = type === 'sine' ? Math.sin(ph * 2 * Math.PI) : type === 'triangle' ? 1 - 4 * Math.abs(ph - 0.5) :
      type === 'sawtooth' ? 2 * ph - 1 : (ph < 0.5 ? 1 : -1);
    d[i] = w * (t >= dur ? 0 : vol * Math.exp(decay * k));
  }
  if (b.copyToChannel) b.copyToChannel(d, 0); else b.getChannelData(0).set(d);
  toneCount++;
  return (tones[key] = b);
}
function beep(freq, dur, type, vol, slide) {
  if (!ac || muted) return;
  try {
    var s = ac.createBufferSource();
    s.buffer = tone(Math.round(freq / 5) * 5, dur, type || 'square', vol || 0.15, slide);
    s.connect(ac.destination);
    s.start();
  } catch (e) { ac = null; }
}
function buzz(heavy) {
  try {
    if (heavy && wx.vibrateLong) wx.vibrateLong({});
    else if (wx.vibrateShort) wx.vibrateShort({ type: 'light' });
  } catch (e) {}
}

// ───────── 存档（storage 兜底；主存档走文件，见 js/save.js）：读坏了当没有 ─────────
function load(key, def) {
  try { var v = wx.getStorageSync(key); return v === '' || v === undefined || v === null ? def : v; } catch (e) { return def; }
}
function save(key, v) { try { wx.setStorageSync(key, v); } catch (e) {} }

// ───────── 绘制小工具（2D 上下文） ─────────
var FONT = 'sans-serif';
function text(s, x, y, size, color, align, weight) {
  var c = ctx2d();
  c.font = (weight || 'normal') + ' ' + size + 'px ' + FONT;
  c.fillStyle = color || '#fff';
  c.textAlign = align || 'center';
  c.textBaseline = 'middle';
  c.fillText(String(s), x, y);
}
function rrect(x, y, w, h, r, c) {
  c = c || ctx2d();
  r = Math.min(r, w / 2, h / 2);
  c.beginPath();
  c.moveTo(x + r, y);
  c.arcTo(x + w, y, x + w, y + h, r);
  c.arcTo(x + w, y + h, x, y + h, r);
  c.arcTo(x, y + h, x, y, r);
  c.arcTo(x, y, x + w, y, r);
  c.closePath();
}
// 按钮：画出来并返回命中框，点按时用 hit() 判
function button(label, cx, cy, w, h, bg, fg, size) {
  var c = ctx2d();
  rrect(cx - w / 2, cy - h / 2, w, h, Math.min(12, h / 2));
  c.fillStyle = bg || '#ffffff'; c.fill();
  text(label, cx, cy + 1, size || Math.round(h * 0.4), fg || '#333', 'center', 'bold');
  return { x: cx - w / 2, y: cy - h / 2, w: w, h: h };
}
function hit(b, x, y) { return !!b && x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h; }
function clamp(v, a, b) { return v < a ? a : v > b ? b : v; }

module.exports = {
  sys: sys, canvas: canvas, W: W, H: H, dpr: dpr, safe: safe, hudTop: hudTop,
  loop: loop, onHide: function (f) { hideCbs.push(f); }, onShow: function (f) { showCbs.push(f); },
  beep: beep, buzz: buzz, setMuted: function (m) { muted = m; }, isMuted: function () { return muted; },
  load: load, save: save, text: text, rrect: rrect, button: button, hit: hit, clamp: clamp
};
Object.defineProperty(module.exports, 'ctx', { get: ctx2d, enumerable: true });
