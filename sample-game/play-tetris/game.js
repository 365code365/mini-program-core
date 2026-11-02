// 俄罗斯方块（竖屏）：7 种方块、SRS 旋转 + 踢墙、硬降 / 软降、暂存、3 块预览、幽灵块、消行动画、等级提速。
// 触摸：点一下旋转、左右拖动按格移动、快速下滑硬降、按住不动软降、上滑暂存；键盘：←→ 移动（按住连移）、
// ↑ / X 顺时针、Z 逆时针、↓ 软降、空格硬降、C / Shift 暂存、P / Esc 暂停。
// 规则在 js/tetris.js，画面在 js/view.js，自动试玩的落点选择在 js/ai.js。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var T = require('js/tetris.js');
var AI = require('js/ai.js');
var FX = require('js/fx.js');
var R = G2.make(B.canvas, B.W, B.H, B.sys.pixelRatio || 2);
var V = require('js/view.js').make(R, B);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
var CHAOS = AUTO && q.chaos === '1';      // ?auto=1&chaos=1：随机落点，几十秒就堆到顶（覆盖结束 / 新纪录 / 重开）
var SAVE_KEY = 'tetris.save', BEST_KEY = 'tetris.best';

var fx = new FX.Fx();
var state = 'ready', paused = false, clock = 0, overAt = 0, shake = 0, newBest = false, games = 0;
var best = Number(B.load(BEST_KEY, 0)) || 0;
var saved = B.load(SAVE_KEY, null);
if (!saved || typeof saved !== 'object' || !saved.board) saved = null;
var game = new T.Game(onEvent), buttons = [], locksSinceSave = 0;

// 帧率：玩的时候要 60 帧（下落、拖动跟手），停着（开始页 / 暂停 / 结束）降到 30 帧省电
var fpsNow = 0;
function fps(n) { if (n !== fpsNow && wx.setPreferredFramesPerSecond) { wx.setPreferredFramesPerSecond(n); fpsNow = n; } }

function centerX() { return V.OX + V.BW / 2; }
function rowY(r) { return V.OY + (r - T.HIDDEN + 0.5) * V.CELL; }
function onEvent(name, d) {
  if (name === 'move') B.beep(330, 0.025, 'square', 0.035);
  else if (name === 'rotate') B.beep(d ? 700 : 560, 0.04, 'triangle', 0.06);
  else if (name === 'hold') B.beep(440, 0.06, 'sine', 0.08, 1.5);
  else if (name === 'drop') { if (d > 1) shake = 0.12; }
  else if (name === 'lock') {
    B.beep(150, 0.06, 'square', 0.07);
    if (++locksSinceSave >= 5) { locksSinceSave = 0; persist(); }
  } else if (name === 'clear') {
    var names = ['', '单消', '双消', '三消', '四消！'];
    fx.say(names[d.n] + ' +' + d.pts, centerX(), rowY(d.rows[0]) - 6, d.n === 4 ? '#ffd54f' : '#ffffff', d.n === 4 ? 28 : 22);
    if (d.combo > 0) fx.say('连击 ×' + d.combo, centerX(), rowY(d.rows[0]) + 26, '#80deea', 18);
    if (d.b2b) fx.say('连续四消 ×1.5', centerX(), rowY(d.rows[0]) - 40, '#ff8a65', 18);
    d.rows.forEach(function (r) {
      for (var x = 0; x < T.COLS; x += 2) {
        var t = game.board[r * T.COLS + x] || 'I';
        fx.burst(V.OX + (x + 0.5) * V.CELL, rowY(r), V.PAL[t][0], 3);
      }
    });
    var base = [0, 523, 659, 784, 1047][d.n];
    for (var i = 0; i < d.n + 1; i++) (function (k) { setTimeout(function () { B.beep(base * Math.pow(1.26, k), 0.09, 'triangle', 0.1); }, k * 55); })(i);
    // 消行振动：四消用 heavy，其余 medium（wx.vibrateShort 的 type 是基础库 2.13 起的参数，老版本忽略）
    try { wx.vibrateShort && wx.vibrateShort({ type: d.n === 4 ? 'heavy' : 'medium' }); } catch (e) {}
  } else if (name === 'level') {
    fx.say('等级 ' + d, centerX(), V.OY + V.BH * 0.35, '#b388ff', 26);
    B.beep(880, 0.15, 'sine', 0.1, 1.5);
  } else if (name === 'over') {
    state = 'over'; overAt = clock; games++;
    B.beep(220, 0.5, 'sawtooth', 0.12, 0.35);
    if (game.score > best) { best = game.score; newBest = true; B.save(BEST_KEY, best); }
    saved = null; B.save(SAVE_KEY, '');
  }
}
// 存档：开局中途切走 / 暂停 / 每锁 5 块存一次；结束时清掉
function persist() {
  if (state !== 'play' || game.over) return;
  saved = game.serialize();
  B.save(SAVE_KEY, saved);
}
function start(resume) {
  fx.clear(); newBest = false; paused = false; shake = 0;
  if (!(resume && saved && game.restore(saved))) game.reset();
  state = 'play'; plan = null;
}

