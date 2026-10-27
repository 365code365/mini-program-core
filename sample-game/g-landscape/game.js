// g-landscape（game-doc/05 GT-O）：横屏包。四角各一个 40×40 的色块（左上红、右上绿、左下蓝、右下黄），点哪画哪（白点），
// 启动时记系统信息与主画布尺寸；转屏（onWindowResize）时按新窗口改主画布尺寸并重画。
var T = require('_lib/t.js');
var c = wx.createCanvas(), g = c.getContext('2d');
var info = wx.getSystemInfoSync();
T.ev('boot', { ww: info.windowWidth, wh: info.windowHeight, cw: c.width, ch: c.height, orient: info.deviceOrientation });
var dots = [];
function draw() {
  var W = c.width, H = c.height;
  g.fillStyle = '#000'; g.fillRect(0, 0, W, H);
  g.fillStyle = '#f00'; g.fillRect(0, 0, 40, 40);
  g.fillStyle = '#0f0'; g.fillRect(W - 40, 0, 40, 40);
  g.fillStyle = '#00f'; g.fillRect(0, H - 40, 40, 40);
  g.fillStyle = '#ff0'; g.fillRect(W - 40, H - 40, 40, 40);
  g.fillStyle = '#fff';
  dots.forEach(function (d) { g.fillRect(d[0] - 3, d[1] - 3, 6, 6); });
}
wx.onTouchStart(function (e) {
  var t = e.changedTouches[0];
  dots.push([t.clientX, t.clientY]);
  T.ev('tap', { x: t.clientX, y: t.clientY });
  draw();
});
wx.onWindowResize(function (r) {
  c.width = r.windowWidth; c.height = r.windowHeight;
  var i = wx.getSystemInfoSync();
  T.ev('resize', { r: r, ww: i.windowWidth, wh: i.windowHeight, orient: i.deviceOrientation, cw: c.width });
  draw();
});
wx.onDeviceOrientationChange(function (o) { T.ev('orient', o); });
globalThis.goRotate = function (v) {
  wx.setDeviceOrientation({ value: v, success: function (r) { T.ev('setOk', { errMsg: r.errMsg }); },
    fail: function (r) { T.ev('setFail', { errMsg: r.errMsg }); } });
};
draw();
T.ready('g-landscape');
