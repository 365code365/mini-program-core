// 模型工厂的底座：合并器（一栋楼 = 一次 draw call）、材质图集、常用几何（含倒角盒子）、自然物（树石花云）。
// 建筑在 buildings.js（零件在 parts.js），村民与宠物在 characters.js / pets3d.js，这里把它们拼成一个对外的 M。
//
// 写实化的几个手法（v4）：
// - 倒角盒子 G.bevel(w,h,d,r)：方块的棱切一刀，光照下棱上有一条亮边，近看不再是「纸盒子」；
// - 平滑着色（flatShading 关掉）：圆柱、球、锥是圆的；需要硬边的（盒子、倒角、石头）几何本身就是面法线；
// - 接地暗角：b.ao = true 时按高度压暗贴地的顶点（离地 0.02~0.16 从 0.78 渐到 1），等于烘了一层接触阴影
//   （压得太深、范围太大时麦田、矮栅栏整片发黑，v4 第一版就是这样）；
// - 颜色抖动：同一种零件每个都带一点点亮度差（木板、瓦片、石块），避免「一个颜色刷到底」的塑料感。
var Parts = require('./parts.js');
var Buildings = require('./buildings.js');
var Chars = require('./characters.js');

var C = {
  stone: 0xd8d2c4, stoneDark: 0xb8b0a0, wood: 0x9c6b43, woodDark: 0x6e4527, roofRed: 0xc9503e, roofBlue: 0x4a7fd4, roofGreen: 0x3f8f5b,
  wall: 0xfff3dc, wallPink: 0xffe1e8, glass: 0x5b8fb8, gold: 0xffc93c, leaf: 0x6cc551, leafDark: 0x4ea43c, trunk: 0x7d5230, wheat: 0xf2c14e,
  water: 0x3fa9d8, white: 0xffffff, black: 0x222222, rock: 0xa7adb4, hay: 0xe9c46a, flower: [0xff6b9a, 0xffd23f, 0xb388ff, 0xffffff]
};

