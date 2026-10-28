// 建筑零件：石砌地基、带木构架的墙、有厚度和出檐的人字顶（瓦片一排排）、四坡顶、带窗框/窗棂/窗台/百叶的窗、
// 带门框和台阶的门、烟囱、灌木、花坛、栅栏、灯笼、木桶、木箱、石板路。全部往同一个 Builder 里加（一栋楼一次 draw call）。
//
// 朝向约定：零件的「正面」是本地 +z；ry 绕 y 轴转（Ry：x' = x·cos + z·sin，z' = −x·sin + z·cos），
// 窗和门贴在墙面上时给墙面中心点 + 朝向，零件自己往外凸一点（0.01）免得和墙面 z-fighting。
function make(M) {
  var G = M.G, C = M.C, tint = M.tint, THREE = M.THREE;
  function rotP(x, y, z, ry, lx, ly, lz) { var c = Math.cos(ry), s = Math.sin(ry); return [x + lx * c + lz * s, y + ly, z - lx * s + lz * c]; }
  // 三角山墙（底边 x∈[-0.5,0.5]、y=0，顶点 (0,1)，沿 z 挤出 1）
  var gableGeo = (function () {
    var sh = new THREE.Shape(); sh.moveTo(-0.5, 0); sh.lineTo(0.5, 0); sh.lineTo(0, 1); sh.closePath();
    var g = new THREE.ExtrudeGeometry(sh, { depth: 1, bevelEnabled: false }); g.translate(0, 0, -0.5);
    // ExtrudeGeometry 的 UV 是形状坐标（x∈[-0.5,0.5]）：挪到 [0,1]
    var uv = g.attributes.uv; for (var i = 0; i < uv.count; i++) uv.setXY(i, uv.getX(i) + 0.5, uv.getY(i));
    var n = g.index ? g.toNonIndexed() : g; n.computeVertexNormals(); return n;
  })();

  var P = {};
  // 地基：比墙宽一圈的石台（粗石纹）
  P.plinth = function (b, x, z, w, d, h) { b.add(G.bevel(w + 0.05, h, d + 0.05, 0.012), tint(0xb9b2a6, 0.55), [x, h / 2, z], null, null, 'rock'); return h; };
  // 墙：倒角盒子；timber = 木构架（四角立柱 + 上下两道圈梁 + 正面斜撑），是欧式小屋最有辨识度的细节
  P.walls = function (b, x, y0, z, w, h, d, color, opt) {
    opt = opt || {};
    b.add(G.bevel(w, h, d, 0.01), tint(color, 0.15), [x, y0 + h / 2, z], null, null, opt.tex || 'plaster');
    if (!opt.timber) return;
    var tw = 0.03, wood = tint(opt.wood || C.woodDark, 0.25);
    for (var sx = -1; sx <= 1; sx += 2) for (var sz = -1; sz <= 1; sz += 2)
      b.add(G.bevel(tw, h + 0.01, tw, 0.006), wood, [x + sx * (w / 2 - tw / 2 + 0.006), y0 + h / 2, z + sz * (d / 2 - tw / 2 + 0.006)], null, null, 'planks');
    [y0 + 0.012, y0 + h - 0.012].forEach(function (yy) {
      b.add(G.bevel(w + 0.014, 0.024, tw * 0.6, 0.005), wood, [x, yy, z + d / 2 + 0.004], null, null, 'planks');
      b.add(G.bevel(w + 0.014, 0.024, tw * 0.6, 0.005), wood, [x, yy, z - d / 2 - 0.004], null, null, 'planks');
      b.add(G.bevel(tw * 0.6, 0.024, d + 0.014, 0.005), wood, [x + w / 2 + 0.004, yy, z], null, null, 'planks');
      b.add(G.bevel(tw * 0.6, 0.024, d + 0.014, 0.005), wood, [x - w / 2 - 0.004, yy, z], null, null, 'planks');
    });
    if (opt.braces) {
      var len = Math.sqrt(h * h * 0.25 + w * w * 0.04), ang = Math.atan2(h * 0.5, w * 0.2);
      for (var k = -1; k <= 1; k += 2) b.add(G.box, wood, [x + k * (w / 2 - w * 0.12), y0 + h * 0.5, z + d / 2 + 0.006], [0, 0, k * (Math.PI / 2 - ang)], [0.016, len, 0.012], 'planks');
    }
  };
  // 人字顶：屋脊沿 x（along='x'，两坡朝 ±z）或沿 z。y = 檐口高度；ph = 屋脊比檐口高多少
  P.gable = function (b, x, y, z, w, d, ph, roof, wall, along) {
    var ax = along !== 'z', span = ax ? d : w, len = (ax ? w : d) + 0.1, hw = span / 2 + 0.06, sl = Math.sqrt(hw * hw + ph * ph), a = Math.atan2(ph, hw), th = 0.035;
    for (var side = -1; side <= 1; side += 2) {
      var cx = side * hw / 2, cy = y + ph / 2 + th * 0.5 * Math.cos(a);
      var rot = ax ? [side * a, 0, 0] : [0, 0, -side * a];
      var pos = ax ? [x, cy, z + cx] : [x + cx, cy, z];
      var dims = ax ? [len, th, sl] : [sl, th, len];
      b.add(G.bevel(dims[0], dims[1], dims[2], 0.008), tint(roof, 0.1), pos, rot, null, 'roof');
      // 瓦片排：每坡 4 道略深的横条（一道一道的阴影线，远看就是一排排瓦）
      for (var r = 0; r < 4; r++) {
        var t = (r + 0.5) / 4, ox = side * hw * (1 - t), oy = y + ph * t + th + 0.004;
        var rp = ax ? [x, oy, z + ox] : [x + ox, oy, z], rd = ax ? [len + 0.01, 0.012, 0.022] : [0.022, 0.012, len + 0.01];
        b.add(G.box, tint(new THREE.Color(roof).multiplyScalar(0.78).getHex(), 0.1), rp, rot, rd, 'roof');
      }
    }
    // 屋脊：一根圆木
    b.add(G.cyl8, tint(new THREE.Color(roof).multiplyScalar(0.7).getHex(), 0.1), [x, y + ph + th + 0.01, z], ax ? [0, 0, Math.PI / 2] : [Math.PI / 2, 0, 0], [0.045, len + 0.02, 0.045], 'roof');
    // 两端山墙：三角
    b.add(gableGeo, tint(wall, 0.15), [x, y, z], ax ? [0, Math.PI / 2, 0] : null, ax ? [span, ph, (ax ? w : d) - 0.002] : [span, ph, d - 0.002], 'plaster');
  };
  // 四坡顶（亭子 / 方塔）：四棱锥 + 一圈檐板
  P.hip = function (b, x, y, z, w, d, ph, roof) {
    b.add(G.bevel(w + 0.1, 0.03, d + 0.1, 0.01), tint(new THREE.Color(roof).multiplyScalar(0.8).getHex(), 0.1), [x, y + 0.015, z], null, null, 'roof');
    b.add(G.cone4, tint(roof, 0.1), [x, y + 0.03 + ph / 2, z], [0, Math.PI / 4, 0], [w * 1.05, ph, d * 1.05], 'roof');
  };
  // 窗：窗框 + 玻璃 + 十字窗棂 + 窗台 + （可选）两扇百叶
  P.win = function (b, x, y, z, ry, w, h, opt) {
    opt = opt || {};
    var fr = tint(opt.frame || 0xf5efe2, 0.3), glass = opt.lit ? 0xffe9a8 : 0x4f7fa8;
    b.add(G.bevel(w + 0.03, h + 0.03, 0.018, 0.005), fr, rotP(x, y, z, ry, 0, 0, 0.006), [0, ry, 0], null, 'planks');
    b.add(G.box, glass, rotP(x, y, z, ry, 0, 0, 0.012), [0, ry, 0], [w, h, 0.01]);
    b.add(G.box, glass === 0x4f7fa8 ? 0x9fd0f0 : 0xfff6d0, rotP(x, y, z, ry, -w * 0.22, h * 0.22, 0.0175), [0, ry, 0.5], [w * 0.12, h * 0.5, 0.002]);   // 玻璃反光
    b.add(G.box, fr, rotP(x, y, z, ry, 0, 0, 0.018), [0, ry, 0], [0.008, h, 0.008]);
    b.add(G.box, fr, rotP(x, y, z, ry, 0, 0, 0.018), [0, ry, 0], [w, 0.008, 0.008]);
    b.add(G.bevel(w + 0.05, 0.016, 0.04, 0.005), fr, rotP(x, y, z, ry, 0, -h / 2 - 0.02, 0.016), [0, ry, 0], null, 'planks');
    if (opt.shutter) for (var k = -1; k <= 1; k += 2)
      b.add(G.bevel(w * 0.5, h + 0.02, 0.012, 0.004), tint(opt.shutter, 0.3), rotP(x, y, z, ry, k * (w * 0.5 + w * 0.25 + 0.02), 0, 0.008), [0, ry, 0], null, 'planks');
    if (opt.box) { // 窗台花箱
      b.add(G.bevel(w + 0.02, 0.03, 0.035, 0.005), tint(C.woodDark, 0.3), rotP(x, y, z, ry, 0, -h / 2 - 0.04, 0.03), [0, ry, 0], null, 'planks');
      for (var f = 0; f < 3; f++) b.add(G.sphMi, C.flower[f % 4], rotP(x, y, z, ry, (f - 1) * w * 0.32, -h / 2 - 0.018, 0.032), null, [0.025, 0.022, 0.025]);
    }
  };
  // 门：门框、木门（拱顶）、门把、门前台阶
  P.door = function (b, x, y, z, ry, w, h, color) {
    var fr = tint(0x8a6a4a, 0.3);
    b.add(G.bevel(w + 0.035, h + 0.02, 0.02, 0.005), fr, rotP(x, y + h / 2, z, ry, 0, 0, 0.006), [0, ry, 0], null, 'planks');
    b.add(G.box, tint(color || C.woodDark, 0.25), rotP(x, y + h / 2 - 0.005, z, ry, 0, 0, 0.012), [0, ry, 0], [w, h - 0.01, 0.012], 'planks');
    b.add(G.cyl, tint(color || C.woodDark, 0.25), rotP(x, y + h - 0.01, z, ry, 0, 0, 0.012), [0, ry + Math.PI / 2, Math.PI / 2], [w * 0.7, 0.012, w], 'planks');   // 拱顶：Rz 先把轴转平，再 Ry 朝外
    b.add(G.sphLo, C.gold, rotP(x, y + h * 0.45, z, ry, w * 0.3, 0, 0.022), null, [0.014, 0.014, 0.014]);
    b.add(G.bevel(w + 0.08, 0.025, 0.07, 0.008), tint(0xc9c2b5, 0.5), rotP(x, y - 0.012, z, ry, 0, 0, 0.04), [0, ry, 0], null, 'rock');
  };
  P.chimney = function (b, x, y, z, h) {
    b.add(G.bevel(0.09, h, 0.09, 0.008), tint(0xb5651d, 0.5), [x, y + h / 2, z], null, null, 'brick');
    b.add(G.bevel(0.115, 0.025, 0.115, 0.006), tint(0x8a5a3a, 0.5), [x, y + h, z], null, null, 'brick');
    b.add(G.box, 0x2a1d14, [x, y + h + 0.013, z], null, [0.06, 0.004, 0.06]);
  };
  P.bush = function (b, x, y, z, s, col) {
    s = s || 1; col = col || C.leafDark;
    b.add(G.ico1, tint(col, 0.4), [x, y + 0.045 * s, z], [x * 3, z * 3, 0], [0.13 * s, 0.1 * s, 0.12 * s], 'leaves');
    b.add(G.ico1, tint(C.leaf, 0.42), [x + 0.04 * s, y + 0.07 * s, z - 0.02 * s], null, [0.08 * s, 0.07 * s, 0.08 * s], 'leaves');
  };
  P.flowerBed = function (b, x, y, z, w, d, k) {
    b.add(G.bevel(w, 0.035, d, 0.008), tint(0x8a6a4a, 0.4), [x, y + 0.017, z], null, null, 'planks');
    b.add(G.box, 0x6b4a2b, [x, y + 0.036, z], null, [w - 0.02, 0.004, d - 0.02], 'field');
    var n = Math.max(2, Math.round(w / 0.05));
    for (var i = 0; i < n; i++) {
      var fx = x - w / 2 + (i + 0.5) * w / n, fz = z + ((i % 2) - 0.5) * d * 0.35;
      b.add(G.sphLo, tint(C.leafDark, 0.4), [fx, y + 0.05, fz], null, [0.03, 0.025, 0.03]);
      b.add(G.sphMi, C.flower[(i + (k || 0)) % 4], [fx, y + 0.07, fz], null, [0.022, 0.02, 0.022]);
    }
  };
  // 栅栏：从 (x0,z0) 到 (x1,z1)，n 根桩 + 两道横栏
  P.fence = function (b, x0, z0, x1, z1, y, n, col) {
    var c = tint(col || 0xf3ead8, 0.35), dx = x1 - x0, dz = z1 - z0, L = Math.sqrt(dx * dx + dz * dz), ry = Math.atan2(dx, dz);
    for (var i = 0; i <= n; i++) { var t = i / n; b.add(G.bevel(0.022, 0.09, 0.022, 0.005), c, [x0 + dx * t, y + 0.045, z0 + dz * t], null, null, 'planks'); }
    [0.03, 0.065].forEach(function (h) { b.add(G.box, c, [x0 + dx / 2, y + h, z0 + dz / 2], [0, ry, 0], [0.01, 0.014, L], 'planks'); });
  };
  P.lantern = function (b, x, y, z, h) {
    h = h || 0.22;
    b.add(G.cyl8, 0x3a3a3a, [x, y + h / 2, z], null, [0.014, h, 0.014], 'metal');
    b.add(G.bevel(0.04, 0.05, 0.04, 0.006), 0xfff0b0, [x, y + h + 0.02, z]);
    b.add(G.cone4, 0x3a3a3a, [x, y + h + 0.055, z], [0, Math.PI / 4, 0], [0.05, 0.025, 0.05], 'metal');
  };
  P.barrel = function (b, x, y, z, s) {
    s = s || 1;
    b.add(G.cyl, tint(C.wood, 0.3), [x, y + 0.045 * s, z], null, [0.07 * s, 0.09 * s, 0.07 * s], 'planks');
    [0.015, 0.075].forEach(function (h) { b.add(G.cyl, 0x4a4a4a, [x, y + h * s, z], null, [0.074 * s, 0.008 * s, 0.074 * s], 'metal'); });
  };
  P.crate = function (b, x, y, z, s, ry) {
    b.add(G.bevel(s, s, s, s * 0.08), tint(0xb08050, 0.3), [x, y + s / 2, z], [0, ry || 0, 0], null, 'planks');
    b.add(G.box, tint(C.woodDark, 0.3), [x, y + s / 2, z], [0, ry || 0, Math.PI / 4], [s * 0.12, s * 1.3, s * 1.02], 'planks');
  };
  // 石板路：从 (x,z) 沿 ry 方向铺 n 块
  P.path = function (b, x, y, z, ry, n, step) {
    for (var i = 0; i < n; i++) {
      var p = rotP(x, y, z, ry, ((i * 37) % 7 - 3) * 0.008, 0.008, i * (step || 0.11));
      b.add(G.bevel(0.09 + (i % 2) * 0.02, 0.016, 0.075, 0.006), tint(0xcfc8bb, 0.5), p, [0, ry + i * 0.3, 0], null, 'rock');
    }
  };
  // 背面和两侧的窗：楼会被转 0/90/180/270°，镜头看到哪两面都有可能 —— 只有正面有门窗的话，转过去就是一堵白板墙
  // n = [背面几扇, 每侧几扇]；y = 窗中心高度
  P.aroundWindows = function (b, x, y, z, w, d, n, opt) {
    var nb = n[0], ns = n[1], ww = opt && opt.w || 0.07, wh = opt && opt.h || 0.065;
    for (var i = 0; i < nb; i++) P.win(b, x + (i - (nb - 1) / 2) * (w / Math.max(nb, 1.6)), y, z - d / 2, Math.PI, ww, wh, opt);
    for (var s = -1; s <= 1; s += 2) for (var j = 0; j < ns; j++)
      P.win(b, x + s * w / 2, y, z + (j - (ns - 1) / 2) * (d / Math.max(ns, 1.6)) * -s, s * Math.PI / 2, ww, wh, opt);
  };
  P.rotP = rotP;
  return P;
}

module.exports = { make: make };
