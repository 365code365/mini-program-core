// 萌果消消乐（竖屏）：8×8 交换消除，4 连出条纹、L / T 型出炸弹、5 连出彩虹球，连锁越多分越高、音越高。
// 12 关步数关卡，每关要收集指定数量的某种水果；过关 1 星，分数过两条星线各加 1 星。排行榜走开放数据域。
// 输入：在水果上往相邻方向滑一下 = 交换；或先点一个、再点相邻的一个。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var T = require('js/tween.js');
var FX = require('js/fx.js');
var F = require('js/fruit.js');
var LV = require('js/levels.js');
var Match = require('js/match.js');
var Rank = require('js/rank.js');
var UI = require('js/ui.js');
var W = B.W, H = B.H, dpr = B.sys.pixelRatio || 2;
var R = G2.make(B.canvas, W, H, dpr);
var gl = R.gl;

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

var top = Math.max(B.hudTop, 44);
var CELL = Math.floor(Math.min((W - 24) / 8, (H - top - 116 - 70) / 8));
var geo = { CELL: CELL, OX: Math.floor((W - CELL * 8) / 2), OY: top + 116 };
var ui = UI.make(R, W, H, top);
var M = new Match(geo, { beep: B.beep, buzz: B.buzz });

// ───────── 存档：解锁到第几关、每关星数与最高分、本机成绩记录（排行榜用） ─────────
var prog = B.load('m3.progress', null);
if (!prog || typeof prog !== 'object') prog = { unlocked: 1, stars: {}, best: {} };
var records = B.load('m3.records', []);
if (!Array.isArray(records)) records = [];
function total() { var s = 0; for (var k in prog.best) s += prog.best[k]; return s; }
var S = { prog: prog, clock: 0, muted: false, geo: geo, total: total, newBest: false, resultAge: 0 };
B.setMuted(!!B.load('m3.muted', false)); S.muted = B.isMuted();

var screen = 'menu', sel = null, drag = null, idle = 0, hint = null, autoT = 0, rankAuto = 0, played = 0;

function today() {
  var d = new Date(), p = function (n) { return (n < 10 ? '0' : '') + n; };
  return d.getFullYear() + '-' + p(d.getMonth() + 1) + '-' + p(d.getDate()) + ' ' + p(d.getHours()) + ':' + p(d.getMinutes());
}
function finish() {
  var res = M.result, no = M.no;
  S.newBest = false; S.resultAge = 0;
  if (res.win) {
    if (res.score > (prog.best[no] || 0)) { prog.best[no] = res.score; S.newBest = true; }
    prog.stars[no] = Math.max(prog.stars[no] || 0, res.stars);
    prog.unlocked = Math.max(prog.unlocked, Math.min(LV.list.length, no + 1));
    if (no === LV.list.length) prog.unlocked = LV.list.length;
    B.save('m3.progress', prog);
    Rank.report(total());
  }
  records.forEach(function (r) { r.latest = false; });
  records.push({ level: no, score: res.score, stars: res.stars, win: res.win, date: today(), latest: true });
  records.sort(function (a, b) { return b.score - a.score; });
  records = records.slice(0, 10);
  B.save('m3.records', records);
  played++;
  screen = 'result';
}

var A = {
  play: function (no) { Rank.close(); sel = null; hint = null; idle = 0; M.start(no); screen = 'play'; },
  retry: function () { A.play(M.no || 1); },
  next: function () { A.play(Math.min(LV.list.length, (M.no || 0) + 1)); },
  menu: function () { Rank.close(); screen = 'menu'; },
  rank: function () { Rank.open({ name: '我', score: total() }, records); },
  mute: function () { S.muted = !S.muted; B.setMuted(S.muted); B.save('m3.muted', S.muted); }
};

