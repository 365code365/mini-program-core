// 十种建筑 + 施工脚手架。返回 { group, parts }：静态零件合并成 parts.main（一次 draw call），会动的单独成网格：
// spin（风车叶，绕 z 转）、spinZ（锯片）、bob（渔船）、lamp / beam（灯塔），chimney 是冒烟点（Vector3）。
// tier = 外观档（1 / 5 / 10 / 15 级换一次）：加层、加旗子、加装饰；地块 1×1，模型落在 [-0.47, 0.47]。
function make(M) {
  var THREE = M.THREE, G = M.G, C = M.C, tint = M.tint, P = M.parts;
  var rotP = P.rotP;

  function newB(type, lv) { var b = new M.Builder(); b.ao = true; b.td = 2.2; b.jit = 0.1; M.reseed(type.length * 31 + lv); return b; }
  // 地面：一圈压实的土 / 碎石院子（比地格小，边缘不切到地格边上 —— 整齐的方台子正是「方块感」的来源）+ 门前石板路
  // 石头铺的院子拆成 3×3 块（一块一份贴图）：整张卵石图铺满 0.9 的院子时，一颗卵石比木箱还大，尺度不对
  function yard(b, w, d, tex, col) {
    tex = tex || 'cobble';
    var n = tex === 'cobble' || tex === 'rock' ? 3 : 1, cw = w / n, cd = d / n;
    for (var i = 0; i < n; i++) for (var j = 0; j < n; j++)
      b.add(G.bevel(cw - 0.006, 0.02, cd - 0.006, 0.006), tint(col || 0xd9cfbd, 0.55), [-w / 2 + cw * (i + 0.5), 0.01, -d / 2 + cd * (j + 0.5)], null, null, tex);
  }
  function flag(b, x, y, z, color) {
    b.add(G.cyl8, tint(C.woodDark), [x, y + 0.22, z], null, [0.016, 0.44, 0.016], 'bark');
    b.add(G.sphLo, C.gold, [x, y + 0.45, z], null, [0.025, 0.025, 0.025]);
    b.add(G.bevel(0.16, 0.09, 0.008, 0.003), color, [x + 0.085, y + 0.37, z], [0, 0, -0.06]);
  }
  function mk(b) { var m = new THREE.Mesh(b.geometry(), M.MAT); m.castShadow = true; m.receiveShadow = true; return m; }

  var T = {};
  T.farm = function (b, t, ex) {
    // 麦田：翻过的土垄 + 一簇簇麦穗；红色谷仓（白边、交叉门）；白色风车（风车叶单独转）
    b.add(G.bevel(0.56, 0.035, 0.52, 0.012), tint(0x8a5a32, 0.6), [-0.15, 0.018, -0.13], null, null, 'field');
    for (var r = 0; r < 5; r++) {
      var rz = -0.34 + r * 0.105;
      b.add(G.bevel(0.52, 0.02, 0.05, 0.01), tint(0x6e4526, 0.6), [-0.15, 0.04, rz], null, null, 'field');
      for (var c = 0; c < 6; c++) {
        var sx = -0.39 + c * 0.095 + ((r * 7 + c * 3) % 5) * 0.004, hh = 0.11 + ((r + c) % 3) * 0.015 + t * 0.012;
        // 一簇三根：细秆往外斜 + 顶上一颗麦穗（拉长的二十面体，硬边看着像一粒粒麦粒）。v4 第一版是一根粗圆锥，远看像一排黄色保龄球瓶
        for (var k3 = 0; k3 < 3; k3++) {
          var ka = k3 * 2.1 + r + c, lx = Math.cos(ka) * 0.012, lz = Math.sin(ka) * 0.012, tilt = 0.18;
          b.add(G.box, tint(0xb8a040, 0.35), [sx + lx, 0.05 + hh / 2, rz + lz], [Math.sin(ka) * tilt, 0, -Math.cos(ka) * tilt], [0.006, hh, 0.006], 'hay');
          b.add(G.ico, tint(C.wheat, 0.25), [sx + lx * 3.2, 0.05 + hh + 0.012, rz + lz * 3.2], [Math.sin(ka) * tilt * 2, 0, -Math.cos(ka) * tilt * 2], [0.022, 0.055, 0.022], 'hay');
        }
      }
    }
    P.fence(b, -0.45, 0.15, 0.13, 0.15, 0.02, 6);
    P.fence(b, -0.45, -0.42, -0.45, 0.15, 0.02, 6);
    var bh = 0.22 + t * 0.04;
    P.plinth(b, 0.27, 0.2, 0.3, 0.26, 0.03);
    P.walls(b, 0.27, 0.03, 0.2, 0.3, bh, 0.26, 0xb83a2e, { tex: 'planks', timber: true, wood: 0xf3ead8 });
    P.gable(b, 0.27, 0.03 + bh, 0.2, 0.3, 0.26, 0.14, 0x6f7b88, 0xb83a2e, 'z');
    P.door(b, 0.27, 0.03, 0.33, 0, 0.11, 0.14, 0xf3ead8);
    b.add(G.box, 0xb83a2e, [0.27, 0.1, 0.348], [0, 0, 0.7], [0.012, 0.16, 0.006]); b.add(G.box, 0xb83a2e, [0.27, 0.1, 0.348], [0, 0, -0.7], [0.012, 0.16, 0.006]);
    P.win(b, 0.27, 0.03 + bh + 0.06, 0.336, 0, 0.06, 0.05, { frame: 0xf3ead8 });
    P.aroundWindows(b, 0.27, 0.03 + bh * 0.55, 0.2, 0.3, 0.26, [1, 1], { frame: 0xf3ead8, w: 0.05, h: 0.05 });
    // 风车：八角塔身 + 帽 + 轴
    b.add(G.taper8, tint(0xf6f1e6, 0.2), [-0.32, 0.03 + 0.26, 0.31], null, [0.17, 0.52, 0.17], 'plaster');
    b.add(G.cone8f, tint(0x6f7b88, 0.1), [-0.32, 0.6, 0.31], null, [0.17, 0.14, 0.17], 'roof');
    P.door(b, -0.32, 0.03, 0.385, 0, 0.05, 0.09);
    P.win(b, -0.32, 0.32, 0.375, 0, 0.035, 0.04);
    b.add(G.cyl8, 0x4a3a2a, [-0.32, 0.5, 0.39], [Math.PI / 2, 0, 0], [0.025, 0.06, 0.025]);
    for (var hb = 0; hb < 2 + t; hb++) b.add(G.cyl, tint(C.hay, 0.3), [0.4 - hb * 0.02, 0.05, -0.05 - hb * 0.1], [Math.PI / 2, 0, 0], [0.08, 0.09, 0.08], 'hay');
    var bb = new M.Builder();
    for (var k = 0; k < 4; k++) {
      var rr = [0, 0, k * Math.PI / 2], ca = Math.cos(k * Math.PI / 2), sa = Math.sin(k * Math.PI / 2);
      bb.add(G.box, 0x6e4527, [-sa * 0.13, ca * 0.13, 0], rr, [0.012, 0.26, 0.012]);
      bb.add(G.box, 0xf6f1e6, [-sa * 0.15 + ca * 0.025, ca * 0.15 + sa * 0.025, -0.004], rr, [0.045, 0.19, 0.004], 'planks');
    }
    bb.add(G.sphLo, 0x4a3a2a, [0, 0, 0.006], null, [0.03, 0.03, 0.03]);
    var bl = new THREE.Mesh(bb.geometry(), M.MAT); var blades = new THREE.Group(); blades.add(bl); blades.position.set(-0.32, 0.5, 0.425); ex(blades, 'spin');
  };
  T.lumber = function (b, t, ex) {
    // 原木小屋：墙是一根根横着摞的圆木（交错出头），绿瓦人字顶；右边是原木堆、砧板和锯台（锯片单独转）
    var x0 = -0.1, z0 = -0.1, w = 0.44, d = 0.36, n = 6 + t, r = 0.026;
    P.plinth(b, x0, z0, w, d, 0.03);
    for (var i = 0; i < n; i++) {
      var y = 0.03 + r + i * r * 1.8, col = tint(i % 2 ? 0x9a6a40 : 0x8a5c36, 0.25);
      b.add(G.cyl8, col, [x0, y, z0 + d / 2], [0, 0, Math.PI / 2], [r * 2, w + 0.06, r * 2], 'bark');
      b.add(G.cyl8, col, [x0, y, z0 - d / 2], [0, 0, Math.PI / 2], [r * 2, w + 0.06, r * 2], 'bark');
      b.add(G.cyl8, col, [x0 + w / 2, y + r * 0.9, z0], [Math.PI / 2, 0, 0], [r * 2, d + 0.06, r * 2], 'bark');
      b.add(G.cyl8, col, [x0 - w / 2, y + r * 0.9, z0], [Math.PI / 2, 0, 0], [r * 2, d + 0.06, r * 2], 'bark');
    }
    var top = 0.03 + n * r * 1.8 + r;
    b.add(G.box, tint(0x7a5232, 0.3), [x0, 0.03 + top / 2, z0], null, [w - 0.02, top - 0.03, d - 0.02], 'planks');
    P.gable(b, x0, top, z0, w, d, 0.17, C.roofGreen, 0x8a5c36, 'x');
    P.door(b, x0 - 0.08, 0.03, z0 + d / 2 + r, 0, 0.1, 0.16);
    P.win(b, x0 + 0.1, 0.15, z0 + d / 2 + r, 0, 0.07, 0.06, { shutter: 0x3f8f5b });
    P.chimney(b, x0 + 0.13, top, z0 - 0.08, 0.18);
    P.aroundWindows(b, x0, 0.15, z0, w + 2 * r, d + 2 * r, [1, 1], { shutter: 0x3f8f5b, w: 0.06, h: 0.055 });
    // 原木堆：金字塔摞法，端面颜色浅（年轮）
    var rows = [3, 2, 1];
    rows.forEach(function (cnt, ry) { for (var j = 0; j < cnt; j++) {
      var ly = 0.035 + ry * 0.06, lz = 0.06 + (j - (cnt - 1) / 2) * 0.065 + 0.0;
      b.add(G.cyl, tint(C.trunk, 0.25), [0.33, ly, lz], [0, 0, Math.PI / 2], [0.06, 0.24, 0.06], 'bark');
      b.add(G.cyl, 0xe8c48a, [0.33 + 0.121, ly, lz], [0, 0, Math.PI / 2], [0.05, 0.004, 0.05]);
      b.add(G.cyl, 0xe8c48a, [0.33 - 0.121, ly, lz], [0, 0, Math.PI / 2], [0.05, 0.004, 0.05]);
    } });
    // 砧板 + 斧头
    b.add(G.cyl, tint(C.trunk, 0.3), [0.2, 0.04, 0.32], null, [0.1, 0.08, 0.1], 'bark');
    b.add(G.cyl, 0xe8c48a, [0.2, 0.081, 0.32], null, [0.09, 0.002, 0.09]);
    b.add(G.box, 0x7a5232, [0.2, 0.12, 0.32], [0, 0, 0.5], [0.012, 0.1, 0.012]);
    b.add(G.bevel(0.04, 0.03, 0.008, 0.003), 0xb8c4cc, [0.18, 0.09, 0.32], [0, 0, 0.5], null, 'metal');
    // 锯台
    b.add(G.bevel(0.2, 0.025, 0.08, 0.006), tint(C.woodDark, 0.3), [-0.25, 0.1, 0.32], null, null, 'planks');
    for (var lg = -1; lg <= 1; lg += 2) b.add(G.box, tint(C.woodDark, 0.3), [-0.25 + lg * 0.08, 0.05, 0.32], null, [0.015, 0.09, 0.06], 'planks');
    var saw = new THREE.Mesh(new THREE.CylinderGeometry(0.06, 0.06, 0.008, 16), new THREE.MeshLambertMaterial({ color: 0xcfd8dc }));
    saw.rotation.x = Math.PI / 2; var sg = new THREE.Group(); sg.add(saw); sg.position.set(-0.25, 0.13, 0.32); ex(sg, 'spinZ');
    for (var tr = 0; tr < 2; tr++) P.bush(b, 0.35 - tr * 0.05, 0, -0.36 + tr * 0.07, 1.1);
  };
  T.quarry = function (b, t, ex) {
    // 采石场：一座嶙峋的石山（切出平台）、码好的石块、木吊架吊着一块石头、铁轨 + 矿车
    yard(b, 0.9, 0.9, 'rock', 0xbdb5a8);
    var rk = [[-0.16, 0.13, -0.16, 0.5, 0.32, 0.42], [0.12, 0.09, -0.24, 0.32, 0.2, 0.28], [-0.32, 0.07, 0.05, 0.22, 0.14, 0.24], [0.02, 0.24, -0.18, 0.26, 0.2, 0.22]];
    rk.forEach(function (q, i) { b.add(G.rock, tint(i % 2 ? 0x9aa1a8 : 0xb1b6bb, 0.6), [q[0], q[1], q[2]], [i, i * 1.7, 0.3], [q[3], q[4], q[5]], 'rock'); });
    b.add(G.bevel(0.22, 0.02, 0.16, 0.008), tint(0xc9c3b8, 0.5), [-0.1, 0.2, -0.05], [0, 0.3, 0], null, 'rock');
    for (var s = 0; s < 3 + t; s++) b.add(G.bevel(0.09, 0.06, 0.07, 0.008), tint(0xd5d0c6, 0.5), [0.25 + (s % 2) * 0.1, 0.05 + Math.floor(s / 2) * 0.062, 0.0 + (s % 3) * 0.02], [0, s * 0.2, 0], null, 'rock');
    // 吊架（A 字形）
    [[-0.38, 0.38], [-0.24, 0.38]].forEach(function (p) {
      b.add(G.box, tint(C.woodDark, 0.3), [p[0], 0.24, p[1] - 0.04], [0.18, 0, 0], [0.02, 0.48, 0.02], 'planks');
      b.add(G.box, tint(C.woodDark, 0.3), [p[0], 0.24, p[1] + 0.04 - 0.08], [-0.18, 0, 0], [0.02, 0.48, 0.02], 'planks');
    });
    b.add(G.cyl8, tint(C.woodDark, 0.3), [-0.31, 0.47, 0.34], [0, 0, Math.PI / 2], [0.025, 0.2, 0.025], 'bark');
    b.add(G.bevel(0.03, 0.4, 0.02, 0.005), tint(C.woodDark, 0.3), [-0.17, 0.43, 0.34], [0, 0, -1.2], null, 'planks');
    b.add(G.cyl6, 0xd9c9a0, [-0.04, 0.33, 0.34], null, [0.006, 0.16, 0.006]);
    b.add(G.bevel(0.07, 0.05, 0.06, 0.008), tint(0xd5d0c6, 0.5), [-0.04, 0.23, 0.34], [0, 0.4, 0], null, 'rock');
    // 铁轨 + 矿车
    for (var sl = 0; sl < 6; sl++) b.add(G.box, tint(C.woodDark, 0.4), [-0.05 + sl * 0.08, 0.025, 0.2], null, [0.025, 0.012, 0.14], 'planks');
    for (var rl = -1; rl <= 1; rl += 2) b.add(G.box, 0x6d6d6d, [0.15, 0.035, 0.2 + rl * 0.045], null, [0.5, 0.012, 0.01], 'metal');
    b.add(G.bevel(0.16, 0.08, 0.11, 0.01), tint(0x6a5a4a, 0.4), [0.2, 0.09, 0.2], null, null, 'metal');
    for (var wh = 0; wh < 4; wh++) b.add(G.cyl, 0x3a3a3a, [0.15 + (wh % 2) * 0.1, 0.05, 0.2 + (wh < 2 ? -1 : 1) * 0.058], [Math.PI / 2, 0, 0], [0.04, 0.012, 0.04], 'metal');
    for (var ore = 0; ore < 4; ore++) b.add(G.ico, tint(0x8e959c, 0.5), [0.17 + (ore % 2) * 0.05, 0.14, 0.18 + (ore >> 1) * 0.04], [ore, ore, 0], [0.05, 0.04, 0.05], 'rock');
    for (var g = 0; g < t; g++) b.add(G.ico, C.gold, [0.24 - g * 0.04, 0.15, 0.22], [g, 0, g], [0.035, 0.03, 0.035]);
    b.add(G.box, 0x7a5232, [0.36, 0.08, 0.35], [0, 0, 0.3], [0.012, 0.15, 0.012]); b.add(G.bevel(0.09, 0.015, 0.015, 0.004), 0x9aa1a8, [0.38, 0.15, 0.35], [0, 0, 0.3, 0], null, 'metal');
  };
  T.bakery = function (b, t, ex, parts) {
    // 面包坊：粉墙木构架、两层（高档加一层）、条纹遮阳篷、橱窗里摆着面包、挂招牌、烟囱冒烟
    var w = 0.56, d = 0.46, h = 0.3 + t * 0.08, x0 = 0, z0 = -0.06;
    yard(b, 0.84, 0.86, 'cobble');
    P.plinth(b, x0, z0, w, d, 0.04);
    P.walls(b, x0, 0.04, z0, w, h, d, C.wallPink, { timber: true, braces: true });
    if (t >= 1) b.add(G.bevel(w + 0.04, 0.03, d + 0.04, 0.008), tint(C.woodDark, 0.3), [x0, 0.04 + h * 0.52, z0], null, null, 'planks');   // 楼层间挑出的木梁
    P.gable(b, x0, 0.04 + h, z0, w, d, 0.2, C.roofRed, C.wallPink, 'x');
    P.chimney(b, 0.17, 0.04 + h + 0.03, z0 - 0.1, 0.24);
    parts.chimney = new THREE.Vector3(0.17, 0.04 + h + 0.3, z0 - 0.1);
    var fz = z0 + d / 2;
    P.door(b, -0.14, 0.04, fz, 0, 0.11, 0.17, 0x6e2e2e);
    P.aroundWindows(b, x0, 0.04 + h * 0.4, z0, w, d, [2, 1], { box: true, shutter: 0xc8506e });
    if (t >= 1) P.aroundWindows(b, x0, 0.04 + h * 0.78, z0, w, d, [2, 1], { w: 0.06, h: 0.055 });
    // 橱窗：大玻璃 + 面包
    P.win(b, 0.11, 0.14, fz, 0, 0.2, 0.12, { frame: 0x6e2e2e });
    for (var br = 0; br < 4; br++) b.add(G.sph, 0xc8843c, [0.03 + br * 0.055, 0.075, fz + 0.035], [0, 0, 0.2], [0.045, 0.025, 0.03]);
    // 遮阳篷：一条条斜着的红白布 + 下沿波浪
    for (var a = 0; a < 8; a++) {
      var ax = -0.24 + a * 0.069;
      b.add(G.box, a % 2 ? 0xfdf6ee : 0xe0525e, [ax, 0.04 + 0.25, fz + 0.06], [0.55, 0, 0], [0.069, 0.006, 0.15]);
      b.add(G.sphLo, a % 2 ? 0xfdf6ee : 0xe0525e, [ax, 0.04 + 0.205, fz + 0.12], null, [0.069, 0.03, 0.012]);
    }
    if (t >= 1) { P.win(b, -0.14, 0.04 + h * 0.75, fz, 0, 0.08, 0.07, { box: true }); P.win(b, 0.14, 0.04 + h * 0.75, fz, 0, 0.08, 0.07, { box: true }); }
    // 招牌（侧挂）：铁臂 + 木牌 + 金面包
    b.add(G.box, 0x333333, [-w / 2 - 0.04, 0.04 + h * 0.62, fz - 0.02], null, [0.08, 0.01, 0.01], 'metal');
    b.add(G.bevel(0.008, 0.07, 0.08, 0.003), tint(C.woodDark, 0.3), [-w / 2 - 0.06, 0.04 + h * 0.62 - 0.045, fz - 0.02], null, null, 'planks');
    b.add(G.sph, C.gold, [-w / 2 - 0.066, 0.04 + h * 0.62 - 0.045, fz - 0.02], null, [0.01, 0.03, 0.05]);
    P.bush(b, -0.36, 0, 0.32); P.bush(b, 0.38, 0, 0.3, 0.8);
    P.barrel(b, 0.36, 0, -0.32); P.crate(b, 0.3, 0, -0.38, 0.07, 0.3);
    P.path(b, -0.14, 0.02, fz + 0.08, 0, 2, 0.1);
  };
  T.dock = function (b, t, ex) {
    // 渔港：伸进海里的栈桥（木桩 + 板）、蓝顶船屋、木桶、渔网架、路灯；小船单独晃
    for (var pz = 0; pz < 6; pz++) b.add(G.bevel(0.36, 0.022, 0.07, 0.005), tint(pz % 2 ? 0xa57a50 : 0x946a44, 0.3), [0.12, 0.12, -0.38 + pz * 0.077], [0, ((pz * 7) % 3 - 1) * 0.02, 0], null, 'planks');
    for (var px = -1; px <= 1; px += 2) for (var pp = 0; pp < 3; pp++) b.add(G.cyl8, tint(C.woodDark, 0.3), [0.12 + px * 0.17, 0.04, -0.38 + pp * 0.22], null, [0.035, 0.22, 0.035], 'bark');
    b.add(G.bevel(0.08, 0.022, 0.5, 0.005), tint(0x946a44, 0.3), [0.4, 0.1, 0.08], null, null, 'planks');
    for (var pq = 0; pq < 3; pq++) b.add(G.cyl8, tint(C.woodDark, 0.3), [0.43, 0.04, -0.12 + pq * 0.2], null, [0.03, 0.2, 0.03], 'bark');
    // 船屋
    P.plinth(b, -0.24, -0.2, 0.3, 0.3, 0.03);
    P.walls(b, -0.24, 0.03, -0.2, 0.3, 0.2 + t * 0.04, 0.3, 0xeaf2f6, { tex: 'planks', timber: true, wood: 0x4a6a8a });
    P.gable(b, -0.24, 0.23 + t * 0.04, -0.2, 0.3, 0.3, 0.13, C.roofBlue, 0xeaf2f6, 'z');
    P.door(b, -0.24, 0.03, -0.05, 0, 0.09, 0.13, 0x4a6a8a);
    P.win(b, -0.09, 0.14, -0.2, Math.PI / 2, 0.06, 0.06, { shutter: 0x4a7fd4 });
    P.aroundWindows(b, -0.24, 0.14, -0.2, 0.3, 0.3, [1, 0], { shutter: 0x4a7fd4, w: 0.06, h: 0.06 });
    P.win(b, -0.39, 0.14, -0.2, -Math.PI / 2, 0.06, 0.06, { shutter: 0x4a7fd4 });
    // 渔网架：两根杆 + 一张网（细格子）
    b.add(G.cyl8, tint(C.woodDark), [-0.38, 0.12, 0.12], null, [0.015, 0.22, 0.015], 'bark'); b.add(G.cyl8, tint(C.woodDark), [-0.38, 0.12, 0.34], null, [0.015, 0.22, 0.015], 'bark');
    for (var nt = 0; nt < 4; nt++) b.add(G.box, 0xd9c9a0, [-0.38, 0.06 + nt * 0.04, 0.23], null, [0.003, 0.003, 0.22]);
    for (var nv = 0; nv < 5; nv++) b.add(G.box, 0xd9c9a0, [-0.38, 0.12, 0.13 + nv * 0.045], null, [0.003, 0.15, 0.003]);
    P.barrel(b, -0.08, 0.13, 0.25 - 0.3, 0.8); P.barrel(b, 0.04, 0.13, -0.28, 0.8); P.crate(b, 0.22, 0.13, -0.3, 0.07);
    P.lantern(b, 0.28, 0.13, 0.04, 0.2);
    for (var f = 0; f < 2; f++) b.add(G.sph, 0x7fa8c9, [0.1 + f * 0.05, 0.145, 0.08], [0, f, 0], [0.07, 0.02, 0.025]);   // 码头上晒的鱼
    // 小船：船身（半截压扁的圆柱）+ 舱 + 桅 + 帆
    var boat = new M.Builder();
    boat.add(G.cyl, 0xe0525e, [0, 0.03, 0], [Math.PI / 2, 0, 0], [0.13, 0.32, 0.08], 'planks');
    boat.add(G.bevel(0.11, 0.012, 0.28, 0.004), 0xf6efe0, [0, 0.07, 0], null, null, 'planks');
    boat.add(G.bevel(0.08, 0.05, 0.08, 0.008), 0xf6efe0, [0, 0.1, -0.04], null, null, 'planks');
    boat.add(G.cyl8, C.woodDark, [0, 0.22, 0.04], null, [0.012, 0.3, 0.012], 'bark');
    boat.add(G.plane, 0xfdf8f0, [0, 0.24, 0.1], [0, Math.PI / 2, 0], [0.12, 0.2, 1]);
    var bt = new THREE.Mesh(boat.geometry(), new THREE.MeshLambertMaterial({ vertexColors: true, map: M.MAT.map, side: THREE.DoubleSide })); bt.position.set(0.3, 0, 0.4); ex(bt, 'bob');
  };
  T.apiary = function (b, t, ex) {
    // 蜂场：一排排木蜂箱（多层箱体 + 盖 + 巢门）、花坛环绕、工具棚
    yard(b, 0.88, 0.88, 'field', 0xb89a6a);
    var n = 2 + Math.min(2, t);
    for (var h = 0; h < n; h++) {
      var hx = -0.22 + (h % 2) * 0.32, hz = -0.24 + Math.floor(h / 2) * 0.3, lay = 2 + (h % 2);
      b.add(G.bevel(0.16, 0.04, 0.14, 0.006), tint(C.woodDark, 0.35), [hx, 0.03, hz], null, null, 'planks');
      for (var l = 0; l < lay; l++) b.add(G.bevel(0.14, 0.06, 0.12, 0.006), tint(l % 2 ? 0xf2d27a : 0xf6e3a8, 0.25), [hx, 0.08 + l * 0.062, hz], null, null, 'planks');
      b.add(G.bevel(0.17, 0.025, 0.15, 0.006), tint(0x9a6a3a, 0.3), [hx, 0.06 + lay * 0.062, hz], null, null, 'roof');
      b.add(G.box, 0x2a1d14, [hx, 0.065, hz + 0.061], null, [0.06, 0.008, 0.004]);
      for (var be = 0; be < 2; be++) { b.add(G.sphMi, 0x2a1d14, [hx + 0.07 - be * 0.12, 0.18 + be * 0.06, hz + 0.1], null, [0.014, 0.012, 0.018]); b.add(G.sphMi, 0xffd23f, [hx + 0.07 - be * 0.12, 0.181 + be * 0.06, hz + 0.1], null, [0.015, 0.006, 0.012]); }
    }
    P.flowerBed(b, 0, 0, 0.38, 0.8, 0.09, 0); P.flowerBed(b, 0.38, 0, 0, 0.09, 0.6, 1);
    // 工具棚
    P.walls(b, -0.33, 0.02, 0.18, 0.18, 0.16, 0.16, 0xd9b67a, { tex: 'planks' });
    P.gable(b, -0.33, 0.18, 0.18, 0.18, 0.16, 0.08, 0x9a6a3a, 0xd9b67a, 'z');
    P.door(b, -0.33, 0.02, 0.26, 0, 0.07, 0.11);
    b.add(G.sph, 0xd98e04, [-0.2, 0.05, 0.12], null, [0.07, 0.08, 0.07]); b.add(G.cyl, 0xa86a00, [-0.2, 0.095, 0.12], null, [0.04, 0.012, 0.04]);   // 蜜罐
  };
  T.warehouse = function (b, t, ex) {
    // 仓库：紫红木板大谷仓（白色边框、交叉大门、阁楼吊门、吊钩），门前斜坡、麻袋、木箱、木桶
    var w = 0.74, d = 0.56, h = 0.34 + t * 0.12;
    yard(b, 0.92, 0.9, 'cobble');
    P.plinth(b, 0, -0.06, w, d, 0.04);
    P.walls(b, 0, 0.04, -0.06, w, h, d, 0x9a5a7a, { tex: 'planks', timber: true, wood: 0xf3ead8 });
    P.gable(b, 0, 0.04 + h, -0.06, w, d, 0.24, 0x6f6488, 0x9a5a7a, 'x');
    var fz = -0.06 + d / 2;
    b.add(G.bevel(0.28, 0.24, 0.02, 0.006), 0xf3ead8, [0, 0.16, fz + 0.006], null, null, 'planks');
    b.add(G.box, tint(0x6e3a56, 0.2), [0, 0.155, fz + 0.012], null, [0.25, 0.21, 0.01], 'planks');
    b.add(G.box, 0xf3ead8, [0, 0.155, fz + 0.019], [0, 0, 0.7], [0.018, 0.32, 0.006]); b.add(G.box, 0xf3ead8, [0, 0.155, fz + 0.019], [0, 0, -0.7], [0.018, 0.32, 0.006]);
    b.add(G.box, 0xf3ead8, [0, 0.155, fz + 0.019], null, [0.012, 0.21, 0.006]);
    P.win(b, 0, 0.04 + h + 0.07, fz, 0, 0.09, 0.08, { frame: 0xf3ead8, shutter: 0x6e3a56 });
    b.add(G.box, tint(C.woodDark, 0.3), [0, 0.04 + h + 0.2, fz + 0.06], null, [0.02, 0.02, 0.14], 'planks');
    b.add(G.cyl6, 0xd9c9a0, [0, 0.04 + h + 0.12, fz + 0.12], null, [0.005, 0.16, 0.005]);
    for (var wn = -1; wn <= 1; wn += 2) P.win(b, wn * 0.27, 0.2, fz, 0, 0.07, 0.07, { frame: 0xf3ead8 });
    P.aroundWindows(b, 0, 0.2, -0.06, w, d, [2, 2], { frame: 0xf3ead8 });
    b.add(G.bevel(0.3, 0.02, 0.12, 0.006), tint(0x9a7050, 0.3), [0, 0.03, fz + 0.07], [-0.15, 0, 0], null, 'planks');
    P.crate(b, -0.32, 0, 0.32, 0.1, 0.2); P.crate(b, -0.33, 0.1, 0.32, 0.07, 0.6); P.crate(b, -0.2, 0, 0.36, 0.08, -0.3);
    P.barrel(b, 0.3, 0, 0.32); P.barrel(b, 0.37, 0, 0.24, 0.9);
    for (var sk = 0; sk < 2 + t; sk++) b.add(G.sph, tint(0xe0cfa8, 0.3), [0.2 - (sk % 3) * 0.06, 0.04 + Math.floor(sk / 3) * 0.06, 0.38], [0, sk, 0.2], [0.07, 0.07, 0.06], 'hay');
  };
  T.market = function (b, t, ex) {
    // 集市：三个摊位（柜台 + 四根柱 + 条纹篷 + 波浪边）摆满货，后面一间小店铺
    yard(b, 0.92, 0.92, 'cobble');
    var cols = [0xe0525e, 0x2f8fc8, 0xf2b33d], goods = [[0xff5a5f, 0xffa94d, 0x8ac926], [0xc8843c, 0xe0a060, 0xd9b67a], [0x7fa8c9, 0x9fc6e0, 0xf2d27a]];
    for (var st = 0; st < 3; st++) {
      var sx = -0.29 + st * 0.29, sz = 0.16;
      b.add(G.bevel(0.24, 0.1, 0.14, 0.008), tint(0xa57a50, 0.3), [sx, 0.06, sz], null, null, 'planks');
      b.add(G.bevel(0.26, 0.018, 0.16, 0.005), tint(0x7a5232, 0.3), [sx, 0.12, sz], null, null, 'planks');
      for (var g = 0; g < 6; g++) b.add(G.sphLo, goods[st][g % 3], [sx - 0.09 + (g % 3) * 0.09, 0.145, sz - 0.03 + (g >> 1 & 1) * 0.06], null, [0.04, 0.035, 0.04]);
      for (var sp = 0; sp < 4; sp++) b.add(G.cyl8, tint(C.woodDark, 0.3), [sx + (sp % 2 ? 0.11 : -0.11), 0.17, sz + (sp < 2 ? -0.07 : 0.07)], null, [0.016, 0.34, 0.016], 'bark');
      for (var aw = 0; aw < 5; aw++) {
        var ax = sx - 0.12 + aw * 0.06;
        b.add(G.box, aw % 2 ? 0xfdf6ee : cols[st], [ax, 0.36, sz], [-0.25, 0, 0], [0.06, 0.008, 0.22]);
        b.add(G.sphLo, aw % 2 ? 0xfdf6ee : cols[st], [ax, 0.33, sz + 0.11], null, [0.06, 0.035, 0.01]);
      }
    }
    P.walls(b, 0, 0.02, -0.28, 0.5, 0.22 + t * 0.05, 0.22, C.wall, { timber: true });
    P.gable(b, 0, 0.24 + t * 0.05, -0.28, 0.5, 0.22, 0.13, C.roofRed, C.wall, 'x');
    P.door(b, -0.1, 0.02, -0.17, 0, 0.08, 0.13);
    P.win(b, 0.1, 0.12, -0.17, 0, 0.1, 0.07, { box: true });
    P.aroundWindows(b, 0, 0.12, -0.28, 0.5, 0.22, [2, 1], { w: 0.06, h: 0.06 });
    b.add(G.bevel(0.2, 0.05, 0.012, 0.004), tint(C.woodDark, 0.3), [0, 0.24 + t * 0.05 - 0.035, -0.165], null, null, 'planks');
    b.add(G.sphLo, C.gold, [0, 0.24 + t * 0.05 - 0.035, -0.158], null, [0.03, 0.03, 0.006]);
    P.barrel(b, 0.4, 0, -0.1, 0.9); P.crate(b, -0.4, 0, -0.08, 0.08); P.lantern(b, 0.42, 0, 0.4); P.lantern(b, -0.42, 0, 0.4);
  };
  T.pethouse = function (b, t, ex) {
    // 宠物小屋：圆墙尖顶的童话屋（瓦片锥顶 + 顶上一颗心）、拱门、圆窗、白篱笆一圈、狗窝、食盆、球
    yard(b, 0.9, 0.9, 'grass', 0xb8e08a);
    P.plinth(b, 0, -0.05, 0.42, 0.42, 0.02);
    b.add(G.cyl, tint(0xfff0f6, 0.15), [0, 0.02 + 0.17, -0.05], null, [0.44, 0.34, 0.44], 'plaster');
    for (var bt = 0; bt < 10; bt++) { var ba = bt / 10 * Math.PI * 2; b.add(G.box, tint(0xc9a0b0, 0.4), [Math.cos(ba) * 0.222, 0.19, -0.05 + Math.sin(ba) * 0.222], [0, -ba, 0], [0.004, 0.34, 0.02], 'planks'); }
    b.add(G.cone, tint(0xe86a8a, 0.1), [0, 0.36 + 0.17, -0.05], null, [0.6, 0.36, 0.6], 'roof');
    for (var rr = 0; rr < 3; rr++) b.add(G.cyl, tint(0xc8506e, 0.1), [0, 0.38 + rr * 0.08, -0.05], null, [0.58 - rr * 0.13, 0.012, 0.58 - rr * 0.13], 'roof');
    b.add(G.sph, 0xff4d6d, [-0.016, 0.74, -0.05], [0, 0, 0.5], [0.04, 0.05, 0.03]); b.add(G.sph, 0xff4d6d, [0.016, 0.74, -0.05], [0, 0, -0.5], [0.04, 0.05, 0.03]);
    P.door(b, 0, 0.02, 0.17, 0, 0.1, 0.15, 0xe86a8a);
    for (var w = 0; w < 2; w++) { var wa = Math.PI / 2 + (w ? 0.9 : -0.9); var wx = Math.cos(wa) * 0.222, wz = -0.05 + Math.sin(wa) * 0.222;
      b.add(G.torus, 0xf3ead8, [wx, 0.25, wz], [0, -wa + Math.PI / 2, 0], [0.09, 0.09, 0.09]); b.add(G.cyl, 0x4f7fa8, [wx, 0.25, wz], [Math.PI / 2, -wa + Math.PI / 2, 0], [0.065, 0.01, 0.065]); }
    // 篱笆
    for (var fe = 0; fe < 4; fe++) { var a0 = fe * Math.PI / 2 + 0.35, a1 = a0 + Math.PI / 2 - 0.7; P.fence(b, Math.cos(a0) * 0.42, Math.sin(a0) * 0.42, Math.cos(a1) * 0.42, Math.sin(a1) * 0.42, 0.01, 3); }
    // 狗窝
    P.walls(b, 0.3, 0.02, 0.26, 0.12, 0.09, 0.12, 0xffd166, { tex: 'planks' });
    P.gable(b, 0.3, 0.11, 0.26, 0.12, 0.12, 0.06, 0x3f8f5b, 0xffd166, 'z');
    b.add(G.cyl, 0x3a2a1a, [0.3, 0.06, 0.321], [Math.PI / 2, 0, 0], [0.06, 0.004, 0.07]);
    b.add(G.cyl, 0x2f8fc8, [-0.25, 0.03, 0.3], null, [0.07, 0.03, 0.07]); b.add(G.cyl, 0xc8843c, [-0.25, 0.046, 0.3], null, [0.055, 0.004, 0.055]);
    b.add(G.sph, 0xff6b6b, [-0.32, 0.04, 0.18], null, [0.05, 0.05, 0.05]); b.add(G.torus, 0xffffff, [-0.32, 0.04, 0.18], [0.5, 0, 0], [0.065, 0.065, 0.05]);
  };
  T.lighthouse = function (b, t, ex) {
    // 灯塔：八角石基 + 红白相间的收分塔身 + 带栏杆的瞭望台 + 玻璃灯室 + 红圆顶；脚下一间看守小屋
    var lh = 0.9 + t * 0.15;
    yard(b, 0.88, 0.88, 'rock', 0xc9c3b8);
    b.add(G.taper8, tint(0xb9b2a6, 0.5), [0, 0.05, 0], null, [0.42, 0.1, 0.42], 'rock');
    var segs = 5, r0 = 0.17, r1 = 0.12;
    for (var i = 0; i < segs; i++) {
      var k0 = i / segs, k1 = (i + 1) / segs, ra = r0 + (r1 - r0) * k0, rb = r0 + (r1 - r0) * k1;
      var g = new THREE.CylinderGeometry(rb, ra, lh / segs, 16); b.add(g, i % 2 ? 0xd64545 : 0xf6f1e6, [0, 0.1 + lh * (k0 + k1) / 2, 0], null, null, 'plaster'); g.dispose();
    }
    for (var wv = 0; wv < 2; wv++) P.win(b, 0, 0.1 + lh * (0.35 + wv * 0.3), r0 - (r0 - r1) * (0.35 + wv * 0.3) + 0.004, 0, 0.035, 0.05);
    P.door(b, 0, 0.1, r0 + 0.004, 0, 0.06, 0.1, 0x4a6a8a);
    var ty = 0.1 + lh;
    b.add(G.cyl, 0x3a3a3a, [0, ty + 0.01, 0], null, [0.36, 0.025, 0.36], 'metal');
    for (var rl = 0; rl < 12; rl++) { var ra2 = rl / 12 * Math.PI * 2; b.add(G.cyl6, 0x3a3a3a, [Math.cos(ra2) * 0.17, ty + 0.05, Math.sin(ra2) * 0.17], null, [0.008, 0.07, 0.008], 'metal'); }
    b.add(G.torus, 0x3a3a3a, [0, ty + 0.085, 0], [Math.PI / 2, 0, 0], [0.43, 0.43, 0.1]);
    for (var gl = 0; gl < 6; gl++) { var ga = gl / 6 * Math.PI * 2; b.add(G.box, 0x2a2a2a, [Math.cos(ga) * 0.095, ty + 0.12, Math.sin(ga) * 0.095], [0, -ga, 0], [0.008, 0.16, 0.008], 'metal'); }
    b.add(G.sph, 0xd64545, [0, ty + 0.2, 0], null, [0.24, 0.16, 0.24], 'roof');
    b.add(G.cone, 0x3a3a3a, [0, ty + 0.3, 0], null, [0.025, 0.08, 0.025]);
    // 看守小屋
    P.walls(b, 0.28, 0.02, 0.24, 0.2, 0.14, 0.18, C.wall, { timber: true });
    P.gable(b, 0.28, 0.16, 0.24, 0.2, 0.18, 0.09, C.roofBlue, C.wall, 'x');
    P.door(b, 0.24, 0.02, 0.33, 0, 0.06, 0.1);
    P.win(b, 0.33, 0.1, 0.33, 0, 0.04, 0.04);
    P.aroundWindows(b, 0.28, 0.1, 0.24, 0.2, 0.18, [1, 1], { w: 0.04, h: 0.04 });
    P.bush(b, -0.32, 0, 0.3); P.bush(b, -0.3, 0, -0.3, 0.8);
    var lamp = new THREE.Mesh(new THREE.SphereGeometry(0.06, 12, 8), new THREE.MeshBasicMaterial({ color: 0xfff3a3 }));
    lamp.position.set(0, ty + 0.11, 0); ex(lamp, 'lamp');
    var beam = new THREE.Mesh(new THREE.ConeGeometry(0.25, 2.2, 12, 1, true), new THREE.MeshBasicMaterial({ color: 0xfff3a3, transparent: true, opacity: 0.18, depthWrite: false, side: THREE.DoubleSide }));
    beam.rotation.z = Math.PI / 2; beam.position.x = 1.1; var bg = new THREE.Group(); bg.add(beam); bg.position.set(0, ty + 0.11, 0); ex(bg, 'beam');
  };

  // 合并好的主体几何按「类型 + 外观档」缓存，同档的房子共用一份（geometry.userData.shared，city.js 拆模型时不 dispose）。
  // 真机实测：拼一座房子 40~85ms（桌面），手机上 70~140ms 一帧；自动经营时每次升级、新建都卡一下。
  // 命中缓存时仍要跑一遍 T[type]：会动的零件（风车叶、灯塔光束）是每座房子自己的 Mesh，主体的 add 交给空合并器丢掉。
  var cache = {}, NULLB = { add: function () { return this; } };
  function building(type, lv) {
    var parts = {}, grp = new THREE.Group(), tier = lv >= 15 ? 3 : lv >= 10 ? 2 : lv >= 5 ? 1 : 0;
    var key = type + ':' + tier, hit = cache[key], b = hit ? NULLB : newB(type, tier);
    var extra = function (obj, name) { grp.add(obj); parts[name] = obj; };
    (T[type] || T.farm)(b, tier, extra, parts);
    if (!hit) {
      if (tier >= 2) flag(b, 0.41, 0.0, -0.41, tier >= 3 ? C.gold : 0x4cc9f0);
      if (tier >= 3) { P.lantern(b, -0.42, 0, 0.42); b.add(G.torus, C.gold, [0.41, 0.03, -0.41], [Math.PI / 2, 0, 0], [0.08, 0.08, 0.08]); }
      hit = cache[key] = b.geometry(); hit.userData.shared = true;
    }
    var main = new THREE.Mesh(hit, M.MAT); main.castShadow = true; main.receiveShadow = true;
    grp.add(main); parts.main = main;
    if (!main.geometry.boundingBox) main.geometry.computeBoundingBox();
    parts.top = main.geometry.boundingBox.max.y;
    return { group: grp, parts: parts };
  }

  // 施工中：木脚手架（立柱 + 横杆 + 斜撑 + 踏板）围着半截墙，地上堆着木料、砖、桶
  var scafGeo = null;
  function scaffold() {
    if (scafGeo) { var sm = new THREE.Mesh(scafGeo, M.MAT); sm.castShadow = true; sm.receiveShadow = true; return sm; }
    var b = newB('scaffold', 1);
    yard(b, 0.86, 0.86, 'field', 0xb89a6a);
    P.plinth(b, 0, 0, 0.56, 0.48, 0.04);
    b.add(G.bevel(0.56, 0.16, 0.48, 0.01), tint(C.wall, 0.2), [0, 0.12, 0], null, null, 'plaster');
    for (var x = -1; x <= 1; x += 2) for (var z = -1; z <= 1; z += 2) b.add(G.cyl6, tint(C.woodDark), [x * 0.33, 0.33, z * 0.29], null, [0.024, 0.66, 0.024], 'bark');
    for (var y = 0; y < 3; y++) {
      var yy = 0.18 + y * 0.2;
      b.add(G.cyl6, tint(C.wood), [0, yy, 0.29], [0, 0, Math.PI / 2], [0.016, 0.7, 0.016], 'bark');
      b.add(G.cyl6, tint(C.wood), [0.33, yy, 0], [Math.PI / 2, 0, 0], [0.016, 0.62, 0.016], 'bark');
      b.add(G.bevel(0.62, 0.012, 0.07, 0.003), tint(0xc8a070, 0.3), [0, yy + 0.012, 0.33], null, null, 'planks');
    }
    b.add(G.cyl6, tint(C.wood), [0, 0.36, 0.3], [0, 0, 0.75], [0.014, 0.8, 0.014], 'bark');
    for (var p = 0; p < 4; p++) b.add(G.bevel(0.32, 0.02, 0.05, 0.004), tint(0xc8a070, 0.3), [0.25, 0.03 + p * 0.022, -0.36 + (p % 2) * 0.01], [0, p * 0.05, 0], null, 'planks');
    for (var br = 0; br < 6; br++) b.add(G.bevel(0.06, 0.03, 0.035, 0.004), tint(0xb5651d, 0.5), [-0.3 + (br % 3) * 0.065, 0.03 + Math.floor(br / 3) * 0.032, 0.38], null, null, 'brick');
    P.barrel(b, -0.38, 0, -0.3, 0.8);
    var m = mk(b); scafGeo = m.geometry; scafGeo.userData.shared = true; return m;
  }

  return { building: building, scaffold: scaffold };
}

module.exports = { make: make };
