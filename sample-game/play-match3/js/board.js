// 8×8 棋盘的纯逻辑（不画、不放音）：找三连、判特效块、特效连锁、下落补位、找可走的步、洗牌。
// 格子是对象 { id, t: 水果种类 0~5（彩虹球为 -1）, sp: 特效, x, y: 画在哪（像素，补间改它）, s: 缩放, a: 透明度 }。
// 特效 sp：'h' 横向条纹（消整行）、'v' 竖向条纹（消整列）、'b' 炸弹（L / T 型生成，消 3×3）、'r' 彩虹球（5 连生成，与谁交换就消谁那一色）
var N = 8;
var seq = 0;

function Board(colors, rand) {
  this.colors = colors;
  this.rand = rand || Math.random;
  this.g = [];
  for (var r = 0; r < N; r++) { this.g.push([]); for (var c = 0; c < N; c++) this.g[r].push(null); }
}
Board.N = N;
var P = Board.prototype;

P.cell = function (t, sp) { return { id: ++seq, t: t, sp: sp || 0, x: 0, y: 0, s: 1, a: 1 }; };
P.at = function (r, c) { return r >= 0 && c >= 0 && r < N && c < N ? this.g[r][c] : null; };
P.pick = function () { return Math.floor(this.rand() * this.colors); };

// 开局：铺满且没有现成的三连，并且至少有一步能走
P.fill = function () {
  for (var tries = 0; tries < 50; tries++) {
    for (var r = 0; r < N; r++) for (var c = 0; c < N; c++) {
      var t, guard = 0;
      do { t = this.pick(); guard++; } while (guard < 20 && (
        (c >= 2 && this.g[r][c - 1].t === t && this.g[r][c - 2].t === t) ||
        (r >= 2 && this.g[r - 1][c].t === t && this.g[r - 2][c].t === t)));
      this.g[r][c] = this.cell(t);
    }
    if (this.hasMove()) return;
  }
};

// 找全部三连（及以上），横竖相交的并成一组（L / T 型）。返回 [{ cells, runs:[{dir,len}], cross }]
P.groups = function () {
  var runs = [], r, c, k;
  for (r = 0; r < N; r++) {
    for (c = 0; c < N;) {
      var a = this.g[r][c], len = 1;
      while (a && a.t >= 0 && c + len < N && this.g[r][c + len] && this.g[r][c + len].t === a.t) len++;
      if (a && a.t >= 0 && len >= 3) { var hs = []; for (k = 0; k < len; k++) hs.push(this.g[r][c + k]); runs.push({ dir: 'h', cells: hs }); }
      c += len;
    }
  }
  for (c = 0; c < N; c++) {
    for (r = 0; r < N;) {
      var b = this.g[r][c], ln = 1;
      while (b && b.t >= 0 && r + ln < N && this.g[r + ln][c] && this.g[r + ln][c].t === b.t) ln++;
      if (b && b.t >= 0 && ln >= 3) { var vs = []; for (k = 0; k < ln; k++) vs.push(this.g[r + k][c]); runs.push({ dir: 'v', cells: vs }); }
      r += ln;
    }
  }
  // 共享格子的 run 并成一组
  var owner = {}, groups = [];
  runs.forEach(function (run) {
    var hitGroups = [];
    run.cells.forEach(function (x) { var g = owner[x.id]; if (g && hitGroups.indexOf(g) < 0) hitGroups.push(g); });
    var g = hitGroups[0] || { cells: [], runs: [], cross: null };
    if (!hitGroups.length) groups.push(g);
    for (var i = 1; i < hitGroups.length; i++) {            // 一条 run 连上了两组：合并
      var o = hitGroups[i];
      o.cells.forEach(function (x) { if (g.cells.indexOf(x) < 0) g.cells.push(x); owner[x.id] = g; });
      g.runs = g.runs.concat(o.runs); groups.splice(groups.indexOf(o), 1);
    }
    run.cells.forEach(function (x) {
      if (owner[x.id] === g) { if (!g.cross) g.cross = x; } else { g.cells.push(x); owner[x.id] = g; }
    });
    g.runs.push(run);
  });
  return groups;
};

// 这一组该生成什么特效块、放在哪。prefer = 玩家刚交换的两格（新块落在玩家手指下）
P.special = function (g, prefer) {
  var maxLen = 0, dirs = {};
  g.runs.forEach(function (run) { maxLen = Math.max(maxLen, run.cells.length); dirs[run.dir] = run; });
  var sp = 0;
  if (maxLen >= 5) sp = 'r';
  else if (dirs.h && dirs.v) sp = 'b';
  else if (maxLen === 4) sp = dirs.h ? 'v' : 'h';             // 横着消 4 个 → 竖条纹（与常见三消一致）
  if (!sp) return null;
  var at = null;
  (prefer || []).forEach(function (p) { if (!at && p && g.cells.indexOf(p) >= 0) at = p; });
  if (!at && sp === 'b') at = g.cross;
  if (!at) at = g.cells[Math.floor(g.cells.length / 2)];
  return { sp: sp, at: at };
};

P.pos = function (x) {
  for (var r = 0; r < N; r++) for (var c = 0; c < N; c++) if (this.g[r][c] === x) return [r, c];
  return null;
};

