// 五子棋（竖屏）：15×15 人机对战，AI 是五元组评分 + α-β 剪枝（workers/ai.js），三档难度；悔棋、认输、连五高亮、战绩与对局存档。
// AI 在 Worker 里算（wx.createWorker，game.json 配 "workers"），主线程照常出帧、「思考中」动画不卡；
// 没有 Worker（或 Worker 报错）时退回主线程：先画出「思考中」那一帧，下一个定时器里再算。
var B = require('js/base.js');
var Rules = require('js/rules.js');
var V = require('js/view.js').make(B);
var N = Rules.N;

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
var SAVE_KEY = 'gomoku.save', REC_KEY = 'gomoku.record', SET_KEY = 'gomoku.settings';
var MIN_THINK = AUTO ? 0.12 : 0.35;      // AI 太快落子没有对弈感：至少「想」这么久

var settings = B.load(SET_KEY, null) || {};
var nextLevel = [1, 2, 3].indexOf(settings.level) >= 0 ? settings.level : 2;
var nextHuman = settings.human === 2 ? 2 : 1;
if (AUTO && q.level) nextLevel = Math.max(1, Math.min(3, +q.level || 2));
var records = B.load(REC_KEY, null);
if (!records || typeof records !== 'object') records = {};
[1, 2, 3].forEach(function (lv) { if (!records[lv]) records[lv] = { w: 0, l: 0, d: 0, streak: 0, bestStreak: 0, fastest: 0 }; });

var board = [], moves = [], human = 1, level = 2, state = 'play', result = null, win = null;
var clock = 0, overAt = 0, ghost = -1, buttons = [], aiMs = -1, nodes = 0, undos = 0, games = 0;
var results = { human: 0, ai: 0, draw: 0 };
function turn() { return moves.length % 2 === 0 ? 1 : 2; }
function humanStones() { var n = 0; for (var k = 0; k < moves.length; k++) if ((k % 2 === 0 ? 1 : 2) === human) n++; return n; }

// ───────── AI：Worker 优先 ─────────
var aiWorker = null, mode = 'worker', mainAI = null, reqId = 0, pending = null;
function makeWorker() {
  try {
    if (!wx.createWorker) throw new Error('wx.createWorker 不可用');
    aiWorker = wx.createWorker('workers/index.js');
    aiWorker.onMessage(onAI);
    if (aiWorker.onError) aiWorker.onError(function (e) { console.warn('[gomoku] Worker 出错，改在主线程算：' + (e && e.message)); fallback(); });
    // 系统回收了 Worker（iOS 内存紧张时会）：重建一个，正在等的那一步重新问
    if (aiWorker.onProcessKilled) aiWorker.onProcessKilled(function () { aiWorker = null; makeWorker(); if (pending) send(pending); });
    mode = 'worker';
  } catch (e) { console.warn('[gomoku] ' + e.message + '，AI 在主线程算'); fallback(); }
}
function fallback() {
  aiWorker = null; mode = 'main';
  if (pending) send(pending);
}
function send(p) {
  var msg = { cmd: 'think', id: p.id, moves: moves.slice(), side: p.side, level: p.level };
  if (aiWorker) { aiWorker.postMessage(msg); return; }
  setTimeout(function () {
    if (!mainAI) mainAI = require('workers/ai.js');
    var r = mainAI.think(msg.moves, msg.side, msg.level);
    onAI({ cmd: 'move', id: msg.id, move: r.move, ms: r.ms, nodes: r.nodes });
  }, 16);
}
function ask(side, lv) {
  pending = { id: ++reqId, side: side, level: lv, t0: clock };
  send(pending);
}
function onAI(m) {
  if (!m || m.cmd !== 'move' || !pending || m.id !== pending.id) return;   // 悔棋 / 新局之后迟到的结果
  var p = pending, wait = Math.max(0, MIN_THINK - (clock - p.t0));
  setTimeout(function () {
    if (pending !== p) return;
    pending = null;
    if (p.side !== human || AUTO) { aiMs = m.ms; nodes = m.nodes; }
    if (m.move >= 0) play(m.move);
  }, wait * 1000);
}

