// 扫雷（竖屏）：初级 9×9 / 中级 16×16 / 高级 16×30，首次点击必不是雷，空白自动展开，
// 长按插旗（自己算按住时长）、双击已翻开的数字快速开周围、双指缩放 + 单指拖动平移大棋盘、计时。
// 画面走 2D 画布：格子在离屏画布上增量重画（js/view.js），画面没变的帧不画（主画布保留上一帧）。
// 存档走 wx.getFileSystemManager()，USER_DATA_PATH 下的 json（js/save.js）。
var B = require('js/base.js');
var Board = require('js/board.js');
var View = require('js/view.js');
var input = require('js/input.js');
var save = require('js/save.js');
var ai = require('js/ai.js');
var S = require('js/screens.js');
var W = B.W, H = B.H, dpr = B.dpr;

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

// ───────── 版面 ─────────
var barCy = Math.min(H, B.safe.bottom || H) - 34;
var AREA = { x: 8, y: B.hudTop + 48, w: W - 16, h: 0 };
AREA.h = barCy - 50 - AREA.y;

// ───────── 状态 ─────────
var data = save.read();
data.best = data.best || {}; data.stats = data.stats || {};
var scene = 'menu', board = null, view = null, time = 0, mode = 'open', newBest = false;
var dirty = true, lastSec = -1, saveDue = 0, overAt = 0, clock = 0, lastTap = { i: -1, t: 0 };
var press = -1, ui = {}, actions = 0, games = 0, wins = 0, losses = 0, frames = 0, drawn = 0;
var autoLevel = 0, autoAcc = 0;

function saveLine() {
  return '存档：' + (save.info.via === 'file' ? 'USER_DATA_PATH/minesweeper/save.json' : '本地缓存（文件系统不可用）');
}
function persist() {
  if (board && board.state === 'play' && board.planted) {
    var d = board.dump(); d.time = time; data.cur = d;
  } else data.cur = null;
  save.write(data);
  saveDue = 0;
}

function start(level, restored) {
  board = restored ? Board.restore(restored) : new Board(level);
  if (!board) { board = new Board(level); restored = null; }
  view = new View(board, AREA, dpr);
  time = restored ? Number(restored.time) || 0 : 0;
  scene = 'play'; newBest = false; press = -1; lastTap.i = -1; mode = 'open';
  dirty = true;
  if (!restored) { data.cur = null; }
}

function finish() {
  var st = data.stats[board.level] || (data.stats[board.level] = { played: 0, won: 0 });
  st.played++; games++; overAt = clock;
  if (board.state === 'won') {
    st.won++; wins++;
    var t = Math.max(1, Math.round(time));
    if (!data.best[board.level] || t < data.best[board.level]) { data.best[board.level] = t; newBest = true; }
    [784, 988, 1175, 1568].forEach(function (f, k) { setTimeout(function () { B.beep(f, 0.09, 'triangle', 0.1); }, k * 70); });
  } else {
    losses++;
    B.beep(140, 0.5, 'sawtooth', 0.16, 0.3); B.buzz(true);
  }
  persist();
}

// 一步操作（手指、PC、自动试玩都走这里）
function act(type, i) {
  if (!board || board.state !== 'play' || i < 0) return;
  var r = 0;
  if (type === 'flag') {
    if (!board.toggleFlag(i)) return;
    B.beep(board.flag[i] ? 880 : 600, 0.04, 'square', 0.06);
    if (!AUTO) B.buzz();                          // 自动试玩一秒插好几面旗，不每次都震
  } else if (type === 'chord') {
    r = board.chord(i);
    if (!r) return;
  } else {
    r = board.reveal(i);
    if (!r) return;
  }
  if (r > 0) B.beep(r > 8 ? 720 : 520, r > 8 ? 0.09 : 0.05, 'triangle', 0.08, r > 8 ? 1.4 : 0);
  actions++;
  dirty = true; saveDue = saveDue || clock + 0.8;
  if (board.state !== 'play') finish();
}

// ───────── 输入 ─────────
input.on('down', function (x, y) {
  if (scene !== 'play' || !board || board.state !== 'play') return;
  var i = view.cellAt(x, y);
  press = i >= 0 && !board.open[i] && !board.flag[i] ? i : -1;
  dirty = true;
});
input.on('up', function () { if (press >= 0) { press = -1; dirty = true; } });
input.on('pan', function (dx, dy) { if (scene === 'play') { view.pan(dx, dy); dirty = true; } });
input.on('pinch', function (k, cx, cy, dx, dy) {
  if (scene !== 'play') return;
  view.zoomAt(k, cx, cy); view.pan(dx, dy); dirty = true;
});
input.on('long', function (x, y) {
  if (scene !== 'play' || !board) return;
  var i = view.cellAt(x, y);
  if (i < 0) return;
  if (board.open[i]) act('chord', i); else act('flag', i);
});
input.on('tap', function (x, y) {
  dirty = true;
  if (scene === 'menu') {
    S.ORDER.forEach(function (k) { if (B.hit(ui[k], x, y)) start(k); });
    if (B.hit(ui.resume, x, y)) start(data.cur.level, data.cur);
    return;
  }
  if (B.hit(ui.face, x, y)) { if (board.state === 'play' && board.planted) persistAbandon(); start(board.level); return; }
  if (B.hit(ui.menu, x, y)) { persist(); scene = 'menu'; return; }
  if (B.hit(ui.mode, x, y)) { mode = mode === 'open' ? 'flag' : 'open'; B.beep(660, 0.03, 'square', 0.05); return; }
  if (B.hit(ui.fit, x, y)) { view.fit(); return; }
  if (board.state !== 'play') {
    if (clock - overAt < 0.4) return;            // 刚结束那一下别误点到按钮
    if (B.hit(ui.again, x, y)) start(board.level);
    else if (B.hit(ui.levels, x, y)) { scene = 'menu'; }
    return;
  }
  var i = view.cellAt(x, y);
  if (i < 0) return;
  var now = Date.now();
  if (board.open[i]) {
    // 双击数字：两次点按落在同一格、间隔 350ms 内（插旗模式下单击数字就开周围）
    if (mode === 'flag' || (lastTap.i === i && now - lastTap.t < 350)) { act('chord', i); lastTap.i = -1; }
    else { lastTap.i = i; lastTap.t = now; }
    return;
  }
  lastTap.i = -1;
  act(mode === 'flag' ? 'flag' : 'open', i);
});
// 中途重开：算一局没赢
function persistAbandon() {
  var st = data.stats[board.level] || (data.stats[board.level] = { played: 0, won: 0 });
  st.played++;
}

