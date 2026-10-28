// g-text（game-doc/05 §2、GT-F3）：文字绘制的 Chrome 参考对比夹具。375×667 的主画布分成 2 列 × 8 行的格子（每格 187×83），
// 每格一种情况，格号 = 行 × 2 + 列。tools/game-compare.py 逐格比引擎与 Chrome 的像素。
// 字体只用 fonts/ 下的文件（wx.loadFont）：两边用同一份字体文件，比的才是光栅化与排版，不是「两台机器的 sans-serif 不一样」。
// fonts/ 不进仓库（Arial.ttf 773KB）：game-compare.py 与 e2e 从 assets/ 拷进来。
var T = require('_lib/t.js');
var c = wx.createCanvas(); var g = c.getContext('2d');
var A = wx.loadFont('fonts/Arial.ttf') || 'sans-serif';
var M = wx.loadFont('fonts/Monaco.ttf') || 'monospace';
var CW = 187, CH = 83;
g.fillStyle = '#ffffff'; g.fillRect(0, 0, c.width, c.height);
function cell(i, f) {
  var x = (i % 2) * CW, y = Math.floor(i / 2) * CH;
  g.save(); g.beginPath(); g.rect(x, y, CW, CH); g.clip(); g.translate(x, y);
  g.fillStyle = '#000000'; g.textAlign = 'left'; g.textBaseline = 'alphabetic'; g.font = '16px ' + A;
  f(); g.restore();
}
cell(0, function () { g.fillText('Hello, World 123', 8, 40); });
cell(1, function () { g.font = 'bold 24px ' + A; g.fillText('Bold Text', 8, 50); });
cell(2, function () { g.font = '12px ' + A; g.fillText('small 12px text: 0.5%', 8, 40); });
cell(3, function () { g.font = '40px ' + A; g.fillText('Big Ag', 8, 60); });
cell(4, function () { g.font = '16px ' + M; g.fillText('mono 0123 {}', 8, 40); });
cell(5, function () { g.textAlign = 'center'; g.fillText('centered', CW / 2, 45); });
cell(6, function () { g.textAlign = 'right'; g.fillText('right aligned', CW - 8, 45); });
cell(7, function () { g.textBaseline = 'top'; g.font = '20px ' + A; g.fillText('top Hg', 8, 20); });
cell(8, function () { g.textBaseline = 'middle'; g.font = '20px ' + A; g.fillText('middle Hg', 8, 41); });
cell(9, function () { g.textBaseline = 'bottom'; g.font = '20px ' + A; g.fillText('bottom Hg', 8, 70); });
cell(10, function () { g.fillStyle = '#203040'; g.fillRect(0, 0, CW, CH); g.fillStyle = '#ffcc00'; g.font = '20px ' + A; g.fillText('on dark', 8, 50); });
cell(11, function () { g.globalAlpha = 0.5; g.fillStyle = '#c00000'; g.font = '20px ' + A; g.fillText('alpha 0.5', 8, 50); });
cell(12, function () { g.strokeStyle = '#0000c0'; g.font = '28px ' + A; g.strokeText('Stroke', 8, 55); });
cell(13, function () { g.font = '20px ' + A; g.fillText('squeezed into 100px', 8, 50, 100); });
cell(14, function () {
  g.font = '18px ' + A; var w = g.measureText('measure me').width;
  g.fillText('measure me', 8, 40); g.fillStyle = '#00a000'; g.fillRect(8, 50, Math.round(w), 4);
  T.ev('measure', { w: w });
});
cell(15, function () { g.translate(30, 70); g.rotate(-Math.PI / 12); g.font = '18px ' + A; g.fillText('rotated -15deg', 0, 0); });
T.ev('fonts', { A: A, M: M });
T.ready('g-text');
var n = 0;
(function tick() { n++; requestAnimationFrame(tick); })();
