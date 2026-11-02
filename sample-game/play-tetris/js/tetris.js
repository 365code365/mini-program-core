// 俄罗斯方块的规则（不碰画面与输入）：7 种方块、SRS 旋转 + 踢墙、7-bag 随机、幽灵块落点、锁定延迟、
// 消行（先标记、动画放完再真删）、计分、等级与下落速度。画面层只读这里的状态，输入层只调这里的方法。
var COLS = 10, ROWS = 22, HIDDEN = 2;            // 最上面 2 行是看不见的出生区
var TYPES = 'IJLOSTZ';
// 出生朝向（SRS 规定的 0 态），I 在 4×4 盒子里、O 在 2×2、其余 3×3
var SPAWN = {
  I: ['....', 'XXXX', '....', '....'],
  J: ['X..', 'XXX', '...'],
  L: ['..X', 'XXX', '...'],
  O: ['XX', 'XX'],
  S: ['.XX', 'XX.', '...'],
  T: ['.X.', 'XXX', '...'],
  Z: ['XX.', '.XX', '...']
};
// 踢墙表（SRS 原文 y 向上；这里用的时候取反成屏幕 y 向下）。键是「起始态 + 目标态」，0 / 1(R) / 2 / 3(L)
var KICK = {
  '01': [[0, 0], [-1, 0], [-1, 1], [0, -2], [-1, -2]], '10': [[0, 0], [1, 0], [1, -1], [0, 2], [1, 2]],
  '12': [[0, 0], [1, 0], [1, -1], [0, 2], [1, 2]], '21': [[0, 0], [-1, 0], [-1, 1], [0, -2], [-1, -2]],
  '23': [[0, 0], [1, 0], [1, 1], [0, -2], [1, -2]], '32': [[0, 0], [-1, 0], [-1, -1], [0, 2], [-1, 2]],
  '30': [[0, 0], [-1, 0], [-1, -1], [0, 2], [-1, 2]], '03': [[0, 0], [1, 0], [1, 1], [0, -2], [1, -2]]
};
var KICK_I = {
  '01': [[0, 0], [-2, 0], [1, 0], [-2, -1], [1, 2]], '10': [[0, 0], [2, 0], [-1, 0], [2, 1], [-1, -2]],
  '12': [[0, 0], [-1, 0], [2, 0], [-1, 2], [2, -1]], '21': [[0, 0], [1, 0], [-2, 0], [1, -2], [-2, 1]],
  '23': [[0, 0], [2, 0], [-1, 0], [2, 1], [-1, -2]], '32': [[0, 0], [-2, 0], [1, 0], [-2, -1], [1, 2]],
  '30': [[0, 0], [1, 0], [-2, 0], [1, -2], [-2, 1]], '03': [[0, 0], [-1, 0], [2, 0], [-1, -2], [2, 1]]
};
// 每种方块 4 个朝向的格子坐标（相对盒子左上角）：盒子按顺时针整体转 = SRS 的「真旋转」
var CELLS = {};
TYPES.split('').forEach(function (t) {
  var m = SPAWN[t].map(function (row) { return row.split('').map(function (c) { return c === 'X'; }); });
  var n = m.length, states = [];
  for (var r = 0; r < 4; r++) {
    var cells = [];
    for (var y = 0; y < n; y++) for (var x = 0; x < n; x++) if (m[y][x]) cells.push([x, y]);
    states.push(cells);
    var next = [];                                 // 顺时针：new[y][x] = old[n-1-x][y]
    for (var yy = 0; yy < n; yy++) { next.push([]); for (var xx = 0; xx < n; xx++) next[yy].push(m[n - 1 - xx][yy]); }
    m = next;
  }
  CELLS[t] = states;
});
var LINE_SCORE = [0, 100, 300, 500, 800];
var LOCK_DELAY = 0.5, LOCK_RESETS = 15, CLEAR_TIME = 0.36;

// 每行下落秒数（Tetris Guideline 的公式）：1 级 1 秒一格，15 级约 0.007 秒
function gravity(level) { return Math.pow(0.8 - (level - 1) * 0.007, level - 1); }

