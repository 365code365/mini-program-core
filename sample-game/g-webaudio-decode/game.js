// g-webaudio-decode（game-doc/05 GT-AU5）：decodeAudioData 对 wav / mp3 / aac(m4a) / ogg(vorbis) 的结果，
// 引擎与 Chrome 各解一遍，tools/game-audio-decode-compare.py 比采样数、声道数、采样率与样本。
// 素材不进仓库：脚本用 ffmpeg 现生成（48k 立体声 1 秒：左 440Hz、右 1320Hz 正弦，幅度 0.5）。
// 每个文件一行 T:EV dec {name, ch, len, rate, parts}，样本（平面 f32 的 base64）分成 3000 字符一行的 T:EV pcm {name, i, s}
// （引擎一行日志上限 4096 字节，game/logs.rs），最后 T:DONE。
var T = require('_lib/t.js');
var ctx = wx.createWebAudioContext();
var fs = wx.getFileSystemManager();
var names = ['t.wav', 't.mp3', 't.m4a', 't.ogg'];
var left = names.length;
names.forEach(function (n) {
  var data;
  try { data = fs.readFileSync('assets/' + n); } catch (e) { T.ev('dec', { name: n, err: 'read ' + e.message }); if (--left === 0) T.done(); return; }
  ctx.decodeAudioData(data).then(function (b) {
    var all = new Float32Array(b.length * b.numberOfChannels);
    for (var c = 0; c < b.numberOfChannels; c++) all.set(b.getChannelData(c), c * b.length);
    var b64 = wx.arrayBufferToBase64(all.buffer), parts = Math.ceil(b64.length / 3000);
    for (var i = 0; i < parts; i++) T.ev('pcm', { name: n, i: i, s: b64.slice(i * 3000, (i + 1) * 3000) });
    T.ev('dec', { name: n, ch: b.numberOfChannels, len: b.length, rate: b.sampleRate, parts: parts });
  }, function (e) {
    T.ev('dec', { name: n, err: String(e && (e.errMsg || e.message || e)) });
  }).then(function () { if (--left === 0) T.done(); });
});
T.ready('g-webaudio-decode');
