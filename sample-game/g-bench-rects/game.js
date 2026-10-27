// g-bench-rects：n 个方块在画布内弹跳（坐标是小数 → 走抗锯齿通用路径）+ 每秒一行 HUD 文字。
// 精灵版（drawImage）在 G4 接 Image 之后用 g-bench-sprites；这个夹具量的是「路径光栅 + 上屏」的成本。
// query：n（默认 500）、seed、dpr=1（按物理像素建主画布，量 ×dpr 档，game-doc/02 §5.5）
var T = require('_lib/t.js');
var c = wx.createCanvas();
if (T.q.dpr) { var i0 = wx.getSystemInfoSync(); c.width = Math.round(i0.windowWidth * i0.pixelRatio); c.height = Math.round(i0.windowHeight * i0.pixelRatio); }
var ctx = c.getContext('2d');
var n = Number(T.q.n) || 500, S = [];
var colors = ['#e74c3c', '#3498db', '#2ecc71', '#f1c40f', '#9b59b6'];
for (var i = 0; i < n; i++) S.push({ x: T.rand() * (c.width - 16), y: T.rand() * (c.height - 16), vx: T.rand() * 4 - 2, vy: T.rand() * 4 - 2, c: colors[i % 5] });
var f = 0, js = 0, last = 0;
T.ready('g-bench-rects');
(function tick(ts) {
  var t0 = wx.getPerformance().now();
  ctx.fillStyle = '#102030'; ctx.fillRect(0, 0, c.width, c.height);
  for (var i = 0; i < n; i++) {
    var s = S[i];
    s.x += s.vx; s.y += s.vy;
    if (s.x < 0 || s.x > c.width - 16) s.vx = -s.vx;
    if (s.y < 0 || s.y > c.height - 16) s.vy = -s.vy;
    ctx.fillStyle = s.c; ctx.fillRect(s.x, s.y, 16, 16);
  }
  ctx.fillStyle = '#ffffff'; ctx.font = '16px sans-serif'; ctx.fillText('n=' + n + ' frame ' + f, 10, 24);
  js += wx.getPerformance().now() - t0;
  if (++f % 60 === 0) { console.log('T:EV bench ' + JSON.stringify({ f: f, jsMsPerFrame: +(js / 60).toFixed(2) })); js = 0; }
  if (ts !== undefined) { T.frame(f, { ts: ts, dt: last ? +(ts - last).toFixed(3) : 0 }); last = ts; }
  requestAnimationFrame(tick);
})();
