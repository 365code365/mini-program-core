// 自动试玩（?auto=1）：一个「会玩的人」。每 0.4 秒（模拟时间）想一次：
// 钱够就按顺序建下一座塔（箭、炮、箭、冰、电、炮…），位置挑「射程内路砖最多」的空地；建不起就升级最便宜的那座；
// 能出兵就立刻出兵（拿提前奖励）。看得出塔在打、钱在涨、波次在推进，无头长稳也靠它。
var D = require('data.js');
var Maps = require('maps.js');
var PLAN = ['arrow', 'cannon', 'arrow', 'ice', 'tesla', 'cannon', 'arrow', 'tesla', 'ice', 'cannon'];

function make(sim) {
  var acc = 0, n = 0, spots = null, mapIdx = -1;
  function rank(S) {
    var m = S.map, list = [];
    for (var k = 0; k < m.cover.length; k++) if (m.cover[k] > 0) list.push({ c: k % Maps.COLS, r: (k / Maps.COLS) | 0, v: m.cover[k] + Math.random() * 0.5 });
    list.sort(function (a, b) { return b.v - a.v; });
    return list;
  }
  function think(S) {
    if (mapIdx !== S.mapIdx) { spots = rank(S); mapIdx = S.mapIdx; n = S.towers.length; }
    var type = PLAN[n % PLAN.length];
    // 有飞行的波次之前：炮塔打不到飞的，换成箭塔
    if (type === 'cannon' && D.hasAir(S.wave) && n % 3 === 0) type = 'arrow';
    if (S.gold >= D.TOWERS[type].cost) {
      for (var i = 0; i < spots.length; i++) {
        if (sim.towerAt(spots[i].c, spots[i].r)) continue;
        if (sim.build(spots[i].c, spots[i].r, type)) { n++; return; }
      }
    }
    // 留够下一座塔的钱之外的才拿去升级（塔少的时候先铺塔）
    if (S.towers.length >= 4) {
      var best = null;
      S.towers.forEach(function (t) { var c = sim.upgradeCost(t); if (c && (!best || c < sim.upgradeCost(best))) best = t; });
      if (best && S.gold - sim.upgradeCost(best) >= (S.towers.length < 10 ? D.TOWERS[PLAN[n % PLAN.length]].cost * 0.5 : 0)) sim.upgrade(best);
    }
  }
  return {
    step: function (S, dt) {
      if (S.state !== 'play') return;
      acc += dt;
      if (acc < 0.4) return;
      acc = 0;
      think(S);
      if (sim.canCall() && (S.wave === 0 || S.countdown > 0)) sim.callWave();
    },
    reset: function () { acc = 0; n = 0; mapIdx = -1; }
  };
}

module.exports = { make: make };
