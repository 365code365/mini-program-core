// 一关的流程：交换（不成立就回弹）→ 消除（缩放消失 + 碎屑）→ 生成特效块 → 下落补位 → 再找三连（连锁，combo +1）→ 判胜负 / 死局洗牌。
// 每一段都是一批补间，补间全部走完才进下一段（phase = 'anim' 时不收输入）。
var Board = require('board.js');
var T = require('tween.js');
var FX = require('fx.js');
var F = require('fruit.js');
var LV = require('levels.js');

// 连击音高：五声音阶往上走（do re mi sol la do' …），连得越多越高
var PENTA = [0, 2, 4, 7, 9, 12, 14, 16, 19, 21, 24, 26, 28];

function Match(geo, snd) {
  this.geo = geo;          // { OX, OY, CELL }
  this.snd = snd;          // { beep(f, dur, type, vol, slide), buzz() }
  this.phase = 'idle';
  this.next = null;
  this.stats = { swaps: 0, invalid: 0, specials: 0, maxCombo: 0, cascades: 0, shuffles: 0 };
}
var P = Match.prototype;

P.start = function (no) {
  var lv = LV.list[no - 1], g = this.geo, self = this;
  this.lv = lv; this.no = no;
  this.moves = lv.moves; this.score = 0; this.combo = 0; this.result = null; this.bonus = 0;
  this.goals = lv.goals.map(function (x) { return { t: x[0], n: x[1], left: x[1] }; });
  this.board = new Board(lv.colors);
  this.board.fill();
  T.clear(); FX.clear();
  // 开场：整盘从上面一列一列掉下来
  for (var r = 0; r < Board.N; r++) for (var c = 0; c < Board.N; c++) {
    var x = this.board.g[r][c];
    x.x = g.OX + c * g.CELL; x.y = g.OY - (Board.N - r) * g.CELL - 20; x.s = 1; x.a = 1;
    T.to(x, { y: g.OY + r * g.CELL }, 0.45, 'outBounce', c * 0.03 + (Board.N - r) * 0.02);
  }
  this.wait(function () { self.phase = 'idle'; });
};

P.wait = function (fn) { this.phase = 'anim'; this.next = fn; };
// 每帧调（在 T.update 之后）：补间走完就进下一段
P.update = function () {
  if (this.phase === 'anim' && !T.busy()) {
    var f = this.next; this.next = null; this.phase = 'idle';
    if (f) f.call(this);
  }
};

P.home = function (x, r, c) { x.x = this.geo.OX + c * this.geo.CELL; x.y = this.geo.OY + r * this.geo.CELL; };
P.center = function (r, c) { var g = this.geo; return [g.OX + c * g.CELL + g.CELL / 2, g.OY + r * g.CELL + g.CELL / 2]; };

P.trySwap = function (r1, c1, r2, c2) {
  if (this.phase !== 'idle' || this.result || this.moves <= 0) return false;
  if (Math.abs(r1 - r2) + Math.abs(c1 - c2) !== 1) return false;
  var b = this.board, A = b.g[r1][c1], Bc = b.g[r2][c2], g = this.geo;
  this.stats.swaps++;
  this.snd.beep(440, 0.05, 'sine', 0.08);
  T.to(A, { x: g.OX + c2 * g.CELL, y: g.OY + r2 * g.CELL }, 0.16, 'inOutQuad');
  T.to(Bc, { x: g.OX + c1 * g.CELL, y: g.OY + r1 * g.CELL }, 0.16, 'inOutQuad');
  this.wait(function () {
    b.swap(A, Bc);
    this.combo = 0;
    if (A.sp === 'r' || Bc.sp === 'r') {                 // 彩虹球：与谁换就消谁那一色；两个彩虹球 = 清全盘
      this.moves--;
      var other = A.sp === 'r' ? Bc : A, seed = [A, Bc];
      if (A.sp === 'r' && Bc.sp === 'r') {
        seed = []; for (var r = 0; r < Board.N; r++) for (var c = 0; c < Board.N; c++) seed.push(b.g[r][c]);
      }
      this.combo = 1;
      this.resolve(seed, other.t, []);
      return;
    }
    var groups = b.groups();
    if (!groups.length) {                                  // 不成立：弹回去（outBack 冲过头再回来）
      this.stats.invalid++;
      b.swap(A, Bc);
      this.snd.beep(180, 0.12, 'square', 0.06, 0.7);
      T.to(A, { x: g.OX + c1 * g.CELL, y: g.OY + r1 * g.CELL }, 0.22, 'outBack');
      T.to(Bc, { x: g.OX + c2 * g.CELL, y: g.OY + r2 * g.CELL }, 0.22, 'outBack');
      this.wait(null);
      return;
    }
    this.moves--;
    this.clearGroups(groups, [A, Bc]);
  });
  return true;
};

P.clearGroups = function (groups, prefer) {
  var b = this.board, cells = [], creates = [];
  this.combo++;
  groups.forEach(function (gp) {
    gp.cells.forEach(function (x) { if (cells.indexOf(x) < 0) cells.push(x); });
    var s = b.special(gp, prefer);
    if (s) creates.push({ sp: s.sp, at: s.at, t: s.at.t });
  });
  this.resolve(cells, undefined, creates);
};

