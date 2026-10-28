// g-video（game-doc/04 G7.7）：wx.createVideo 盖在游戏画面上播包内的 clip.mp4，游戏画面照常每帧动。
// 事件进日志：T:EV play / pause / ended / timeUpdate（每秒只记一次）/ error；3s 后 pause、4s 后 play、7s 后 destroy
var T = require('_lib/t.js');
var c = wx.createCanvas(), g = c.getContext('2d'), info = wx.getSystemInfoSync();
var v = wx.createVideo({ x: 20, y: 80, width: info.windowWidth - 40, height: (info.windowWidth - 40) * 9 / 16, src: 'clip.mp4', loop: true, controls: false });
var lastSec = -1;
v.onPlay(function () { T.ev('play', {}); });
v.onPause(function () { T.ev('pause', {}); });
v.onEnded(function () { T.ev('ended', {}); });
v.onError(function (e) { T.ev('error', e); });
v.onTimeUpdate(function (r) { var s = Math.floor(r.position); if (s !== lastSec) { lastSec = s; T.ev('timeUpdate', r); } });
v.play();
setTimeout(function () { v.pause(); }, 3000);
setTimeout(function () { v.play(); }, 4000);
setTimeout(function () { v.destroy(); T.ev('destroyed', {}); }, 7000);
var t0 = Date.now();
function frame() {
    var t = (Date.now() - t0) / 1000;
    g.fillStyle = '#203040'; g.fillRect(0, 0, c.width, c.height);
    g.fillStyle = '#f0a020'; g.fillRect((Math.sin(t * 2) * 0.4 + 0.4) * c.width, c.height * 0.75, c.width * 0.2, c.width * 0.2);
    requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
T.ready('g-video');
