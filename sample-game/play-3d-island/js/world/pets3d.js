// 十一只宠物的 3D 模型（Q 版：大头、圆身子、短腿），照着 AI 立绘的配色与特征拼：
// 柴犬的白嘴筒与卷尾、橘猫的虎斑、熊猫的黑眼圈、狐狸的白尾尖、独角兽的彩虹鬃毛、小金龙的翅膀和背刺……
// 每个零件带 aRig（见 characters.js）：腿 3/4 交替摆、尾巴 5 摇、头 6 点头、翅膀 7 扇；高约 0.3（村民 0.44），正面 +z。
function make(M) {
  var G = M.G, THREE = M.THREE;
  var BLACK = 0x1a1210, WHITE = 0xffffff, PINK = 0xff9fb4, NOSE = 0x2a1a1a;

  function darker(c, k) { return new THREE.Color(c).multiplyScalar(k).getHex(); }
  // 眼睛：黑亮的椭圆 + 大小两颗高光（「有神」全靠这两颗白点）
  function eyes(b, y, z, gap, s, H, col) {
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.sphLo, col || BLACK, [e * gap, y, z], null, [s, s * 1.2, s * 0.6], null, H);
      b.add(G.sphMi, WHITE, [e * gap + s * 0.22, y + s * 0.3, z + s * 0.28], null, [s * 0.36, s * 0.36, s * 0.15], null, H);
      b.add(G.sphMi, WHITE, [e * gap - s * 0.2, y - s * 0.28, z + s * 0.3], null, [s * 0.16, s * 0.16, s * 0.08], null, H);
    }
  }
  function blush(b, y, z, gap, H) { for (var e = -1; e <= 1; e += 2) b.add(G.sphMi, PINK, [e * gap, y, z], null, [0.03, 0.016, 0.01], null, H); }
  // 四条腿：左前 + 右后一组（3），右前 + 左后一组（4）；hy = 髋高，len = 腿长
  function legs4(b, col, sx, zf, zb, hy, len, r, foot) {
    [[-sx, zf, 3], [sx, zb, 3], [sx, zf, 4], [-sx, zb, 4]].forEach(function (L) {
      var rig = [L[2], hy, L[1], 0];
      b.add(G.cyl, col, [L[0], hy - len / 2, L[1]], null, [r, len, r], null, rig);
      b.add(G.sph, foot || col, [L[0], 0.016, L[1] + r * 0.15], null, [r * 1.15, r * 0.75, r * 1.3], null, rig);
    });
  }
  // 标准四足：身体椭球 + 头（大）+ 腿；返回头的参数，物种再往上加耳朵、口鼻、尾巴
  function quad(b, o) {
    var body = o.body, hy = o.hy || 0.1;
    legs4(b, o.leg || body, o.lx || 0.045, 0.04, -0.045, hy, hy - 0.01, o.lr || 0.04, o.foot);
    b.add(G.sph, body, [0, hy + 0.035, -0.005], null, [o.bw || 0.15, o.bh || 0.12, o.bl || 0.19], null, [0, 0, 0, 0]);
    if (o.belly) b.add(G.sph, o.belly, [0, hy + 0.02, 0.02], null, [(o.bw || 0.15) * 0.75, (o.bh || 0.12) * 0.7, (o.bl || 0.19) * 0.7], null, [0, 0, 0, 0]);
    var H = [6, hy + 0.07, 0.05, 0], hr = o.hr || 0.17, hcy = hy + 0.07 + hr * 0.5, hcz = 0.075;
    b.add(G.sph, o.head || body, [0, hcy, hcz], null, [hr, hr * 0.92, hr * 0.9], null, H);
    return { H: H, y: hcy, z: hcz, r: hr / 2 };
  }

  var S = {};
  S.chick = function (b, d) {
    // 小鸡：一颗圆滚滚的黄团子 + 呆毛 + 小翅膀 + 橙色嘴和脚
    var H = [6, 0.12, 0, 0];
    [[-0.03, 3], [0.03, 4]].forEach(function (L) {
      b.add(G.cyl, d.ear, [L[0], 0.04, 0.0], null, [0.012, 0.06, 0.012], null, [L[1], 0.07, 0, 0]);
      for (var t = -1; t <= 1; t++) b.add(G.box, d.ear, [L[0] + t * 0.012, 0.006, 0.018], [0, t * 0.5, 0], [0.008, 0.008, 0.035], null, [L[1], 0.07, 0, 0]);
    });
    b.add(G.sph, d.body, [0, 0.14, 0], null, [0.22, 0.21, 0.21], null, [0, 0, 0, 0]);
    b.add(G.sph, darker(d.body, 1.04), [0, 0.12, 0.03], null, [0.16, 0.14, 0.16], null, [0, 0, 0, 0]);
    for (var w = -1; w <= 1; w += 2) b.add(G.sph, darker(d.body, 0.9), [w * 0.105, 0.13, -0.01], [0, 0, w * 0.4], [0.035, 0.09, 0.08], null, [7, 0.16, w * 0.09, 0]);
    for (var f = 0; f < 3; f++) b.add(G.cone, darker(d.body, 0.92), [(f - 1) * 0.016, 0.26, 0.0], [0, 0, (f - 1) * 0.4], [0.02, 0.05, 0.02], null, H);
    b.add(G.cone, d.ear, [0, 0.15, 0.115], [Math.PI / 2, 0, 0], [0.04, 0.04, 0.025], null, H);
    eyes(b, 0.18, 0.093, 0.04, 0.026, H); blush(b, 0.15, 0.095, 0.065, H);
    b.add(G.sph, darker(d.body, 0.9), [0, 0.11, -0.105], null, [0.06, 0.05, 0.04], null, [5, 0.12, -0.1, 0]);
  };
  S.bunny = function (b, d) {
    var q = quad(b, { body: d.body, hy: 0.07, lx: 0.04, lr: 0.035, bw: 0.15, bh: 0.13, bl: 0.17, hr: 0.18, foot: 0xfff0f0 });
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.sph, d.body, [e * 0.035, q.y + 0.13, q.z - 0.02], [-0.15, 0, e * -0.15], [0.05, 0.17, 0.035], null, q.H);
      b.add(G.sph, d.ear, [e * 0.035, q.y + 0.13, q.z - 0.004], [-0.15, 0, e * -0.15], [0.026, 0.13, 0.01], null, q.H);
    }
    eyes(b, q.y + 0.01, q.z + 0.074, 0.042, 0.022, q.H); blush(b, q.y - 0.02, q.z + 0.07, 0.06, q.H);
    b.add(G.sphMi, 0xff8fab, [0, q.y - 0.015, q.z + 0.083], null, [0.018, 0.012, 0.01], null, q.H);
    b.add(G.sph, WHITE, [0, 0.12, -0.1], null, [0.065, 0.065, 0.06], null, [5, 0.12, -0.09, 0]);
  };
  S.pup = function (b, d) {
    // 柴犬：橘色 + 白嘴筒白胸白袜、竖三角耳（内侧深色）、背上一个卷尾巴
    var q = quad(b, { body: d.body, belly: 0xfff6ea, hy: 0.09, lx: 0.045, lr: 0.038, bw: 0.15, bh: 0.12, bl: 0.2, hr: 0.17, foot: 0xfff6ea });
    b.add(G.sph, 0xfff6ea, [0, q.y - 0.03, q.z + 0.055], null, [0.1, 0.07, 0.08], null, q.H);
    b.add(G.sph, 0xfff6ea, [0, q.y - 0.01, q.z + 0.04], null, [0.15, 0.08, 0.1], null, q.H);
    b.add(G.sph, NOSE, [0, q.y - 0.018, q.z + 0.095], null, [0.026, 0.018, 0.016], null, q.H);
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.cone4, d.body, [e * 0.05, q.y + 0.085, q.z - 0.01], [0, Math.PI / 4, e * -0.25], [0.06, 0.07, 0.04], null, q.H);
      b.add(G.cone4, d.ear, [e * 0.05, q.y + 0.08, q.z + 0.003], [0, Math.PI / 4, e * -0.25], [0.035, 0.05, 0.02], null, q.H);
    }
    eyes(b, q.y + 0.018, q.z + 0.07, 0.042, 0.02, q.H); blush(b, q.y - 0.01, q.z + 0.07, 0.068, q.H);
    var T = [5, 0.18, -0.09, 0];
    b.add(G.torus, d.body, [0, 0.225, -0.09], [0, Math.PI / 2, 0], [0.11, 0.11, 0.11], null, T);
    b.add(G.sph, 0xfff6ea, [0, 0.27, -0.07], null, [0.04, 0.04, 0.04], null, T);
  };
  S.piggy = function (b, d) {
    var q = quad(b, { body: d.body, hy: 0.075, lx: 0.05, lr: 0.04, bw: 0.18, bh: 0.15, bl: 0.2, hr: 0.18, foot: 0xc87a8a });
    b.add(G.cyl, d.ear, [0, q.y - 0.015, q.z + 0.085], [Math.PI / 2, 0, 0], [0.065, 0.03, 0.05], null, q.H);
    for (var n = -1; n <= 1; n += 2) b.add(G.sphMi, 0x9a4a5a, [n * 0.012, q.y - 0.015, q.z + 0.1], null, [0.01, 0.014, 0.004], null, q.H);
    for (var e = -1; e <= 1; e += 2) b.add(G.cone4, d.ear, [e * 0.06, q.y + 0.07, q.z], [0.5, Math.PI / 4, e * -0.5], [0.05, 0.05, 0.03], null, q.H);
    eyes(b, q.y + 0.025, q.z + 0.07, 0.045, 0.019, q.H); blush(b, q.y - 0.005, q.z + 0.068, 0.07, q.H);
    b.add(G.torus, d.ear, [0, 0.14, -0.105], [0, Math.PI / 2, 0], [0.04, 0.04, 0.05], null, [5, 0.14, -0.1, 0]);
  };
  S.cat = function (b, d) {
    // 橘猫：虎斑条纹、尖耳、白嘴筒和胡须、一条翘起来的长尾巴
    var q = quad(b, { body: d.body, belly: 0xfff3e0, hy: 0.09, lx: 0.042, lr: 0.035, bw: 0.14, bh: 0.12, bl: 0.2, hr: 0.17, foot: 0xfff3e0 });
    for (var s = 0; s < 3; s++) b.add(G.sph, d.ear, [0, 0.17, -0.06 + s * 0.045], null, [0.15, 0.03, 0.018], null, [0, 0, 0, 0]);
    for (var hs = 0; hs < 3; hs++) b.add(G.sph, d.ear, [(hs - 1) * 0.022, q.y + 0.075, q.z + 0.02], [0, 0, (hs - 1) * 0.2], [0.012, 0.03, 0.04], null, q.H);
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.cone4, d.body, [e * 0.055, q.y + 0.08, q.z - 0.005], [0, Math.PI / 4, e * -0.3], [0.06, 0.07, 0.04], null, q.H);
      b.add(G.cone4, 0xffb8a8, [e * 0.055, q.y + 0.076, q.z + 0.007], [0, Math.PI / 4, e * -0.3], [0.032, 0.045, 0.02], null, q.H);
      for (var wk = 0; wk < 2; wk++) b.add(G.box, 0x6a4a3a, [e * 0.075, q.y - 0.02 + wk * 0.012, q.z + 0.075], [0, 0, e * (wk ? 0.15 : -0.1)], [0.06, 0.003, 0.002], null, q.H);
    }
    b.add(G.sph, 0xfff3e0, [0, q.y - 0.025, q.z + 0.06], null, [0.08, 0.05, 0.05], null, q.H);
    b.add(G.sphMi, 0xff8fab, [0, q.y - 0.008, q.z + 0.085], null, [0.016, 0.01, 0.01], null, q.H);
    eyes(b, q.y + 0.015, q.z + 0.072, 0.042, 0.022, q.H, 0x2a4a1a);
    var T = [5, 0.13, -0.1, 0];
    for (var t = 0; t < 4; t++) b.add(G.cyl, t === 3 ? d.ear : d.body, [0, 0.13 + t * 0.045, -0.12 - Math.sin(t * 0.5) * 0.03], [-0.4 + t * 0.25, 0, 0], [0.032, 0.06, 0.032], null, T);
  };
  S.panda = function (b, d) {
    var q = quad(b, { body: WHITE, leg: BLACK, hy: 0.08, lx: 0.05, lr: 0.045, bw: 0.18, bh: 0.15, bl: 0.2, hr: 0.19 });
    b.add(G.sph, BLACK, [0, 0.13, 0.03], null, [0.185, 0.07, 0.07], null, [0, 0, 0, 0]);           // 黑肩带
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.sph, BLACK, [e * 0.065, q.y + 0.08, q.z - 0.01], null, [0.055, 0.055, 0.04], null, q.H);
      b.add(G.sph, BLACK, [e * 0.04, q.y + 0.012, q.z + 0.07], [0, 0, e * 0.6], [0.045, 0.06, 0.02], null, q.H);
    }
    eyes(b, q.y + 0.015, q.z + 0.082, 0.04, 0.016, q.H, 0x3a2a20);
    b.add(G.sph, 0xf6f0ea, [0, q.y - 0.03, q.z + 0.07], null, [0.06, 0.04, 0.04], null, q.H);
    b.add(G.sph, NOSE, [0, q.y - 0.02, q.z + 0.09], null, [0.022, 0.014, 0.012], null, q.H);
    blush(b, q.y - 0.03, q.z + 0.072, 0.07, q.H);
    b.add(G.sph, WHITE, [0, 0.12, -0.105], null, [0.04, 0.04, 0.03], null, [5, 0.12, -0.1, 0]);
  };
  S.frog = function (b, d) {
    // 青蛙：扁胖的身子、头顶两颗大眼泡、宽嘴、浅色肚皮、蹲着的后腿
    var belly = 0xe8f5b0, H = [6, 0.08, 0.02, 0];
    [[-0.05, 0.05, 3], [0.05, 0.05, 4]].forEach(function (L) { b.add(G.sph, d.body, [L[0], 0.03, L[1]], null, [0.04, 0.06, 0.04], null, [L[2], 0.06, L[1], 0]); });
    for (var k = -1; k <= 1; k += 2) {
      b.add(G.sph, d.ear, [k * 0.085, 0.05, -0.04], null, [0.07, 0.07, 0.11], null, [k < 0 ? 4 : 3, 0.08, -0.04, 0]);
      b.add(G.sph, d.ear, [k * 0.1, 0.012, 0.0], null, [0.05, 0.02, 0.08], null, [k < 0 ? 4 : 3, 0.08, -0.04, 0]);
    }
    b.add(G.sph, d.body, [0, 0.09, 0], null, [0.22, 0.16, 0.22], null, [0, 0, 0, 0]);
    b.add(G.sph, belly, [0, 0.07, 0.05], null, [0.16, 0.1, 0.13], null, [0, 0, 0, 0]);
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.sph, d.body, [e * 0.055, 0.17, 0.04], null, [0.075, 0.07, 0.07], null, H);
      b.add(G.sph, WHITE, [e * 0.055, 0.175, 0.065], null, [0.05, 0.05, 0.03], null, H);
    }
    eyes(b, 0.177, 0.082, 0.055, 0.02, H);
    b.add(G.halfTorus, 0x3a5a10, [0, 0.11, 0.095], [Math.PI, 0, 0], [0.09, 0.05, 0.03], null, H);
    blush(b, 0.12, 0.1, 0.08, H);
  };
  S.fox = function (b, d) {
    // 狐狸：橙红 + 白胸白嘴、黑袜、大尖耳（黑耳尖）、一条蓬松的大尾巴（白尾尖）
    var q = quad(b, { body: d.body, belly: WHITE, leg: 0x2a1a14, hy: 0.1, lx: 0.042, lr: 0.032, bw: 0.14, bh: 0.11, bl: 0.2, hr: 0.17 });
    b.add(G.sph, WHITE, [0, q.y - 0.03, q.z + 0.055], [0.2, 0, 0], [0.13, 0.07, 0.11], null, q.H);
    b.add(G.cone, d.body, [0, q.y - 0.012, q.z + 0.08], [Math.PI / 2, 0, 0], [0.07, 0.07, 0.05], null, q.H);
    b.add(G.sph, NOSE, [0, q.y - 0.012, q.z + 0.118], null, [0.02, 0.016, 0.014], null, q.H);
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.cone4, d.body, [e * 0.055, q.y + 0.1, q.z - 0.01], [0, Math.PI / 4, e * -0.25], [0.07, 0.1, 0.04], null, q.H);
      b.add(G.cone4, 0x2a1a14, [e * 0.067, q.y + 0.14, q.z - 0.01], [0, Math.PI / 4, e * -0.25], [0.03, 0.03, 0.03], null, q.H);
      b.add(G.cone4, WHITE, [e * 0.055, q.y + 0.095, q.z + 0.002], [0, Math.PI / 4, e * -0.25], [0.04, 0.065, 0.02], null, q.H);
    }
    eyes(b, q.y + 0.02, q.z + 0.068, 0.042, 0.02, q.H, 0x3a1a0a);
    var T = [5, 0.13, -0.1, 0];
    b.add(G.sph, d.body, [0, 0.16, -0.17], [-0.7, 0, 0], [0.1, 0.1, 0.2], null, T);
    b.add(G.sph, WHITE, [0, 0.23, -0.24], [-0.7, 0, 0], [0.07, 0.07, 0.08], null, T);
  };
  S.owl = function (b, d) {
    // 猫头鹰：蛋形身子、浅色胸口带鳞纹、两只大眼盘、耳羽、小钩嘴、翅膀能扇
    var H = [6, 0.17, 0, 0], face = d.ear;
    [[-0.035, 3], [0.035, 4]].forEach(function (L) { for (var t = -1; t <= 1; t++) b.add(G.sph, 0xf2a93b, [L[0] + t * 0.012, 0.01, 0.03], null, [0.012, 0.012, 0.03], null, [L[1], 0.05, 0, 0]); });
    b.add(G.sph, d.body, [0, 0.13, 0], null, [0.2, 0.22, 0.19], null, [0, 0, 0, 0]);
    b.add(G.sph, face, [0, 0.11, 0.035], null, [0.14, 0.15, 0.14], null, [0, 0, 0, 0]);
    for (var sc = 0; sc < 6; sc++) b.add(G.sphMi, darker(face, 0.8), [((sc % 3) - 1) * 0.03, 0.08 + Math.floor(sc / 3) * 0.035, 0.103], null, [0.016, 0.01, 0.006], null, [0, 0, 0, 0]);
    for (var w = -1; w <= 1; w += 2) b.add(G.sph, darker(d.body, 0.82), [w * 0.1, 0.13, -0.01], [0, 0, w * 0.15], [0.04, 0.14, 0.12], null, [7, 0.2, w * 0.09, 0]);
    b.add(G.sph, d.body, [0, 0.25, 0.0], null, [0.19, 0.15, 0.17], null, H);
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.cyl, face, [e * 0.042, 0.255, 0.072], [Math.PI / 2, 0, 0], [0.075, 0.012, 0.075], null, H);
      b.add(G.cone, darker(d.body, 0.75), [e * 0.06, 0.33, 0.0], [0, 0, e * -0.4], [0.035, 0.07, 0.03], null, H);
    }
    eyes(b, 0.256, 0.08, 0.042, 0.026, H, 0x2a1a0a);
    for (var r = -1; r <= 1; r += 2) b.add(G.torus, 0xf2a93b, [r * 0.042, 0.256, 0.079], null, [0.07, 0.07, 0.05], null, H);
    b.add(G.cone, 0xf2a93b, [0, 0.23, 0.093], [Math.PI * 0.85, 0, 0], [0.025, 0.04, 0.02], null, H);
    b.add(G.cone, darker(d.body, 0.8), [0, 0.06, -0.1], [-2.2, 0, 0], [0.08, 0.06, 0.03], null, [0, 0, 0, 0]);
  };
  S.unicorn = function (b, d) {
    // 独角兽：白马（长腿、长脸）、金色螺旋角、彩虹鬃毛和尾巴、金蹄
    var q = quad(b, { body: d.body, hy: 0.13, lx: 0.042, lr: 0.035, bw: 0.15, bh: 0.13, bl: 0.22, hr: 0.16, foot: 0xffd166 });
    b.add(G.sph, d.body, [0, q.y - 0.025, q.z + 0.065], null, [0.1, 0.08, 0.1], null, q.H);
    for (var n = -1; n <= 1; n += 2) b.add(G.sphMi, 0xd9a0c0, [n * 0.02, q.y - 0.025, q.z + 0.112], null, [0.012, 0.01, 0.006], null, q.H);
    b.add(G.cone, 0xffd166, [0, q.y + 0.12, q.z + 0.03], [0.3, 0, 0], [0.032, 0.11, 0.032], null, q.H);
    for (var rg = 0; rg < 3; rg++) b.add(G.torus, 0xffe8a0, [0, q.y + 0.09 + rg * 0.025, q.z + 0.022 + rg * 0.008], [Math.PI / 2 + 0.3, 0, 0], [0.04 - rg * 0.01, 0.04 - rg * 0.01, 0.03], null, q.H);
    for (var e = -1; e <= 1; e += 2) b.add(G.cone, d.body, [e * 0.05, q.y + 0.075, q.z - 0.03], [0, 0, e * -0.3], [0.03, 0.06, 0.025], null, q.H);
    eyes(b, q.y + 0.01, q.z + 0.068, 0.045, 0.024, q.H, 0x4a2a6a); blush(b, q.y - 0.02, q.z + 0.07, 0.065, q.H);
    var RAIN = [0xff6b9a, 0xffb84d, 0xffe066, 0x7ed6a5, 0x6ab8ff, 0xb388ff];
    for (var m = 0; m < 6; m++) b.add(G.sph, RAIN[m], [0, q.y + 0.06 - m * 0.025, q.z - 0.065 - m * 0.012], null, [0.05, 0.05, 0.05], null, m < 3 ? q.H : [0, 0, 0, 0]);
    for (var t = 0; t < 4; t++) b.add(G.sph, RAIN[(t * 2) % 6], [0, 0.17 - t * 0.03, -0.13 - t * 0.02], [-0.6, 0, 0], [0.05, 0.05, 0.08], null, [5, 0.18, -0.11, 0]);
  };
  S.dragon = function (b, d) {
    // 小金龙：金色身子 + 浅色肚皮横纹、两只犄角、粉色背刺、小蝠翼会扇、长尾巴带尖
    var q = quad(b, { body: d.body, belly: 0xfff3b0, hy: 0.09, lx: 0.048, lr: 0.04, bw: 0.16, bh: 0.13, bl: 0.2, hr: 0.18 });
    for (var bs = 0; bs < 3; bs++) b.add(G.box, darker(0xfff3b0, 0.9), [0, 0.07 + bs * 0.025, 0.08], null, [0.07, 0.004, 0.01], null, [0, 0, 0, 0]);
    b.add(G.sph, d.body, [0, q.y - 0.025, q.z + 0.07], null, [0.11, 0.075, 0.09], null, q.H);
    for (var n = -1; n <= 1; n += 2) b.add(G.sphMi, 0x8a5a10, [n * 0.022, q.y - 0.015, q.z + 0.112], null, [0.01, 0.008, 0.005], null, q.H);
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.cone, 0xfff3d0, [e * 0.045, q.y + 0.1, q.z - 0.03], [-0.4, 0, e * -0.3], [0.025, 0.08, 0.025], null, q.H);
      b.add(G.cone4, d.ear, [e * 0.08, q.y + 0.04, q.z - 0.02], [0, 0, e * -1.2], [0.03, 0.05, 0.02], null, q.H);
    }
    eyes(b, q.y + 0.02, q.z + 0.075, 0.045, 0.024, q.H, 0x5a1a0a); blush(b, q.y - 0.015, q.z + 0.078, 0.07, q.H);
    for (var sp = 0; sp < 4; sp++) b.add(G.cone4, d.ear, [0, 0.19 - sp * 0.012, 0.03 - sp * 0.05], [-0.3, Math.PI / 4, 0], [0.03, 0.045, 0.03], null, [0, 0, 0, 0]);
    for (var w = -1; w <= 1; w += 2) {
      var Wg = [7, 0.18, w * 0.06, 0];
      b.add(G.cyl6, 0xd99a00, [w * 0.1, 0.22, -0.02], [0, 0, w * -0.9], [0.012, 0.1, 0.012], null, Wg);
      b.add(G.sph, d.ear, [w * 0.11, 0.2, -0.03], [0, 0, w * -0.5], [0.09, 0.1, 0.012], null, Wg);
    }
    var T = [5, 0.11, -0.1, 0];
    for (var t = 0; t < 4; t++) b.add(G.cyl, d.body, [0, 0.09 - t * 0.012, -0.12 - t * 0.04], [-1.3, 0, 0], [0.05 - t * 0.009, 0.06, 0.05 - t * 0.009], null, T);
    b.add(G.cone4, d.ear, [0, 0.05, -0.29], [-1.6, 0, 0], [0.04, 0.05, 0.02], null, T);
  };

  function build(def) {
    var b = new M.Builder();
    b.add(G.box, 0xffffff, [0, -1, 0], null, [0.0001, 0.0001, 0.0001], null, [0, 0, 0, 0]);   // 第一件就带 rig：整份几何都有 aRig
    (S[def.id] || S.pup)(b, def);
    return b.geometry();
  }
  return { build: build };
}

module.exports = { make: make };
