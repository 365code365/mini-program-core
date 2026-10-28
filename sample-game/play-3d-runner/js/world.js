// 跑酷的 3D 世界：玩家原地跑（z = 0，面朝 -z），轨道、站台、楼房、路灯、障碍、金币以速度往 +z 流过来，出了镜头就回收重用。
// 道床（碎石 + 枕木）画进一张离屏画布当纹理、站台是平铺材质，卷动都只改纹理偏移；钢轨沿 z 处处一样，不用动。
// 楼房、路灯、障碍、金币都是对象池，不在游戏中途新建网格。模型在 js/models.js，贴图来自 js/art.js。
var Models = require('./models.js');
var LANE = 2.2, SPAWN_Z = -140, KILL_Z = 12, TRACK_W = LANE * 3 + 0.8, ROAD_L = 230, PLAT_W = 4;

function make(THREE, aspect, art) {
  var M = Models.make(THREE, art), man = (art && art.manifest) || {};
  var scene = new THREE.Scene();
  var FOG = new THREE.Color(man.fog || '#cfe3f2'), SKY = new THREE.Color(man.skyTop || '#7fb7ec');
  scene.background = M.has('sky') ? SKY : FOG;
  scene.fog = new THREE.Fog(FOG, 45, 150);
  var cam = new THREE.PerspectiveCamera(62, aspect, 0.1, 240);
  scene.add(new THREE.HemisphereLight(0xe4f1ff, 0x8a7a66, 1.0));
  var sun = new THREE.DirectionalLight(0xfff0d8, 1.05); sun.position.set(-6, 12, 7); scene.add(sun);

  // ───────── 远景：AI 天际线贴在雾外的一块背板上（不吃雾），地平线对着相机高度 ─────────
  if (M.has('sky')) {
    var sky = new THREE.Mesh(new THREE.PlaneGeometry(400, 100), new THREE.MeshBasicMaterial({ map: M.tex('sky'), fog: false, depthWrite: false }));
    sky.position.set(0, 4 + 0.27 * 100, -200); sky.renderOrder = -1; scene.add(sky);
  }

  // ───────── 道床：碎石底 + 每条道一排枕木，一张 512² 画布 = TRACK_W 见方 ─────────
  var tc = wx.createCanvas(); tc.width = 512; tc.height = 512;
  var g = tc.getContext('2d'), PX = 512 / TRACK_W;
  var bal = art && art.img && art.img.ballast, slp = art && art.img && art.img.sleeper;
  if (bal) { for (var by = 0; by < 4; by++) for (var bx = 0; bx < 4; bx++) g.drawImage(bal, bx * 128, by * 128, 128, 128); }
  else { g.fillStyle = '#6b6660'; g.fillRect(0, 0, 512, 512); g.fillStyle = '#58534d'; for (var i = 0; i < 300; i++) g.fillRect(Math.random() * 512, Math.random() * 512, 4, 3); }
  g.fillStyle = 'rgba(0,0,0,0.18)'; g.fillRect(0, 0, 10, 512); g.fillRect(502, 0, 10, 512);
  [-1, 0, 1].forEach(function (l) {
    var cx = 256 + l * LANE * PX, sw = 1.9 * PX, sh = 0.24 * PX;
    for (var k = 0; k < 8; k++) {
      var y = k * 64 + 24;
      g.fillStyle = 'rgba(0,0,0,0.35)'; g.fillRect(cx - sw / 2 + 2, y + 3, sw, sh);           // 枕木的影子
      if (slp) g.drawImage(slp, (k * 61) % 300, (k * 97) % 400, 200, 40, cx - sw / 2, y, sw, sh);
      else { g.fillStyle = '#7a5532'; g.fillRect(cx - sw / 2, y, sw, sh); }
    }
  });
  var roadTex = new THREE.CanvasTexture(tc), ROAD_REP = ROAD_L / TRACK_W;
  roadTex.wrapS = THREE.ClampToEdgeWrapping; roadTex.wrapT = THREE.RepeatWrapping; roadTex.repeat.set(1, ROAD_REP); roadTex.anisotropy = 4;
  if (THREE.SRGBColorSpace) roadTex.colorSpace = THREE.SRGBColorSpace;
  var road = new THREE.Mesh(new THREE.PlaneGeometry(TRACK_W, ROAD_L), new THREE.MeshLambertMaterial({ map: roadTex }));
  road.rotation.x = -Math.PI / 2; road.position.z = -ROAD_L / 2 + 20; scene.add(road);
  // 钢轨：每条道两根，沿 z 一样，所以不卷
  var railG = new THREE.BoxGeometry(0.09, 0.14, ROAD_L), railM = M.mat(0xc9ced6, 'metal', { shiny: 80, spec: 0xaaaaaa });
  [-1, 0, 1].forEach(function (l) { [-0.72, 0.72].forEach(function (o) { var r = new THREE.Mesh(railG, railM); r.position.set(l * LANE + o, 0.07, road.position.z); scene.add(r); }); });
  // 站台：两侧抬高的水泥台，平铺材质卷动
  var platTex = M.tex('concrete', 2, ROAD_L / 4);
  var platM = platTex ? new THREE.MeshLambertMaterial({ map: platTex, color: 0xf0f0f0 }) : new THREE.MeshLambertMaterial({ color: 0xb9b9b4 });
  var edgeM = M.mat(0xffd23f);
  [-1, 1].forEach(function (s) {
    var pl = new THREE.Mesh(new THREE.BoxGeometry(PLAT_W, 0.4, ROAD_L), platM);
    pl.position.set(s * (TRACK_W / 2 + PLAT_W / 2), 0.2, road.position.z); scene.add(pl);
    var ed = new THREE.Mesh(new THREE.BoxGeometry(0.25, 0.02, ROAD_L), edgeM);                  // 站台边的黄线
    ed.position.set(s * (TRACK_W / 2 + 0.3), 0.41, road.position.z); scene.add(ed);
  });
  var ground = new THREE.Mesh(new THREE.PlaneGeometry(300, ROAD_L), M.mat(0x9a9a8e));
  ground.rotation.x = -Math.PI / 2; ground.position.set(0, -0.02, road.position.z); scene.add(ground);

  // ───────── 楼房（每侧一串，挨个排）、路灯 ─────────
  var BX = TRACK_W / 2 + PLAT_W + 0.4, buildings = [], farZ = { '-1': 0, '1': 0 };
  function placeBuilding(m, side) {
    M.shapeBuilding(m);
    var d = m.scale.z, z = farZ[side] - d / 2 - 0.4 - Math.random() * 1.5;
    m.position.set(side * (BX + m.scale.x / 2 + Math.random() * 1.2), m.scale.y / 2, z);
    farZ[side] = z - d / 2;
  }
  [-1, 1].forEach(function (side) {
    farZ[side] = KILL_Z + 4;
    while (farZ[side] > SPAWN_Z - 30) { var m = M.building(); m.userData.side = side; placeBuilding(m, side); scene.add(m); buildings.push(m); }
  });
  var lamps = [], LAMP_GAP = 16;
  for (var li = 0; li < 20; li++) {
    var side = li % 2 ? 1 : -1, lp = M.lamp(side);
    lp.position.set(side * (TRACK_W / 2 + PLAT_W - 0.5), 0.4, KILL_Z - (li >> 1) * LAMP_GAP - (side > 0 ? LAMP_GAP / 2 : 0));
    scene.add(lp); lamps.push(lp);
  }

  // ───────── 障碍与金币的对象池 ─────────
  var obstacles = [], coins = [];
  var DEPTH = { low: 0.4, high: 0.4, train: 12 };
  var pool = { low: [], high: [], train: [] };
  function take(kind) {
    var o = pool[kind].pop();
    if (!o) { o = M[kind](); o.userData = { kind: kind, depth: DEPTH[kind] }; scene.add(o); }
    o.visible = true; obstacles.push(o);
    return o;
  }
  var coinPool = [];
  function takeCoin() {
    var c = coinPool.pop();
    if (!c) { c = M.coin(); scene.add(c); }
    c.visible = true; coins.push(c);
    return c;
  }
  // 开局把常用的先建好（中途第一次 take 会建网格、编着色器，那一帧会顿）
  ['low', 'high', 'train'].forEach(function (k) { for (var n = 0; n < 3; n++) { var o = take(k); o.visible = false; } });
  obstacles.forEach(function (o) { pool[o.userData.kind].push(o); }); obstacles = [];
  for (var cn = 0; cn < 30; cn++) takeCoin().visible = false;
  coins.forEach(function (c) { coinPool.push(c); }); coins = [];

  // ───────── 生成：每隔一段放一排；至少留一条道不是火车 ─────────
  var nextAt = 0;
  function spawnRow(z) {
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

  var H = M.hero(); scene.add(H.root);
  var blob = new THREE.Mesh(new THREE.CircleGeometry(0.5, 20), new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.3, depthWrite: false }));
  blob.rotation.x = -Math.PI / 2; blob.position.y = 0.03; scene.add(blob);
  var Pose = require('./pose.js'), pose = Pose.make(H);

  var roadScroll = 0, camShake = 0, cx = 0, cvx = 0, cy = 0;
  function scroll(dz) {
    roadScroll += dz;
    roadTex.offset.y = (roadScroll / TRACK_W) % 1;
    if (platTex) platTex.offset.y = (roadScroll / 4) % 1;
    farZ['-1'] += dz; farZ['1'] += dz;                 // 先挪「最远处」：回收的楼接在挪过之后的末尾
    for (var i = 0; i < buildings.length; i++) {
      var m = buildings[i]; m.position.z += dz;
      if (m.position.z - m.scale.z / 2 > KILL_Z) placeBuilding(m, m.userData.side);
    }
    for (var j = 0; j < lamps.length; j++) { lamps[j].position.z += dz; if (lamps[j].position.z > KILL_Z) lamps[j].position.z -= LAMP_GAP * 10; }
  }
  return {
    scene: scene, cam: cam,
    reset: function () {
      obstacles.forEach(function (o) { o.visible = false; pool[o.userData.kind].push(o); }); obstacles = [];
      coins.forEach(function (c) { c.visible = false; coinPool.push(c); }); coins = [];
      camShake = 0; pose.reset();
      for (var z = -40; z > SPAWN_Z; z -= 22) spawnRow(z);
      nextAt = 0;
    },
    // 推进一帧：返回 true = 撞上了
    step: function (dt, speed, p, onCoin) {
      var dz = speed * dt;
      scroll(dz);
      if (speed > 0) {
        nextAt -= dz;
        if (nextAt <= 0) { spawnRow(SPAWN_Z); nextAt = 16 + speed * 0.55 + Math.random() * 8; }
      }
      var hit = false;
      obstacles = obstacles.filter(function (o) {
        o.position.z += dz;
        var u = o.userData, cz = o.position.z;
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
    pose: function (p, state, clock, dt) {
      pose.apply(p, state, clock, dt);
      blob.position.x = p.x; blob.scale.setScalar(Math.max(0.45, 1 - p.y * 0.18)); blob.material.opacity = 0.3 * Math.max(0.35, 1 - p.y * 0.2);
    },
    // 相机：横向用比角色软的弹簧跟（换道时画面晚半拍、带一点侧倾），速度越快视角越广
    camera: function (p, dt, state, speed) {
      camShake = Math.max(0, camShake - dt);
      var w = 7, n = dt > 1 / 90 ? 2 : 1, h = dt / n;
      for (var i = 0; i < n; i++) { cvx += (w * w * (p.x * 0.6 - cx) - 2 * w * cvx) * h; cx += cvx * h; }
      cy += (p.y * 0.3 - cy) * (1 - Math.exp(-dt * 6));
      var sh = camShake * 0.6, fov = 60 + Math.min(1, Math.max(0, (speed - 14) / 20)) * 10;
      if (Math.abs(cam.fov - fov) > 0.01) { cam.fov = fov; cam.updateProjectionMatrix(); }
      cam.position.set(cx + (Math.random() - 0.5) * sh, 4.3 + cy + (Math.random() - 0.5) * sh, 7.8);
      cam.lookAt(cx * 1.2, 1.3 + cy * 0.6, -8);
      cam.rotateZ(-cvx * 0.012);
    },
    count: function () { return obstacles.length + coins.length; }
  };
}

module.exports = { make: make, LANE: LANE };