// ───────── 输入 ─────────
function cellAt(x, y) {
  var c = Math.floor((x - geo.OX) / CELL), r = Math.floor((y - geo.OY) / CELL);
  return r >= 0 && c >= 0 && r < 8 && c < 8 ? [r, c] : null;
}
function swap(a, b) { if (M.trySwap(a[0], a[1], b[0], b[1])) { sel = null; hint = null; idle = 0; } }
B.on('down', function (x, y) {
  drag = null;
  if (screen !== 'play' || Rank.isOpen() || M.result) return;
  var p = cellAt(x, y); if (p) drag = { p: p, x: x, y: y };
});
B.on('move', function (x, y) {
  if (!drag) return;
  var dx = x - drag.x, dy = y - drag.y;
  if (Math.max(Math.abs(dx), Math.abs(dy)) < CELL * 0.4) return;
  var p = drag.p, to = Math.abs(dx) > Math.abs(dy) ? [p[0], p[1] + (dx > 0 ? 1 : -1)] : [p[0] + (dy > 0 ? 1 : -1), p[1]];
  drag = null;
  if (to[0] >= 0 && to[1] >= 0 && to[0] < 8 && to[1] < 8) swap(p, to);
});
B.on('up', function () { drag = null; });
B.on('tap', function (x, y) {
  if (Rank.isOpen()) { Rank.close(); return; }                  // 排行面板：点哪儿都关
  for (var i = ui.btns.length - 1; i >= 0; i--) {
    if (B.hit(ui.btns[i], x, y)) { B.beep(660, 0.04, 'sine', 0.06); ui.btns[i].fn(ui.btns[i].no); return; }
  }
  if (screen !== 'play' || M.result || M.phase !== 'idle') return;
  var p = cellAt(x, y); if (!p) { sel = null; return; }
  if (sel && Math.abs(sel[0] - p[0]) + Math.abs(sel[1] - p[1]) === 1) swap(sel, p);
  else sel = sel && sel[0] === p[0] && sel[1] === p[1] ? null : p;
});
B.on('key', function (k) {
  if (k === 'ok') { if (screen === 'menu') A.play(Math.min(prog.unlocked, LV.list.length)); else if (screen === 'result') (M.result.win ? A.next : A.retry)(); }
  if (k === 'pause' && screen === 'play') A.menu();
});

// ───────── 自动试玩：棋盘空闲就挑最好的一步；结算后开一下排行榜，再进下一关 / 重来 ─────────
function auto(dt) {
  autoT += dt;
  if (screen === 'menu' && autoT > 0.6) { autoT = 0; A.play(Math.min(prog.unlocked, LV.list.length)); }
  else if (screen === 'play' && M.phase === 'idle' && !M.result && autoT > 0.25) {
    autoT = 0;
    var m = M.bestMove(); if (m) swap([m.r1, m.c1], [m.r2, m.c2]);
  } else if (screen === 'result') {
    if (q.hold === '1') return;                                  // ?auto=1&hold=1：停在第一次结算（截结算面板用）
    if (!Rank.isOpen() && rankAuto === 0 && autoT > 1) { A.rank(); rankAuto = 1; }
    else if (autoT > 2.4) { Rank.close(); rankAuto = 0; autoT = 0; (M.result.win ? A.next : A.retry)(); }
  }
}

function update(dt) {
  S.clock += dt;
  T.update(dt); FX.update(dt);
  if (screen === 'play') {
    M.update();
    if (M.result && M.phase === 'idle' && !T.busy()) finish();
    if (M.phase === 'idle' && !M.result) {
      idle += dt;
      if (idle > 6 && !hint) hint = M.bestMove();               // 6 秒没动：提示一步（两个水果一起轻轻跳）
    }
  }
  if (screen === 'result') S.resultAge += dt;
  if (AUTO) auto(dt);
}