// ───────── 对局 ─────────
function persist() {
  if (state === 'play' && moves.length) B.save(SAVE_KEY, { moves: moves, human: human, level: level });
  else B.save(SAVE_KEY, '');
}
function start() {
  pending = null; ghost = -1; win = null; result = null; aiMs = -1; nodes = 0;
  board = []; for (var i = 0; i < N * N; i++) board.push(0);
  moves = []; level = nextLevel; human = nextHuman; state = 'play'; autoUndone = false;
  B.save(SET_KEY, { level: nextLevel, human: nextHuman });
  persist();
  if (turn() !== human) ask(3 - human, level);
}
function restore(s) {
  if (!s || !Array.isArray(s.moves) || !s.moves.length) return false;
  nextLevel = [1, 2, 3].indexOf(s.level) >= 0 ? s.level : nextLevel; nextHuman = s.human === 2 ? 2 : 1;
  start();
  for (var i = 0; i < s.moves.length; i++) {
    var m = s.moves[i];
    if (!(m >= 0 && m < N * N) || board[m]) { start(); return false; }
    board[m] = turn(); moves.push(m);
  }
  if (turn() !== human) ask(3 - human, level);
  return true;
}
function play(i) {
  if (state !== 'play' || i < 0 || board[i]) return false;
  var p = turn();
  board[i] = p; moves.push(i);
  B.beep(p === 1 ? 420 : 520, 0.05, 'triangle', 0.12);
  var line = Rules.fiveAt(board, i);
  if (line) { win = line; end(p === human ? 'human' : 'ai'); return true; }
  if (moves.length === N * N) { end('draw'); return true; }
  persist();
  if (turn() !== human) ask(3 - human, level);
  return true;
}
function end(r) {
  state = 'over'; result = r; overAt = clock; pending = null; games++;
  var rec = records[level];
  if (r === 'human') {
    rec.w++; rec.streak++; rec.bestStreak = Math.max(rec.bestStreak, rec.streak);
    if (!rec.fastest || moves.length < rec.fastest) rec.fastest = moves.length;
    results.human++;
    [523, 659, 784, 1047].forEach(function (f, k) { setTimeout(function () { B.beep(f, 0.12, 'triangle', 0.12); }, k * 90); });
  } else if (r === 'draw') { rec.d++; results.draw++; }
  else {
    rec.l++; rec.streak = 0; results.ai++;
    B.beep(330, 0.4, 'sawtooth', 0.1, 0.5);
  }
  B.buzz();
  B.save(REC_KEY, records);
  persist();
}
// 悔棋：撤到「轮到你」为止，并且至少撤掉你自己的一手（AI 正在想就连它这一步一起取消）
function undo() {
  if (state !== 'play' || !humanStones()) return false;
  pending = null;
  while (moves.length) {
    var i = moves.pop(); board[i] = 0;
    if ((moves.length % 2 === 0 ? 1 : 2) === human) break;   // 刚撤掉的是你的子
  }
  undos++; persist();
  B.beep(300, 0.06, 'sine', 0.08, 0.7);
  if (!AUTO && wx.showToast) wx.showToast({ title: '已悔棋', icon: 'none', duration: 800 });
  return true;
}
function resign() {
  if (state !== 'play') return;
  if (!wx.showModal) { end('resign'); return; }
  wx.showModal({
    title: '认输', content: '确定认输吗？本局将记为负。', confirmText: '认输', cancelText: '再想想',
    success: function (r) { if (r.confirm && state === 'play') end('resign'); }
  });
}
function newGame() {
  if (state === 'play' && humanStones() > 0 && wx.showModal) {
    wx.showModal({
      title: '重新开始', content: '当前对局还没下完，重新开始将记为负。', confirmText: '重新开始', cancelText: '继续下',
      success: function (r) { if (r.confirm) { if (state === 'play') end('resign'); start(); } }
    });
    return;
  }
  start();
}
// 难度 / 先后手：还没落过子就立刻换一局，下到一半则下一局生效
function setting(lv, hm) {
  if (lv) nextLevel = lv;
  if (hm) nextHuman = hm;
  B.save(SET_KEY, { level: nextLevel, human: nextHuman });
  if (state === 'over' || humanStones() === 0) start();
}

// ───────── 输入：按下显示准星，抬手落在准星那一点（手指挡着也看得见落哪） ─────────
var touch = null;
function myTurn() { return state === 'play' && !pending && turn() === human && !AUTO; }
B.on('down', function (x, y, id) {
  if (touch) return;
  touch = { id: id, x: x, y: y };
  ghost = myTurn() ? V.pick(x, y) : -1;
});
B.on('move', function (x, y, id) { if (touch && touch.id === id && ghost >= 0) { var g = V.pick(x, y); if (g >= 0) ghost = g; } });
B.on('up', function (x, y, id) {
  if (!touch || touch.id !== id) return;
  var t = touch; touch = null;
  if (ghost >= 0) { var g = ghost; ghost = -1; if (myTurn()) play(g); return; }
  if (Math.abs(x - t.x) > 20 || Math.abs(y - t.y) > 20) return;
  for (var i = 0; i < buttons.length; i++) {
    var b = buttons[i]; if (!B.hit(b[1], x, y)) continue;
    var k = b[0];
    if (k === 'undo') undo(); else if (k === 'resign') resign(); else if (k === 'new') newGame();
    else if (k.indexOf('lv') === 0) setting(+k.slice(2), 0);
    else if (k.indexOf('first') === 0) setting(0, +k.slice(5));
    return;
  }
});
B.on('key', function (k) { if (k === 'ok' && state === 'over') start(); });
B.onHide(persist);

// ───────── 自动试玩：你这一方也交给 AI（初级 / 中级交替），每局第 9 手悔一次棋，结束 1.5 秒后换先后手再来 ─────────
var autoNext = 0, autoUndone = false;
function autoPlay() {
  if (state === 'over') { if (clock - overAt > 1.5) { nextHuman = 3 - human; start(); } return; }
  if (pending || turn() !== human || clock < autoNext) return;
  if (!autoUndone && moves.length >= 9) { autoUndone = true; undo(); autoNext = clock + 0.3; return; }
  ask(human, games % 2 ? 1 : 2);
  autoNext = clock + 0.1;
}

function update(dt) {
  clock += dt;
  if (AUTO) autoPlay();
}
function render(ctx) {
  buttons = V.draw(ctx, {
    board: board, moves: moves, human: human, level: level, turn: turn(), state: state, thinking: !!pending && pending.side !== human,
    win: win, result: result, record: records[level], clock: clock, ghost: ghost, nextLevel: nextLevel, nextHuman: nextHuman, aiMs: aiMs
  });
}

GameGlobal.__play = function () {
  return { state: state, moves: moves.length, games: games, results: results, level: level, human: human, thinking: !!pending,
    aiMs: aiMs, nodes: nodes, worker: mode, undos: undos, record: records[level] };
};

makeWorker();
if (AUTO || !restore(B.load(SAVE_KEY, null))) start();
B.ctx;                                    // 主画布拿 2D 上下文（base.js 用到才拿；先拿好，loop 每帧设 dpr 变换）
B.loop(update, render);