function make(THREE, art) {
  var TEX = (art && art.manifest && art.manifest.textures) || { white: 15 }, IN = 0.02;
  var m4 = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler(), v = new THREE.Vector3(), s = new THREE.Vector3();
  var nm = new THREE.Matrix3(), col = new THREE.Color(), tmp = new THREE.Vector3();
  var seed = 1;
  function jrnd() { seed = (seed * 16807) % 2147483647; return seed / 2147483647; }

  // ───────── 合并器 ─────────
  // add(几何, 颜色, 位置, 旋转, 缩放, 材质格, 骨骼)：rig = [肢体号, 轴心 y, 轴心 z, 换色槽]（characters.js 的着色器按它摆动四肢、换衣服颜色）
  function Builder() { this.p = []; this.n = []; this.c = []; this.u = []; this.r = null; this.ao = false; this.jit = 0; this.td = 0; }
  Builder.prototype.add = function (geo, color, pos, rot, scl, tex, rig) {
    // 非索引版本挂在原几何上复用：一座房子要 add 一百多次，每次 toNonIndexed 都新分配一份（真机上的大头之一）
    var g = geo.index ? (geo.userData.ni || (geo.userData.ni = geo.toNonIndexed())) : geo;
    e.set(rot ? rot[0] : 0, rot ? rot[1] : 0, rot ? rot[2] : 0);
    m4.compose(v.set(pos ? pos[0] : 0, pos ? pos[1] : 0, pos ? pos[2] : 0), q.setFromEuler(e), s.set(scl ? scl[0] : 1, scl ? scl[1] : 1, scl ? scl[2] : 1));
    nm.getNormalMatrix(m4);
    col.set(color);
    if (this.jit) { var j = 1 + (jrnd() - 0.5) * this.jit; col.r = Math.min(1, col.r * j); col.g = Math.min(1, col.g * j); col.b = Math.min(1, col.b * j); }
    var P = g.attributes.position, N = g.attributes.normal, UV = g.attributes.uv;
    var cell = TEX[tex || 'white']; if (cell === undefined) cell = TEX.white === undefined ? 15 : TEX.white;
    var cu = cell % 4, cv = Math.floor(cell / 4);
    if (rig && !this.r) { this.r = []; for (var z = 0; z < this.p.length / 3; z++) this.r.push(0, 0, 0, 0); }
    // 贴图密度：一格贴图在世界里铺 1/td 那么大。小零件（门、窗框、木箱）只取格子里的一小块，纹理颗粒和大墙面一样粗细，
    // 否则一扇 0.14 宽的门上挤着整张木板图（细得像噪点），一面 0.8 的墙又被拉成大色块 —— 「贴图贴上去的」感觉就是这么来的。
    var win = 1, ou = 0, ov = 0;
    if (this.td && tex) {
      if (!geo.boundingBox) geo.computeBoundingBox();
      var bb = geo.boundingBox, ex = Math.max((bb.max.x - bb.min.x) * Math.abs(s.x), (bb.max.y - bb.min.y) * Math.abs(s.y), (bb.max.z - bb.min.z) * Math.abs(s.z));
      win = Math.min(1, ex * this.td); ou = jrnd() * (1 - win); ov = jrnd() * (1 - win);
    }
    // 热循环直接读 .array、矩阵展开成局部量：getX / applyMatrix4 / normalize 每个顶点十来次方法调用，
    // 真机 JS 引擎上一座房子（2 万顶点）光这些调用就 100ms 上下。运算顺序与 three 的实现一致，结果逐位相同。
    var pa = P.array, na = N.array, ua = UV ? UV.array : null, me = m4.elements, ne = nm.elements;
    var pp = this.p, nn = this.n, cc = this.c, uo = this.u, rr = this.r, ao = this.ao, cr = col.r, cg = col.g, cb = col.b;
    var r0 = rig ? rig[0] : 0, r1 = rig ? rig[1] || 0 : 0, r2 = rig ? rig[2] || 0 : 0, r3 = rig ? rig[3] || 0 : 0;
    for (var i = 0, i3 = 0; i < P.count; i++, i3 += 3) {
      var uu = ua ? ua[i * 2] : 0.5, vv = ua ? ua[i * 2 + 1] : 0.5;
      uu = Math.max(0, Math.min(1, uu)); vv = Math.max(0, Math.min(1, vv));   // 防越界：超出 [0,1] 会取到图集隔壁格
      uu = ou + uu * win; vv = ov + vv * win;
      uo.push((cu + IN + uu * (1 - 2 * IN)) / 4, 1 - (cv + IN + (1 - vv) * (1 - 2 * IN)) / 4);
      var x = pa[i3], y = pa[i3 + 1], z = pa[i3 + 2];
      var w = 1 / (me[3] * x + me[7] * y + me[11] * z + me[15]);
      var px = (me[0] * x + me[4] * y + me[8] * z + me[12]) * w, py = (me[1] * x + me[5] * y + me[9] * z + me[13]) * w, pz = (me[2] * x + me[6] * y + me[10] * z + me[14]) * w;
      pp.push(px, py, pz);
      var k = 1;
      if (ao) { var hy = Math.max(0, Math.min(1, (py - 0.02) / 0.14)); k = 0.78 + 0.22 * hy * (2 - hy); }
      x = na[i3]; y = na[i3 + 1]; z = na[i3 + 2];
      var nx = ne[0] * x + ne[3] * y + ne[6] * z, ny = ne[1] * x + ne[4] * y + ne[7] * z, nz = ne[2] * x + ne[5] * y + ne[8] * z;
      var len = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1;
      nn.push(nx * (1 / len), ny * (1 / len), nz * (1 / len));
      cc.push(cr * k, cg * k, cb * k);
      if (rr) rr.push(r0, r1, r2, r3);
    }
    return this;
  };
  Builder.prototype.geometry = function () {
    var g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(this.p, 3));
    g.setAttribute('normal', new THREE.Float32BufferAttribute(this.n, 3));
    g.setAttribute('color', new THREE.Float32BufferAttribute(this.c, 3));
    g.setAttribute('uv', new THREE.Float32BufferAttribute(this.u, 2));
    if (this.r) g.setAttribute('aRig', new THREE.Float32BufferAttribute(this.r, 4));
    // 包围盒 / 包围球直接扫数组（与 three 的 computeBoundingSphere 同算法：盒心 + 最远点）：
    // three 那版每个顶点走 fromBufferAttribute，真机上一座房子要 100ms 以上，比拼几何本身还慢
    var a = g.attributes.position.array, n = a.length, x0 = Infinity, y0 = Infinity, z0 = Infinity, x1 = -Infinity, y1 = -Infinity, z1 = -Infinity, i;
    for (i = 0; i < n; i += 3) {
      var x = a[i], y = a[i + 1], z = a[i + 2];
      if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; if (z < z0) z0 = z; if (z > z1) z1 = z;
    }
    g.boundingBox = new THREE.Box3(new THREE.Vector3(x0, y0, z0), new THREE.Vector3(x1, y1, z1));
    var cx = (x0 + x1) / 2, cy = (y0 + y1) / 2, cz = (z0 + z1) / 2, r2 = 0;
    for (i = 0; i < n; i += 3) { var dx = cx - a[i], dy = cy - a[i + 1], dz = cz - a[i + 2], d = dx * dx + dy * dy + dz * dz; if (d > r2) r2 = d; }
    g.boundingSphere = new THREE.Sphere(new THREE.Vector3(cx, cy, cz), Math.sqrt(r2));
    return g;
  };
  var MAT = new THREE.MeshLambertMaterial({ vertexColors: true });
  if (art && art.textures) {
    var atlas = new THREE.Texture(art.textures);
    atlas.needsUpdate = true; atlas.anisotropy = 4;
    atlas.minFilter = THREE.LinearMipmapLinearFilter; atlas.magFilter = THREE.LinearFilter;
    if (THREE.SRGBColorSpace) atlas.colorSpace = THREE.SRGBColorSpace;
    MAT.map = atlas;
  }
  var textured = !!MAT.map;
  function tint(c, k) { if (!textured) return c; var x = new THREE.Color(c); return x.lerp(new THREE.Color(0xffffff), k === undefined ? 0.45 : k).getHex(); }
  function mesh(b) { return new THREE.Mesh(b.geometry(), MAT); }

  // 硬边几何：转成非索引再按面算法线（平滑着色下也是一个面一个法线）
  function flat(g) { if (!g.index) { g.computeVertexNormals(); return g; } var n = g.toNonIndexed(); n.computeVertexNormals(); g.dispose(); return n; }
  // 倒角盒子：6 个面 + 12 条 45° 棱面 + 8 个角三角 = 44 个三角（普通盒子 12 个）。
  // 顶点 V(符号, k)：第 k 轴取满 ±half，另两轴缩进 r。UV 按三角形法线的主轴做盒式投影（每面 [0,1]）。
  var bevelCache = {};
  function bevel(w, h, d, r) {
    r = Math.min(r === undefined ? 0.015 : r, w / 2.2, h / 2.2, d / 2.2);
    var key = [w, h, d, r].map(function (x) { return x.toFixed(3); }).join(',');
    if (bevelCache[key]) return bevelCache[key];
    var half = [w / 2, h / 2, d / 2], pos = [], uvs = [];
    function V(s, k) { var p = []; for (var a = 0; a < 3; a++) p[a] = s[a] * (a === k ? half[a] : half[a] - r); return p; }
    function tri(A, B, Cc) {
      var ux = B[0] - A[0], uy = B[1] - A[1], uz = B[2] - A[2], vx = Cc[0] - A[0], vy = Cc[1] - A[1], vz = Cc[2] - A[2];
      var nx = uy * vz - uz * vy, ny = uz * vx - ux * vz, nz = ux * vy - uy * vx;
      if (nx * (A[0] + B[0] + Cc[0]) + ny * (A[1] + B[1] + Cc[1]) + nz * (A[2] + B[2] + Cc[2]) < 0) { var t = B; B = Cc; Cc = t; }
      var ax = Math.abs(nx), ay = Math.abs(ny), az = Math.abs(nz), iu = ax >= ay && ax >= az ? 2 : 0, iv = ay >= ax && ay >= az ? 2 : 1;
      [A, B, Cc].forEach(function (p) { pos.push(p[0], p[1], p[2]); uvs.push(p[iu] / (2 * half[iu]) + 0.5, p[iv] / (2 * half[iv]) + 0.5); });
    }
    function quad(A, B, Cc, D) { tri(A, B, Cc); tri(A, Cc, D); }
    var S = [-1, 1];
    for (var k = 0; k < 3; k++) S.forEach(function (sk) {               // 面
      var o1 = (k + 1) % 3, o2 = (k + 2) % 3, s = [0, 0, 0]; s[k] = sk;
      var c = [[-1, -1], [1, -1], [1, 1], [-1, 1]].map(function (q) { var t = s.slice(); t[o1] = q[0]; t[o2] = q[1]; return V(t, k); });
      quad(c[0], c[1], c[2], c[3]);
    });
    for (var m = 0; m < 3; m++) S.forEach(function (s1) { S.forEach(function (s2) {   // 棱：沿 m 轴
      var k1 = (m + 1) % 3, k2 = (m + 2) % 3, lo = [0, 0, 0], hi = [0, 0, 0];
      lo[k1] = hi[k1] = s1; lo[k2] = hi[k2] = s2; lo[m] = -1; hi[m] = 1;
      quad(V(lo, k1), V(hi, k1), V(hi, k2), V(lo, k2));
    }); });
    S.forEach(function (a) { S.forEach(function (b2) { S.forEach(function (c2) { var s = [a, b2, c2]; tri(V(s, 0), V(s, 1), V(s, 2)); }); }); });   // 角
    var g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
    g.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
    g.computeVertexNormals();
    return (bevelCache[key] = g);
  }

  var G = {
    box: new THREE.BoxGeometry(1, 1, 1),
    cyl: new THREE.CylinderGeometry(0.5, 0.5, 1, 12),
    cyl8: new THREE.CylinderGeometry(0.5, 0.5, 1, 8),
    cyl6: new THREE.CylinderGeometry(0.5, 0.5, 1, 6),
    taper: new THREE.CylinderGeometry(0.38, 0.5, 1, 14),        // 上细下粗：灯塔、风车塔、树干
    taper8: flat(new THREE.CylinderGeometry(0.4, 0.5, 1, 8)),   // 八角塔（硬边）
    cone4: flat(new THREE.ConeGeometry(0.72, 1, 4)),
    cone: new THREE.ConeGeometry(0.5, 1, 14),
    cone6: new THREE.ConeGeometry(0.5, 1, 6),
    cone8f: flat(new THREE.ConeGeometry(0.5, 1, 8)),
    ico: flat(new THREE.IcosahedronGeometry(0.5, 0)),           // 石头：硬边
    rock: flat(new THREE.DodecahedronGeometry(0.5, 0)),
    ico1: new THREE.IcosahedronGeometry(0.5, 1),                 // 树冠、云：平滑的团
    blob: new THREE.IcosahedronGeometry(0.5, 2),
    // 三角形预算：一栋楼 ≤ 4k、一只宠物 ≤ 2.5k、一个村民 ≤ 2k（全岛满档 ~10 万）。球是大头：16×12 的球一颗 350 个三角，
    // 麦穗、货物、花这种指甲盖大的东西用 sphMi（6×4，40 个），身体、头用 sph（12×8）
    sph: new THREE.SphereGeometry(0.5, 12, 8),
    sphLo: new THREE.SphereGeometry(0.5, 8, 5),
    sphMi: new THREE.SphereGeometry(0.5, 6, 4),
    hemi: new THREE.SphereGeometry(0.5, 16, 8, 0, Math.PI * 2, 0, Math.PI / 2),
    torus: new THREE.TorusGeometry(0.4, 0.1, 6, 16),
    halfTorus: new THREE.TorusGeometry(0.4, 0.12, 6, 12, Math.PI),
    plane: new THREE.PlaneGeometry(1, 1),
    prism: (function () { var g = new THREE.CylinderGeometry(0.5, 0.5, 1, 3); g.rotateZ(Math.PI / 2); return flat(g); })(),
    bevel: bevel
  };

  var M = { Builder: Builder, G: G, C: C, MAT: MAT, mesh: mesh, tint: tint, textured: textured, THREE: THREE, jrnd: jrnd,
    reseed: function (x) { seed = 1 + (Math.abs(Math.floor(x * 9973)) % 2147483646); } };

  // ───────── 自然：树（针叶 / 阔叶 / 椰子）、石头、花丛、云 ─────────
  M.tree = function (b, x, y, z, kind, sc) {
    sc = sc || 1;
    var lean = ((x * 13.7 + z * 7.1) % 1) * 0.12 - 0.06;
    if (kind === 0) {
      b.add(G.taper, tint(C.trunk), [x, y + 0.1 * sc, z], null, [0.06 * sc, 0.2 * sc, 0.06 * sc], 'bark');
      for (var t = 0; t < 3; t++) b.add(G.cone8f, tint(t % 2 ? C.leaf : C.leafDark, 0.35), [x, y + (0.26 + t * 0.13) * sc, z], [0, t * 0.4, lean], [(0.4 - t * 0.09) * sc, (0.26 - t * 0.03) * sc, (0.4 - t * 0.09) * sc], 'leaves');
    } else if (kind === 1) {
      b.add(G.taper, tint(C.trunk), [x, y + 0.14 * sc, z], [0, 0, lean], [0.07 * sc, 0.28 * sc, 0.07 * sc], 'bark');
      b.add(G.ico1, tint(C.leafDark, 0.4), [x, y + 0.36 * sc, z], [x, z, 0], [0.4 * sc, 0.3 * sc, 0.38 * sc], 'leaves');
      b.add(G.ico1, tint(C.leaf, 0.4), [x + 0.06 * sc, y + 0.46 * sc, z - 0.04 * sc], [z, x, 0], [0.3 * sc, 0.26 * sc, 0.3 * sc], 'leaves');
      b.add(G.ico1, tint(0x7fd35e, 0.45), [x - 0.08 * sc, y + 0.42 * sc, z + 0.06 * sc], null, [0.22 * sc, 0.2 * sc, 0.22 * sc], 'leaves');
    } else {
      // 椰子树：弯的树干（三节）+ 下垂的叶子 + 椰子
      var px = x, py = y, bend = 0.04 * sc;
      for (var s2 = 0; s2 < 3; s2++) { b.add(G.cyl8, 0xb08968, [px, py + 0.1 * sc, z], [0, 0, -0.12 - s2 * 0.08], [0.05 * sc, 0.21 * sc, 0.05 * sc], 'bark'); px += bend; py += 0.19 * sc; }
      for (var k = 0; k < 6; k++) {
        var a = k * Math.PI / 3;
        b.add(G.bevel(0.3, 0.012, 0.09, 0.004), 0x4caf6a, [px + Math.cos(a) * 0.13 * sc, py + 0.02 * sc, z + Math.sin(a) * 0.13 * sc], [0, -a, -0.45], [sc, sc, sc], 'leaves');
      }
      for (var cc = 0; cc < 3; cc++) b.add(G.sphLo, 0x6b4a2b, [px + Math.cos(cc * 2.1) * 0.03, py - 0.02 * sc, z + Math.sin(cc * 2.1) * 0.03], null, [0.04 * sc, 0.04 * sc, 0.04 * sc]);
    }
  };
  M.rock = function (b, x, y, z, sc) {
    b.add(G.rock, tint(C.rock, 0.7), [x, y + 0.05 * sc, z], [x, z, 0], [0.2 * sc, 0.13 * sc, 0.18 * sc], 'rock');
    b.add(G.rock, tint(0x959ca4, 0.6), [x + 0.09 * sc, y + 0.03 * sc, z + 0.05 * sc], [z, x, 1], [0.09 * sc, 0.07 * sc, 0.08 * sc], 'rock');
  };
  M.flowers = function (b, x, y, z, k) {
    b.add(G.ico1, tint(C.leafDark, 0.4), [x, y + 0.03, z], null, [0.18, 0.07, 0.14], 'leaves');
    for (var i = 0; i < 5; i++) b.add(G.sphMi, C.flower[(k + i) % 4], [x + Math.cos(i * 1.3) * 0.06, y + 0.07, z + Math.sin(i * 1.3) * 0.045], null, [0.035, 0.03, 0.035]);
  };
  M.cloud = function (rnd) {
    var b = new Builder(), n = 3 + Math.floor(rnd() * 3);
    for (var i = 0; i < n; i++) b.add(G.blob, C.white, [i * 0.45 - n * 0.2, rnd() * 0.15, rnd() * 0.3], null, [0.6 + rnd() * 0.3, 0.42 + rnd() * 0.2, 0.5]);
    return new THREE.Mesh(b.geometry(), new THREE.MeshLambertMaterial({ vertexColors: true, transparent: true, opacity: 0.92 }));
  };

  M.parts = Parts.make(M);
  var bl = Buildings.make(M);
  M.building = bl.building; M.scaffold = bl.scaffold;
  var ch = Chars.make(M);
  M.crowd = ch.crowd; M.pet = ch.pet; M.petRig = ch.petRig; M.blobs = ch.blobs;
  return M;
}

module.exports = { make: make, C: C };
