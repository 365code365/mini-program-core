// 岛：MAP×MAP 的方块地格（高度分沙滩 / 草地 / 小丘三档，带棋盘色差），一个合并网格；水面是自定义着色器（起伏 + 岸边白沫 + 深浅渐变）；
// 天空颜色、太阳角度、雾随一天的时间走（一天 8 分钟）；树石花等装饰也合并成一个网格，地格上建了楼就重建装饰网格。
var D = require('../data/defs.js');

function seeded(seed) { return function () { seed = (seed + 0x6D2B79F5) >>> 0; var t = seed; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }

var WATER_VS = [
  'uniform float uTime; varying vec2 vXZ; varying float vWave;',
  'void main() {',
  '  vec3 p = position;',
  '  float w = sin(p.x * 0.9 + uTime * 1.3) * 0.5 + sin(p.z * 1.1 - uTime * 1.1) * 0.5 + sin((p.x + p.z) * 0.6 + uTime * 0.7) * 0.3;',
  '  p.y += w * 0.035; vWave = w; vXZ = p.xz;',
  '  gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.0);',
  '}'].join('\n');
// 不写 precision：three.js 给 ShaderMaterial 两边都补同一个 precision；自己写成 mediump 的话 uTime 两边精度不一样，链接校验失败
var WATER_FS = [
  'uniform float uTime; uniform vec3 uDeep; uniform vec3 uShallow; uniform vec3 uFoam; uniform float uR; uniform float uLight;',
  'varying vec2 vXZ; varying float vWave;',
  'void main() {',
  '  float d = length(vXZ);',
  '  float shore = smoothstep(uR + 2.2, uR - 0.2, d);',                       // 离岛越近越浅
  '  vec3 c = mix(uDeep, uShallow, shore);',
  '  float ring = sin(d * 6.0 - uTime * 1.6) * 0.5 + 0.5;',                    // 一圈圈往外推的浪线
  '  float foam = smoothstep(0.85, 1.0, ring) * smoothstep(uR + 1.4, uR - 0.1, d) * 0.55;',
  // 波光：两组方向不同、频率不成整数比的波叠起来，只取最亮的尖（单组 sin×sin 会排成整齐的点阵，很假）
  '  float sp = sin(vXZ.x * 3.7 + vXZ.y * 1.3 + uTime * 1.7) * sin(vXZ.y * 4.3 - vXZ.x * 2.1 - uTime * 1.3) * sin((vXZ.x + vXZ.y) * 2.9 + uTime);',
  '  float sparkle = smoothstep(0.82, 1.0, sp) * 0.25 * shore;',
  '  c = mix(c, uFoam, clamp(foam + sparkle, 0.0, 1.0));',
  '  c *= 0.85 + vWave * 0.05;',
  '  gl_FragColor = vec4(c * uLight, 1.0);',
  '}'].join('\n');

function make(THREE, M, state) {
  var N = D.MAP, H = N / 2, rnd = seeded(20261028);
  var group = new THREE.Group();

  // ───────── 地格：高度 = 径向衰减 + 噪声 ─────────
  var tiles = [];          // tiles[z][x] = { x, z, h, kind: 'water'|'sand'|'grass'|'hill', cx, cz }
  var R = N / 2 - 1.3;
  for (var z = 0; z < N; z++) {
    tiles.push([]);
    for (var x = 0; x < N; x++) {
      var cx = x - H + 0.5, cz = z - H + 0.5;
      var d = Math.sqrt(cx * cx + cz * cz) / R;
      var n = Math.sin(x * 1.3) * Math.cos(z * 1.1) * 0.12 + Math.sin(x * 0.4 + z * 0.7) * 0.15 + rnd() * 0.08;
      var v = 1 - d + n;
      var kind = v < 0.05 ? 'water' : v < 0.2 ? 'sand' : v > 0.82 && rnd() < 0.5 ? 'hill' : 'grass';
      tiles[z].push({ x: x, z: z, cx: cx, cz: cz, kind: kind, h: kind === 'water' ? 0 : kind === 'sand' ? 0.16 : kind === 'grass' ? 0.3 : 0.44 });
    }
  }
  function tile(x, z) { return tiles[z] && tiles[z][x]; }
  function key(x, z) { return x + ',' + z; }
  // 能不能用：起始圈内 + 扩出来的
  function unlocked(x, z) {
    var t = tile(x, z); if (!t || t.kind === 'water') return false;
    return Math.sqrt(t.cx * t.cx + t.cz * t.cz) <= D.START_R || state.expanded.indexOf(key(x, z)) >= 0;
  }
  function coastal(x, z) {
    var w = false;
    [[1, 0], [-1, 0], [0, 1], [0, -1]].forEach(function (o) { var t = tile(x + o[0], z + o[1]); if (!t || t.kind === 'water') w = true; });
    return w;
  }
  // 可扩：没解锁的陆地、且挨着已解锁的
  function expandable(x, z) {
    var t = tile(x, z); if (!t || t.kind === 'water' || unlocked(x, z)) return false;
    return [[1, 0], [-1, 0], [0, 1], [0, -1]].some(function (o) { return unlocked(x + o[0], z + o[1]); });
  }

  // ───────── 地面网格（锁着的地格压暗、去饱和） ─────────
  var ground = null;
  var PAL = { sand: [0xf2dfa7, 0xeed49a], grass: [0x8fd16a, 0x84c75f], hill: [0x6fbf5a, 0x66b552] };
  var SIDE = { sand: 0xd9bf7f, grass: 0x8a6a46, hill: 0x7a5c3c };
  function buildGround() {
    if (ground) { group.remove(ground); ground.geometry.dispose(); }
    var b = new M.Builder(), c = new THREE.Color(), gray = new THREE.Color(0x8a9a8a);
    for (var z = 0; z < N; z++) for (var x = 0; x < N; x++) {
      var t = tiles[z][x];
      if (t.kind === 'water') continue;
      // 有 AI 材质：顶面用草 / 沙 / 高草贴图，棋盘格只差一点亮度；侧面统一崖壁贴图。锁着的压暗去饱和
      c.set(M.textured ? ((x + z) % 2 ? 0xffffff : 0xe8f0e0) : PAL[t.kind][(x + z) % 2]);
      if (!unlocked(x, z)) c.lerp(gray, 0.45).multiplyScalar(0.78);
      b.add(M.G.box, c.getHex(), [t.cx, t.h - 0.02, t.cz], null, [1, 0.04, 1], { sand: 'sand', grass: 'grass', hill: 'hillgrass' }[t.kind]);
      var side = new THREE.Color(M.textured ? 0xffffff : SIDE[t.kind]); if (!unlocked(x, z)) side.lerp(gray, 0.4);
      b.add(M.G.box, side.getHex(), [t.cx, (t.h - 0.04) / 2 - 0.3, t.cz], null, [1, t.h - 0.04 + 0.6, 1], t.kind === 'sand' ? 'sand' : 'cliff');
    }
    ground = new THREE.Mesh(b.geometry(), M.MAT);
    ground.receiveShadow = true;
    ground.matrixAutoUpdate = false;
    group.add(ground);
  }

  // ───────── 装饰：每块空地格随机放树 / 石头 / 花（种子固定，存档换了机器也一样） ─────────
  var decorKinds = {};
  var dr = seeded(77);
  for (var z2 = 0; z2 < N; z2++) for (var x2 = 0; x2 < N; x2++) {
    var tt = tiles[z2][x2], r = dr();
    if (tt.kind === 'water') continue;
    if (tt.kind === 'sand') decorKinds[key(x2, z2)] = r < 0.25 ? { k: 'palm', s: 0.8 + dr() * 0.4 } : r < 0.35 ? { k: 'rock', s: 0.6 + dr() * 0.6 } : null;
    else decorKinds[key(x2, z2)] = r < 0.35 ? { k: 'tree', t: Math.floor(dr() * 2), s: 0.7 + dr() * 0.5, n: 1 + Math.floor(dr() * 2) } : r < 0.5 ? { k: 'flowers', c: Math.floor(dr() * 4) } : r < 0.58 ? { k: 'rock', s: 0.6 + dr() * 0.5 } : null;
  }
  var decor = null;
  function buildDecor(occupied) {
    if (decor) { group.remove(decor); decor.geometry.dispose(); }
    var b = new M.Builder(), j = seeded(5);
    for (var z = 0; z < N; z++) for (var x = 0; x < N; x++) {
      var dk = decorKinds[key(x, z)], t = tiles[z][x];
      if (!dk || occupied[key(x, z)]) continue;
      var ox = (j() - 0.5) * 0.4, oz = (j() - 0.5) * 0.4;
      if (dk.k === 'palm') M.tree(b, t.cx + ox, t.h, t.cz + oz, 2, dk.s);
      else if (dk.k === 'tree') for (var i = 0; i < dk.n; i++) M.tree(b, t.cx + ox + i * 0.25 - 0.12, t.h, t.cz + oz - i * 0.2, dk.t, dk.s * (1 - i * 0.2));
      else if (dk.k === 'rock') M.rock(b, t.cx + ox, t.h, t.cz + oz, dk.s);
      else if (dk.k === 'flowers') M.flowers(b, t.cx + ox, t.h, t.cz + oz, dk.c);
    }
    decor = new THREE.Mesh(b.geometry(), M.MAT);
    decor.castShadow = true; decor.matrixAutoUpdate = false;
    group.add(decor);
  }

  // ───────── 水 ─────────
  var waterU = { uTime: { value: 0 }, uDeep: { value: new THREE.Color(0x1d6fa5) }, uShallow: { value: new THREE.Color(0x4fd1d9) },
    uFoam: { value: new THREE.Color(0xffffff) }, uR: { value: R }, uLight: { value: 1 } };
  var water = new THREE.Mesh(new THREE.PlaneGeometry(60, 60, 60, 60), new THREE.ShaderMaterial({ uniforms: waterU, vertexShader: WATER_VS, fragmentShader: WATER_FS }));
  water.geometry.rotateX(-Math.PI / 2);
  water.position.y = 0.02;
  group.add(water);

  // ───────── 天：一天 8 分钟，太阳绕岛转；颜色在几组关键色之间插 ─────────
  var DAY_LEN = 480;
  var KEYS = [   // t（0 = 午夜）、天色、雾、阳光、环境光强
    [0.0, 0x10183a, 0.25, 0.15], [0.22, 0x2b3d7a, 0.35, 0.3], [0.28, 0xffb38a, 0.7, 0.75], [0.35, 0x8fd3ff, 1.0, 1.0],
    [0.65, 0x8fd3ff, 1.0, 1.0], [0.74, 0xff9a76, 0.75, 0.75], [0.8, 0x3b3f8a, 0.4, 0.35], [1.0, 0x10183a, 0.25, 0.15]
  ];
  var sky = new THREE.Color(), tmpA = new THREE.Color(), tmpB = new THREE.Color();
  function dayState(t) {
    var i = 0; while (i < KEYS.length - 2 && KEYS[i + 1][0] <= t) i++;
    var a = KEYS[i], b = KEYS[i + 1], k = (t - a[0]) / (b[0] - a[0]);
    sky.copy(tmpA.set(a[1])).lerp(tmpB.set(b[1]), k);
    return { sky: sky, sun: a[2] + (b[2] - a[2]) * k, amb: a[3] + (b[3] - a[3]) * k, night: t < 0.25 || t > 0.78 };
  }

  return {
    group: group, tiles: tiles, tile: tile, key: key, unlocked: unlocked, coastal: coastal, expandable: expandable, N: N, H: H,
    rebuild: function (occupied) { buildGround(); buildDecor(occupied || {}); },
    rebuildDecor: buildDecor,
    tileAt: function (wx, wz) { var x = Math.floor(wx + H), z = Math.floor(wz + H); return tile(x, z) || null; },
    // 一帧：水波时间、昼夜；返回 dayState（灯光由场景按它设）
    step: function (time) {
      waterU.uTime.value = time;
      // 从上午开始（每次进游戏都是白天），按游戏时间走；不跟墙上时钟，免得一打开就是黑夜
      var t = (0.38 + time / DAY_LEN) % 1, ds = dayState(t);
      waterU.uLight.value = 0.45 + ds.amb * 0.55;
      ds.t = t;
      return ds;
    }
  };
}

module.exports = { make: make, seeded: seeded };
