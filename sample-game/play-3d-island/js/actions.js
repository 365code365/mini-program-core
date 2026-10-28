// 玩家操作（界面、点地图、自动驾驶都调这里，规则只写一遍）：收获、升级、加速、旋转、建造摆放、扩地、点地图。
var D = require('./data/defs.js');
var E = require('./core/economy.js');

function make(G) {
  var s = G.state;
  function uidNext() { return s.nextUid++; }

  function collect(b) {
    var d = D.BUILDINGS[b.type];
    if (!d.produce) return 0;
    var cap = G.eco.cap(d.produce);
    if (s.res[d.produce] >= cap) { G.ui.toast(D.RES[d.produce].name + '仓库满了，造仓库或去集市卖掉', d.produce); G.sfx('error'); return 0; }
    var n = G.eco.collect(b);
    if (n > 0) {
      var p = G.city.pos(b), sp = G.camera.project(p.x, p.y, p.z), to = G.hud.slot(d.produce);
      G.ui.fly(d.produce, sp.x, sp.y, to.x, to.y, Math.min(6, 2 + Math.floor(n / 10)));
      G.city.burst(b, d.produce, 4);
      G.sfx('collect'); G.vibrate();
      if (s.tutorial === 1) s.tutorial = 2;
    }
    return n;
  }
  function upgrade(b) {
    var d = D.BUILDINGS[b.type];
    if (b.busyKind) return G.ui.toast('施工中，等它完工');
    if (b.lv >= d.maxLv) return G.ui.toast('已经满级了');
    var cost = G.eco.upgradeCost(b);
    if (!G.eco.has(cost)) { var m = G.eco.missing(cost), k = Object.keys(m)[0]; G.sfx('error'); return G.ui.toast(D.RES[k].name + '不够（还差 ' + E.fmt(m[k]) + '）', k); }
    collect(b);
    G.eco.spend(cost, 'upgrade');
    var t = G.eco.upgradeTime(b);
    b.busyKind = 'upgrade'; b.busyUntil = Date.now() + t * 1000; b.busyTotal = t;
    G.sfx('build'); G.city.sync(); G.save();
  }
  function skip(b) {
    if (!b.busyKind) return;
    var left = (b.busyUntil - Date.now()) / 1000, cost = { gem: E.skipCost(left) };
    var go = function () {
      if (!G.eco.spend(cost, 'skip')) { G.ui.toast('钻石不够', 'gem'); return G.open('shop', 'gems'); }
      b.busyUntil = Date.now(); G.sfx('upgrade'); G.save();
    };
    if (cost.gem < 20 || G.auto) return go();
    wx.showModal({ title: '立即完成', content: '花 ' + cost.gem + ' 钻石马上完工？', confirmText: '完成', success: function (r) { if (r.confirm) go(); } });
  }
  function rotate(b) { b.rot = ((b.rot || 0) + 1) % 4; G.city.sync(); G.sfx('whoosh'); G.save(); }

  // ───────── 摆放 ─────────
  function check(type, x, z) {
    var d = D.BUILDINGS[type], t = G.terrain.tile(x, z);
    if (!t || t.kind === 'water') return '这里是海';
    if (!G.terrain.unlocked(x, z)) return '这块地还没开拓';
    if (G.city.at(x, z)) return '这里已经有建筑了';
    if (d.coastal && !G.terrain.coastal(x, z)) return d.name + '只能建在海边';
    if (t.kind === 'hill' && type === 'farm') return '小丘上种不了麦子';
    return null;
  }
  function startPlace(type) {
    G.select(null);
    var tt = G.terrain.tileAt(G.camera.state().x, G.camera.state().z) || G.terrain.tile(G.terrain.N >> 1, G.terrain.N >> 1);
    var best = bestSpot(type, tt.x, tt.z);
    G.placing = { type: type, x: best[0], z: best[1], rot: 0, ok: false, why: '', dirty: true };
    G.camera.focus(G.terrain.tile(best[0], best[1]).cx, G.terrain.tile(best[0], best[1]).cz);
    refreshPlace();
    G.hud.hide(); G.build.showPlace(true);
  }
  // 离 (x, z) 最近的能放的地格（没有就原地）
  function bestSpot(type, x, z) {
    var best = null, bd = 1e9;
    for (var zz = 0; zz < G.terrain.N; zz++) for (var xx = 0; xx < G.terrain.N; xx++) {
      if (check(type, xx, zz)) continue;
      var d2 = (xx - x) * (xx - x) + (zz - z) * (zz - z);
      if (d2 < bd) { bd = d2; best = [xx, zz]; }
    }
    return best || [x, z];
  }
  function movePlace(x, z) { if (!G.placing) return; G.placing.x = x; G.placing.z = z; refreshPlace(); }
  function refreshPlace() {
    var p = G.placing; if (!p) return;
    var why = check(p.type, p.x, p.z);
    p.ok = !why; p.why = why || ''; p.dirty = true;
    G.city.showGhost(p.type, p.x, p.z, p.ok);
  }
  function cancelPlace() { G.placing = null; G.city.hideGhost(); G.build.showPlace(false); G.hud.show(); }
  function rotatePlace() { if (G.placing) { G.placing.rot = (G.placing.rot + 1) % 4; G.sfx('whoosh'); } }
  function confirmPlace() {
    var p = G.placing; if (!p) return false;
    if (!p.ok) { G.ui.toast(p.why); G.sfx('error'); return false; }
    var cost = G.eco.buildCost(p.type);
    if (!G.eco.spend(cost, 'build')) { G.ui.toast('资源不够了'); G.sfx('error'); return false; }
    var t = Math.max(3, Math.round(D.BUILDINGS[p.type].time * 0.8));
    var b = { uid: uidNext(), type: p.type, x: p.x, z: p.z, rot: p.rot, lv: 1, store: 0, busyKind: 'build', busyUntil: Date.now() + t * 1000, busyTotal: t };
    s.buildings.push(b);
    cancelPlace();
    G.terrain.rebuildDecor(G.city.occupied());
    G.city.sync();
    G.sfx('build'); G.vibrate();
    if (s.tutorial === 0) s.tutorial = 1;
    G.save();
    return b;
  }

  // ───────── 扩地 ─────────
  function expand(x, z) {
    var cost = D.expandCost(s.expanded.length);
    if (!G.terrain.expandable(x, z)) return false;
    if (!G.eco.spend(cost, 'expand')) { G.ui.toast('金币不够', 'coin'); G.sfx('error'); return false; }
    s.expanded.push(G.terrain.key(x, z));
    G.terrain.rebuild(G.city.occupied());
    G.sfx('upgrade'); G.ui.toast('开拓了新地块！', 'island'); G.save();
    return true;
  }

  // ───────── 点地图 ─────────
  function tapWorld(sx, sy) {
    var p = G.camera.pick(sx, sy); if (!p) return;
    var t = G.terrain.tileAt(p.x, p.z); if (!t) return;
    if (G.placing) { movePlace(t.x, t.z); G.camera.focus(t.cx, t.cz); return; }
    var b = G.city.at(t.x, t.z);
    if (b) {
      if (!b.busyKind && D.BUILDINGS[b.type].produce && b.store >= 1) collect(b);
      G.select(b); G.sfx('tap');
      return;
    }
    G.select(null);
    if (G.terrain.expandable(t.x, t.z)) G.dialogs.expand(t.x, t.z, D.expandCost(s.expanded.length));
    else if (!G.terrain.unlocked(t.x, t.z) && t.kind !== 'water') G.ui.toast('先开拓挨着岛的地块', 'lock');
  }
  // 摆放中：镜头中心下的地格跟着走（拖地图 = 挪虚影）
  function followCenter() {
    if (!G.placing) return;
    var c = G.camera.state(), t = G.terrain.tileAt(c.x, c.z);
    if (t && (t.x !== G.placing.x || t.z !== G.placing.z)) movePlace(t.x, t.z);
  }

  return { collect: collect, upgrade: upgrade, skip: skip, rotate: rotate, startPlace: startPlace, cancelPlace: cancelPlace, rotatePlace: rotatePlace,
    confirmPlace: confirmPlace, expand: expand, tapWorld: tapWorld, followCenter: followCenter, check: check, bestSpot: bestSpot, movePlace: movePlace };
}

module.exports = { make: make };
