// 角色：村民（一群人一次 draw call）与宠物（一只一次），四肢会摆、尾巴会摇、翅膀会扇。
//
// 为什么用顶点着色器摆四肢而不是一节节网格：村民 10 个 × 头身手脚 5 节 = 50 次 draw call，本引擎每次 draw call
// 都要过一次 JS → 原生，手机上这一项就要几毫秒。现在一个村民 = InstancedMesh 里的一个实例：
// - 每个顶点带 aRig = (肢体号, 轴心 y, 轴心 z 或 x, 换色槽)；
// - 每个实例带 iWalk = (步伐相位, 摆幅, 尾巴/翅膀相位, 抬头角) 与四个换色（衣服 / 裤子 / 头发 / 帽子，帽子 a=0 就不显示）；
// - 着色器里按肢体号绕轴心转（手臂、腿绕 x 前后摆，尾巴绕 y，翅膀绕 z），法线一起转。
// 宠物种类各不相同、几何不同，所以一只一个 InstancedMesh（count = 1），借同一套着色器摆腿。
// 角色不投阴影贴图（深度材质里没有这套摆动，影子会对不上），脚下统一画一块软的圆形影子（也是一个 InstancedMesh）。
var Pets3D = require('./pets3d.js');

var RIG_HEAD = [
  'attribute vec4 aRig; attribute vec4 iWalk; attribute vec3 iC1; attribute vec3 iC2; attribute vec3 iC3; attribute vec4 iC4;',
  'mat3 rigX(float a) { float c = cos(a), s = sin(a); return mat3(1.0, 0.0, 0.0, 0.0, c, s, 0.0, -s, c); }',
  'mat3 rigY(float a) { float c = cos(a), s = sin(a); return mat3(c, 0.0, -s, 0.0, 1.0, 0.0, s, 0.0, c); }',
  'mat3 rigZ(float a) { float c = cos(a), s = sin(a); return mat3(c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0); }'
].join('\n');
// 肢体号：1 左臂 2 右臂（反相）3 左腿 4 右腿（反相）5 尾巴 6 头 7 翅膀（轴心 x 存在 aRig.z，符号 = 哪一边）8 耳朵（随头 + 抖）
var RIG_NORMAL = [
  'float rl = aRig.x, ph = iWalk.x, am = iWalk.y; mat3 R = mat3(1.0); vec3 piv = vec3(0.0, aRig.y, aRig.z);',
  'if (rl > 0.5 && rl < 1.5) R = rigX(sin(ph) * am * 0.9);',
  'else if (rl > 1.5 && rl < 2.5) R = rigX(-sin(ph) * am * 0.9);',
  'else if (rl > 2.5 && rl < 3.5) R = rigX(-sin(ph) * am * 0.75);',
  'else if (rl > 3.5 && rl < 4.5) R = rigX(sin(ph) * am * 0.75);',
  'else if (rl > 4.5 && rl < 5.5) R = rigY(sin(iWalk.z) * 0.45);',
  'else if (rl > 5.5 && rl < 6.5) R = rigX(iWalk.w + sin(ph * 2.0) * am * 0.06);',
  'else if (rl > 6.5 && rl < 7.5) { float sx = sign(aRig.z); R = rigZ(sx * (0.2 + sin(iWalk.z * 1.7) * 0.35)); piv = vec3(aRig.z, aRig.y, 0.0); }',
  'else if (rl > 7.5 && rl < 8.5) R = rigX(iWalk.w) * rigZ(sin(iWalk.z * 0.9) * 0.08);',
  'vec3 objectNormal = R * vec3(normal);',
  '#ifdef USE_TANGENT', 'vec3 objectTangent = vec3(tangent.xyz);', '#endif'
].join('\n');
var RIG_BEGIN = [
  'vec3 transformed = R * (position - piv) + piv;',
  'if (aRig.w > 3.5 && iC4.a < 0.5) transformed = vec3(0.0, -10.0, 0.0);'
].join('\n');
var RIG_COLOR = [
  '#include <color_vertex>',
  'float sl = aRig.w;',
  'if (sl > 0.5 && sl < 1.5) vColor.xyz *= iC1;',
  'else if (sl > 1.5 && sl < 2.5) vColor.xyz *= iC2;',
  'else if (sl > 2.5 && sl < 3.5) vColor.xyz *= iC3;',
  'else if (sl > 3.5) vColor.xyz *= iC4.rgb;'
].join('\n');

