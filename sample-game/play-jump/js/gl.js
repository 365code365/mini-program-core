// WebGL 小工具：4×4 矩阵（列主序，与 GLSL 一致）、着色器程序、网格（交错的 位置 + 法线，Uint16 下标）。
// 只用 WebGL 1：微信小游戏低端机仍有只给 WebGL 1 的，且这个游戏用不上 WebGL 2 的东西。

// ───────── 矩阵 ─────────
function m4() { var m = new Float32Array(16); m[0] = m[5] = m[10] = m[15] = 1; return m; }
// out 可以就是 a 或 b（mul(m, x, m) 原地右乘）：先算进暂存再拷回，否则边写边读算错
var T16 = new Float32Array(16);
function mul(a, b, out) {
  var o = out || new Float32Array(16);
  for (var c = 0; c < 4; c++) for (var r = 0; r < 4; r++) {
    T16[c * 4 + r] = a[r] * b[c * 4] + a[4 + r] * b[c * 4 + 1] + a[8 + r] * b[c * 4 + 2] + a[12 + r] * b[c * 4 + 3];
  }
  o.set(T16);
  return o;
}
function ortho(l, r, b, t, n, f) {
  var m = m4();
  m[0] = 2 / (r - l); m[5] = 2 / (t - b); m[10] = -2 / (f - n);
  m[12] = -(r + l) / (r - l); m[13] = -(t + b) / (t - b); m[14] = -(f + n) / (f - n);
  return m;
}
function lookAt(e, c, up) {
  var zx = e[0] - c[0], zy = e[1] - c[1], zz = e[2] - c[2], zl = Math.hypot(zx, zy, zz); zx /= zl; zy /= zl; zz /= zl;
  var xx = up[1] * zz - up[2] * zy, xy = up[2] * zx - up[0] * zz, xz = up[0] * zy - up[1] * zx, xl = Math.hypot(xx, xy, xz);
  xx /= xl; xy /= xl; xz /= xl;
  var yx = zy * xz - zz * xy, yy = zz * xx - zx * xz, yz = zx * xy - zy * xx;
  var m = m4();
  m[0] = xx; m[4] = xy; m[8] = xz; m[1] = yx; m[5] = yy; m[9] = yz; m[2] = zx; m[6] = zy; m[10] = zz;
  m[12] = -(xx * e[0] + xy * e[1] + xz * e[2]); m[13] = -(yx * e[0] + yy * e[1] + yz * e[2]); m[14] = -(zx * e[0] + zy * e[1] + zz * e[2]);
  return m;
}
function translate(x, y, z) { var m = m4(); m[12] = x; m[13] = y; m[14] = z; return m; }
function scale(x, y, z) { var m = m4(); m[0] = x; m[5] = y; m[10] = z; return m; }
// 绕任意轴（单位向量）转 a 弧度
function rotate(ax, ay, az, a) {
  var c = Math.cos(a), s = Math.sin(a), t = 1 - c, m = m4();
  m[0] = t * ax * ax + c; m[1] = t * ax * ay + s * az; m[2] = t * ax * az - s * ay;
  m[4] = t * ax * ay - s * az; m[5] = t * ay * ay + c; m[6] = t * ay * az + s * ax;
  m[8] = t * ax * az + s * ay; m[9] = t * ay * az - s * ax; m[10] = t * az * az + c;
  return m;
}
// 法线矩阵 = 模型矩阵左上 3×3 的逆转置（非等比缩放时法线才不歪）
function normalMat(m) {
  var a = m[0], b = m[1], c = m[2], d = m[4], e = m[5], f = m[6], g = m[8], h = m[9], i = m[10];
  var A = e * i - f * h, B = -(d * i - f * g), C = d * h - e * g;
  var det = a * A + b * B + c * C || 1e-9;
  return new Float32Array([A / det, B / det, C / det,
    -(b * i - c * h) / det, (a * i - c * g) / det, -(a * h - b * g) / det,
    (b * f - c * e) / det, -(a * f - c * d) / det, (a * e - b * d) / det]);
}
function xform(m, x, y, z) {
  var w = m[3] * x + m[7] * y + m[11] * z + m[15];
  return [(m[0] * x + m[4] * y + m[8] * z + m[12]) / w, (m[1] * x + m[5] * y + m[9] * z + m[13]) / w];
}