B.onHide(function () { if (scene === 'play') persist(); });
B.onShow(function () { dirty = true; });

// ───────── 自动试玩：初级 → 中级 → 高级轮着来，高级放大后镜头跟着走 ─────────
function autoStep(dt) {
  if (scene === 'menu') { start(S.ORDER[autoLevel % 3]); autoLevel++; if (board.level === 'hard') view.zoomAt(1.5, W / 2, AREA.y + AREA.h / 2); return; }
  if (board.state !== 'play') {
    if (clock - overAt > 1.2) { scene = 'menu'; dirty = true; }
    return;
  }
  autoAcc += dt;
  var every = board.level === 'easy' ? 0.25 : board.level === 'mid' ? 0.14 : 0.09;
  if (autoAcc < every) return;
  autoAcc = 0;
  var m = ai.next(board);
  if (!m) return;
  if (view.zoomed()) view.follow(m.i, 0.35);
  act(m.type, m.i);
}

function update(dt) {
  clock += dt; frames++;
  input.update();
  if (AUTO) autoStep(dt);
  if (scene !== 'play' || !board) return;
  if (board.state === 'play' && board.planted) {
    time = Math.min(5999, time + dt);
    if (Math.floor(time) !== lastSec) { lastSec = Math.floor(time); dirty = true; }
  }
  if (input.holdProgress() >= 0 && press >= 0) dirty = true;    // 长按进度环在走
  if (saveDue && clock >= saveDue) persist();
}

// ───────── 绘制：画面没变就不画 ─────────
function render(ctx) {
  if (view && view.flush()) dirty = true;
  if (!dirty) return;
  dirty = false; drawn++;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  if (scene === 'menu') { ui = S.menu(ctx, { data: data, saveLine: saveLine() }); return; }
  ctx.fillStyle = '#121c30'; ctx.fillRect(0, 0, W, H);
  B.rrect(AREA.x - 2, AREA.y - 2, AREA.w + 4, AREA.h + 4, 10, ctx); ctx.fillStyle = '#0b1324'; ctx.fill();
  ctx.save();
  ctx.beginPath(); ctx.rect(AREA.x, AREA.y, AREA.w, AREA.h); ctx.clip();
  view.draw(ctx);
  if (press >= 0) {
    var r = view.cellRect(press), p = input.holdProgress();
    ctx.fillStyle = 'rgba(255,255,255,0.28)'; ctx.fillRect(r.x, r.y, r.s, r.s);
    if (p > 0.15) {                                // 长按进度环：转满就插旗
      ctx.strokeStyle = '#ff6b6b'; ctx.lineWidth = 3;
      ctx.beginPath(); ctx.arc(r.x + r.s / 2, r.y + r.s / 2, r.s * 0.62, -Math.PI / 2, -Math.PI / 2 + p * Math.PI * 2); ctx.stroke();
    }
  }
  ctx.restore();
  var st = { board: board, time: time, mode: mode, zoomed: view.zoomed(), pressing: press >= 0, data: data, newBest: newBest };
  var h = S.hud(ctx, st), b = S.bar(ctx, st, barCy);
  ui = { face: h.face, menu: b.menu, mode: b.mode, fit: b.fit };
  if (board.state !== 'play') {
    var o = S.over(ctx, st, AREA.y + AREA.h - 90);
    ui.again = o.again; ui.levels = o.levels;
  }
}

GameGlobal.__play = function () {
  return {
    scene: scene, level: board && board.level, state: board && board.state, cols: board && board.cols, rows: board && board.rows,
    mines: board && board.mines, opened: board && board.opened, flags: board && board.flags, time: Math.round(time),
    actions: actions, games: games, wins: wins, losses: losses, zoom: view ? +(view.s / view.minS).toFixed(2) : 0,
    best: data.best, save: { via: save.info.via, writes: save.info.writes, bytes: save.info.bytes, size: save.size(), error: save.info.error },
    input: input.stats, frames: frames, drawn: drawn
  };
};

// 无头走查用：屏上第一个没翻开的格子、第一个数字格的中心（逻辑像素），拿去喂触摸事件
GameGlobal.__probe = function () {
  var out = { hidden: null, number: null };
  if (!board) return out;
  for (var i = 0; i < board.n; i++) {
    var r = view.cellRect(i), c = [Math.round(r.x + r.s / 2), Math.round(r.y + r.s / 2)];
    if (view.cellAt(c[0], c[1]) !== i) continue;   // 不在可见区域
    if (!out.hidden && !board.open[i] && !board.flag[i]) out.hidden = c;
    if (!out.number && board.open[i] && board.num[i]) out.number = c;
  }
  return out;
};

B.loop(update, render);
