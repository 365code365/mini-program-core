// 音效播放：sfx/*.mp3 是 tools/gen-fighter-sfx.py 离线合成的（多层打击、失真、卷积混响、解说）。
// 启动时每个文件 decodeAudioData 一次，之后每次只建一个 BufferSource + 一个 Gain（真机 ~0.6ms，不掉帧）。
//
// 三件让它听着不「电子」的小事：
// - 同一个音每次随机变调 ±4%（playbackRate）、音量 ±10%：连打十拳不是十声复读机；
// - 同一个音最多同时响 3 个，再来就把最老的掐掉：超必杀 5 段命中叠在一起不会糊成一团、也不会爆音；
// - 解说一次只放一条（新的顶掉旧的），人声不叠。
// 缺文件 / 解码失败的那条静音，别的照常。
var NAMES = ['swingL', 'swingH', 'hitL', 'hitH', 'block', 'fire', 'ice', 'rise', 'superStart', 'super', 'clash', 'ko', 'thud',
  'step', 'dash', 'ui', 'select', 'v_round1', 'v_round2', 'v_round3', 'v_fight', 'v_ko', 'v_perfect', 'v_win', 'v_lose',
  'v_timeover', 'v_draw', 'v_title'];
// 每条的基准音量（混音比例：打击声最响，脚步、界面音小）
var VOL = { step: 0.35, ui: 0.5, select: 0.7, swingL: 0.55, swingH: 0.65, dash: 0.5, thud: 0.8, block: 0.75 };
var MAX_SAME = 3;

var ac = null, master = null, bufs = {}, live = {}, voice = null, muted = false;

function init() {
  try { ac = wx.createWebAudioContext(); } catch (e) { ac = null; }
  if (!ac) return Promise.resolve(0);
  master = ac.createGain();
  master.gain.value = 0.9;
  master.connect(ac.destination);
  var fs = wx.getFileSystemManager(), ok = 0;
  return Promise.all(NAMES.map(function (n) {
    var data;
    try { data = fs.readFileSync('sfx/' + n + '.mp3'); } catch (e) { return null; }
    return new Promise(function (res) {
      try {
        ac.decodeAudioData(data, function (b) { bufs[n] = b; ok++; res(); }, function () { res(); });
      } catch (e) { res(); }
    });
  })).then(function () { return ok; });
}

// opt：{ vol: 倍数, pitch: 固定变调（不随机） }
function play(name, opt) {
  if (muted || !ac) return;
  var b = bufs[name];
  if (!b) return;
  try {
    var isVoice = name.charAt(0) === 'v' && name.charAt(1) === '_';
    var s = ac.createBufferSource(), g = ac.createGain();
    s.buffer = b;
    var rate = opt && opt.pitch ? opt.pitch : isVoice ? 1 : 0.96 + Math.random() * 0.08;
    if (s.playbackRate) s.playbackRate.value = rate;
    g.gain.value = (VOL[name] || 1) * (opt && opt.vol || 1) * (isVoice ? 1 : 0.9 + Math.random() * 0.2);
    s.connect(g); g.connect(master);
    s.start();
    if (isVoice) { if (voice) stop(voice); voice = s; return; }
    var q = live[name] || (live[name] = []);
    q.push(s);
    if (q.length > MAX_SAME) stop(q.shift());
    s.onended = function () { var i = q.indexOf(s); if (i >= 0) q.splice(i, 1); };
  } catch (e) {}
}
function stop(s) { try { s.stop(); } catch (e) {} }

module.exports = {
  init: init, play: play,
  setMuted: function (m) { muted = m; },
  resume: function () { try { if (ac && ac.resume) ac.resume(); } catch (e) {} }
};
