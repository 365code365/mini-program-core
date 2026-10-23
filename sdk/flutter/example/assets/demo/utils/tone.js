// 合成一段正弦波 WAV（16 位单声道）：示例里没有音频素材，<audio> 与 decodeAudioData 都用它。
// 返回 ArrayBuffer；交给 wx.createBufferURL 就是一个能播的 wxfile://tmp/….wav
function sineWav(freq, seconds, rate) {
  rate = rate || 16000;
  const n = Math.round(seconds * rate);
  const buf = new ArrayBuffer(44 + n * 2);
  const dv = new DataView(buf);
  const w = (o, s) => { for (let i = 0; i < s.length; i++) dv.setUint8(o + i, s.charCodeAt(i)); };
  w(0, 'RIFF'); dv.setUint32(4, 36 + n * 2, true); w(8, 'WAVE'); w(12, 'fmt ');
  dv.setUint32(16, 16, true); dv.setUint16(20, 1, true); dv.setUint16(22, 1, true);
  dv.setUint32(24, rate, true); dv.setUint32(28, rate * 2, true); dv.setUint16(32, 2, true); dv.setUint16(34, 16, true);
  w(36, 'data'); dv.setUint32(40, n * 2, true);
  for (let i = 0; i < n; i++) {
    // 头尾各 20ms 淡入淡出，免得爆音
    const fade = Math.min(1, i / (rate * 0.02), (n - i) / (rate * 0.02));
    dv.setInt16(44 + i * 2, Math.round(Math.sin(2 * Math.PI * freq * i / rate) * 0.4 * fade * 32767), true);
  }
  return buf;
}

module.exports = { sineWav };
