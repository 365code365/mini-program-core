// 数独（竖屏）：四档难度，题目随机生成且唯一解（随机终盘 + 对称挖洞 + 求解器校验，分帧做，js/gen.js），
// 候选数笔记、冲突高亮（同行 / 列 / 宫重复）、提示 3 次、撤销、计时；进度与最佳用时存 storage，退出重进接着下。
// 键盘（PC）：1~9 填数、0 / 退格 / Delete 擦除、方向键移动、N 笔记、Z 撤销、H 提示。
// 画面走 WebGL（js/gl2d.js，与 play-snake 同一份），交互提示走 wx.showModal / wx.showToast。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var Gen = require('js/gen.js');
var Puzzle = require('js/puzzle.js');
var solver = require('js/solver.js');
var D = require('js/draw.js');
var W = B.W, H = B.H;
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

var best = B.load('sudoku.best', {}) || {}, stats = B.load('sudoku.stats', {}) || {};
var saved = B.load('sudoku.progress', null) || null;
var scene = 'menu', gen = null, p = null, sel = -1, notes = false, ui = {}, won = false, wave = -1, newBest = false;
var dirty = true, lastSec = -1, drawn = 0;
var clock = 0, games = 0, actions = 0, lastGen = null, toasts = 0, modals = 0, keys = 0;

function toast(title, icon) {
  toasts++;
  try { wx.showToast({ title: title, icon: icon || 'none', duration: 1200 }); } catch (e) {}
}
function modal(o) {
  modals++;
  try { wx.showModal(o); } catch (e) { if (o.success) o.success({ confirm: true, cancel: false }); }
}
function persist() {
  if (p && !won) { saved = p.dump(); B.save('sudoku.progress', saved); }
  else { saved = null; B.save('sudoku.progress', ''); }
}

function newGame(diff) {
  dirty = true;
  gen = new Gen(diff); scene = 'gen'; p = null; won = false; wave = -1; sel = -1; notes = false;
}
function startPuzzle(pz) {
  dirty = true;
  p = pz; scene = 'play'; won = false; wave = -1; newBest = false; notes = false;
  sel = -1;
  for (var i = 0; i < 81; i++) if (!p.val[i]) { sel = i; break; }
  persist();
}
// 选难度：有没下完的局先确认（覆盖存档）
function pickDiff(d) {
  if (saved && !AUTO) {
    modal({
      title: '开始新的一局？', content: '上一局（' + Gen.DIFFS[saved.diff].name + '）的进度会被覆盖',
      confirmText: '开始', cancelText: '再想想',
      success: function (r) { if (r.confirm) newGame(d); }
    });
  } else newGame(d);
}

// ───────── 操作（手指、键盘、自动试玩都走这里） ─────────
function input(d) {
  dirty = true;
  if (!p || won) return;
  if (sel < 0) { toast('先点一个格子'); return; }
  var r = notes ? p.note(sel, d) : p.set(sel, d);
  if (r === 'given') { toast('题目数字不能修改'); return; }
  if (r === 'filled') { toast('先擦掉这格的数字再记笔记'); return; }
  actions++;
  if (r === 'set' && p.conf[sel]) { B.beep(180, 0.16, 'square', 0.08); B.buzz(); }
  else B.beep(r === 'note' ? 990 : 660, 0.04, 'triangle', 0.07);
  after();
}
function erase() {
  dirty = true;
  if (!p || won || sel < 0) return;
  var r = p.erase(sel);
  if (r === 'given') toast('题目数字不能修改');
  else if (r === 'erase') { actions++; B.beep(440, 0.04, 'triangle', 0.06); after(); }
}
function undo() {
  dirty = true;
  if (!p || won) return;
  var i = p.back();
  if (i < 0) { toast('没有可以撤销的步骤'); return; }
  sel = i; actions++;
  B.beep(520, 0.05, 'sine', 0.07, 0.7);
  after();
}
function hint() {
  dirty = true;
  if (!p || won) return;
  if (p.hints <= 0) { toast('提示已用完'); return; }
  var i = p.hint(sel);
  if (i < 0) return;
  sel = i; actions++;
  toast('第 ' + ((i / 9 | 0) + 1) + ' 行第 ' + (i % 9 + 1) + ' 列是 ' + p.val[i] + '（还剩 ' + p.hints + ' 次）');
  B.beep(880, 0.08, 'sine', 0.08, 1.3);
  after();
}
function toggleNotes() {
  dirty = true;
  if (!p || won) return;
  notes = !notes;
  toast(notes ? '笔记模式：开（点数字记候选）' : '笔记模式：关');
}
function move(dx, dy) {
  dirty = true;
  if (!p || won) return;
  if (sel < 0) { sel = 40; return; }
  var r = (sel / 9 | 0) + dy, c = sel % 9 + dx;
  sel = ((r + 9) % 9) * 9 + (c + 9) % 9;
}
function after() {
  if (p.solved()) win(); else persist();
}
function win() {
  won = true; wave = 0; games++;
  var D0 = Gen.DIFFS[p.diff], t = Math.max(1, Math.round(p.time));
  stats[D0.key] = (stats[D0.key] || 0) + 1;
  if (!best[D0.key] || t < best[D0.key]) { best[D0.key] = t; newBest = true; }
  B.save('sudoku.best', best); B.save('sudoku.stats', stats);
  persist();
  [523, 659, 784, 1047].forEach(function (f, k) { setTimeout(function () { B.beep(f, 0.12, 'triangle', 0.09); }, k * 90); });
  var content = '用时 ' + D.mmss(t) + (newBest ? ' · 新纪录' : '') + ' · 用了 ' + (3 - p.hints) + ' 次提示';
  setTimeout(function () {
    if (AUTO) { toast('完成！' + content, 'success'); return; }
    modal({
      title: D0.name + '完成！', content: content, confirmText: '再来一局', cancelText: '返回菜单',
      success: function (r) { dirty = true; if (r.confirm) newGame(p.diff); else scene = 'menu'; }
    });
  }, 1300);
}

