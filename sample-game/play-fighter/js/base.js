// 可玩示例共用的小底座（四个 play-* 各拷一份，保持每个包自包含，能单独打包上传）。
// play-fighter 这份另加：'noise' 波形（拳脚打击声）、原始按键事件 keydown / keyup（格斗要同时按住方向和攻击键）。
// 只用微信小游戏公开 API：wx.createCanvas / requestAnimationFrame / wx.onTouch* / wx.onKeyDown /
// wx.createWebAudioContext / wx.getStorageSync / wx.vibrateShort / wx.onShow / wx.onHide。
// 坐标一律是逻辑像素（windowWidth × windowHeight）：主画布是物理像素，每帧开头按 dpr 设一次变换。
// 主画布的 2D 上下文**用到才拿**（B.ctx 第一次被读时）：一块画布只能有一种上下文，
// WebGL 的游戏（play-jump）要对同一块主画布 getContext('webgl')，先拿了 2D 就拿不到了
var sys = wx.getSystemInfoSync();
// 第一次 wx.createCanvas() 拿到的是上屏的主画布，之后的都是离屏的：与 js/adapter.js（three.js 的游戏）共用同一块，
// 谁先 require 谁建，存在 GameGlobal.__mainCanvas
var canvas = GameGlobal.__mainCanvas || (GameGlobal.__mainCanvas = wx.createCanvas());
var ctx = null;
function ctx2d() { return ctx || (ctx = canvas.getContext('2d')); }
var W = sys.windowWidth, H = sys.windowHeight;
var dpr = canvas.width / W;

// 顶部留给状态栏与右上角胶囊按钮；横屏时左右两侧可能有刘海
var safe = sys.safeArea || { top: sys.statusBarHeight || 20, left: 0, right: W, bottom: H };
var hudTop = safe.top + 8;
try {
  var menu = wx.getMenuButtonBoundingClientRect && wx.getMenuButtonBoundingClientRect();
  if (menu && menu.bottom > 0 && menu.bottom < H / 3) hudTop = Math.max(hudTop, menu.bottom + 8);
} catch (e) {}

// ───────── 主循环：dt 封顶 50ms，切后台暂停，回前台不跳帧 ─────────
var paused = false, hideCbs = [], showCbs = [], errors = 0;
function loop(update, render) {
  var last = 0;
  function tick(ts) {
    var dt = last ? Math.min((ts - last) / 1000, 0.05) : 1 / 60;
    last = ts;
    // 先排下一帧再跑：update / render 抛异常时循环不断（否则画面定死、日志里什么都没有），异常只打印前几次
    requestAnimationFrame(tick);
    try {
      if (!paused) update(dt);
      if (ctx) ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      render(ctx);
    } catch (e) {
      if (errors++ < 3) console.error('[loop] ' + (e && e.stack || e));
    }
  }
  wx.onShow && wx.onShow(function () { paused = false; last = 0; showCbs.forEach(function (f) { f(); }); });
  wx.onHide && wx.onHide(function () { paused = true; hideCbs.forEach(function (f) { f(); }); });
  requestAnimationFrame(tick);
}

// ───────── 输入：按下 / 移动 / 抬起 + 归一好的点按与滑动；键盘（PC）给方向与确认 ─────────
var handlers = {};
var starts = {};                          // identifier → { x, y, t }
// 同一事件可挂多个处理函数（格斗里屏幕按键和菜单各听各的 down）
function on(name, f) { (handlers[name] || (handlers[name] = [])).push(f); }
function emit(name, a, b, c) { var fs = handlers[name]; if (fs) for (var i = 0; i < fs.length; i++) fs[i](a, b, c); }
wx.onTouchStart(function (e) {
  e.changedTouches.forEach(function (p) {
    starts[p.identifier] = { x: p.clientX, y: p.clientY, t: e.timeStamp };
    emit('down', p.clientX, p.clientY, p.identifier);
  });
});
wx.onTouchMove(function (e) {
  e.changedTouches.forEach(function (p) { emit('move', p.clientX, p.clientY, p.identifier); });
});
function ended(e, cancelled) {
  e.changedTouches.forEach(function (p) {
    var s = starts[p.identifier]; delete starts[p.identifier];
    emit('up', p.clientX, p.clientY, p.identifier);
    if (!s || cancelled) return;
    var dx = p.clientX - s.x, dy = p.clientY - s.y, d = Math.sqrt(dx * dx + dy * dy);
    if (d < 20) emit('tap', p.clientX, p.clientY);
    else if (Math.abs(dx) > Math.abs(dy)) emit('swipe', dx > 0 ? 'right' : 'left');
    else emit('swipe', dy > 0 ? 'down' : 'up');
  });
}
wx.onTouchEnd(function (e) { ended(e, false); });
wx.onTouchCancel(function (e) { ended(e, true); });

var KEYS = { ArrowUp: 'up', KeyW: 'up', w: 'up', ArrowDown: 'down', KeyS: 'down', s: 'down',
  ArrowLeft: 'left', KeyA: 'left', a: 'left', ArrowRight: 'right', KeyD: 'right', d: 'right',
  Space: 'ok', ' ': 'ok', Enter: 'ok', KeyP: 'pause', p: 'pause', Escape: 'pause' };