// 从一批要消的格子出发，把被波及的特效块连锁展开。返回 { set: {id: cell}, list, effects: [{sp, r, c, t}] }
P.expand = function (cells, rainbowColor) {
  var set = {}, list = [], queue = [], effects = [], self = this;
  function add(x) { if (x && !set[x.id]) { set[x.id] = x; list.push(x); if (x.sp) queue.push(x); } }
  cells.forEach(add);
  while (queue.length) {
    var x = queue.shift(), p = this.pos(x); if (!p) continue;
    var r = p[0], c = p[1], i, j;
    effects.push({ sp: x.sp, r: r, c: c, t: x.t });
    if (x.sp === 'h') for (i = 0; i < N; i++) add(this.g[r][i]);
    else if (x.sp === 'v') for (i = 0; i < N; i++) add(this.g[i][c]);
    else if (x.sp === 'b') for (i = -1; i <= 1; i++) for (j = -1; j <= 1; j++) add(this.at(r + i, c + j));
    else if (x.sp === 'r') {
      var col = rainbowColor !== undefined && rainbowColor >= 0 ? rainbowColor : self.mostCommon();
      rainbowColor = -1;                                     // 第二个被波及的彩虹球改消「当前最多的那色」
      for (i = 0; i < N; i++) for (j = 0; j < N; j++) if (this.g[i][j] && this.g[i][j].t === col) add(this.g[i][j]);
    }
  }
  return { set: set, list: list, effects: effects };
};
P.mostCommon = function () {
  var n = [0, 0, 0, 0, 0, 0], best = 0;
  for (var r = 0; r < N; r++) for (var c = 0; c < N; c++) { var x = this.g[r][c]; if (x && x.t >= 0) n[x.t]++; }
  for (var t = 1; t < 6; t++) if (n[t] > n[best]) best = t;
  return best;
};

P.swap = function (a, b) {
  var pa = this.pos(a), pb = this.pos(b);
  this.g[pa[0]][pa[1]] = b; this.g[pb[0]][pb[1]] = a;
};

// 消掉 set 里的格子后下落补位。返回 [{ cell, fromR, toR, c, fresh }]（fresh = 从顶上新掉进来的）
P.collapse = function (set) {
  var moves = [];
  for (var c = 0; c < N; c++) {
    var write = N - 1;
    for (var r = N - 1; r >= 0; r--) {
      var x = this.g[r][c];
      if (x && !set[x.id]) {
        if (write !== r) moves.push({ cell: x, fromR: r, toR: write, c: c, fresh: false });
        this.g[write][c] = x; write--;
      }
    }
    var k = 0;
    for (var w = write; w >= 0; w--) {
      var nc = this.cell(this.pick());
      this.g[w][c] = nc;
      moves.push({ cell: nc, fromR: -1 - k, toR: w, c: c, fresh: true });
      k++;
    }
  }
  return moves;
};

// 一步交换会不会成立（有三连，或有彩虹球参与）
P.swapMakesMatch = function (r1, c1, r2, c2) {
  var a = this.g[r1][c1], b = this.g[r2][c2];
  if (!a || !b) return 0;
  if (a.sp === 'r' || b.sp === 'r') return 8;
  this.g[r1][c1] = b; this.g[r2][c2] = a;
  var n = this.lineAt(r1, c1) + this.lineAt(r2, c2);
  this.g[r1][c1] = a; this.g[r2][c2] = b;
  return n;
};
// (r, c) 这一格横竖各连了几个（≥3 才算），用来快速判可走与给 AI 打分
P.lineAt = function (r, c) {
  var x = this.g[r][c]; if (!x || x.t < 0) return 0;
  var t = x.t, h = 1, v = 1, i;
  for (i = c - 1; i >= 0 && this.g[r][i] && this.g[r][i].t === t; i--) h++;
  for (i = c + 1; i < N && this.g[r][i] && this.g[r][i].t === t; i++) h++;
  for (i = r - 1; i >= 0 && this.g[i][c] && this.g[i][c].t === t; i--) v++;
  for (i = r + 1; i < N && this.g[i][c] && this.g[i][c].t === t; i++) v++;
  return (h >= 3 ? h : 0) + (v >= 3 ? v : 0);
};
// 全部能走的步，按「消得多 / 能出特效」打分。给自动试玩和「提示」用
P.moves = function () {
  var out = [];
  for (var r = 0; r < N; r++) for (var c = 0; c < N; c++) {
    if (c + 1 < N) { var a = this.swapMakesMatch(r, c, r, c + 1); if (a) out.push({ r1: r, c1: c, r2: r, c2: c + 1, score: a }); }
    if (r + 1 < N) { var b = this.swapMakesMatch(r, c, r + 1, c); if (b) out.push({ r1: r, c1: c, r2: r + 1, c2: c, score: b }); }
  }
  var g = this.g;
  out.forEach(function (m) {                     // 带特效块的那一步更值
    var x = g[m.r1][m.c1], y = g[m.r2][m.c2];
    if (x.sp) m.score += 3; if (y.sp) m.score += 3;
    if (m.score >= 4 && m.score < 8) m.score += 1;
  });
  return out;
};
P.hasMove = function () { return this.moves().length > 0; };

// 死局洗牌：只打乱普通水果的种类（特效块留在原地），直到没有现成三连且有步可走
P.shuffle = function () {
  var normal = [], r, c;
  for (r = 0; r < N; r++) for (c = 0; c < N; c++) if (!this.g[r][c].sp) normal.push(this.g[r][c]);
  var types = normal.map(function (x) { return x.t; });
  for (var tries = 0; tries < 100; tries++) {
    for (var i = types.length - 1; i > 0; i--) { var j = Math.floor(this.rand() * (i + 1)), tmp = types[i]; types[i] = types[j]; types[j] = tmp; }
    normal.forEach(function (x, k) { x.t = types[k]; });
    if (!this.groups().length && this.hasMove()) return true;
  }
  // 打乱不出来（颜色太少的极端情况）：整盘重铺
  this.fill();
  return false;
};

module.exports = Board;
