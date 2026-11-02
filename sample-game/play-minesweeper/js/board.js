// 扫雷的棋盘逻辑（不碰画面）：埋雷、翻开、自动展开、插旗、快速开周围（chord）、胜负判定。
// 格子下标 i = y * cols + x，状态都放在定长 Uint8Array 里（高级 16×30 = 480 格，每次翻开不分配对象）。
// 每次改动的格子记进 changed，画面只重画这些格子（js/view.js）。
var LEVELS = {
  easy: { name: '初级', cols: 9, rows: 9, mines: 10 },
  mid: { name: '中级', cols: 16, rows: 16, mines: 40 },
  // 经典高级是 30×16（横的）；竖屏把它竖过来，16 列 × 30 行，屏幕放不下就缩放平移
  hard: { name: '高级', cols: 16, rows: 30, mines: 99 }
};

function Board(level) {
  var L = LEVELS[level];
  this.level = level; this.cols = L.cols; this.rows = L.rows; this.mines = L.mines;
  var n = L.cols * L.rows;
  this.n = n;
  this.mine = new Uint8Array(n);
  this.open = new Uint8Array(n);
  this.flag = new Uint8Array(n);
  this.num = new Uint8Array(n);
  this.planted = false;
  this.state = 'play';           // play / won / lost
  this.boom = -1;                // 踩到的那颗
  this.opened = 0; this.flags = 0;
  this.changed = [];
  this.stack = new Int32Array(n); // 展开用的栈（不递归：高级一片空白几百格，递归深了 QuickJS 会爆栈）
  // 每格的邻居表（最多 8 个，-1 结尾），算一次，之后数雷、展开、chord 都直接查
  this.nb = new Int16Array(n * 9);
  for (var i = 0; i < n; i++) {
    var x = i % L.cols, y = (i / L.cols) | 0, k = 0;
    for (var dy = -1; dy <= 1; dy++) for (var dx = -1; dx <= 1; dx++) {
      if (!dx && !dy) continue;
      var nx = x + dx, ny = y + dy;
      if (nx >= 0 && ny >= 0 && nx < L.cols && ny < L.rows) this.nb[i * 9 + k++] = ny * L.cols + nx;
    }
    this.nb[i * 9 + k] = -1;
  }
}
Board.LEVELS = LEVELS;

// 首次点击必不是雷：雷避开点的那格和它的邻居（这样第一下一定是空白、一定能展开一片）
Board.prototype.plant = function (safeIdx) {
  var avoid = new Uint8Array(this.n), cands = [], i, j;
  avoid[safeIdx] = 1;
  for (j = safeIdx * 9; this.nb[j] >= 0; j++) avoid[this.nb[j]] = 1;
  for (i = 0; i < this.n; i++) if (!avoid[i]) cands.push(i);
  if (cands.length < this.mines) {               // 格子太少时退一步：只避开点的那一格
    cands = []; for (i = 0; i < this.n; i++) if (i !== safeIdx) cands.push(i);
  }
  for (i = 0; i < this.mines; i++) {             // 部分 Fisher–Yates：前 mines 个就是雷
    var r = i + Math.floor(Math.random() * (cands.length - i)), t = cands[i];
    cands[i] = cands[r]; cands[r] = t;
    this.mine[cands[i]] = 1;
  }
  this.count();
  this.planted = true;
};
Board.prototype.count = function () {
  for (var i = 0; i < this.n; i++) {
    var c = 0;
    for (var j = i * 9; this.nb[j] >= 0; j++) c += this.mine[this.nb[j]];
    this.num[i] = c;
  }
};

// 翻开一格；空白（周围 0 雷）就一路展开。返回翻开的格数，踩雷返回 -1
Board.prototype.reveal = function (i) {
  if (this.state !== 'play' || this.open[i] || this.flag[i]) return 0;
  if (!this.planted) this.plant(i);
  if (this.mine[i]) { this.open[i] = 1; this.changed.push(i); this.boom = i; this.lose(); return -1; }
  var sp = 0, got = 0;
  this.stack[sp++] = i; this.open[i] = 1;
  while (sp) {
    var c = this.stack[--sp];
    got++; this.changed.push(c);
    if (this.num[c]) continue;
    for (var j = c * 9; this.nb[j] >= 0; j++) {
      var m = this.nb[j];
      if (this.open[m] || this.mine[m]) continue;
      if (this.flag[m]) { this.flag[m] = 0; this.flags--; }   // 插错旗的空白也一并翻开（与 Windows 扫雷一致）
      this.open[m] = 1; this.stack[sp++] = m;
    }
  }
  this.opened += got;
  if (this.opened === this.n - this.mines) this.win();
  return got;
};
Board.prototype.toggleFlag = function (i) {
  if (this.state !== 'play' || this.open[i]) return false;
  this.flag[i] ^= 1; this.flags += this.flag[i] ? 1 : -1;
  this.changed.push(i);
  return true;
};
// 快速开周围：已翻开的数字格，周围旗数 == 数字时，把周围没插旗的都翻开（旗插错就会踩雷）
Board.prototype.chord = function (i) {
  if (this.state !== 'play' || !this.open[i] || !this.num[i]) return 0;
  var f = 0, j;
  for (j = i * 9; this.nb[j] >= 0; j++) f += this.flag[this.nb[j]];
  if (f !== this.num[i]) return 0;
  var got = 0;
  for (j = i * 9; this.nb[j] >= 0; j++) {
    var r = this.reveal(this.nb[j]);
    if (r < 0) return -1;
    got += r;
  }
  return got;
};
Board.prototype.win = function () {
  this.state = 'won';
  for (var i = 0; i < this.n; i++) if (this.mine[i] && !this.flag[i]) { this.flag[i] = 1; this.flags++; this.changed.push(i); }
};
Board.prototype.lose = function () {
  this.state = 'lost';
  for (var i = 0; i < this.n; i++) if (this.mine[i] || this.flag[i]) this.changed.push(i);   // 亮出全部雷与插错的旗
};

// 存档：雷 / 翻开 / 旗压成字符串（每格一个字符），读回来重算数字
Board.prototype.dump = function () {
  var s = '';
  for (var i = 0; i < this.n; i++) s += String.fromCharCode(48 + this.mine[i] + this.open[i] * 2 + this.flag[i] * 4);
  return { level: this.level, cells: s, planted: this.planted };
};
Board.restore = function (d) {
  if (!d || !LEVELS[d.level]) return null;
  var b = new Board(d.level);
  if (typeof d.cells !== 'string' || d.cells.length !== b.n) return null;
  for (var i = 0; i < b.n; i++) {
    var v = d.cells.charCodeAt(i) - 48;
    b.mine[i] = v & 1; b.open[i] = (v >> 1) & 1; b.flag[i] = (v >> 2) & 1;
    b.opened += b.open[i]; b.flags += b.flag[i];
  }
  b.planted = !!d.planted;
  b.count();
  return b;
};

module.exports = Board;
