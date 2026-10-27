// g-lifecycle（game-doc/05 GT-LC）：onShow / onHide 与启动参数记进日志、画在屏上（真机切前后台时肉眼可看）。
// 每次 onHide / onShow 一行 T:EV hide / show；onHide 里带 bgm.paused（音频应当已经停了）。
var T = require('_lib/t.js');
var launch = wx.getLaunchOptionsSync();
T.ev('launch', launch);
var bgm = wx.createInnerAudioContext();
var hides = 0, shows = 0;
var onShow = function (o) { shows++; T.ev('show', { n: shows, scene: o.scene, query: o.query }); };
wx.onShow(onShow);
wx.onHide(function (o) { hides++; T.ev('hide', { n: hides, arg: o, bgmPaused: bgm.paused }); });
globalThis.goOff = function () { wx.offShow(onShow); };
globalThis.goExit = function (kind) {
  var api = kind === 'restart' ? wx.restartMiniProgram : wx.exitMiniProgram;
  api({ success: function (r) { T.ev('exit', { kind: kind, errMsg: r.errMsg }); } });
};
var c = wx.createCanvas(), g = c.getContext('2d');
(function tick() {
  g.fillStyle = '#203040'; g.fillRect(0, 0, c.width, c.height);
  g.fillStyle = '#fff'; g.font = '20px sans-serif';
  g.fillText('scene ' + launch.scene + '  hide ' + hides + '  show ' + shows, 20, 60);
  requestAnimationFrame(tick);
})();
T.ready('g-lifecycle');