// ───────── 输入 ─────────
B.on('tap', function (x, y) {
  dirty = true;
  if (scene === 'menu') {
    for (var d = 0; d < 4; d++) if (B.hit(ui[d], x, y)) { pickDiff(d); return; }
    if (B.hit(ui.resume, x, y)) { var pz = Puzzle.restore(saved); if (pz) startPuzzle(pz); else { saved = null; toast('存档读不出来，已重置'); } }
    return;
  }
  if (scene !== 'play') return;
  if (B.hit(ui.menu, x, y)) { persist(); scene = 'menu'; if (!won) toast('进度已保存'); return; }
  if (B.hit(ui.fresh, x, y)) {
    if (won || !p.moves) { newGame(p.diff); return; }
    modal({ title: '重新出一题？', content: '当前这局的进度会被覆盖', confirmText: '新局', cancelText: '继续下',
      success: function (r) { if (r.confirm) newGame(p.diff); } });
    return;
  }
  if (B.hit(ui.board, x, y)) { sel = Math.floor((y - D.L.y) / D.L.cell) * 9 + Math.floor((x - D.L.x) / D.L.cell); return; }
  if (B.hit(ui.undo, x, y)) return undo();
  if (B.hit(ui.erase, x, y)) return erase();
  if (B.hit(ui.notes, x, y)) return toggleNotes();
  if (B.hit(ui.hint, x, y)) return hint();
  for (var k = 1; k <= 9; k++) if (B.hit(ui.pad && ui.pad[k], x, y)) return input(k);
});
B.on('key', function (k) {
  if (scene !== 'play') return;
  dirty = true;
  keys++;
  if (k === 'up') move(0, -1); else if (k === 'down') move(0, 1); else if (k === 'left') move(-1, 0); else if (k === 'right') move(1, 0);
});
// 数字键：主键盘与小键盘都认（key 是 '5'，code 是 'Digit5' / 'Numpad5'）
B.on('rawkey', function (key, code) {
  if (scene !== 'play') return false;
  dirty = true;
  var m = /^(?:Digit|Numpad)([0-9])$/.exec(code), d = m ? +m[1] : /^[0-9]$/.test(key) ? +key : -1;
  if (d >= 1) { keys++; input(d); return true; }
  if (d === 0 || key === 'Backspace' || key === 'Delete' || code === 'Backspace' || code === 'Delete') { keys++; erase(); return true; }
  var c = (key || '').toLowerCase();
  if (c === 'n' || code === 'KeyN') { keys++; toggleNotes(); return true; }
  if (c === 'z' || code === 'KeyZ') { keys++; undo(); return true; }
  if (c === 'h' || code === 'KeyH') { keys++; hint(); return true; }
  return false;                                  // 方向键等交给 base.js 归一成 'key'
});
B.onHide(function () { if (scene === 'play') persist(); });
B.onShow(function () { dirty = true; });

