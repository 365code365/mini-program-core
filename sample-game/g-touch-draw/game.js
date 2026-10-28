// g-touch-draw（game-doc/07 §5，GT-T1~T8；A3.1、A3.3）：每根手指一种颜色画点，日志记每个触摸事件。
// 同一帧到达的点在同一帧的 rAF 里画出（pending 由 rAF 消费），T:FRAME 记这一帧画了几个点。
var T = require('_lib/t.js');
var c = wx.createCanvas(); var ctx = c.getContext('2d');
var k = c.width / wx.getSystemInfoSync().windowWidth;
var colors = ['#ff0000', '#00ff00', '#0000ff', '#ffff00', '#ff00ff', '#00ffff', '#ff8800', '#8800ff', '#008800', '#880000'];
var slot = {};   // identifier → 颜色槽，按首次出现顺序分配（不依赖 identifier 的取值规律）
var next = 0, pending = [];
function brief(e) {
  return { t: +e.timeStamp.toFixed(1),
           ids: e.touches.map(function (x) { return x.identifier; }),
           ch: e.changedTouches.map(function (x) { return [x.identifier, +x.clientX.toFixed(1), +x.clientY.toFixed(1)]; }),
           eq: e.changedTouches.every(function (x) { return x.clientX === x.pageX && x.clientX === x.screenX; }) };
}
['Start', 'Move', 'End', 'Cancel'].forEach(function (k2) {
  wx['onTouch' + k2](function (e) {
    T.ev('touch' + k2.toLowerCase(), brief(e));
    e.changedTouches.forEach(function (p) {
      if (!(p.identifier in slot)) slot[p.identifier] = next++ % 10;
      pending.push([slot[p.identifier], p.clientX, p.clientY]);
    });
  });
});
var n = 0, last = 0, down = 0;
wx.onTouchStart(function (e) { down = e.touches.length; });
wx.onTouchEnd(function (e) { down = e.touches.length; });
wx.onTouchCancel(function (e) { down = e.touches.length; });
function tick(ts) {
  n++;
  // 同一帧到达的点在同一帧画出（A3.3）：本帧 rAF 消费 pending
  pending.forEach(function (p) { ctx.fillStyle = colors[p[0]]; ctx.fillRect(Math.round(p[1] * k) - 3, Math.round(p[2] * k) - 3, 6, 6); });
  // 左上角 HUD 每帧重画（按着几根手指 + 帧号的低位当进度条）：每帧都有新内容上屏，真机量具才量得出屏上重复
  ctx.fillStyle = '#202020'; ctx.fillRect(0, 0, 120, 14);
  ctx.fillStyle = '#ffffff'; ctx.fillRect(0, 0, 12 * down, 6); ctx.fillRect(0, 8, n % 120, 6);
  T.frame(n, { drawn: pending.length, ts: ts, dt: last ? +(ts - last).toFixed(3) : 0 });
  last = ts;
  pending = [];
  requestAnimationFrame(tick);
}
ctx.fillStyle = '#000'; ctx.fillRect(0, 0, c.width, c.height);
T.ready('g-touch-draw');
requestAnimationFrame(tick);
