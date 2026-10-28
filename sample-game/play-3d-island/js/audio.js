// 声音全部合成（不带音频文件）：音效是预先算好的 AudioBuffer（第一次用时合成、缓存），背景音乐是五声音阶的小曲，
// 按小节提前 0.6 秒排进 WebAudio 的时间线（BufferSource.start(when)）。
// 不现建 Oscillator 的原因见 js/base.js：本引擎里建一个 Oscillator 在真机上 ~25ms。
function make() {
  var ac = null;
  try { ac = wx.createWebAudioContext(); } catch (e) { return { sfx: function () {}, music: function () {}, step: function () {} }; }
  var SR = 22050, cache = {}, master = ac.createGain(), musicGain = ac.createGain();
  master.gain.value = 0.9; musicGain.gain.value = 0.18;
  master.connect(ac.destination); musicGain.connect(ac.destination);
  // 一个音：波形 + 起音 / 指数衰减包络；f1 给了就是滑音；harm 叠一个八度泛音让声音「圆」一点
  function tone(f0, dur, wave, vol, f1, harm) {
    var key = [f0, dur, wave, vol, f1, harm].join('|');
    if (cache[key]) return cache[key];
    var n = Math.floor(SR * dur), b = ac.createBuffer(1, n, SR), d = new Float32Array(n), ph = 0, ph2 = 0;
    for (var i = 0; i < n; i++) {
      var t = i / n, f = f1 ? f0 * Math.pow(f1 / f0, t) : f0;
      ph += f / SR; ph -= Math.floor(ph); ph2 += 2 * f / SR; ph2 -= Math.floor(ph2);
      var w = wave === 'sine' ? Math.sin(ph * 6.2832) : wave === 'tri' ? 1 - 4 * Math.abs(ph - 0.5) : wave === 'sq' ? (ph < 0.5 ? 0.6 : -0.6) : (Math.random() * 2 - 1);
      if (harm) w += Math.sin(ph2 * 6.2832) * harm;
      var env = Math.min(1, i / (SR * 0.004)) * Math.exp(-t * 5);
      d[i] = w * env * vol;
    }
    if (b.copyToChannel) b.copyToChannel(d, 0); else b.getChannelData(0).set(d);
    return (cache[key] = b);
  }
  function play(buf, when, dest) { var s = ac.createBufferSource(); s.buffer = buf; s.connect(dest || master); s.start(when || 0); }
  var on = { sfx: true, music: true };
  var SFX = {
    tap: function () { play(tone(880, 0.05, 'sine', 0.25)); },
    collect: function () { play(tone(988, 0.08, 'tri', 0.3, 0, 0.3)); play(tone(1319, 0.12, 'tri', 0.25, 0, 0.3), ac.currentTime + 0.06); },
    coin: function () { play(tone(1568, 0.09, 'sq', 0.12)); play(tone(2093, 0.14, 'sq', 0.1), ac.currentTime + 0.05); },
    build: function () { [0, 0.09, 0.18].forEach(function (t, i) { play(tone(220 + i * 40, 0.08, 'noise', 0.25), ac.currentTime + t); }); },
    upgrade: function () { [523, 659, 784, 1047].forEach(function (f, i) { play(tone(f, 0.16, 'tri', 0.25, 0, 0.4), ac.currentTime + i * 0.07); }); },
    buy: function () { play(tone(1319, 0.1, 'sine', 0.3, 0, 0.5)); play(tone(1760, 0.2, 'sine', 0.3, 0, 0.5), ac.currentTime + 0.08); },
    error: function () { play(tone(220, 0.18, 'sq', 0.18, 160)); },
    hatch: function () { play(tone(400, 0.3, 'tri', 0.3, 900, 0.3)); },
    pop: function () { play(tone(700, 0.07, 'sine', 0.3, 1100)); },
    sr: function () { [784, 988, 1175].forEach(function (f, i) { play(tone(f, 0.2, 'tri', 0.25, 0, 0.4), ac.currentTime + i * 0.06); }); },
    ssr: function () { [523, 659, 784, 1047, 1319, 1568].forEach(function (f, i) { play(tone(f, 0.35, 'tri', 0.25, 0, 0.5), ac.currentTime + i * 0.08); }); },
    levelup: function () { [392, 523, 659, 784, 1047].forEach(function (f, i) { play(tone(f, 0.3, 'tri', 0.28, 0, 0.5), ac.currentTime + i * 0.1); }); },
    whoosh: function () { play(tone(300, 0.18, 'noise', 0.12)); }
  };
  // 背景音乐：C 大调五声，四小节循环（和弦根音 + 琶音）
  var SCALE = [261.6, 293.7, 329.6, 392, 440, 523.3, 587.3, 659.3], CHORDS = [0, 4, 3, 2], BEAT = 0.36;
  var nextBar = 0, bar = 0;
  function scheduleBar(t0) {
    var root = CHORDS[bar % 4];
    play(tone(SCALE[root] / 2, BEAT * 3.8, 'sine', 0.5, 0, 0.2), t0, musicGain);
    for (var i = 0; i < 8; i++) {
      var deg = (root + [0, 2, 4, 2, 5, 4, 2, 4][i] + (bar % 2 && i > 5 ? 1 : 0)) % SCALE.length;
      if (i === 3 && bar % 4 === 3) continue;
      play(tone(SCALE[deg], BEAT * 0.9, 'tri', 0.35, 0, 0.25), t0 + i * BEAT / 2, musicGain);
    }
    bar++;
  }
  return {
    sfx: function (name) { if (on.sfx && SFX[name]) try { SFX[name](); } catch (e) {} },
    set: function (sfx, music) { on.sfx = sfx; on.music = music; musicGain.gain.value = music ? 0.18 : 0; },
    step: function () {
      if (!on.music) return;
      var now = ac.currentTime;
      if (nextBar < now) nextBar = now + 0.1;
      while (nextBar < now + 0.6) { scheduleBar(nextBar); nextBar += BEAT * 4; }
    }
  };
}
module.exports = { make: make };
