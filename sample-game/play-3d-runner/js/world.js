// 跑酷的 3D 世界：玩家原地跑（z = 0，面朝 -z），路面、楼房、障碍、金币以速度往 +z 流过来，出了镜头就回收重用。
// 路面是一张贴图（跑道线），卷动只改纹理偏移；楼房、障碍、金币都是对象池，不在游戏中途新建网格。
var LANE = 2.2, SPAWN_Z = -140, KILL_Z = 12;

function make(THREE, aspect) {
  var scene = new THREE.Scene();
  var SKY = 0x9fd8ff;
  scene.background = new THREE.Color(SKY);
  scene.fog = new THREE.Fog(SKY, 40, 135);
  var cam = new THREE.PerspectiveCamera(62, aspect, 0.1, 200);
  scene.add(new THREE.HemisphereLight(0xffffff, 0x557755, 0.9));
  var sun = new THREE.DirectionalLight(0xffffff, 0.9); sun.position.set(-5, 10, 6); scene.add(sun);

  // ───────── 路面：跑道线画在离屏画布上当纹理，按跑过的距离卷 ─────────
  var tc = wx.createCanvas(); tc.width = 256; tc.height = 256;
  var g = tc.getContext('2d');
  g.fillStyle = '#5b5f6b'; g.fillRect(0, 0, 256, 256);
  g.fillStyle = '#4c505b'; for (var i = 0; i < 40; i++) g.fillRect(Math.random() * 256, Math.random() * 256, 6, 3);
  g.fillStyle = '#e8e8e8'; g.fillRect(0, 0, 6, 256); g.fillRect(250, 0, 6, 256);            // 路肩
  g.fillStyle = '#d0d0d0'; g.fillRect(83, 0, 4, 128); g.fillRect(169, 0, 4, 128);           // 虚线
  var roadTex = new THREE.CanvasTexture(tc);
  roadTex.wrapS = THREE.ClampToEdgeWrapping; roadTex.wrapT = THREE.RepeatWrapping; roadTex.repeat.set(1, 30);
  if (THREE.SRGBColorSpace) roadTex.colorSpace = THREE.SRGBColorSpace;
  var road = new THREE.Mesh(new THREE.PlaneGeometry(LANE * 3 + 0.6, 220), new THREE.MeshLambertMaterial({ map: roadTex }));
  road.rotation.x = -Math.PI / 2; road.position.z = -90; scene.add(road);
  var grass = new THREE.Mesh(new THREE.PlaneGeometry(200, 220), new THREE.MeshLambertMaterial({ color: 0x78c060 }));
  grass.rotation.x = -Math.PI / 2; grass.position.set(0, -0.02, -90); scene.add(grass);

  // ───────── 对象池 ─────────
  var boxG = new THREE.BoxGeometry(1, 1, 1);
  var mats = {};
  function mat(c) { return mats[c] || (mats[c] = new THREE.MeshLambertMaterial({ color: c })); }
  var BUILD_COLORS = [0xf4a261, 0xe9c46a, 0x2a9d8f, 0xe76f51, 0x8ab17d, 0xb5838d, 0x6d6875, 0xa8dadc];
  var buildings = [];
  for (var b = 0; b < 36; b++) {
    var m = new THREE.Mesh(boxG, mat(BUILD_COLORS[b % BUILD_COLORS.length]));
    var side = b % 2 ? 1 : -1;
    resetBuilding(m, side, SPAWN_Z + (b >> 1) * 8.5);
    m.userData.side = side;
    scene.add(m); buildings.push(m);
  }
  function resetBuilding(m, side, z) {
    var w = 3 + Math.random() * 4, h = 4 + Math.random() * 14, d = 4 + Math.random() * 4;
    m.scale.set(w, h, d);
    m.position.set(side * (LANE * 1.5 + 3 + w / 2 + Math.random() * 2), h / 2, z);
    m.material = mat(BUILD_COLORS[Math.floor(Math.random() * BUILD_COLORS.length)]);
  }

  // 障碍：low 矮栏（跳）、high 高栏（铲：下面能过）、train 火车（只能换道）
  var obstacles = [], coins = [];
  function makeObstacle(kind) {
    var grp = new THREE.Group();
    if (kind === 'low') {
      var bar = new THREE.Mesh(boxG, mat(0xe63946)); bar.scale.set(1.9, 0.35, 0.3); bar.position.y = 0.75; grp.add(bar);
      var stripe = new THREE.Mesh(boxG, mat(0xffffff)); stripe.scale.set(0.5, 0.36, 0.31); stripe.position.y = 0.75; grp.add(stripe);
      [-0.8, 0.8].forEach(function (x) { var l = new THREE.Mesh(boxG, mat(0x333333)); l.scale.set(0.12, 0.9, 0.12); l.position.set(x, 0.45, 0); grp.add(l); });
      grp.userData = { kind: kind, depth: 0.4 };
    } else if (kind === 'high') {
      var top = new THREE.Mesh(boxG, mat(0xf4a300)); top.scale.set(2.0, 0.5, 0.3); top.position.y = 1.9; grp.add(top);
      [-0.9, 0.9].forEach(function (x) { var l = new THREE.Mesh(boxG, mat(0x333333)); l.scale.set(0.15, 2.2, 0.15); l.position.set(x, 1.1, 0); grp.add(l); });
      grp.userData = { kind: kind, depth: 0.4 };
    } else {
      var body = new THREE.Mesh(boxG, mat(0x3d5a80)); body.scale.set(2.0, 2.6, 12); body.position.y = 1.5; grp.add(body);
      var band = new THREE.Mesh(boxG, mat(0xee6c4d)); band.scale.set(2.02, 0.5, 12.02); band.position.y = 1.4; grp.add(band);
      var glass = new THREE.Mesh(boxG, mat(0xcfe8ff)); glass.scale.set(1.6, 0.8, 0.05); glass.position.set(0, 2.1, 6.01); grp.add(glass);
      grp.userData = { kind: kind, depth: 12 };
    }
    grp.visible = false; scene.add(grp);
    return grp;
  }
  var pool = { low: [], high: [], train: [] };
  function take(kind) {
    var o = pool[kind].pop() || makeObstacle(kind);
    o.visible = true; obstacles.push(o);
    return o;
  }
  var coinG = new THREE.CylinderGeometry(0.42, 0.42, 0.1, 20), coinM = new THREE.MeshLambertMaterial({ color: 0xffd23f, emissive: 0x553300 });
  var coinPool = [];
  function takeCoin() {
    var c = coinPool.pop();
    if (!c) { c = new THREE.Mesh(coinG, coinM); c.rotation.x = Math.PI / 2; scene.add(c); }
    c.visible = true; coins.push(c);
    return c;
  }

  // ───────── 生成：每隔一段放一排；至少留一条道不是火车 ─────────
  var nextAt = 0;
  function spawnRow(z, speed) {
    var lanes = [-1, 0, 1], kinds = [];
    var n = Math.random() < 0.35 ? 2 : 1;
    lanes.sort(function () { return Math.random() - 0.5; });
    for (var i = 0; i < n; i++) {
      var r = Math.random(), kind = r < 0.38 ? 'low' : r < 0.7 ? 'high' : 'train';
      if (kind === 'train' && kinds.indexOf('train') >= 0) kind = 'low';
      kinds.push(kind);
      var o = take(kind);
      o.position.set(lanes[i] * LANE, 0, z - (kind === 'train' ? 6 : 0));
      o.userData.lane = lanes[i];
    }
    // 金币：一条空道上一串；或者矮栏上方一道弧（跳过去顺手吃）
    var free = lanes[n];
    for (var k = 0; k < 5; k++) {
      var c = takeCoin();
      c.position.set(free * LANE, 1.0, z + 6 + k * 2.2);
      c.userData.lane = free;
    }
    if (kinds[0] === 'low') for (var j = -2; j <= 2; j++) {
      var c2 = takeCoin();
      c2.position.set(lanes[0] * LANE, 1.2 + (2.4 - Math.abs(j) * 0.55), z + j * 1.3);
      c2.userData.lane = lanes[0];
    }
  }

  // ───────── 角色：方块拼的小人，四肢绕肩 / 胯摆 ─────────
  var hero = new THREE.Group(); scene.add(hero);
  var skin = mat(0xffcf99), shirt = mat(0x4361ee), pants = mat(0x22223b), shoe = mat(0xffffff);
  function part(m, sx, sy, sz, x, y, z, parent) { var o = new THREE.Mesh(boxG, m); o.scale.set(sx, sy, sz); o.position.set(x, y, z); (parent || hero).add(o); return o; }
  var torso = part(shirt, 0.62, 0.72, 0.36, 0, 1.25, 0);
  part(skin, 0.42, 0.42, 0.42, 0, 1.86, 0);
  part(pants, 0.44, 0.1, 0.44, 0, 2.12, 0);                                            // 头发
  function limb(x, y, m, len, foot) {
    var piv = new THREE.Group(); piv.position.set(x, y, 0); hero.add(piv);
    part(m, 0.2, len, 0.22, 0, -len / 2, 0, piv);
    if (foot) part(shoe, 0.24, 0.12, 0.34, 0, -len, -0.05, piv);
    return piv;
  }
  var legL = limb(-0.16, 0.9, pants, 0.8, true), legR = limb(0.16, 0.9, pants, 0.8, true);
  var armL = limb(-0.42, 1.55, shirt, 0.62), armR = limb(0.42, 1.55, shirt, 0.62);
  var blob = new THREE.Mesh(new THREE.CircleGeometry(0.55, 20), new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.25, depthWrite: false }));
  blob.rotation.x = -Math.PI / 2; blob.position.y = 0.02; scene.add(blob);

  var roadScroll = 0, camShake = 0;
  return {
    scene: scene, cam: cam,
    reset: function () {
      obstacles.forEach(function (o) { o.visible = false; pool[o.userData.kind].push(o); }); obstacles = [];
      coins.forEach(function (c) { c.visible = false; coinPool.push(c); }); coins = [];
      nextAt = -40; camShake = 0;
      for (var z = -40; z > SPAWN_Z; z -= 22) spawnRow(z, 14);
      nextAt = 0;
    },
    // 推进一帧：返回 true = 撞上了
    step: function (dt, speed, p, onCoin) {
      var dz = speed * dt;
      roadScroll += dz;
      roadTex.offset.y = (roadScroll / 220 * 30) % 1;
      buildings.forEach(function (m) { m.position.z += dz; if (m.position.z > KILL_Z + 6) resetBuilding(m, m.userData.side, m.position.z - 153); });
      if (speed > 0) {
        nextAt -= dz;
        if (nextAt <= 0) { spawnRow(SPAWN_Z, speed); nextAt = 16 + speed * 0.55 + Math.random() * 8; }
      }
      var hit = false;
      obstacles = obstacles.filter(function (o) {
        o.position.z += dz;
        var u = o.userData, cz = o.position.z + (u.kind === 'train' ? 0 : 0);
        if (onCoin && Math.abs(p.x - u.lane * LANE) < 1.05 && Math.abs(cz) < u.depth / 2 + 0.3) {
          if (u.kind === 'train') hit = true;
          else if (u.kind === 'low' && p.y < 0.95) hit = true;
          else if (u.kind === 'high' && p.slide <= 0) hit = true;
        }
        if (o.position.z > KILL_Z + u.depth) { o.visible = false; pool[u.kind].push(o); return false; }
        return true;
      });
      coins = coins.filter(function (c) {
        c.position.z += dz; c.rotation.z += dt * 4;
        if (onCoin && Math.abs(c.position.z) < 0.7 && Math.abs(p.x - c.userData.lane * LANE) < 0.9 && Math.abs(p.y + 1 - c.position.y) < 1.4) {
          c.visible = false; coinPool.push(c); onCoin(); return false;
        }
        if (c.position.z > KILL_Z) { c.visible = false; coinPool.push(c); return false; }
        return true;
      });
      if (hit) camShake = 0.4;
      return hit;
    },
    // 前方 range 格内这条道上最近的障碍（自动驾驶用）
    ahead: function (lane, range) {
      var best = null;
      obstacles.forEach(function (o) {
        var u = o.userData, front = o.position.z + u.depth / 2;
        if (u.lane === lane && front > -range && o.position.z - u.depth / 2 < 0.5 && (!best || front > best.z)) best = { kind: u.kind, z: front };
      });
      return best;
    },
    pose: function (p, state, clock) {
      hero.position.set(p.x, p.y, 0);
      var run = state === 'play' ? p.run : state === 'ready' ? clock * 3 : 0, air = p.y > 0.01;
      var sw = air ? 0.9 : Math.sin(run) * 0.9;
      legL.rotation.x = air ? -0.9 : sw; legR.rotation.x = air ? 0.4 : -sw;
      armL.rotation.x = -sw * 0.9; armR.rotation.x = sw * 0.9;
      hero.rotation.z = (p.lane * LANE - p.x) * -0.12;                                  // 换道时往那边歪一点
      if (p.slide > 0) { hero.rotation.x = -1.1; hero.position.y = p.y + 0.35; } else hero.rotation.x = state === 'over' ? 0.4 : 0;
      torso.position.y = 1.25 + (air || p.slide > 0 ? 0 : Math.abs(Math.cos(run)) * 0.06);
      blob.position.x = p.x; blob.scale.setScalar(Math.max(0.5, 1 - p.y * 0.15));
    },
    camera: function (p, dt, state) {
      camShake = Math.max(0, camShake - dt);
      var sh = camShake * 0.6;
      cam.position.set(p.x * 0.55 + (Math.random() - 0.5) * sh, 4.3 + p.y * 0.25 + (Math.random() - 0.5) * sh, 7.8);
      cam.lookAt(p.x * 0.75, 1.3 + p.y * 0.2, -8);
    },
    count: function () { return obstacles.length + coins.length; }
  };
}

module.exports = { make: make, LANE: LANE };
