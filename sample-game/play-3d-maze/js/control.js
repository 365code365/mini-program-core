// 输入：虚拟摇杆（屏幕下半部任意处按下就地出摇杆）、重力感应（wx.startDeviceMotionListening）、键盘（PC 方向键 / WASD）。
// 三者合成一个方向向量 (x, y)，长度 ≤ 1：x 向右、y 向下（= 世界坐标 +x / +z，相机朝向固定为北，所以屏幕方向直接对世界方向）。
//
// 重力感应：第一次读数当「放平」的基准，之后按 gamma（左右倾）/ beta（前后倾）相对基准的角度算，倾 22° 满速、2° 以内算死区。
// 宿主不支持（桌面端、没有陀螺仪）时 start 走 fail：自动切回摇杆并 toast 提示，不卡流程。
var B = require('base.js');
var JOY_R = 52;

function make() {
  var joy = { id: null, ox: 0, oy: 0, x: 0, y: 0 };
  var tilt = { want: false, on: false, state: 'off', base: null, x: 0, y: 0, samples: 0 };
  var listening = false;

  function onMotion(r) {
    if (!tilt.on) return;
    tilt.samples++;
    var beta = Number(r.beta) || 0, gamma = Number(r.gamma) || 0;
    if (!tilt.base) tilt.base = { beta: beta, gamma: gamma };
    function axis(d) { var a = Math.abs(d); return a < 2 ? 0 : Math.sign(d) * Math.min(1, (a - 2) / 20); }
    tilt.x = axis(gamma - tilt.base.gamma);
    tilt.y = axis(beta - tilt.base.beta);
  }
  function startTilt(quiet) {
    if (!wx.startDeviceMotionListening || !wx.onDeviceMotionChange) { fail('unsupported', quiet); return; }
    if (!listening) { wx.onDeviceMotionChange(onMotion); listening = true; }
    tilt.state = 'starting';
    wx.startDeviceMotionListening({
      interval: 'game',
      success: function () { tilt.on = true; tilt.base = null; tilt.state = 'on'; if (!quiet) toast('重力感应已开启，放平手机后倾斜控制'); },
      fail: function (e) { fail('fail', quiet, e); }
    });
  }
  function fail(why, quiet) {
    tilt.on = false; tilt.want = false; tilt.state = why; tilt.x = tilt.y = 0;
    if (!quiet) toast('当前设备不支持重力感应，请用摇杆');
  }
  function stopTilt() {
    if (tilt.on && wx.stopDeviceMotionListening) wx.stopDeviceMotionListening({});
    tilt.on = false; tilt.x = tilt.y = 0;
    if (tilt.state === 'on' || tilt.state === 'starting') tilt.state = 'off';
  }
  function toast(t) { try { wx.showToast({ title: t, icon: 'none', duration: 1600 }); } catch (e) {} }

  // 切后台停传感器（省电，也是官方建议）；回前台照原样再开，并重新取基准（拿手的姿势可能变了）
  B.onHide(function () { if (tilt.on) { stopTilt(); tilt.want = true; } joy.id = null; joy.x = joy.y = 0; });
  B.onShow(function () { if (tilt.want) startTilt(true); });

  return {
    R: JOY_R, joy: joy, tilt: tilt,
    toggleTilt: function (quiet) {
      if (tilt.on || tilt.state === 'starting') { tilt.want = false; stopTilt(); if (!quiet) toast('已切回摇杆'); }
      else { tilt.want = true; startTilt(quiet); }
    },
    recalibrate: function () { tilt.base = null; },
    down: function (x, y, id) { if (joy.id !== null) return; joy.id = id; joy.ox = x; joy.oy = y; joy.x = joy.y = 0; },
    move: function (x, y, id) {
      if (id !== joy.id) return;
      var dx = x - joy.ox, dy = y - joy.oy, d = Math.sqrt(dx * dx + dy * dy);
      if (d > JOY_R) { dx = dx / d * JOY_R; dy = dy / d * JOY_R; }
      joy.x = dx / JOY_R; joy.y = dy / JOY_R;
    },
    up: function (x, y, id) { if (id === joy.id) { joy.id = null; joy.x = joy.y = 0; } },
    // 合成方向：摇杆优先（手在上面就听手的），其次键盘，最后重力
    vector: function () {
      if (joy.id !== null && (joy.x || joy.y)) return { x: joy.x, y: joy.y, src: 'joy' };
      var kx = (B.held.right ? 1 : 0) - (B.held.left ? 1 : 0), ky = (B.held.down ? 1 : 0) - (B.held.up ? 1 : 0);
      if (kx || ky) { var l = Math.sqrt(kx * kx + ky * ky); return { x: kx / l, y: ky / l, src: 'key' }; }
      if (tilt.on && (tilt.x || tilt.y)) {
        var tl = Math.sqrt(tilt.x * tilt.x + tilt.y * tilt.y);
        return tl > 1 ? { x: tilt.x / tl, y: tilt.y / tl, src: 'tilt' } : { x: tilt.x, y: tilt.y, src: 'tilt' };
      }
      return { x: 0, y: 0, src: 'none' };
    }
  };
}

module.exports = { make: make };
