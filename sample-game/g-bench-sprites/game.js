// g-bench-sprites（game-doc/07 §9，门槛见 05 §5.2）：n 个 32×32 精灵在画布内弹跳；每帧整屏清 + 画（游戏的典型负载）。
// 与 07 的源码相比多两样，量具要：每帧一行 T:FRAME（带 ts / dt，tools/android-game-jank.sh 算 rAF 间隔），
// 每 60 帧一行 T:EV bench（jsMsPerFrame）。query：n（默认 500）、dpr=1（按物理像素建主画布，量 ×dpr 档）、quiet=1（不打 T:FRAME）
var T = require('_lib/t.js');
var c = wx.createCanvas(); var ctx = c.getContext('2d');
if (T.q.dpr) { var i0 = wx.getSystemInfoSync(); c.width = Math.round(i0.windowWidth * i0.pixelRatio); c.height = Math.round(i0.windowHeight * i0.pixelRatio); }
var img = wx.createImage(); img.src = 'assets/sprite32.png';
var n = Number(T.q.n) || 500, S = [];
for (var i = 0; i < n; i++) S.push({ x: T.rand() * (c.width - 32), y: T.rand() * (c.height - 32), vx: T.rand() * 4 - 2, vy: T.rand() * 4 - 2 });
var f = 0, js = 0, last = 0;
img.onload = function () {
  T.ready('g-bench-sprites');
  (function tick(ts) {
    var t0 = wx.getPerformance().now();
    ctx.fillStyle = '#123'; ctx.fillRect(0, 0, c.width, c.height);
    for (var i = 0; i < n; i++) { var s = S[i];
      s.x += s.vx; s.y += s.vy;
      if (s.x < 0 || s.x > c.width - 32) s.vx = -s.vx;
      if (s.y < 0 || s.y > c.height - 32) s.vy = -s.vy;
      ctx.drawImage(img, s.x | 0, s.y | 0); }
    js += wx.getPerformance().now() - t0;
    if (++f % 60 === 0) { T.ev('bench', { f: f, jsMsPerFrame: +(js / 60).toFixed(2) }); js = 0; }
    if (ts !== undefined && !T.q.quiet) { T.frame(f, { ts: ts, dt: last ? +(ts - last).toFixed(3) : 0 }); }
    if (ts !== undefined) last = ts;
    requestAnimationFrame(tick);
  })();
};
img.onerror = function (e) { T.fail('sprite', JSON.stringify(e)); };