var held = {};
if (wx.onKeyDown) {
  wx.onKeyDown(function (e) {
    emit('keydown', e.code || e.key);
    var k = KEYS[e.code] || KEYS[e.key]; if (!k) return;
    if (!held[k]) emit('key', k);
    held[k] = true;
  });
  wx.onKeyUp && wx.onKeyUp(function (e) { emit('keyup', e.code || e.key); var k = KEYS[e.code] || KEYS[e.key]; if (k) held[k] = false; });
}

// ───────── 音效：WebAudio 播短音，没有就静音（不影响玩） ─────────
// 每种音（波形、频率、时长、音量、滑音）第一次用时在 JS 里合成成一段 AudioBuffer 缓存起来，之后每次只建一个 BufferSource 播。
// 为什么不每次现建 Oscillator：本引擎里 createOscillator + 设 type / frequency 在真机上一次 ~25ms（一加 PLY110 实测
// 16ms + 9ms），2048 每滑一下响一声，这一下就是一次掉帧；BufferSource 一次 ~0.6ms。频率按 5Hz 取整，避免随机音高把缓存撑爆
var ac = null, muted = false, tones = {}, toneCount = 0;
try { ac = wx.createWebAudioContext ? wx.createWebAudioContext() : null; } catch (e) { ac = null; }
function tone(freq, dur, type, vol, slide) {
  var key = type + '|' + freq + '|' + dur + '|' + vol + '|' + (slide || 1);
  var b = tones[key];
  if (b) return b;
  if (toneCount > 200) { tones = {}; toneCount = 0; }
  // 22.05kHz 合成就够短促音效用（混音时引擎重采样），第一次合成的 JS 循环省一半
  var sr = 22050, len = Math.max(1, Math.floor(sr * (dur + 0.02)));
  b = ac.createBuffer(1, len, sr);
  var d = new Float32Array(len), ph = 0, f1 = Math.max(30, freq * (slide || 1)), decay = Math.log(0.001 / vol);
  for (var i = 0; i < len; i++) {
    var t = i / sr, k = Math.min(1, t / dur);
    var f = slide ? freq * Math.pow(f1 / freq, k) : freq;      // 指数滑音（= exponentialRampToValueAtTime）
    ph += f / sr; ph -= Math.floor(ph);
    var w = type === 'noise' ? Math.random() * 2 - 1 : type === 'sine' ? Math.sin(ph * 2 * Math.PI) : type === 'triangle' ? 1 - 4 * Math.abs(ph - 0.5) :
      type === 'sawtooth' ? 2 * ph - 1 : (ph < 0.5 ? 1 : -1);
    var env = t >= dur ? 0 : vol * Math.exp(decay * k);        // 指数衰减到 0.001（= gain 的 exponentialRamp）
    d[i] = w * env;
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
function buzz() { try { wx.vibrateShort && wx.vibrateShort({ type: 'light' }); } catch (e) {} }

// ───────── 存档：读坏了当没有 ─────────
function load(key, def) {
  try { var v = wx.getStorageSync(key); return v === '' || v === undefined || v === null ? def : v; } catch (e) { return def; }
}
function save(key, v) { try { wx.setStorageSync(key, v); } catch (e) {} }

// ───────── 绘制小工具 ─────────
var FONT = 'sans-serif';
function text(s, x, y, size, color, align, weight) {
  var ctx = ctx2d();
  ctx.font = (weight || 'normal') + ' ' + size + 'px ' + FONT;
  ctx.fillStyle = color || '#fff';
  ctx.textAlign = align || 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(s), x, y);
}
function rrect(x, y, w, h, r) {
  var ctx = ctx2d();
  r = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}
// 按钮：画出来并返回命中框，点按时用 hit() 判
function button(label, cx, cy, w, h, bg, fg) {
  var ctx = ctx2d();
  rrect(cx - w / 2, cy - h / 2, w, h, h / 2);
  ctx.fillStyle = bg || '#ffffff'; ctx.fill();
  text(label, cx, cy + 1, Math.round(h * 0.42), fg || '#333', 'center', 'bold');
  return { x: cx - w / 2, y: cy - h / 2, w: w, h: h };
}
function hit(b, x, y) { return !!b && x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h; }
function clamp(v, a, b) { return v < a ? a : v > b ? b : v; }
function rand(a, b) { return a + Math.random() * (b - a); }

module.exports = {
  sys: sys, canvas: canvas, W: W, H: H, dpr: dpr, safe: safe, hudTop: hudTop,
  loop: loop, on: on, held: held,
  onHide: function (f) { hideCbs.push(f); }, onShow: function (f) { showCbs.push(f); },
  beep: beep, buzz: buzz, setMuted: function (m) { muted = m; }, isMuted: function () { return muted; },
  load: load, save: save,
  text: text, rrect: rrect, button: button, hit: hit, clamp: clamp, rand: rand
};
Object.defineProperty(module.exports, 'ctx', { get: ctx2d, enumerable: true });
