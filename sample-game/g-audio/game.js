// g-audio（game-doc/07 §6，GT-AU7 / AU8，A5.2）：BGM 循环 + 32 个音效对象。素材先跑 tools/gen-game-assets.py。
// 点屏幕放一个音效；query auto=1 时每 500ms 自动放一个（真机无人值守量欠载 / 延迟）。
// 每秒一行 T:EV clock（bgm.currentTime 与 performance.now，验证播放位置跟着真设备走）
var T = require('_lib/t.js');
var bgmSrc = T.q.bgm || 'assets/bgm.wav';
var bgm = wx.createInnerAudioContext(); bgm.src = bgmSrc; bgm.loop = true; bgm.volume = 0.4;
['Canplay', 'Play', 'Pause', 'Stop', 'Ended', 'Error', 'Waiting', 'Seeking', 'Seeked'].forEach(function (e) {
  bgm['on' + e](function (r) { T.ev('bgm.' + e, r || {}); });
});
bgm.play();
var pool = [];
for (var i = 0; i < 32; i++) { var a = wx.createInnerAudioContext(); a.src = 'assets/click.wav'; pool.push(a); }
var idx = 0;
function click() { var a = pool[idx++ % 32]; a.stop(); a.play(); }
wx.onTouchStart(click);
if (T.q.auto) setInterval(click, 500);
wx.onAudioInterruptionBegin && wx.onAudioInterruptionBegin(function () { T.ev('interruptBegin', {}); });
wx.onAudioInterruptionEnd && wx.onAudioInterruptionEnd(function () { T.ev('interruptEnd', {}); });
wx.onHide && wx.onHide(function () { T.ev('hide', { bgmPaused: bgm.paused }); });
wx.onShow && wx.onShow(function () { T.ev('show', { bgmPaused: bgm.paused }); });
var c = wx.createCanvas(), g = c.getContext('2d'), n = 0, t0 = 0, b0 = 0;
(function tick() {
  n++;
  g.fillStyle = '#102030'; g.fillRect(0, 0, c.width, c.height);
  g.fillStyle = '#ffcc00'; g.fillRect(20, 100, (bgm.currentTime / 10) * (c.width - 40), 20);
  if (n % 60 === 0) {
    var now = wx.getPerformance().now(), b = bgm.currentTime;
    if (t0) T.ev('clock', { dtMs: +(now - t0).toFixed(1), dBgmMs: +((b - b0) * 1000).toFixed(1), bgm: +b.toFixed(3), paused: bgm.paused });
    t0 = now; b0 = b;
  }
  requestAnimationFrame(tick);
})();
T.ready('g-audio');
