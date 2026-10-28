// 岛上会走的：村民（在空地格上按 BFS 去随机一座建筑旁边，到了站一会儿、东张西望）与宠物（住进小屋的在小屋附近的空地上闲逛：
// 走一段 → 停下摇尾巴 / 低头闻闻 → 偶尔蹦一下）。动画只写 iWalk（步伐相位、摆幅、尾巴相位、抬头角），四肢在着色器里摆。
var D = require('../data/defs.js');
var T = require('./terrain.js');

var MAX_WALKERS = 10;

function make(THREE, M, terrain, state, group, at) {
  var rnd = T.seeded(4242), clock = 0;
  var crowd = M.crowd(MAX_WALKERS); group.add(crowd.mesh);
  var blobs = M.blobs(MAX_WALKERS + D.PETS.length); group.add(blobs.mesh);
  function walkable(x, z) { return terrain.unlocked(x, z) && !at(x, z); }
  function groundY(x, z) { var t = terrain.tileAt(x, z); return t ? t.h : 0.3; }
  // 转身：最短方向插值到目标朝向
  function turn(cur, want, k) { var d = want - cur; while (d > Math.PI) d -= Math.PI * 2; while (d < -Math.PI) d += Math.PI * 2; return cur + d * Math.min(1, k); }

  // ───────── 村民 ─────────
  var walkers = [];
  function bfs(sx, sz, goal) {
    var Q = [[sx, sz]], prev = {}, k0 = terrain.key(sx, sz); prev[k0] = null;
    while (Q.length) {
      var c = Q.shift();
      if (goal(c[0], c[1])) { var path = [], k = terrain.key(c[0], c[1]); while (k) { var p = k.split(','); path.unshift([+p[0], +p[1]]); k = prev[k]; } return path; }
      [[1, 0], [-1, 0], [0, 1], [0, -1]].forEach(function (o) {
        var nx = c[0] + o[0], nz = c[1] + o[1], nk = terrain.key(nx, nz);
        if (nk in prev || !walkable(nx, nz)) return;
        prev[nk] = terrain.key(c[0], c[1]); Q.push([nx, nz]);
      });
    }
    return null;
  }
  function newTrip(w) {
    if (!state.buildings.length) return;
    var b = state.buildings[Math.floor(rnd() * state.buildings.length)];
    var path = bfs(w.x, w.z, function (x, z) { return Math.abs(x - b.x) + Math.abs(z - b.z) === 1 && (x !== w.x || z !== w.z); });
    w.path = path ? path.slice(1) : null; w.wait = path ? 0 : 1 + rnd() * 2;
    // 路点在格子里错开一点（几个人不会走成一条线、踩在同一个点上）
    w.jx = (rnd() - 0.5) * 0.4; w.jz = (rnd() - 0.5) * 0.4;
    if (b) { var tb = terrain.tile(b.x, b.z); w.face = tb; }
  }
  function syncWalkers() {
    var want = Math.min(MAX_WALKERS, 2 + state.buildings.length);
    while (walkers.length < want) {
      var spots = [];
      for (var z = 0; z < terrain.N; z++) for (var x = 0; x < terrain.N; x++) if (walkable(x, z)) spots.push([x, z]);
      if (!spots.length) break;
      var s = spots[Math.floor(rnd() * spots.length)], t = terrain.tile(s[0], s[1]);
      walkers.push({ px: t.cx, pz: t.cz, yaw: rnd() * 6.28, x: s[0], z: s[1], path: null, wait: rnd() * 2, speed: 0.45 + rnd() * 0.25, phase: rnd() * 6, amp: 0, jx: 0, jz: 0, look: 0 });
    }
  }
  function stepWalkers(dt) {
    walkers.forEach(function (w, i) {
      var moving = false;
      if (w.wait > 0) {
        w.wait -= dt; if (w.wait <= 0) newTrip(w);
        if (w.face) w.yaw = turn(w.yaw, Math.atan2(w.face.cx - w.px, w.face.cz - w.pz), dt * 4);
      } else if (!w.path || !w.path.length) { w.wait = 1.5 + rnd() * 3; }
      else {
        var nx = w.path[0], t = terrain.tile(nx[0], nx[1]);
        if (!walkable(nx[0], nx[1])) { w.path = null; w.wait = 0.5; }
        else {
          var last = w.path.length === 1, tx = t.cx + w.jx, tz = t.cz + w.jz;
          var dx = tx - w.px, dz = tz - w.pz, d = Math.sqrt(dx * dx + dz * dz), step = w.speed * dt;
          if (d <= step) { w.px = tx; w.pz = tz; w.x = nx[0]; w.z = nx[1]; w.path.shift(); if (last) w.wait = 1.5 + rnd() * 3; }
          else { w.px += dx / d * step; w.pz += dz / d * step; w.yaw = turn(w.yaw, Math.atan2(dx, dz), dt * 8); moving = true; }
        }
      }
      // 步伐：走的时候按速度推相位，摆幅渐变（停下时手脚慢慢收回来，不会「啪」地定住）
      w.amp += ((moving ? 0.75 : 0) - w.amp) * Math.min(1, dt * 6);
      w.phase += dt * (moving ? w.speed * 22 : 2);
      w.look = moving ? 0 : Math.sin(clock * 0.7 + i) * 0.15;
      var bob = Math.abs(Math.sin(w.phase)) * 0.012 * w.amp / 0.75;
      var y = groundY(w.px, w.pz);
      crowd.set(i, w.px, y + bob, w.pz, w.yaw, w.phase, w.amp, w.look);
      blobs.set(i, w.px, y, w.pz, 0.2);
    });
    crowd.commit();
  }

  // ───────── 宠物 ─────────
  var pets = {}, petList = [];
  function house() { return state.buildings.filter(function (b) { return b.type === 'pethouse' && b.busyKind !== 'build'; })[0]; }
  function petSpot(h) {
    // 小屋所在格 + 周围 8 格里的空地（小屋格本身只用院子外圈）
    var c = [];
    for (var dz = -1; dz <= 1; dz++) for (var dx = -1; dx <= 1; dx++) { var x = h.x + dx, z = h.z + dz; if ((dx || dz) && walkable(x, z)) c.push([x, z]); }
    var t = terrain.tile(h.x, h.z);
    if (!c.length || rnd() < 0.35) { var a = rnd() * 6.28; return { x: t.cx + Math.cos(a) * 0.35, z: t.cz + Math.sin(a) * 0.35 }; }
    var s = c[Math.floor(rnd() * c.length)], ts = terrain.tile(s[0], s[1]);
    return { x: ts.cx + (rnd() - 0.5) * 0.7, z: ts.cz + (rnd() - 0.5) * 0.7 };
  }
  function syncPets() {
    var h = house();
    D.PETS.forEach(function (def) {
      var o = state.pets[def.id], want = h && o && o.home;
      if (want && !pets[def.id]) {
        var m = M.petRig(def), t = terrain.tile(h.x, h.z), a = rnd() * 6.28;
        m.scale.setScalar(def.rarity === 'SSR' ? 0.82 : 0.72);   // 0.3 高的模型缩到 ~0.22：村民 0.44，宠物到人腰上下
        group.add(m);
        pets[def.id] = { def: def, mesh: m, px: t.cx + Math.cos(a) * 0.4, pz: t.cz + Math.sin(a) * 0.4, yaw: a, goal: null, wait: rnd() * 2, hop: 0, phase: rnd() * 6, amp: 0, wag: 0, speed: 0.22 + rnd() * 0.1 };
      }
      if (!want && pets[def.id]) { group.remove(pets[def.id].mesh); pets[def.id].mesh.geometry.dispose(); delete pets[def.id]; }
    });
    if (Object.keys(pets).length !== petList.length) petList = Object.keys(pets).map(function (k) { return pets[k]; });
    return h;
  }
  function stepPets(dt, h) {
    var n = walkers.length;
    petList.forEach(function (p, i) {
      var moving = false;
      if (h) {
        if (p.wait > 0) { p.wait -= dt; if (p.wait <= 0) p.goal = petSpot(h); }
        else if (p.goal) {
          var dx = p.goal.x - p.px, dz = p.goal.z - p.pz, d = Math.sqrt(dx * dx + dz * dz), st = p.speed * dt;
          // 别和别的宠物挤在一起：近了就往外推一点
          petList.forEach(function (o) { if (o === p) return; var ox = p.px - o.px, oz = p.pz - o.pz, od = ox * ox + oz * oz; if (od < 0.025 && od > 1e-6) { p.px += ox * dt * 4; p.pz += oz * dt * 4; } });
          var ht = terrain.tile(h.x, h.z), rx = p.px - ht.cx, rz = p.pz - ht.cz, gx = p.goal.x - ht.cx, gz = p.goal.z - ht.cz;
          var r0 = Math.sqrt(rx * rx + rz * rz), r1 = Math.sqrt(gx * gx + gz * gz);
          if (d <= st) { p.goal = null; p.wait = 1 + rnd() * 3; if (rnd() < 0.3) p.hop = 1; }
          else if (r0 < 0.5 && r1 < 0.5 && d > 0.15) {
            // 都在小屋这一格里：绕着屋子走（沿圆弧），直线过去会从屋子中间穿过去
            var a0 = Math.atan2(rz, rx), a1 = Math.atan2(gz, gx), da = a1 - a0;
            while (da > Math.PI) da -= Math.PI * 2; while (da < -Math.PI) da += Math.PI * 2;
            var rr = Math.max(0.3, r0 + (r1 - r0) * 0.1), na = a0 + Math.sign(da) * Math.min(Math.abs(da), st / rr);
            var nx2 = ht.cx + Math.cos(na) * rr, nz2 = ht.cz + Math.sin(na) * rr;
            p.yaw = turn(p.yaw, Math.atan2(nx2 - p.px, nz2 - p.pz), dt * 6); p.px = nx2; p.pz = nz2; moving = true;
          }
          else { p.px += dx / d * st; p.pz += dz / d * st; p.yaw = turn(p.yaw, Math.atan2(dx, dz), dt * 6); moving = true; }
        } else p.wait = 0.5;
      }
      p.amp += ((moving ? 0.9 : 0) - p.amp) * Math.min(1, dt * 6);
      p.phase += dt * (moving ? 26 : 1.5);
      p.wag += dt * (moving ? 8 : 12);
      p.hop = Math.max(0, p.hop - dt * 2.2);
      var y = groundY(p.px, p.pz), jump = Math.sin(p.hop * Math.PI) * 0.12;
      p.mesh.position.set(p.px, y + jump + Math.abs(Math.sin(p.phase)) * 0.01 * p.amp, p.pz);
      p.mesh.rotation.y = p.yaw;
      p.mesh.userData.anim(p.phase, p.amp, p.wag, moving ? 0 : Math.sin(clock * 0.9 + i * 2) * 0.18);
      blobs.set(n + i, p.px, y, p.pz, 0.2 * (1 - jump * 2));
    });
    blobs.commit(n + petList.length);
  }

  return {
    sync: function () { syncWalkers(); return syncPets(); },
    step: function (dt) { clock += dt; var h = syncPets(); stepWalkers(dt); stepPets(dt, h); },
    walkers: function () { return walkers.length; },
    walkerPos: function (i) { var w = walkers[i]; return w ? { x: w.px, z: w.pz } : null; },
    pets: function () { return petList.length; }
  };
}

module.exports = { make: make };