// ───────── 输入 ─────────
var touch = null, das = { dir: 0, t: 0 };
function tapButton(x, y) {
  for (var i = buttons.length - 1; i >= 0; i--) {
    var b = buttons[i];
    if (!B.hit(b[1], x, y)) continue;
    var id = b[0];
    if (id === 'start') start(false);
    else if (id === 'resume') { if (state === 'ready') start(true); else paused = false; }
    else if (id === 'restart') start(false);
    else if (id === 'pause') { paused = true; persist(); }
    else if (id === 'hold') { if (state === 'play' && !paused) game.doHold(); }
    else continue;
    return true;
  }
  return false;
}
B.on('down', function (x, y, id) {
  if (touch) return;
  touch = { id: id, x0: x, y0: y, ax: x, t0: clock, moved: false, soft: false };
});
B.on('move', function (x, y, id) {
  if (!touch || touch.id !== id || state !== 'play' || paused) return;
  // 跟手：横向每划过约一格就挪一格（不等抬手），一次拖动能连挪几格
  var step = V.CELL * 0.85;
  while (x - touch.ax >= step) { game.move(1); touch.ax += step; touch.moved = true; }
  while (touch.ax - x >= step) { game.move(-1); touch.ax -= step; touch.moved = true; }
});
B.on('up', function (x, y, id) {
  var t = touch;
  if (!t || t.id !== id) return;
  touch = null;
  var dx = x - t.x0, dy = y - t.y0, dt = clock - t.t0, d = Math.sqrt(dx * dx + dy * dy);
  if (d < 14 && dt < 0.35 && !t.soft) { if (!tapButton(x, y) && state === 'play' && !paused) game.rotate(1); return; }
  if (state !== 'play' || paused || t.soft) return;
  if (dy > 50 && dy > Math.abs(dx) * 1.4 && dt < 0.45) game.hardDrop();
  else if (dy < -50 && -dy > Math.abs(dx) * 1.4) game.doHold();
});
B.on('key', function (k) {
  if (state !== 'play') { if (k === 'ok') start(state === 'ready' && !!saved); return; }
  if (k === 'pause') { paused = !paused; if (paused) persist(); return; }
  if (paused) { if (k === 'ok') paused = false; return; }
  if (k === 'left' || k === 'right') { var s = k === 'left' ? -1 : 1; game.move(s); das.dir = s; das.t = 0.16; }
  else if (k === 'up' || k === 'cw') game.rotate(1);
  else if (k === 'ccw') game.rotate(-1);
  else if (k === 'ok') game.hardDrop();
  else if (k === 'hold') game.doHold();
});
B.onHide(function () { if (state === 'play') { paused = true; persist(); } });

// 键盘按住左右：0.16s 后每 0.05s 连移一格（DAS / ARR）
function keyRepeat(dt) {
  if (!das.dir) return;
  if (!B.held[das.dir < 0 ? 'left' : 'right']) { das.dir = 0; return; }
  das.t -= dt;
  while (das.t <= 0) { game.move(das.dir); das.t += 0.05; }
}

// ───────── 自动试玩：每 0.05s 按一下「暂存 / 旋转 / 左右 / 下落」，执行 js/ai.js 选好的落点 ─────────
var plan = null, planFor = -1, autoT = 0, autoSoft = 0;
function autoPlay(dt) {
  if (!game.cur || game.clearing) { plan = null; return; }
  autoT -= dt;
  if (autoT > 0) return;
  autoT = 0.05;
  var id = game.pieces * 2 + (game.canHold ? 0 : 1);
  if (!plan || planFor !== id) {
    plan = CHAOS ? { hold: false, r: Math.floor(Math.random() * 4), x: Math.floor(Math.random() * 9) } : AI.plan(game);
    planFor = id; autoSoft = 0;
  }
  if (!plan) return;
  if (plan.hold) { game.doHold(); plan = null; return; }
  var c = game.cur;
  if (c.r !== plan.r) { if (!game.rotate(plan.r === (c.r + 3) % 4 ? -1 : 1)) plan.r = c.r; return; }
  if (c.x !== plan.x) { if (!game.move(plan.x > c.x ? 1 : -1)) plan.x = c.x; return; }
  // 每 6 块里有 1 块用软降落到底（覆盖软降与锁定延迟），其余硬降
  if (game.pieces % 6 === 5) { autoSoft = 1; return; }
  game.hardDrop();
}

function update(dt) {
  clock += dt;
  fx.update(dt);
  if (shake > 0) shake = Math.max(0, shake - dt);
  if (AUTO && state === 'ready') start(false);
  if (AUTO && state === 'over' && clock - overAt > 1.5) start(false);
  fps(state === 'play' && !paused ? 60 : 30);
  if (state !== 'play' || paused) return;
  if (touch && !touch.moved && !touch.soft && clock - touch.t0 > 0.3) touch.soft = true;   // 按住不动 0.3s = 软降
  keyRepeat(dt);
  if (AUTO) autoPlay(dt);
  var soft = (touch && touch.soft) || !!B.held.down || (AUTO && autoSoft > 0);
  game.update(dt, soft);
}
function render() {
  buttons = V.draw({ game: game, state: state, paused: paused, best: best, newBest: newBest, saved: saved,
    clock: clock, overAt: overAt, shake: shake, fx: fx });
}

GameGlobal.__play = function () {
  return { state: state, paused: paused, score: game.score, lines: game.lines, level: game.level, pieces: game.pieces,
    best: best, games: games, hold: game.hold, next: game.queue.slice(0, 3).join(''), stats: game.stats, cell: V.CELL };
};

fps(30);
B.loop(update, render);
