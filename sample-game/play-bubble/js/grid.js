// 六边形网格（「奇偶行错开」排法）：宽行 COLS 个泡泡，窄行 COLS-1 个、向右错开半个泡泡。
// 下压一行 = 在顶上插一行，插进去的那行宽窄与原来的第 0 行相反（wide0 翻转），下面的行坐标不变、整体下移一行高。
// 格子里存颜色下标（0~5），空格是 -1。
var SQ3 = Math.sqrt(3);

function Grid(cols, left, top, r) {
  this.cols = cols; this.left = left; this.top = top; this.r = r; this.rowH = r * SQ3;
  this.rows = []; this.wide0 = true; this.version = 0;
}
Grid.prototype.wide = function (i) { return (i % 2 === 0) === this.wide0; };
Grid.prototype.len = function (i) { return this.wide(i) ? this.cols : this.cols - 1; };
Grid.prototype.ensure = function (i) { while (this.rows.length <= i) { var n = this.rows.length, a = []; for (var c = 0; c < this.len(n); c++) a.push(-1); this.rows.push(a); } };
Grid.prototype.get = function (i, c) { var row = this.rows[i]; return row && c >= 0 && c < row.length ? row[c] : -1; };
Grid.prototype.set = function (i, c, v) { this.ensure(i); this.rows[i][c] = v; this.version++; };
Grid.prototype.x = function (i, c) { return this.left + this.r + c * 2 * this.r + (this.wide(i) ? 0 : this.r); };
Grid.prototype.y = function (i) { return this.top + this.r + i * this.rowH; };
// 六个邻居（越界的不给）
Grid.prototype.neighbors = function (i, c) {
  var out = [], d = this.wide(i) ? -1 : 0, self = this;
  function add(a, b) { if (a >= 0 && b >= 0 && b < self.len(a)) out.push([a, b]); }
  add(i, c - 1); add(i, c + 1);
  add(i - 1, c + d); add(i - 1, c + d + 1); add(i + 1, c + d); add(i + 1, c + d + 1);
  return out;
};
Grid.prototype.attached = function (i, c) {
  if (i === 0) return true;
  var ns = this.neighbors(i, c);
  for (var k = 0; k < ns.length; k++) if (this.get(ns[k][0], ns[k][1]) >= 0) return true;
  return false;
};
// 撞停时落到哪一格：附近三行里「空着、且挨着顶或别的泡泡」的格子中离撞停点最近的
Grid.prototype.snap = function (x, y) {
  var est = Math.round((y - this.top - this.r) / this.rowH), best = null, bd = 1e18;
  for (var span = 1; span <= 3 && !best; span++) {
    for (var i = Math.max(0, est - span); i <= est + span; i++) {
      for (var c = 0; c < this.len(i); c++) {
        if (this.get(i, c) >= 0 || !this.attached(i, c)) continue;
        var dx = this.x(i, c) - x, dy = this.y(i) - y, d = dx * dx + dy * dy;
        if (d < bd) { bd = d; best = [i, c]; }
      }
    }
  }
  return best;
};
// 圆心 (x, y) 的泡泡是否碰到网格里的泡泡（只查附近两三行）
Grid.prototype.collides = function (x, y, reach) {
  var ri = Math.floor((y - this.top) / this.rowH), r2 = reach * reach;
  if (ri - 1 > this.lowestCached()) return false;       // 还在最低一行泡泡的下面：一格都不用查（模拟的大半步数都在这里）
  for (var i = Math.max(0, ri - 1); i <= ri + 1 && i < this.rows.length; i++) {
    var row = this.rows[i], yy = this.y(i);
    for (var c = 0; c < row.length; c++) {
      if (row[c] < 0) continue;
      var dx = this.x(i, c) - x, dy = yy - y;
      if (dx * dx + dy * dy < r2) return true;
    }
  }
  return false;
};
// 从 (i, c) 出发同色连通的一片（含自己）；color 不给就用格子里的
Grid.prototype.cluster = function (i, c, color) {
  var k = color === undefined ? this.get(i, c) : color, seen = {}, out = [[i, c]], q = [[i, c]];
  seen[i + ',' + c] = 1;
  while (q.length) {
    var p = q.pop(), ns = this.neighbors(p[0], p[1]);
    for (var n = 0; n < ns.length; n++) {
      var key = ns[n][0] + ',' + ns[n][1];
      if (seen[key] || this.get(ns[n][0], ns[n][1]) !== k) continue;
      seen[key] = 1; out.push(ns[n]); q.push(ns[n]);
    }
  }
  return out;
};
// 悬空的泡泡：从第 0 行出发连不到的全部
Grid.prototype.floating = function () {
  var seen = {}, q = [], out = [];
  for (var c = 0; c < this.len(0); c++) if (this.get(0, c) >= 0) { seen['0,' + c] = 1; q.push([0, c]); }
  while (q.length) {
    var p = q.pop(), ns = this.neighbors(p[0], p[1]);
    for (var n = 0; n < ns.length; n++) {
      var key = ns[n][0] + ',' + ns[n][1];
      if (seen[key] || this.get(ns[n][0], ns[n][1]) < 0) continue;
      seen[key] = 1; q.push(ns[n]);
    }
  }
  for (var i = 0; i < this.rows.length; i++) for (var cc = 0; cc < this.rows[i].length; cc++)
    if (this.rows[i][cc] >= 0 && !seen[i + ',' + cc]) out.push([i, cc]);
  return out;
};
Grid.prototype.pushRow = function (colorOf) {
  this.wide0 = !this.wide0;
  var a = [];
  for (var c = 0; c < this.len(0); c++) a.push(colorOf(c));
  this.rows.unshift(a);
  this.version++;
};
Grid.prototype.count = function () { var n = 0; this.each(function () { n++; }); return n; };
Grid.prototype.each = function (f) {
  for (var i = 0; i < this.rows.length; i++) for (var c = 0; c < this.rows[i].length; c++) if (this.rows[i][c] >= 0) f(i, c, this.rows[i][c]);
};
Grid.prototype.colors = function () { var s = {}, out = []; this.each(function (i, c, k) { if (!s[k]) { s[k] = 1; out.push(k); } }); return out; };
Grid.prototype.lowestCached = function () {
  if (this._lowVer !== this.version) { this._low = this.lowest(); this._lowVer = this.version; }
  return this._low;
};
Grid.prototype.lowest = function () { var m = -1; this.each(function (i) { if (i > m) m = i; }); return m; };
// 从底下删掉 n 行（复活用）
Grid.prototype.dropBottom = function (n) {
  var low = this.lowest();
  for (var i = Math.max(0, low - n + 1); i <= low; i++) for (var c = 0; c < this.len(i); c++) this.rows[i][c] = -1;
  this.version++;
};

module.exports = Grid;