function make(M) {
  var THREE = M.THREE, G = M.G;
  // 角色材质：Phong（有一点高光，皮毛、眼睛不再是哑光纸片）+ 顶点色 + 骨骼摆动
  function rigMaterial() {
    var m = new THREE.MeshPhongMaterial({ vertexColors: true, shininess: 28, specular: 0x2a2a2a });
    m.onBeforeCompile = function (sh) {
      sh.vertexShader = sh.vertexShader
        .replace('#include <common>', '#include <common>\n' + RIG_HEAD)
        .replace('#include <beginnormal_vertex>', RIG_NORMAL)
        .replace('#include <begin_vertex>', RIG_BEGIN)
        .replace('#include <color_vertex>', RIG_COLOR);
    };
    m.customProgramCacheKey = function () { return 'rig1'; };
    return m;
  }
  var RIG = rigMaterial();
  // 给一个几何配上实例属性（count 个）
  function instAttrs(geo, count) {
    var A = {
      iWalk: new THREE.InstancedBufferAttribute(new Float32Array(count * 4), 4),
      iC1: new THREE.InstancedBufferAttribute(new Float32Array(count * 3).fill(1), 3),
      iC2: new THREE.InstancedBufferAttribute(new Float32Array(count * 3).fill(1), 3),
      iC3: new THREE.InstancedBufferAttribute(new Float32Array(count * 3).fill(1), 3),
      iC4: new THREE.InstancedBufferAttribute(new Float32Array(count * 4).fill(1), 4)
    };
    for (var k in A) { A[k].setUsage(THREE.DynamicDrawUsage); geo.setAttribute(k, A[k]); }
    return A;
  }

  // ───────── 村民（Q 版两头身）：头 0.19、身 0.12、腿 0.1；正面 +z ─────────
  var SKIN = 0xffd9b8, EYE = 0x2a1a12;
  function villagerGeo() {
    var b = new M.Builder();
    var leg = function (sx, limb) {
      b.add(G.cyl, 0xffffff, [sx, 0.075, 0], null, [0.05, 0.1, 0.05], null, [limb, 0.13, 0, 2]);
      b.add(G.sph, 0x5a3a24, [sx, 0.022, 0.012], null, [0.056, 0.04, 0.075], null, [limb, 0.13, 0, 0]);
    };
    leg(-0.032, 3); leg(0.032, 4);
    b.add(G.cyl, 0xffffff, [0, 0.135, 0], null, [0.12, 0.035, 0.11], null, [0, 0, 0, 2]);                          // 裤腰
    b.add(G.taper, 0xffffff, [0, 0.195, 0], null, [0.115, 0.1, 0.1], null, [0, 0, 0, 1]);                          // 上衣
    b.add(G.sph, 0xffffff, [0, 0.245, 0], null, [0.1, 0.035, 0.085], null, [0, 0, 0, 1]);                          // 肩
    b.add(G.box, 0xffffff, [0, 0.19, 0.051], null, [0.012, 0.09, 0.004], null, [0, 0, 0, 2]);                       // 门襟
    var arm = function (sx, limb) {
      var k = sx > 0 ? 1 : -1;
      b.add(G.cyl, 0xffffff, [sx, 0.2, 0], [0, 0, k * 0.18], [0.036, 0.09, 0.036], null, [limb, 0.24, 0, 1]);
      b.add(G.sph, SKIN, [sx + k * 0.01, 0.148, 0], null, [0.036, 0.036, 0.036], null, [limb, 0.24, 0, 0]);
    };
    arm(-0.068, 1); arm(0.068, 2);
    var H = [6, 0.25, 0, 0];
    b.add(G.cyl, SKIN, [0, 0.26, 0], null, [0.04, 0.03, 0.04], null, H);
    b.add(G.sph, SKIN, [0, 0.335, 0], null, [0.19, 0.175, 0.18], null, H);
    for (var e = -1; e <= 1; e += 2) {
      b.add(G.sph, EYE, [e * 0.032, 0.333, 0.083], null, [0.022, 0.032, 0.012], null, H);
      b.add(G.sphMi, 0xffffff, [e * 0.032 + 0.006, 0.341, 0.089], null, [0.008, 0.009, 0.004], null, H);
      b.add(G.sphMi, 0xff9a9a, [e * 0.055, 0.31, 0.074], null, [0.028, 0.014, 0.008], null, H);
      b.add(G.sphMi, SKIN, [e * 0.092, 0.33, 0], null, [0.022, 0.034, 0.02], null, H);                              // 耳朵
      b.add(G.box, EYE, [e * 0.033, 0.362, 0.082], [0, 0, e * -0.15], [0.026, 0.005, 0.004], null, H);              // 眉毛
    }
    b.add(G.sphMi, 0xf2b896, [0, 0.318, 0.089], null, [0.014, 0.012, 0.01], null, H);                               // 鼻子
    b.add(G.box, 0xb85a4a, [0, 0.295, 0.084], null, [0.022, 0.005, 0.004], null, H);                                 // 嘴
    var HR = [6, 0.25, 0, 3];
    b.add(G.hemi, 0xffffff, [0, 0.346, -0.006], [-0.15, 0, 0], [0.2, 0.15, 0.195], null, HR);                       // 头发顶
    b.add(G.sph, 0xffffff, [0, 0.33, -0.035], null, [0.185, 0.16, 0.13], null, HR);                                  // 后脑
    b.add(G.sph, 0xffffff, [0.03, 0.385, 0.06], [0.3, 0, -0.4], [0.09, 0.03, 0.06], null, HR);                       // 刘海
    var HT = [6, 0.25, 0, 4];
    b.add(G.cyl, 0xffffff, [0, 0.41, 0], [-0.08, 0, 0], [0.27, 0.012, 0.26], null, HT);                             // 草帽檐
    b.add(G.cyl, 0xffffff, [0, 0.44, -0.004], [-0.08, 0, 0], [0.13, 0.055, 0.13], null, HT);
    return b.geometry();
  }
  var SHIRTS = [0xe63946, 0x457b9d, 0x2a9d8f, 0xf4a261, 0x9d4edd, 0xffb703, 0x6ab04c, 0xf78fb3], PANTS = [0x3d5a80, 0x5c4033, 0x2b2d42, 0x6b705c],
    HAIR = [0x3b2a20, 0x6b4226, 0x1e1e1e, 0xd9a05b, 0x8a3b12], HAT = [0xf1d8a0, 0xe8c070, 0x9ad0ec];
  // 一群村民：crowd(n) → { mesh, set(i, x, y, z, yaw, phase, amp, look) }
  function crowd(n) {
    var geo = villagerGeo(), A = instAttrs(geo, n), mesh = new THREE.InstancedMesh(geo, RIG, n), c = new THREE.Color();
    mesh.frustumCulled = false; mesh.count = 0;
    for (var i = 0; i < n; i++) {
      c.set(SHIRTS[i % SHIRTS.length]); A.iC1.setXYZ(i, c.r, c.g, c.b);
      c.set(PANTS[(i * 3) % PANTS.length]); A.iC2.setXYZ(i, c.r, c.g, c.b);
      c.set(HAIR[(i * 2 + 1) % HAIR.length]); A.iC3.setXYZ(i, c.r, c.g, c.b);
      c.set(HAT[i % HAT.length]); A.iC4.setXYZ(i, c.r, c.g, c.b); A.iC4.setW(i, i % 3 === 1 ? 0 : 1);
    }
    var m4 = new THREE.Matrix4(), q = new THREE.Quaternion(), up = new THREE.Vector3(0, 1, 0), p = new THREE.Vector3(), s = new THREE.Vector3(1, 1, 1);
    return {
      mesh: mesh,
      set: function (i, x, y, z, yaw, phase, amp, look, sq) {
        q.setFromAxisAngle(up, yaw); s.set(1, sq || 1, 1); m4.compose(p.set(x, y, z), q, s); mesh.setMatrixAt(i, m4);
        A.iWalk.setXYZW(i, phase, amp, phase * 0.5, look || 0);
        if (i >= mesh.count) mesh.count = i + 1;
      },
      commit: function () { mesh.instanceMatrix.needsUpdate = true; A.iWalk.needsUpdate = true; }
    };
  }

  // ───────── 宠物：一只一个 InstancedMesh(count 1)，set(phase, amp, wag, look) 驱动 ─────────
  var P3 = Pets3D.make(M);
  function petRig(def) {
    var geo = P3.build(def), A = instAttrs(geo, 1), mesh = new THREE.InstancedMesh(geo, RIG, 1);
    mesh.setMatrixAt(0, new THREE.Matrix4()); mesh.frustumCulled = false;
    mesh.userData.anim = function (phase, amp, wag, look) { A.iWalk.setXYZW(0, phase, amp, wag, look || 0); A.iWalk.needsUpdate = true; };
    mesh.userData.anim(0, 0, 0, 0);
    return mesh;
  }
  // 软影子：径向渐变的圆（画在小画布上），所有角色共用一个 InstancedMesh
  function blobs(n) {
    var cv = wx.createCanvas(); cv.width = cv.height = 64;
    var c = cv.getContext('2d'), g = c.createRadialGradient(32, 32, 2, 32, 32, 31);
    g.addColorStop(0, 'rgba(0,0,0,0.42)'); g.addColorStop(0.6, 'rgba(0,0,0,0.22)'); g.addColorStop(1, 'rgba(0,0,0,0)');
    c.fillStyle = g; c.fillRect(0, 0, 64, 64);
    var t = new THREE.CanvasTexture(cv);
    var geo = new THREE.PlaneGeometry(1, 1); geo.rotateX(-Math.PI / 2);
    var mesh = new THREE.InstancedMesh(geo, new THREE.MeshBasicMaterial({ map: t, transparent: true, depthWrite: false, color: 0x000000 }), n);
    mesh.material.color.set(0xffffff); mesh.frustumCulled = false; mesh.count = 0; mesh.renderOrder = 1;
    var m4 = new THREE.Matrix4(), q = new THREE.Quaternion(), p = new THREE.Vector3(), s = new THREE.Vector3();
    return { mesh: mesh, set: function (i, x, y, z, r) { s.set(r, 1, r); m4.compose(p.set(x, y + 0.006, z), q, s); mesh.setMatrixAt(i, m4); if (i >= mesh.count) mesh.count = i + 1; },
      commit: function (count) { mesh.count = count; mesh.instanceMatrix.needsUpdate = true; } };
  }

  return { crowd: crowd, petRig: petRig, pet: petRig, blobs: blobs, RIG: RIG };
}

module.exports = { make: make };
