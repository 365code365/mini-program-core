// 自动试玩的瞄准：把 8°~172° 每 1.5° 一个角度都模拟一遍（与瞄准线同一个 simulate），给落点打分挑一个。
// 一百多次模拟一帧做完在 QuickJS 里要几十毫秒，所以分到几帧里做（每帧 16 个角度），做完再瞄、再发。
// 打分：能凑成 ≥3 消除的最好（越大片越好，顺带打掉的悬空泡泡另加分）；凑不成就挨着同色放、越靠上越好；
// 不是每次都挑第一名（前三名里随机），免得自动试玩像外挂、每关都一样。
var Shot = require('./shot.js');

function Planner() { this.angles = []; for (var a = 8; a <= 172; a += 1.5) this.angles.push(a * Math.PI / 180); this.reset(); }
Planner.prototype.reset = function () { this.i = 0; this.results = []; this.done = false; };
Planner.prototype.step = function (grid, sx, sy, color, n) {
  for (var k = 0; k < n && this.i < this.angles.length; k++, this.i++) {
    var ang = this.angles[this.i], sim = Shot.simulate(grid, sx, sy, ang);
    if (!sim.cell) continue;
    this.results.push({ ang: ang, score: rate(grid, sim.cell, color) + sim.bounces * -2 });
  }
  if (this.i >= this.angles.length) this.done = true;
  return this.done;
};
Planner.prototype.pick = function () {
  var r = this.results.slice().sort(function (a, b) { return b.score - a.score; });
  if (!r.length) return { ang: Math.PI / 2, score: 0 };
  // 四分之一的时候「手滑」随便打一个落点：否则自动试玩几乎不会输，复活 / 结算那条路就走不到
  if (Math.random() < 0.25) return r[Math.floor(Math.random() * r.length)];
  var top = r.filter(function (x) { return x.score >= r[0].score - 15; }).slice(0, 3);
  return top[Math.floor(Math.random() * top.length)];
};

function rate(grid, cell, color) {
  var i = cell[0], c = cell[1];
  var cl = grid.cluster(i, c, color), n = cl.length;
  // cluster 从这一格出发数同色邻居：这一格本身还空着，所以 n 含它自己
  if (n >= 3) {
    var saved = [];
    cl.forEach(function (p) { saved.push(grid.get(p[0], p[1])); grid.rows[p[0]] && (grid.rows[p[0]][p[1]] = -1); });
    var drop = grid.floating().length;
    cl.forEach(function (p, k) { if (grid.rows[p[0]]) grid.rows[p[0]][p[1]] = saved[k]; });
    return 100 + n * 12 + drop * 18;
  }
  return n * 20 - i * 2;
}
module.exports = Planner;