// 消掉 cells（连锁展开特效块）→ 生成 creates 里的特效块 → 下落 → 接着找三连
P.resolve = function (cells, rainbowColor, creates) {
  var b = this.board, self = this, res = b.expand(cells, rainbowColor);
  var combo = Math.max(1, this.combo);
  this.stats.maxCombo = Math.max(this.stats.maxCombo, combo);
  if (combo > 1) this.stats.cascades++;
  // 分数：每个 10 × 连击倍数，生成特效块 +20
  var gain = res.list.length * 10 * combo + creates.length * 20;
  this.score += gain;
  res.list.forEach(function (x) {
    if (x.t < 0) return;
    self.goals.forEach(function (gl) { if (gl.t === x.t && gl.left > 0) gl.left--; });
  });
  // 音：连击越多音越高；有特效块炸开再加一声滑音
  var semi = PENTA[Math.min(PENTA.length - 1, combo - 1)];
  this.snd.beep(523 * Math.pow(2, semi / 12), 0.14, 'triangle', 0.12);
  var cx = 0, cy = 0, geo = this.geo;
  res.effects.forEach(function (e) {
    var p = self.center(e.r, e.c);
    if (e.sp === 'h') FX.beam('h', p[0], p[1]);
    else if (e.sp === 'v') FX.beam('v', p[0], p[1]);
    else if (e.sp === 'b') FX.beam('ring', p[0], p[1]);
    else if (e.sp === 'r') FX.beam('rainbow', p[0], p[1]);
  });
  if (res.effects.length) { this.snd.beep(880, 0.25, 'sawtooth', 0.06, 0.4); this.snd.buzz(); }
  res.list.forEach(function (x) {
    cx += x.x; cy += x.y;
    FX.burst(x.x + geo.CELL / 2, x.y + geo.CELL / 2, x.t >= 0 ? F.COLORS[x.t] : '#ffffff', 5);
    var keep = false;
    creates.forEach(function (cr) { if (cr.at === x) keep = true; });
    T.to(x, { s: keep ? 1.25 : 0, a: keep ? 1 : 0 }, 0.2, 'inBack');
  });
  if (res.list.length) {
    cx /= res.list.length; cy /= res.list.length;
    FX.float('+' + gain, cx + geo.CELL / 2, cy + geo.CELL / 2, combo > 1 ? '#ffeb3b' : '#ffffff', combo > 1 ? 24 : 20);
    if (combo > 1) FX.float('连击 ×' + combo, geo.OX + geo.CELL * 4, geo.OY - 14, '#ff7043', 22);
  }
  this.wait(function () {
    // 特效块落在原位（替掉被消的那一格，它不在 res.set 里，跟着这一列一起下落）
    creates.forEach(function (cr) {
      var p = b.pos(cr.at); if (!p) return;
      var nc = b.cell(cr.sp === 'r' ? -1 : cr.t, cr.sp);
      self.home(nc, p[0], p[1]); nc.s = 0.4;
      b.g[p[0]][p[1]] = nc;
      T.to(nc, { s: 1 }, 0.28, 'outBack');
      self.stats.specials++;
    });
    var falls = b.collapse(res.set);
    falls.forEach(function (m) {
      var d = m.toR - m.fromR;
      m.cell.x = geo.OX + m.c * geo.CELL;
      if (m.fresh) { m.cell.y = geo.OY + m.fromR * geo.CELL; m.cell.s = 1; m.cell.a = 1; }
      T.to(m.cell, { y: geo.OY + m.toR * geo.CELL }, 0.16 + 0.045 * d, 'outBounce');
    });
    if (creates.length) this.snd.beep(1046, 0.18, 'sine', 0.1, 1.5);
    this.wait(this.cascade);
  });
};

P.cascade = function () {
  var groups = this.board.groups();
  if (groups.length) return this.clearGroups(groups, null);
  this.combo = 0;
  this.endCheck();
};

P.goalsDone = function () { return this.goals.every(function (g) { return g.left <= 0; }); };

P.endCheck = function () {
  var lv = this.lv;
  if (this.goalsDone()) {
    this.bonus = this.moves * 60;
    this.score += this.bonus;
    this.result = { win: true, stars: this.score >= lv.star3 ? 3 : this.score >= lv.star2 ? 2 : 1, score: this.score, bonus: this.bonus };
    var self = this;
    [523, 659, 784, 1046].forEach(function (f, i) { setTimeout(function () { self.snd.beep(f, 0.12, 'triangle', 0.12); }, i * 90); });
    return;
  }
  if (this.moves <= 0) {
    this.result = { win: false, stars: 0, score: this.score, bonus: 0 };
    this.snd.beep(330, 0.4, 'sawtooth', 0.1, 0.5);
    return;
  }
  if (!this.board.hasMove()) {                     // 死局：洗牌（缩一下再弹回来）
    this.stats.shuffles++;
    var b = this.board;
    b.shuffle();
    for (var r = 0; r < Board.N; r++) for (var c = 0; c < Board.N; c++) {
      var x = b.g[r][c]; this.home(x, r, c); x.s = 0.2;
      T.to(x, { s: 1 }, 0.35, 'outBack', (r + c) * 0.015);
    }
    FX.float('没有可走的步，重新洗牌', this.geo.OX + this.geo.CELL * 4, this.geo.OY + this.geo.CELL * 4, '#ffffff', 18);
    this.wait(null);
  }
};

// 自动试玩 / 提示：挑分最高的一步（同分随机）
P.bestMove = function () {
  var ms = this.board.moves();
  if (!ms.length) return null;
  var top = 0; ms.forEach(function (m) { m.score += Math.random() * 0.5; if (m.score > ms[top].score) top = ms.indexOf(m); });
  return ms[top];
};

module.exports = Match;
