// g-sensors（game-doc/04 A7.3）：加速计 / 陀螺仪 / 罗盘 / DeviceMotion 按 interval 采样，每秒一行 T:EV rate {各自回调次数, 最后一个值}
var T = require('_lib/t.js');
var iv = T.q.interval || 'game';
var n = { acc: 0, gyro: 0, compass: 0, motion: 0 }, last = {};
wx.onAccelerometerChange(function (r) { n.acc++; last.acc = r; });
wx.onGyroscopeChange(function (r) { n.gyro++; last.gyro = r; });
wx.onCompassChange(function (r) { n.compass++; last.compass = r; });
wx.onDeviceMotionChange(function (r) { n.motion++; last.motion = r; });
function done(tag) { return { success: function () { T.ev('start', { s: tag, ok: true }); }, fail: function (e) { T.ev('start', { s: tag, ok: false, e: e }); } }; }
wx.startAccelerometer(Object.assign({ interval: iv }, done('acc')));
wx.startGyroscope(Object.assign({ interval: iv }, done('gyro')));
wx.startCompass(done('compass'));
wx.startDeviceMotionListening(Object.assign({ interval: iv }, done('motion')));
setInterval(function () { T.ev('rate', { n: n, last: last }); n = { acc: 0, gyro: 0, compass: 0, motion: 0 }; }, 1000);
T.ready('g-sensors');