// ───────── 程序与网格 ─────────
function program(gl, vs, fs) {
  function sh(type, src) {
    var s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error('shader: ' + gl.getShaderInfoLog(s));
    return s;
  }
  var p = gl.createProgram();
  gl.attachShader(p, sh(gl.VERTEX_SHADER, vs)); gl.attachShader(p, sh(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error('link: ' + gl.getProgramInfoLog(p));
  var u = {}, n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
  for (var k = 0; k < n; k++) { var info = gl.getActiveUniform(p, k); u[info.name] = gl.getUniformLocation(p, info.name); }
  return { p: p, u: u, a: function (name) { return gl.getAttribLocation(p, name); } };
}
function mesh(gl, verts, idx) {
  var vb = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, vb); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(verts), gl.STATIC_DRAW);
  var ib = gl.createBuffer(); gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ib); gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint16Array(idx), gl.STATIC_DRAW);
  return { vb: vb, ib: ib, n: idx.length };
}
// 方块：x、z 在 [-1, 1]，y 在 [0, 1]；没有底面（永远看不见）
function box(gl) {
  var v = [], ix = [];
  var faces = [
    [[0, 1, 0], [[-1, 1, -1], [1, 1, -1], [1, 1, 1], [-1, 1, 1]]],
    [[-1, 0, 0], [[-1, 0, -1], [-1, 0, 1], [-1, 1, 1], [-1, 1, -1]]],
    [[1, 0, 0], [[1, 0, -1], [1, 1, -1], [1, 1, 1], [1, 0, 1]]],
    [[0, 0, -1], [[-1, 0, -1], [-1, 1, -1], [1, 1, -1], [1, 0, -1]]],
    [[0, 0, 1], [[-1, 0, 1], [1, 0, 1], [1, 1, 1], [-1, 1, 1]]]
  ];
  faces.forEach(function (f) {
    var b = v.length / 6;
    f[1].forEach(function (p) { v.push(p[0], p[1], p[2], f[0][0], f[0][1], f[0][2]); });
    ix.push(b, b + 1, b + 2, b, b + 2, b + 3);
  });
  return mesh(gl, v, ix);
}
// 车床体：剖面 [[半径, 高], …] 绕 y 轴转一圈；每段剖面一圈环，法线绕圈平滑、沿剖面按段（圆锥、柱面、顶盖都对）
function lathe(gl, profile, seg) {
  var v = [], ix = [];
  for (var i = 0; i + 1 < profile.length; i++) {
    var p0 = profile[i], p1 = profile[i + 1];
    var dr = p1[0] - p0[0], dy = p1[1] - p0[1], l = Math.hypot(dr, dy) || 1;
    var nr = dy / l, ny = -dr / l;
    var b = v.length / 6;
    for (var k = 0; k <= seg; k++) {
      var a = k / seg * Math.PI * 2, c = Math.cos(a), s = Math.sin(a);
      v.push(p0[0] * c, p0[1], p0[0] * s, nr * c, ny, nr * s);
      v.push(p1[0] * c, p1[1], p1[0] * s, nr * c, ny, nr * s);
    }
    for (var k2 = 0; k2 < seg; k2++) { var q = b + k2 * 2; ix.push(q, q + 1, q + 3, q, q + 3, q + 2); }
  }
  return mesh(gl, v, ix);
}
function sphere(gl, r, cy, seg) {
  var v = [], ix = [], rings = seg >> 1;
  for (var i = 0; i <= rings; i++) {
    var t = i / rings * Math.PI, y = Math.cos(t), rr = Math.sin(t);
    for (var k = 0; k <= seg; k++) {
      var a = k / seg * Math.PI * 2, nx = rr * Math.cos(a), nz = rr * Math.sin(a);
      v.push(nx * r, cy + y * r, nz * r, nx, y, nz);
    }
  }
  for (var i2 = 0; i2 < rings; i2++) for (var k3 = 0; k3 < seg; k3++) {
    var q = i2 * (seg + 1) + k3;
    ix.push(q, q + seg + 1, q + 1, q + 1, q + seg + 1, q + seg + 2);
  }
  return mesh(gl, v, ix);
}
// 平面圆环（涟漪）/ 圆片（影子）：在 y=0 平面，外半径 1、内半径 inner
function ring(gl, inner, seg) {
  var v = [], ix = [];
  for (var k = 0; k <= seg; k++) {
    var a = k / seg * Math.PI * 2, c = Math.cos(a), s = Math.sin(a);
    v.push(c * inner, 0, s * inner, 0, 1, 0, c, 0, s, 0, 1, 0);
  }
  for (var k2 = 0; k2 < seg; k2++) { var q = k2 * 2; ix.push(q, q + 1, q + 3, q, q + 3, q + 2); }
  return mesh(gl, v, ix);
}

function hex(c) { var n = parseInt(c.slice(1), 16); return [(n >> 16 & 255) / 255, (n >> 8 & 255) / 255, (n & 255) / 255]; }

module.exports = { m4: m4, mul: mul, ortho: ortho, lookAt: lookAt, translate: translate, scale: scale, rotate: rotate,
  normalMat: normalMat, xform: xform, program: program, box: box, lathe: lathe, sphere: sphere, ring: ring, hex: hex };
