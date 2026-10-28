// 岛上的东西：建筑（按存档同步，等级跨档或施工状态变了才重建网格）、施工脚手架、摆放预览（虚影 + 绿 / 红底圈）、
// 选中（弹一下 + 底圈）、头顶气泡（可收 / 满仓 / 施工中，Sprite）、会动的零件、烟；村民与宠物在 crowd.js。
//
// 宠物与村民都是真 3D 模型（characters.js / pets3d.js），会走路、摆腿。v3 用立绘做过广告牌（Sprite），
// 结果是一张张平面贴纸，比楼还大、互相叠着，和 3D 场景对不上，已经去掉（立绘只留在界面里）。
//
// 气泡防重叠：建筑一多，头顶的气泡在屏幕上挤成一团（点不准、看着乱）。每 4 帧把气泡投影到屏幕，
// 按优先级（满仓 > 施工 > 可收，同级按库存比例）贪心保留：离已保留的气泡不到 MIN_GAP 逻辑像素的先藏起来，
// 收掉一个、镜头拉近，藏着的自然露出来。点地块收获不依赖气泡，藏起来的也照样能点楼收。
var D = require('../data/defs.js');
var T = require('./terrain.js');
var Crowd = require('./crowd.js');

var MIN_GAP = 34, BUBBLE = 0.4;

