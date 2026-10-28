// WebGL 场景：正交等距相机、带光照的方块 / 圆柱 / 棋子，影子、涟漪、粒子，以及屏幕空间的矩形（背景渐变、文字贴图、遮罩）。
// 世界坐标与玩法层一致：x、z 是地面，y 向上，单位 = 一格；方块底在 y=0。
var G = require('gl.js');

var VS_LIT = [
  'attribute vec3 a_pos; attribute vec3 a_nrm;',
  'uniform mat4 u_mvp; uniform mat3 u_nm; varying vec3 v_n;',
  'void main() { v_n = u_nm * a_nrm; gl_Position = u_mvp * vec4(a_pos, 1.0); }'
].join('\n');
// 相机从 (-x, +y, -z) 看过去：屏幕右边是 +z、左边是 +x，看得见的侧面左边是 -z 面、右边是 -x 面。
// 光从左上方来：顶面最亮、左侧（-z 面）次之、右侧（-x 面）最暗（与 2D 版三个面的配色同一感觉）；输出预乘 alpha
var FS_LIT = [
  'precision mediump float;',
  'varying vec3 v_n; uniform vec3 u_col; uniform float u_alpha; uniform float u_lit; uniform vec3 u_light;',
  'void main() {',
  '  float d = max(dot(normalize(v_n), u_light), 0.0);',
  '  vec3 c = mix(u_col, u_col * (0.6 + 0.45 * d), u_lit);',
  '  gl_FragColor = vec4(c * u_alpha, u_alpha);',
  '}'
].join('\n');
var VS_SCREEN = [
  'attribute vec2 a_p; uniform vec4 u_rect; varying vec2 v_uv;',
  'void main() { v_uv = vec2(a_p.x, 1.0 - a_p.y); gl_Position = vec4(mix(u_rect.xy, u_rect.zw, a_p), 0.0, 1.0); }'
].join('\n');
// u_mode：0 贴图（文字，已预乘）、1 纯色、2 竖直渐变（u_c0 上 → u_c1 下）
var FS_SCREEN = [
  'precision mediump float;',
  'varying vec2 v_uv; uniform float u_mode; uniform float u_alpha; uniform vec4 u_c0; uniform vec4 u_c1; uniform sampler2D u_tex;',
  'void main() {',
  '  vec4 c = u_mode < 0.5 ? texture2D(u_tex, v_uv) : (u_mode < 1.5 ? vec4(u_c0.rgb * u_c0.a, u_c0.a) : mix(u_c0, u_c1, v_uv.y));',
  '  gl_FragColor = c * u_alpha;',
  '}'
].join('\n');

var LIGHT = (function () { var x = -0.15, y = 1.0, z = -0.45, l = Math.hypot(x, y, z); return new Float32Array([x / l, y / l, z / l]); })();
var ID3 = new Float32Array([1, 0, 0, 0, 1, 0, 0, 0, 1]);
var PIECE = '#4b4675';

