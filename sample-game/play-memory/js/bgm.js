// 背景音乐：wx.createInnerAudioContext 循环播 audio/bgm.mp3（8 秒八音盒五声旋律，首尾相接）。
// 切后台暂停、回前台接着播（只在用户开着音乐、且切走前确实在播时才恢复）；开关存档。
var ctx = null, on = true, wasPlaying = false, err = '';

function init(enabled) {
  on = enabled !== false;
  try {
    ctx = wx.createInnerAudioContext();
    ctx.src = 'audio/bgm.mp3';
    ctx.loop = true;
    ctx.volume = 0.45;
    ctx.onError(function (e) { err = (e && (e.errMsg || e.errCode)) || 'error'; console.log('背景音乐播放失败：' + err); });
  } catch (e) { ctx = null; err = String(e && e.message || e); }
  wx.onHide && wx.onHide(function () { wasPlaying = !!ctx && on; if (ctx) ctx.pause(); });
  wx.onShow && wx.onShow(function () { if (ctx && on && wasPlaying) ctx.play(); });
  if (on) play();
}
function play() { if (ctx) ctx.play(); }
function set(v) { on = v; if (!ctx) return; if (on) ctx.play(); else ctx.pause(); }
function state() {
  if (!ctx) return { ok: false, err: err };
  return { ok: true, on: on, paused: ctx.paused, loop: ctx.loop, t: Math.round(ctx.currentTime * 10) / 10, err: err };
}
module.exports = { init: init, set: set, isOn: function () { return on; }, state: state };
