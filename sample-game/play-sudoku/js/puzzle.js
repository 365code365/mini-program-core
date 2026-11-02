// 一局数独的状态与操作：填数、擦除、候选数笔记、撤销、提示、冲突、完成判定、存档。
// 不碰画面。冲突按规则算（同行 / 同列 / 同宫重复的数字），不偷看答案；完成 = 填满且与答案一致
// （题目唯一解，所以「填满且无冲突」就等于答对）。
var solver = require('./solver.js');
var ROW = solver.ROW, COL = solver.COL, BOX = solver.BOX;
var PEERS = [];                                  // 每格的 20 个同行 / 同列 / 同宫格
for (var i = 0; i < 81; i++) {
  var p = [];
  for (var j = 0; j < 81; j++) if (j !== i && (ROW[j] === ROW[i] || COL[j] === COL[i] || BOX[j] === BOX[i])) p.push(j);
  PEERS.push(p);
}
var UNDO_MAX = 200;

function Puzzle(diff, grid, sol) {
  this.diff = diff;
  this.sol = new Uint8Array(sol);
  this.given = new Uint8Array(81);
  this.val = new Uint8Array(81);
  this.notes = new Uint16Array(81);
  this.hinted = new Uint8Array(81);
  this.conf = new Uint8Array(81);
  for (var i = 0; i < 81; i++) if (grid[i]) { this.given[i] = 1; this.val[i] = grid[i]; }
  this.undo = []; this.hints = 3; this.time = 0; this.moves = 0;
  this.clues = 0; for (var k = 0; k < 81; k++) this.clues += this.given[k];
  this.recheck();
}
Puzzle.PEERS = PEERS;

Puzzle.prototype.snap = function (i) { return { i: i, v: this.val[i], n: this.notes[i], h: this.hinted[i], peers: null }; };
Puzzle.prototype.push = function (e) { this.undo.push(e); if (this.undo.length > UNDO_MAX) this.undo.shift(); this.moves++; };

// 填数：同一格再填同一个数 = 擦掉；填进去时顺手把同行同列同宫笔记里的这个数划掉（撤销时一并恢复）
Puzzle.prototype.set = function (i, d, hint) {
  if (this.given[i]) return 'given';
  if (this.val[i] === d && !hint) return this.erase(i);
  var e = this.snap(i), bit = 1 << (d - 1);
  e.peers = [];
  this.val[i] = d; this.notes[i] = 0; this.hinted[i] = hint ? 1 : 0;
  var ps = PEERS[i];
  for (var k = 0; k < ps.length; k++) {
    var j = ps[k];
    if (this.notes[j] & bit) { e.peers.push(j, this.notes[j]); this.notes[j] &= ~bit; }
  }
  this.push(e);
  this.recheck();
  return 'set';
};
Puzzle.prototype.erase = function (i) {
  if (this.given[i]) return 'given';
  if (!this.val[i] && !this.notes[i]) return 'empty';
  this.push(this.snap(i));
  this.val[i] = 0; this.notes[i] = 0; this.hinted[i] = 0;
  this.recheck();
  return 'erase';
};
Puzzle.prototype.note = function (i, d) {
  if (this.given[i]) return 'given';
  if (this.val[i]) return 'filled';
  this.push(this.snap(i));
  this.notes[i] ^= 1 << (d - 1);
  return 'note';
};
Puzzle.prototype.back = function () {
  var e = this.undo.pop();
  if (!e) return -1;
  this.val[e.i] = e.v; this.notes[e.i] = e.n; this.hinted[e.i] = e.h;
  if (e.peers) for (var k = 0; k < e.peers.length; k += 2) this.notes[e.peers[k]] = e.peers[k + 1];
  this.moves++;
  this.recheck();
  return e.i;
};
// 提示：选中的格子空着或填错了就填它；否则挑候选最少的空格。返回填的格子，用完返回 -1
Puzzle.prototype.hint = function (sel) {
  if (this.hints <= 0) return -1;
  var i = sel >= 0 && !this.given[sel] && this.val[sel] !== this.sol[sel] ? sel : this.easiest();
  if (i < 0) return -1;
  this.hints--;
  this.set(i, this.sol[i], true);
  return i;
};
// 候选最少的空格（或填错的格子）：自动试玩与提示都用
Puzzle.prototype.easiest = function () {
  var best = -1, bc = 10;
  for (var i = 0; i < 81; i++) {
    if (this.val[i] === this.sol[i]) continue;
    if (this.val[i]) return i;                   // 填错的先改
    var cnt = solver.POP[solver.candidates(this.val, i)];
    if (cnt < bc) { bc = cnt; best = i; }
  }
  return best;
};
Puzzle.prototype.recheck = function () {
  this.conf.fill(0);
  var n = 0;
  for (var i = 0; i < 81; i++) {
    var v = this.val[i];
    if (!v) continue;
    var ps = PEERS[i];
    for (var k = 0; k < ps.length; k++) if (this.val[ps[k]] === v) { this.conf[i] = 1; n++; break; }
  }
  this.conflicts = n;
  var f = 0; for (var j = 0; j < 81; j++) if (this.val[j]) f++;
  this.filled = f;
};
Puzzle.prototype.solved = function () {
  if (this.filled < 81 || this.conflicts) return false;
  for (var i = 0; i < 81; i++) if (this.val[i] !== this.sol[i]) return false;
  return true;
};
// 某个数字已经填了几个（数字键盘上显示「还剩几个」、填满 9 个变灰）
Puzzle.prototype.used = function (d) { var c = 0; for (var i = 0; i < 81; i++) if (this.val[i] === d && !this.conf[i]) c++; return c; };
Puzzle.prototype.noteCount = function () { var c = 0; for (var i = 0; i < 81; i++) c += solver.POP[this.notes[i]]; return c; };

// 存档：数字盘压成 81 个字符，笔记是 81 个数，撤销栈留最近 60 步
function digits(a) { var s = ''; for (var i = 0; i < 81; i++) s += a[i]; return s; }
function undigits(s) { var a = new Uint8Array(81); for (var i = 0; i < 81; i++) a[i] = s.charCodeAt(i) - 48; return a; }
Puzzle.prototype.dump = function () {
  var g = new Uint8Array(81); for (var i = 0; i < 81; i++) g[i] = this.given[i] ? this.val[i] : 0;
  return {
    diff: this.diff, grid: digits(g), sol: digits(this.sol), val: digits(this.val), hinted: digits(this.hinted),
    notes: Array.prototype.slice.call(this.notes), hints: this.hints, time: Math.round(this.time * 10) / 10,
    undo: this.undo.slice(-60), moves: this.moves
  };
};
Puzzle.restore = function (d) {
  try {
    if (!d || typeof d.grid !== 'string' || d.grid.length !== 81 || d.sol.length !== 81) return null;
    var p = new Puzzle(d.diff, undigits(d.grid), undigits(d.sol));
    var v = undigits(d.val), h = undigits(d.hinted || '');
    for (var i = 0; i < 81; i++) {
      if (!p.given[i]) { p.val[i] = v[i] <= 9 ? v[i] : 0; p.hinted[i] = h[i] === 1 ? 1 : 0; }
      p.notes[i] = (d.notes && d.notes[i]) & 511;
    }
    p.hints = Math.max(0, Math.min(3, d.hints | 0)); p.time = Number(d.time) || 0;
    p.undo = Array.isArray(d.undo) ? d.undo : []; p.moves = d.moves | 0;
    p.recheck();
    return p;
  } catch (e) { return null; }
};

module.exports = Puzzle;