function Game(emit) {
  this.emit = emit || function () {};
  this.reset();
}
var P = Game.prototype;
P.reset = function () {
  this.board = [];
  for (var i = 0; i < COLS * ROWS; i++) this.board.push(0);
  this.queue = []; this.refill();
  this.cur = null; this.hold = null; this.canHold = true;
  this.score = 0; this.lines = 0; this.level = 1; this.combo = -1; this.b2b = false;
  this.fall = 0; this.lockT = 0; this.resets = 0; this.lowest = 0;
  this.clearing = null;                          // { rows: [...], t } 消行动画进行中
  this.over = false; this.pieces = 0; this.ver = 0; // ver：棋盘每变一次 +1（画面据此重录图层）
  this.stats = { tetris: 0, holds: 0, hard: 0, softRows: 0, kicks: 0, moves: 0, rotates: 0 };
  this.spawn();
};
// 7-bag：每 7 块里 7 种各一次，队列里永远至少有 7 块（预览要 3 块）
P.refill = function () {
  while (this.queue.length < 7) {
    var bag = TYPES.split('');
    for (var i = bag.length - 1; i > 0; i--) { var j = Math.floor(Math.random() * (i + 1)); var t = bag[i]; bag[i] = bag[j]; bag[j] = t; }
    this.queue = this.queue.concat(bag);
  }
};
P.at = function (x, y) { return x < 0 || x >= COLS || y >= ROWS ? 'W' : y < 0 ? 0 : this.board[y * COLS + x]; };
P.fits = function (t, r, x, y) {
  var c = CELLS[t][r];
  for (var i = 0; i < 4; i++) if (this.at(x + c[i][0], y + c[i][1])) return false;
  return true;
};
P.spawn = function (type) {
  var t = type || this.queue.shift();
  this.refill();
  var x = t === 'O' ? 4 : 3, y = 0;
  this.cur = { t: t, r: 0, x: x, y: y };
  this.fall = 0; this.lockT = 0; this.resets = 0;
  if (!this.fits(t, 0, x, y)) { this.cur = null; this.gameOver(); return false; }
  if (this.fits(t, 0, x, y + 1)) this.cur.y++;   // Guideline：出生后立刻落一格，露出在可见区顶上
  this.lowest = this.cur.y;
  return true;
};
P.gameOver = function () { if (!this.over) { this.over = true; this.emit('over'); } };
// 在地面上挪动 / 旋转会重置锁定计时，但最多 15 次（防止无限拖）
P.touched = function () {
  var c = this.cur;
  if (c.y > this.lowest) { this.lowest = c.y; this.resets = 0; }
  if (!this.fits(c.t, c.r, c.x, c.y + 1) && this.resets < LOCK_RESETS) { this.lockT = 0; this.resets++; }
};
P.move = function (dx) {
  var c = this.cur;
  if (!c || this.clearing || this.over || !this.fits(c.t, c.r, c.x + dx, c.y)) return false;
  c.x += dx; this.stats.moves++; this.touched(); this.emit('move');
  return true;
};
// dir = 1 顺时针 / -1 逆时针；按踢墙表依次试 5 个偏移，第一个放得下的生效
P.rotate = function (dir) {
  var c = this.cur;
  if (!c || this.clearing || this.over || c.t === 'O') return false;
  var to = (c.r + (dir > 0 ? 1 : 3)) % 4, table = (c.t === 'I' ? KICK_I : KICK)['' + c.r + to];
  for (var i = 0; i < table.length; i++) {
    var nx = c.x + table[i][0], ny = c.y - table[i][1];
    if (this.fits(c.t, to, nx, ny)) {
      c.x = nx; c.y = ny; c.r = to;
      this.stats.rotates++;
      if (i) this.stats.kicks++;
      this.touched(); this.emit('rotate', i);
      return true;
    }
  }
  return false;
};
P.ghostY = function () {
  var c = this.cur; if (!c) return 0;
  var y = c.y;
  while (this.fits(c.t, c.r, c.x, y + 1)) y++;
  return y;
};
P.hardDrop = function () {
  var c = this.cur;
  if (!c || this.clearing || this.over) return 0;
  var g = this.ghostY(), n = g - c.y;
  c.y = g; this.score += n * 2; this.stats.hard++;
  this.emit('drop', n);
  this.lock();
  return n;
};
P.doHold = function () {
  var c = this.cur;
  if (!c || !this.canHold || this.clearing || this.over) return false;
  var prev = this.hold;
  this.hold = c.t; this.canHold = false; this.stats.holds++;
  this.emit('hold');
  this.spawn(prev || undefined);
  return true;
};
P.lock = function () {
  var c = this.cur, cells = CELLS[c.t][c.r], above = true;
  for (var i = 0; i < 4; i++) {
    var x = c.x + cells[i][0], y = c.y + cells[i][1];
    if (y >= 0) this.board[y * COLS + x] = c.t;
    if (y >= HIDDEN) above = false;
  }
  this.cur = null; this.canHold = true; this.pieces++; this.ver++;
  var full = [];
  for (var yy = 0; yy < ROWS; yy++) {
    var k = 0;
    for (var xx = 0; xx < COLS; xx++) if (this.board[yy * COLS + xx]) k++;
    if (k === COLS) full.push(yy);
  }
  this.emit('lock', { x: c.x, y: c.y, t: c.t });
  if (above && !full.length) { this.gameOver(); return; }   // 整块锁在出生区之上：Lock Out
  if (full.length) {
    var n = full.length, pts = LINE_SCORE[n] * this.level;
    if (n === 4) { if (this.b2b) pts = Math.round(pts * 1.5); this.b2b = true; this.stats.tetris++; } else this.b2b = false;
    this.combo++;
    if (this.combo > 0) pts += 50 * this.combo * this.level;
    this.score += pts;
    this.clearing = { rows: full, t: 0 };
    this.emit('clear', { rows: full, n: n, pts: pts, combo: this.combo, b2b: this.b2b && n === 4 });
  } else {
    this.combo = -1;
    this.spawn();
  }
};
// 消行动画放完才真删行：删之前棋盘保持原样，画面能在原位置上做闪白 / 收缩
P.finishClear = function () {
  var rows = this.clearing.rows;
  for (var i = 0; i < rows.length; i++) {
    this.board.splice(rows[i] * COLS, COLS);
    for (var k = 0; k < COLS; k++) this.board.unshift(0);
  }
  var lvl = this.level;
  this.lines += rows.length;
  this.level = Math.min(20, 1 + Math.floor(this.lines / 10));
  if (this.level > lvl) this.emit('level', this.level);
  this.clearing = null; this.ver++;
  this.spawn();
};
// 每帧推进：重力下落、锁定延迟、消行动画。soft = 是否按着软降（重力 ×20，每格 +1 分）
P.update = function (dt, soft) {
  if (this.over) return;
  if (this.clearing) {
    this.clearing.t += dt;
    if (this.clearing.t >= CLEAR_TIME) this.finishClear();
    return;
  }
  var c = this.cur; if (!c) return;
  var g = gravity(this.level), step = soft ? Math.min(g / 20, 0.05) : g;
  if (this.fits(c.t, c.r, c.x, c.y + 1)) {
    this.lockT = 0;
    this.fall += dt;
    while (this.fall >= step && this.fits(c.t, c.r, c.x, c.y + 1)) {
      this.fall -= step; c.y++;
      if (soft) { this.score += 1; this.stats.softRows++; }
      if (c.y > this.lowest) { this.lowest = c.y; this.resets = 0; }
    }
    if (!this.fits(c.t, c.r, c.x, c.y + 1)) this.fall = 0;
  } else {
    this.lockT += dt;
    if (this.lockT >= LOCK_DELAY || this.resets >= LOCK_RESETS) this.lock();
  }
};
// 存档：棋盘压成一个字符串，其余是小对象；读回来校验一遍，坏档当没有
P.serialize = function () {
  return {
    v: 1, board: this.board.map(function (c) { return c || '.'; }).join(''), queue: this.queue.join(''),
    cur: this.cur ? this.cur.t : null, hold: this.hold, score: this.score, lines: this.lines, level: this.level,
    combo: this.combo, b2b: this.b2b, pieces: this.pieces
  };
};
P.restore = function (s) {
  if (!s || s.v !== 1 || typeof s.board !== 'string' || s.board.length !== COLS * ROWS) return false;
  this.reset();
  this.board = s.board.split('').map(function (c) { return TYPES.indexOf(c) >= 0 ? c : 0; });
  this.queue = String(s.queue || '').split('').filter(function (c) { return TYPES.indexOf(c) >= 0; }); this.refill();
  this.hold = TYPES.indexOf(s.hold) >= 0 ? s.hold : null;
  this.score = +s.score || 0; this.lines = +s.lines || 0; this.level = Math.max(1, +s.level || 1);
  this.combo = s.combo === undefined ? -1 : +s.combo; this.b2b = !!s.b2b; this.pieces = +s.pieces || 0;
  this.over = false; this.ver++;
  return this.spawn(TYPES.indexOf(s.cur) >= 0 ? s.cur : undefined);
};

module.exports = { Game: Game, COLS: COLS, ROWS: ROWS, HIDDEN: HIDDEN, CELLS: CELLS, TYPES: TYPES, gravity: gravity, CLEAR_TIME: CLEAR_TIME };
