// g-webgl（game-doc/04 G8，A8.6）：转动的立方体（深度测试 + 2D 画布做的纹理），每秒一行 T:EV fps。
// query：res=dpr（默认，画布 = 窗口 × pixelRatio，与常见引擎一致）| res=1（逻辑分辨率）；n=立方体个数（默认 1）
var T = require('_lib/t.js');
var info = wx.getSystemInfoSync();
var scale = T.q.res === '1' ? 1 : info.pixelRatio;
var c = wx.createCanvas();
c.width = Math.round(info.windowWidth * scale); c.height = Math.round(info.windowHeight * scale);
var gl = c.getContext('webgl', { antialias: true });
if (!gl) { T.ev('nogl', {}); throw new Error('no webgl'); }
var dbg = gl.getExtension('WEBGL_debug_renderer_info');
T.ev('gl', { w: c.width, h: c.height, renderer: dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : '', attrs: gl.getContextAttributes() });
function sh(t, s) { var x = gl.createShader(t); gl.shaderSource(x, s); gl.compileShader(x); if (!gl.getShaderParameter(x, gl.COMPILE_STATUS)) T.ev('err', { log: gl.getShaderInfoLog(x) }); return x; }
var p = gl.createProgram();
gl.attachShader(p, sh(gl.VERTEX_SHADER, 'attribute vec3 pos; attribute vec2 uv; uniform mat4 m; varying vec2 v; void main(){ v = uv; gl_Position = m * vec4(pos, 1.0); }'));
gl.attachShader(p, sh(gl.FRAGMENT_SHADER, 'precision mediump float; varying vec2 v; uniform sampler2D t; void main(){ gl_FragColor = texture2D(t, v); }'));
gl.linkProgram(p); gl.useProgram(p);
// 立方体 6 面 × 2 三角形
var P = [], U = [];
var faces = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
faces.forEach(function (f, i) {
    var s = i % 2 ? -1 : 1;
    [[-1, -1], [1, -1], [1, 1], [-1, -1], [1, 1], [-1, 1]].forEach(function (q) {
        var v = [0, 0, 0]; v[f[0]] = q[0]; v[f[1]] = q[1] * s; v[f[2]] = s; P.push(v[0], v[1], v[2]); U.push(q[0] * 0.5 + 0.5, q[1] * 0.5 + 0.5);
    });
});
function buf(a, loc, n) { var b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(a), gl.STATIC_DRAW); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, n, gl.FLOAT, false, 0, 0); }
buf(P, gl.getAttribLocation(p, 'pos'), 3); buf(U, gl.getAttribLocation(p, 'uv'), 2);
var tc = wx.createCanvas(); tc.width = tc.height = 64; var g = tc.getContext('2d');
g.fillStyle = '#f0a020'; g.fillRect(0, 0, 64, 64); g.fillStyle = '#203040'; g.fillRect(8, 8, 48, 48); g.fillStyle = '#ffffff'; g.font = '32px sans-serif'; g.fillText('W', 20, 44);
var t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, tc); gl.generateMipmap(gl.TEXTURE_2D);
gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
gl.enable(gl.DEPTH_TEST);
var mloc = gl.getUniformLocation(p, 'm'), n = Number(T.q.n || 1);
function mat(a, b, sx, ox) {
    var ca = Math.cos(a), sa = Math.sin(a), cb = Math.cos(b), sb = Math.sin(b), asp = c.width / c.height;
    // 绕 y 再绕 x，缩放 0.4、按宽高比修正、z 压到 [-0.5, 0.5]
    return new Float32Array([ca * sx / asp, sa * sb * sx, -sa * cb * 0.2, 0, 0, cb * sx, sb * 0.2, 0, sa * sx / asp, -ca * sb * sx, ca * cb * 0.2, 0, ox, 0, 0.5, 1]);
}
var t0 = Date.now(), frames = 0, last = t0;
function frame() {
    var s = (Date.now() - t0) / 1000;
    gl.viewport(0, 0, c.width, c.height);
    gl.clearColor(0.1, 0.12, 0.15, 1); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    for (var i = 0; i < n; i++) { gl.uniformMatrix4fv(mloc, false, mat(s + i, s * 0.7, n > 1 ? 0.15 : 0.4, n > 1 ? (i / (n - 1)) * 1.6 - 0.8 : 0)); gl.drawArrays(gl.TRIANGLES, 0, 36); }
    frames++;
    var now = Date.now();
    if (now - last >= 1000) { T.ev('fps', { fps: frames, err: gl.getError() }); frames = 0; last = now; }
    requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
// toast=1：1.5s 后弹 2s 的 Toast（覆盖层要在 CPU 上合成：这段时间走读回 + 辅助上下文上屏）
if (T.q.toast) setTimeout(function () { wx.showToast({ title: 'WebGL + Toast', duration: 2000 }); T.ev('toast', {}); }, 1500);
T.ready('g-webgl');