// ───────── 自动试玩：四档轮着出题、做题；每局先演示笔记、故意填错再撤销、用提示，然后按「候选最少」逐格填 ─────────
var autoDiff = 0, autoAcc = 0, autoStep = 0, autoWait = 0;
var PLAN = ['notesOn', 'note', 'note', 'notesOff', 'wrong', 'undo'];
function autoTick(dt) {
  if (scene === 'menu') { newGame(autoDiff++ % 4); autoStep = 0; return; }
  if (scene !== 'play') return;
  if (won) { autoWait += dt; if (autoWait > 2.6) { autoWait = 0; scene = 'menu'; dirty = true; } return; }
  autoAcc += dt;
  if (autoAcc < 0.12) return;
  autoAcc = 0;
  var s = PLAN[autoStep++], i = p.easiest();
  if (s === 'notesOn' || s === 'notesOff') { sel = i; toggleNotes(); return; }
  if (s === 'note') {
    var m = solver.candidates(p.val, i), dd = 1;
    while (dd <= 9 && !(m & (1 << (dd - 1)) && !(p.notes[i] & (1 << (dd - 1))))) dd++;
    sel = i; if (dd <= 9) input(dd); return;
  }
  if (s === 'wrong') {                           // 故意填一个与同行冲突的数：冲突高亮，下一步撤销
    var ps = Puzzle.PEERS[i];
    for (var k = 0; k < ps.length; k++) if (p.val[ps[k]]) { sel = i; input(p.val[ps[k]]); return; }
    return;
  }
  if (s === 'undo') { undo(); return; }
  if (autoStep === 14 || autoStep === 26) { hint(); return; }
  if (i >= 0) { sel = i; input(p.sol[i]); }
}

function update(dt) {
  clock += dt;
  if (AUTO) autoTick(dt);
  if (scene === 'gen') {
    // 每帧最多算 8ms：出题时进度条照常动
    if (gen.step(8)) {
      lastGen = { diff: gen.diff, clues: gen.clues, checks: gen.checks, ms: gen.ms, unique: solver.count(gen.grid, 2) === 1 };
      startPuzzle(new Puzzle(gen.diff, gen.grid, gen.sol));
      gen = null;
    }
  }
  if (scene === 'play' && p && !won) p.time = Math.min(359999, p.time + dt);
  if (wave >= 0) { wave += dt; if (wave > 1.3) wave = -1; dirty = true; }
  if (scene === 'gen') dirty = true;
  var sec = p ? Math.floor(p.time) : -1;
  if (sec !== lastSec) { lastSec = sec; dirty = true; }
}

// 画面没变的帧不画：WebGL 这一帧不提交任何绘制，宿主就接着显示上一帧（与浏览器的合成规则一致）。
// 数独大部分时间是静止的，只有点按、计时跳秒、完成波浪、出题进度时才重画
function render() {
  if (!dirty) return;
  dirty = false; drawn++;
  R.begin(D.C.bg);
  if (scene === 'menu') ui = D.menu(R, { best: best, stats: stats, saved: saved });
  else if (scene === 'gen') D.generating(R, gen);
  else {
    var st = { p: p, sel: won ? -1 : sel, notes: notes, wave: wave };
    var h = D.hud(R, st);
    ui = { menu: h.menu, fresh: h.fresh, board: D.board(R, st) };
    var t = D.tools(R, st);
    ui.undo = t.undo; ui.erase = t.erase; ui.notes = t.notes; ui.hint = t.hint;
    ui.pad = D.pad(R, st);
  }
  R.end();
}

GameGlobal.__play = function () {
  return {
    scene: scene, diff: p ? Gen.DIFFS[p.diff].name : gen ? Gen.DIFFS[gen.diff].name : null,
    clues: p && p.clues, filled: p && p.filled, conflicts: p && p.conflicts, notes: p ? p.noteCount() : 0,
    hints: p && p.hints, undo: p && p.undo.length, moves: p && p.moves, time: p ? Math.round(p.time) : 0, sel: sel,
    notesMode: notes, won: won, games: games, actions: actions, toasts: toasts, modals: modals, keys: keys,
    gen: lastGen, best: best, drawn: drawn
  };
};

B.loop(update, render);
