// 截图 / 真机对照用的「满岛」存档：query 里 demo=1 时，开局把整座岛扩开、每种建筑按不同等级摆一排、宠物全部住进小屋。
// 只改内存里的 state，不存盘（G.save 照常会写，测试机上用 fresh=1 再开一次就回到正常进度）。
// cam=x,z,dist,yaw 把镜头直接放到某处（近看模型用）。
var D = require('./data/defs.js');

function fill(state, terrain) {
  var N = terrain.N, spots = [];
  state.expanded = [];
  for (var z = 0; z < N; z++) for (var x = 0; x < N; x++) {
    var t = terrain.tile(x, z); if (!t || t.kind === 'water') continue;
    if (!terrain.unlocked(x, z)) state.expanded.push(terrain.key(x, z));
  }
  for (var z2 = 0; z2 < N; z2++) for (var x2 = 0; x2 < N; x2++) { var tt = terrain.tile(x2, z2); if (tt && tt.kind !== 'water') spots.push(tt); }
  // 离中心近的先用；渔港要海边
  spots.sort(function (a, b) { return (a.cx * a.cx + a.cz * a.cz) - (b.cx * b.cx + b.cz * b.cz); });
  var used = {}, uid = 1, plan = [
    ['farm', 1], ['farm', 6], ['lumber', 3], ['lumber', 12], ['quarry', 2], ['quarry', 16], ['warehouse', 4], ['warehouse', 11],
    ['market', 6], ['bakery', 2], ['bakery', 10], ['pethouse', 5], ['lighthouse', 5], ['apiary', 7], ['dock', 3], ['dock', 15], ['farm', 15]
  ];
  state.buildings = [];
  plan.forEach(function (p, i) {
    var want = D.BUILDINGS[p[0]].coastal;
    var s = spots.filter(function (t) { return !used[terrain.key(t.x, t.z)] && (!want || terrain.coastal(t.x, t.z)) && (want || (t.x + t.z) % 2 === 0); })[0];
    if (!s) return;
    used[terrain.key(s.x, s.z)] = true;
    state.buildings.push({ uid: uid++, type: p[0], x: s.x, z: s.z, rot: i % 4, lv: p[1], store: i % 3 ? D.BUILDINGS[p[0]].cap || 0 : 3, busyKind: null, busyUntil: 0 });
  });
  state.nextUid = uid;
  D.PETS.forEach(function (p) { state.pets[p.id] = { n: 1, home: true }; });
  state.lv = 12; state.tutorial = 2;
  state.res.coin = 88888; state.res.gem = 999;
}

module.exports = { fill: fill };