function make(THREE, M, terrain, state, icons, project) {
  var group = new THREE.Group(), items = {}, clock = 0, rnd = T.seeded(99), frame = 0;
  var ringGeo = new THREE.RingGeometry(0.48, 0.56, 32); ringGeo.rotateX(-Math.PI / 2);
  function tileOf(b) { return terrain.tile(b.x, b.z); }
  function tier(lv) { return lv >= 15 ? 3 : lv >= 10 ? 2 : lv >= 5 ? 1 : 0; }

  // ───────── 气泡 ─────────
  function bubble(name) {
    var s = new THREE.Sprite(new THREE.SpriteMaterial({ map: icons.texture(name), depthTest: false, transparent: true }));
    s.scale.set(BUBBLE, BUBBLE, 1); s.renderOrder = 10; s.userData.icon = name;
    return s;
  }
  function setBubble(it, name) {
    if (it.bubbleName === name) return;
    if (it.bubble) { it.root.remove(it.bubble); it.bubble.material.dispose(); it.bubble = null; }
    it.bubbleName = name;
    if (name) { it.bubble = bubble(name); it.root.add(it.bubble); }
  }

  // ───────── 建筑 ─────────
  function visualKey(b) { return b.type + ':' + (b.busyKind === 'build' ? 'scaffold' : tier(b.lv)); }
  function sync() {
    var seen = {};
    state.buildings.forEach(function (b) {
      seen[b.uid] = true;
      var it = items[b.uid];
      if (!it) { it = items[b.uid] = { root: new THREE.Group(), vk: null, bounce: 0 }; group.add(it.root); }
      var t = tileOf(b);
      it.root.position.set(t.cx, t.h, t.cz);
      it.root.rotation.y = (b.rot || 0) * Math.PI / 2;
      var vk = visualKey(b);
      if (vk !== it.vk) {
        if (it.model) { it.root.remove(it.model.group); it.model.group.traverse(function (o) { if (o.geometry && o.geometry !== ringGeo && !o.geometry.userData.shared) o.geometry.dispose(); }); }
        it.model = b.busyKind === 'build' ? { group: new THREE.Group().add(M.scaffold()), parts: { top: 0.7 } } : M.building(b.type, b.lv);
        it.root.add(it.model.group);
        it.top = Math.max(0.55, it.model.parts.top || 0.7);
        if (it.vk) it.bounce = 1;
        it.vk = vk;
      }
    });
    for (var uid in items) if (!seen[uid]) { group.remove(items[uid].root); delete items[uid]; }
  }
  function occupied() { var o = {}; state.buildings.forEach(function (b) { o[terrain.key(b.x, b.z)] = b; }); return o; }
  function at(x, z) { return state.buildings.filter(function (b) { return b.x === x && b.z === z; })[0] || null; }
  var crowd = Crowd.make(THREE, M, terrain, state, group, at);

  // ───────── 摆放预览 ─────────
  var ghost = null, ghostRing = new THREE.Mesh(ringGeo, new THREE.MeshBasicMaterial({ color: 0x55ff88, transparent: true, opacity: 0.85, depthWrite: false }));
  ghostRing.visible = false; group.add(ghostRing);
  function showGhost(type, x, z, ok) {
    if (!ghost || ghost.userData.type !== type) {
      if (ghost) group.remove(ghost);
      ghost = M.building(type, 1).group; ghost.userData.type = type;
      ghost.traverse(function (o) { if (o.material) { o.material = o.material.clone(); o.material.transparent = true; o.material.opacity = 0.6; } });
      group.add(ghost);
    }
    var t = terrain.tile(x, z); if (!t) return;
    ghost.visible = true; ghost.position.set(t.cx, t.h + 0.05 + Math.sin(clock * 6) * 0.03, t.cz);
    ghostRing.visible = true; ghostRing.position.set(t.cx, t.h + 0.02, t.cz);
    ghostRing.material.color.set(ok ? 0x55ff88 : 0xff5566);
  }
  function hideGhost() { if (ghost) ghost.visible = false; ghostRing.visible = false; }

  // ───────── 选中 ─────────
  var selRing = new THREE.Mesh(ringGeo, new THREE.MeshBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.9, depthWrite: false }));
  selRing.visible = false; group.add(selRing);
  var selected = null;
  function select(b) {
    selected = b ? b.uid : null;
    if (b) { items[b.uid].bounce = 0.6; var t = tileOf(b); selRing.position.set(t.cx, t.h + 0.02, t.cz); }
    selRing.visible = !!b;
  }

  // ───────── 烟（面包坊）：小球往上飘、变大、变淡；对象池 ─────────
  var puffs = [], puffGeo = new THREE.IcosahedronGeometry(0.05, 1);
  function puff(pos) {
    var p = puffs.filter(function (q) { return !q.visible; })[0];
    if (!p) { p = new THREE.Mesh(puffGeo, new THREE.MeshLambertMaterial({ color: 0xf4f4f4, transparent: true, depthWrite: false })); puffs.push(p); group.add(p); }
    p.visible = true; p.position.copy(pos); p.userData.t = 0; p.scale.setScalar(1);
  }
  var sparks = [];
  function burst(b, name, n) {
    var t = tileOf(b), it = items[b.uid], y0 = t.h + (it ? it.top : 0.6);
    for (var i = 0; i < Math.min(8, n); i++) {
      var s = bubble(name); s.scale.set(0.2, 0.2, 1);
      s.position.set(t.cx, y0, t.cz);
      s.userData.v = new THREE.Vector3((rnd() - 0.5) * 1.6, 2 + rnd() * 1.2, (rnd() - 0.5) * 1.6); s.userData.t = 0;
      group.add(s); sparks.push(s);
    }
  }

  // 气泡防重叠（见文件头）
  var wp = new THREE.Vector3();
  function declutter() {
    var list = [];
    state.buildings.forEach(function (b) {
      var it = items[b.uid]; if (!it || !it.bubble) return;
      it.bubble.getWorldPosition(wp);
      var sp = project(wp.x, wp.y, wp.z), full = /!$/.test(it.bubbleName);
      list.push({ it: it, x: sp.x, y: sp.y, pri: (full ? 3 : it.bubbleName === 'hammer' ? 2 : 1) + (b.store || 0) / 1e4 });
    });
    list.sort(function (a, b) { return b.pri - a.pri; });
    var kept = [];
    list.forEach(function (e) {
      var ok = kept.every(function (k) { var dx = k.x - e.x, dy = k.y - e.y; return dx * dx + dy * dy >= MIN_GAP * MIN_GAP; });
      e.it.bubble.visible = ok; if (ok) kept.push(e);
    });
  }

  return {
    group: group, sync: function () { sync(); crowd.sync(); }, at: at, occupied: occupied, select: select, selected: function () { return selected; },
    showGhost: showGhost, hideGhost: hideGhost, burst: burst,
    pos: function (b) { var t = tileOf(b), it = items[b.uid]; return { x: t.cx, y: t.h + (it ? it.top : 0.8), z: t.cz }; },
    walkers: crowd.walkers, petsOut: crowd.pets, walkerPos: crowd.walkerPos,
    bubblesShown: function () { var n = 0; for (var k in items) if (items[k].bubble && items[k].bubble.visible) n++; return n; },
    step: function (dt, now, night, eco) {
      clock += dt; frame++;
      state.buildings.forEach(function (b) {
        var it = items[b.uid]; if (!it) return;
        var P = it.model.parts, d = D.BUILDINGS[b.type];
        if (P.spin) P.spin.rotation.z += dt * 2.5;
        if (P.spinZ) P.spinZ.rotation.z -= dt * 9;
        if (P.bob) { P.bob.position.y = Math.sin(clock * 1.6 + b.uid) * 0.02; P.bob.rotation.z = Math.sin(clock * 1.2) * 0.06; }
        if (P.beam) { P.beam.visible = night; P.beam.rotation.y += dt * 0.8; }
        if (P.chimney && b.store < eco.storeCap(b) && rnd() < dt * 2.5) puff(it.root.localToWorld(P.chimney.clone()));
        if (it.bounce > 0) { it.bounce = Math.max(0, it.bounce - dt * 2); var k = 1 + Math.sin(it.bounce * Math.PI * 3) * it.bounce * 0.15; it.model.group.scale.set(k, 1 / k, k); }
        var name = null;
        if (b.busyKind) name = 'hammer';
        else if (d.produce && b.store >= 1) name = b.store >= eco.storeCap(b) ? d.produce + '!' : d.produce;
        setBubble(it, name);
        if (it.bubble) {
          var full = /!$/.test(name);
          it.bubble.position.y = it.top + 0.28 + Math.sin(clock * 3 + b.uid) * 0.04;
          var s = full ? BUBBLE * 1.15 + Math.sin(clock * 8) * 0.03 : BUBBLE; it.bubble.scale.set(s, s, 1);
        }
      });
      if (frame % 4 === 0) declutter();
      puffs.forEach(function (p) {
        if (!p.visible) return;
        p.userData.t += dt; var t = p.userData.t;
        p.position.y += dt * 0.3; p.position.x += dt * 0.08;
        p.scale.setScalar(1 + t * 2); p.material.opacity = Math.max(0, 0.75 - t * 0.45);
        if (t > 1.6) p.visible = false;
      });
      sparks = sparks.filter(function (s) {
        s.userData.t += dt; s.userData.v.y -= 9 * dt;
        s.position.addScaledVector(s.userData.v, dt);
        s.material.opacity = Math.max(0, 1 - s.userData.t * 1.4);
        if (s.userData.t > 0.75) { group.remove(s); s.material.dispose(); return false; }
        return true;
      });
      crowd.step(dt);
      if (selected && items[selected]) selRing.material.opacity = 0.55 + Math.sin(clock * 5) * 0.35;
    }
  };
}

module.exports = { make: make };
