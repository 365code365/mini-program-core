// g-square：每 rAF 右移 1px 的方块（GT-K1~K9，A2.2/A2.3；真机节拍）。期望见 game-doc/07 §2
var T = require('_lib/t.js');
var c = wx.createCanvas();
var ctx = c.getContext('2d');
var fps = Number(T.q.fps) || 0;
if (fps) wx.setPreferredFramesPerSecond(fps);
var info = wx.getSystemInfoSync();
var n = 0, last = -1;
function tick(ts) {
  n++;
  // 方块左边沿 x = n（逻辑像素 × 画布缩放），帧号写进左上角 8 个像素的颜色里（便于像素断言）
  var k = c.width / info.windowWidth;
  ctx.fillStyle = '#000000'; ctx.fillRect(0, 0, c.width, c.height);
  ctx.fillStyle = '#ff0000'; ctx.fillRect(Math.round(n * k), Math.round(100 * k), Math.round(20 * k), Math.round(20 * k));
  ctx.fillStyle = 'rgb(' + (n & 255) + ',' + ((n >> 8) & 255) + ',0)'; ctx.fillRect(0, 0, 2, 2);
  if (!T.q.quiet) T.frame(n, { ts: ts, dt: last < 0 ? 0 : +(ts - last).toFixed(3), now: +wx.getPerformance().now().toFixed(3) });
  last = ts;
  if (n < (Number(T.q.max) || 1e9)) requestAnimationFrame(tick); else T.done({ frames: n });
}
T.ready('g-square');
requestAnimationFrame(tick);