// ───────── 绘制 ─────────
function drawCells() {
  var g = M.board.g, half = CELL / 2, fr = CELL * 0.4;
  R.end();                                                       // 新掉进来的水果从棋盘上沿外面落进来：裁到棋盘里
  gl.enable(gl.SCISSOR_TEST);
  gl.scissor(Math.round(geo.OX * dpr), Math.round((H - geo.OY - CELL * 8) * dpr), Math.round(CELL * 8 * dpr), Math.round(CELL * 8 * dpr));
  if (sel) R.rrect(geo.OX + sel[1] * CELL + 2, geo.OY + sel[0] * CELL + 2, CELL - 4, CELL - 4, 8, 'rgba(255,193,7,0.55)');
  // 停在原位、不缩放的普通水果 / 条纹录成一个图层（一次 drawArrays）；签名 = 这些格子的 id 与位置的哈希，变了才重录。
  // 为什么：每个水果一个精灵是 64 次「flush + 绑缓冲 + 设 uniform + drawArrays」过桥，无头一帧多 ~13ms（实测）；
  // 交换 / 消除 / 下落时只有动着的那几个走精灵
  var h = 7;
  for (var r0 = 0; r0 < 8; r0++) for (var c0 = 0; c0 < 8; c0++) {
    var y0 = g[r0][c0];
    h = (Math.imul(h, 31) + (isStatic(y0, r0, c0) ? y0.id : 0)) | 0;
  }
  R.layer('cells', h, recordStatic);
  for (var r = 0; r < 8; r++) for (var c = 0; c < 8; c++) {
    var x = g[r][c]; if (!x || x.a <= 0.01 || x.s <= 0.01 || isStatic(x, r, c)) continue;
    var cx = x.x + half, cy = x.y + half, s = x.s;
    if (hint && ((hint.r1 === r && hint.c1 === c) || (hint.r2 === r && hint.c2 === c))) s *= 1 + Math.abs(Math.sin(S.clock * 6)) * 0.12;
    if (x.sp === 'r') { R.sprite('rainbow', CELL, function () { F.rainbow(R, fr); }, cx, cy, s * (1 + Math.sin(S.clock * 5) * 0.05), x.a); continue; }
    R.sprite('f' + x.t, CELL, makeFruit(x.t), cx, cy, s, x.a);
    if (x.sp === 'h' || x.sp === 'v') R.sprite('s' + x.sp, CELL, x.sp === 'h' ? stripeH : stripeV, cx, cy, s, x.a);
    else if (x.sp === 'b') R.sprite('bomb', CELL, bombFn, cx, cy, s * (1 + Math.sin(S.clock * 7) * 0.06), x.a);
  }
  FX.draw(R, { x: geo.OX, y: geo.OY, w: CELL * 8, h: CELL * 8 });
  R.end();
  gl.disable(gl.SCISSOR_TEST);
}
// 静态 = 在自己那格、没缩放没变透明、不是会呼吸的炸弹 / 彩虹球、不是提示中的那两格
function isStatic(x, r, c) {
  if (!x || x.sp === 'b' || x.sp === 'r') return false;
  if (hint && ((hint.r1 === r && hint.c1 === c) || (hint.r2 === r && hint.c2 === c))) return false;
  return Math.abs(x.x - (geo.OX + c * CELL)) < 0.01 && Math.abs(x.y - (geo.OY + r * CELL)) < 0.01 &&
    Math.abs(x.s - 1) < 0.001 && Math.abs(x.a - 1) < 0.001;
}
function recordStatic() {
  var g = M.board.g, half = CELL / 2, fr = CELL * 0.4;
  for (var r = 0; r < 8; r++) for (var c = 0; c < 8; c++) {
    var x = g[r][c]; if (!isStatic(x, r, c)) continue;
    R.save(); R.translate(x.x + half, x.y + half);
    F.fruit(R, x.t, fr);
    if (x.sp === 'h' || x.sp === 'v') F.stripes(R, fr, x.sp);
    R.restore();
  }
}
// 精灵的录制函数只在第一次（或图集重置后）调，提前建好，帧里不再造闭包
var fruitFns = [0, 1, 2, 3, 4, 5].map(function (t) { return function () { F.fruit(R, t, CELL * 0.4); }; });
function makeFruit(t) { return fruitFns[t]; }
function stripeH() { F.stripes(R, CELL * 0.4, 'h'); }
function stripeV() { F.stripes(R, CELL * 0.4, 'v'); }
function bombFn() { F.bombRing(R, CELL * 0.4); }

function render() {
  R.begin('#fff1d0');
  ui.reset();
  ui.background();
  if (screen === 'menu') ui.menu(S, A);
  else {
    ui.boardBg(geo);
    drawCells();
    ui.hud(S, M, A);
    FX.drawFloats(R);
    if (screen === 'result') ui.result(S, M, A);
  }
  if (Rank.isOpen()) {
    R.rect(0, 0, W, H, 'rgba(0,0,0,0.5)');
    var rx = (W - Rank.W) / 2, ry = Math.max(top, (H - Rank.H) / 2 - 20);
    Rank.draw(R, rx, ry, records);
    R.text('点任意处关闭', W / 2, ry + Rank.H - 22, 13, '#a1887f', 'center');
  }
  R.end();
}

Rank.init(dpr);
B.onHide(function () { drag = null; });

GameGlobal.__play = function () {
  return {
    screen: screen, level: M.no || 0, moves: M.moves, score: M.score || 0, combo: M.combo, phase: M.phase,
    goals: (M.goals || []).map(function (g) { return LV.NAMES[g.t] + ' ' + (g.n - g.left) + '/' + g.n; }),
    result: M.result, stats: M.stats, played: played, unlocked: prog.unlocked, stars: prog.stars, total: total(),
    rank: { open: Rank.isOpen(), source: Rank.source(), records: records.length }, cell: CELL
  };
};

B.loop(update, render);
