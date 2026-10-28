// e-three（G9.1 / A8.2）：three.js r160 经最小 adapter 跑在小游戏里。query：frames=N 画到第 N 帧停（对照截图用，默认一直转）
var T = require('_lib/t.js');
var canvas = require('_lib/adapter.js');
var THREE = require('./libs/three.js');
var build = require('./scene.js');
var info = wx.getSystemInfoSync();
var dpr = T.q.dpr ? Number(T.q.dpr) : info.pixelRatio;
var s = build(THREE, canvas, info.windowWidth, info.windowHeight, dpr);
T.ev('three', { rev: THREE.REVISION, w: canvas.width, h: canvas.height, webgl2: s.renderer.capabilities.isWebGL2 });
var n = 0, stop = Number(T.q.frames || 0), last = Date.now(), cnt = 0;
wx.onTouchStart(function (r) { T.ev('touch', { x: r.touches[0].clientX }); });
function frame() {
    s.step(n++);
    cnt++;
    if (Date.now() - last >= 1000) { T.ev('fps', { fps: cnt, calls: s.renderer.info.render.calls }); cnt = 0; last = Date.now(); }
    if (stop && n >= stop) { T.ev('done', { n: n }); return; }
    requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
T.ready('e-three');