function make(canvas, W, H, S, originY) {
  var gl = canvas.getContext('webgl', { antialias: true, alpha: false, depth: true, preserveDrawingBuffer: false });
  if (!gl) return null;
  var lit = G.program(gl, VS_LIT, FS_LIT), scr = G.program(gl, VS_SCREEN, FS_SCREEN);
  var aPos = lit.a('a_pos'), aNrm = lit.a('a_nrm'), aP = scr.a('a_p');
  var meshes = {
    box: G.box(gl),
    cyl: G.lathe(gl, [[1, 0], [1, 1], [0, 1]], 40),
    disc: G.lathe(gl, [[1, 0], [0, 0]], 32),          // 朝上的圆片（影子 / 圆柱顶上的装饰）
    ring: G.ring(gl, 0.86, 48),
    // 棋子：底座 + 圆锥身子（剖面从下往上沿外表面走，法线才朝外）
    body: G.lathe(gl, [[0, 0], [0.29, 0], [0.29, 0.05], [0.26, 0.07], [0.1, 0.64], [0, 0.66]], 28),
    head: G.sphere(gl, 0.2, 0.82, 24)
  };
  var quad = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, quad);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1]), gl.STATIC_DRAW);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);

  // 正交相机：从 (-x, +y, -z) 方向看向 +x+z，仰角约 30°；看的点画在屏幕 originY 处（离顶部的比例）
  var hw = W / 2 / S, top = H * originY / S, bottom = -H * (1 - originY) / S;
  var proj = G.ortho(-hw, hw, bottom, top, 0.1, 200);
  var vp = G.m4(), tmp = new Float32Array(16), mvp = new Float32Array(16);
  var cam = { x: 0, y: 0, z: 0 };
  function setCamera(x, y, z) {
    cam.x = x; cam.y = y; cam.z = z;
    G.mul(proj, G.lookAt([x - 30, y + 24.5, z - 30], [x, y, z], [0, 1, 0]), vp);
  }
  setCamera(0, 0, 0);

  var bound = null;                                    // 当前绑着哪个网格（同一网格连画不重复绑）
  function bindMesh(m) {
    if (bound === m) return;
    bound = m;
    gl.bindBuffer(gl.ARRAY_BUFFER, m.vb);
    gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, 24, 0);
    gl.vertexAttribPointer(aNrm, 3, gl.FLOAT, false, 24, 12);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, m.ib);
  }
  function useLit() {
    gl.useProgram(lit.p);
    gl.disableVertexAttribArray(aP);
    gl.enableVertexAttribArray(aPos); gl.enableVertexAttribArray(aNrm);
    gl.uniform3fv(lit.u.u_light, LIGHT);
    bound = null;
  }
  function draw(m, model, color, alpha, lighting) {
    bindMesh(m);
    G.mul(vp, model, mvp);
    gl.uniformMatrix4fv(lit.u.u_mvp, false, mvp);
    gl.uniformMatrix3fv(lit.u.u_nm, false, lighting ? G.normalMat(model) : ID3);
    var c = typeof color === 'string' ? G.hex(color) : color;
    gl.uniform3f(lit.u.u_col, c[0], c[1], c[2]);
    gl.uniform1f(lit.u.u_alpha, alpha);
    gl.uniform1f(lit.u.u_lit, lighting ? 1 : 0);
    gl.drawElements(gl.TRIANGLES, m.n, gl.UNSIGNED_SHORT, 0);
  }
  function at(x, y, z, sx, sy, sz) { return G.mul(G.translate(x, y, z), G.scale(sx, sy, sz), tmp.slice()); }

  // ───────── 屏幕空间矩形（逻辑像素） ─────────
  function useScreen() {
    gl.useProgram(scr.p);
    gl.disableVertexAttribArray(aPos); gl.disableVertexAttribArray(aNrm);
    gl.bindBuffer(gl.ARRAY_BUFFER, quad);
    gl.enableVertexAttribArray(aP);
    gl.vertexAttribPointer(aP, 2, gl.FLOAT, false, 0, 0);
    bound = null;
  }
  function rect(x, y, w, h) {
    gl.uniform4f(scr.u.u_rect, x / W * 2 - 1, 1 - (y + h) / H * 2, (x + w) / W * 2 - 1, 1 - y / H * 2);
  }
  function screenRect(x, y, w, h, mode, alpha, c0, c1, tex) {
    rect(x, y, w, h);
    gl.uniform1f(scr.u.u_mode, mode);
    gl.uniform1f(scr.u.u_alpha, alpha);
    if (c0) gl.uniform4f(scr.u.u_c0, c0[0], c0[1], c0[2], c0[3] === undefined ? 1 : c0[3]);
    if (c1) gl.uniform4f(scr.u.u_c1, c1[0], c1[1], c1[2], c1[3] === undefined ? 1 : c1[3]);
    if (tex) gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }

  return {
    gl: gl, cam: cam, setCamera: setCamera,
    // 一帧开头：清屏 + 背景渐变（GPU 上画整屏渐变几乎不要钱，不用像 2D 版那样缓存）
    begin: function (top3, bottom3) {
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.clearColor(bottom3[0], bottom3[1], bottom3[2], 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.disable(gl.DEPTH_TEST); gl.disable(gl.BLEND);
      useScreen();
      screenRect(0, 0, W, H, 2, 1, top3, bottom3);
      gl.enable(gl.DEPTH_TEST); gl.depthMask(true);
      useLit();
    },
    block: function (b, h) {
      var col = G.hex(b.colors[0]);
      draw(b.shape === 'cyl' ? meshes.cyl : meshes.box, at(b.x, 0, b.z, b.w, h, b.w), col, 1, true);
      if (b.stripe) {                                   // 顶面装饰：方块一道横条、圆柱一个小圆，比底色亮一点
        var c2 = [col[0] + (1 - col[0]) * 0.4, col[1] + (1 - col[1]) * 0.4, col[2] + (1 - col[2]) * 0.4];
        if (b.shape === 'cyl') draw(meshes.disc, at(b.x, h + 0.003, b.z, b.w * 0.45, 1, b.w * 0.45), c2, 1, false);
        else draw(meshes.box, at(b.x, h, b.z, b.w * 1.001, 0.004, b.w * 0.35), c2, 1, true);
      }
    },
    // 棋子：squash 0~1 蓄力压扁；spin 空翻角度，绕 (ax, 0, az) 轴、以身子中间为支点
    piece: function (x, y, z, squash, spin, ax, az) {
      var m = G.translate(x, y, z);
      if (spin) {
        G.mul(m, G.translate(0, 0.42, 0), m);
        G.mul(m, G.rotate(ax, 0, az, spin), m);
        G.mul(m, G.translate(0, -0.42, 0), m);
      }
      G.mul(m, G.scale(1 + squash * 0.18, 1 - squash * 0.35, 1 + squash * 0.18), m);
      draw(meshes.body, m, PIECE, 1, true);
      draw(meshes.head, m, PIECE, 1, true);
    },
    // 半透明的东西：影子、涟漪、粒子（先画完不透明的再画；不写深度，但仍被近处的方块挡住）
    beginTransparent: function () {
      gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA); gl.depthMask(false);
    },
    shadow: function (x, y, z) { draw(meshes.disc, at(x + 0.06, y + 0.004, z - 0.06, 0.3, 1, 0.3), [0, 0, 0], 0.2, false); },
    ripple: function (b, h, t) {
      var r = 0.35 + t * 0.9;
      draw(meshes.ring, at(b.x, h + 0.006, b.z, r, 1, r), [1, 1, 1], Math.max(0, 1 - t), false);
    },
    dust: function (list) {
      list.forEach(function (d) { draw(meshes.box, at(d.x, d.y, d.z, 0.035, 0.07, 0.035), [1, 1, 1], Math.min(1, d.life * 3), false); });
    },
    // 屏幕层（文字、遮罩）：关深度，按逻辑像素摆
    beginScreen: function () { gl.disable(gl.DEPTH_TEST); gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA); useScreen(); },
    image: function (tex, x, y, w, h, alpha) { screenRect(x, y, w, h, 0, alpha, null, null, tex); },
    fill: function (rgba, x, y, w, h) { screenRect(x, y, w, h, 1, 1, rgba); },
    // 世界坐标 → 逻辑像素（飘字跟着落点走）
    project: function (x, y, z) { var p = G.xform(vp, x, y, z); return [(p[0] + 1) / 2 * W, (1 - p[1]) / 2 * H]; }
  };
}

module.exports = { make: make };
