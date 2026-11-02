// 语音：wx.getRecorderManager 录、InnerAudioContext 放。
// 录音管理器是全局单例、监听只能加不能删，所以只注册一次，事件转给当前绑定的页面。
// 设备不能录音（桌面端、没给麦克风权限）时 onError，问用户要不要用示例语音（一段摇篮曲）。
const SAMPLE = { path: '/audio/lullaby.wav', duration: 6, sample: true };
let page = null, inited = false, recTimer = null;

function init() {
  if (inited) return;
  inited = true;
  const rm = wx.getRecorderManager();
  rm.onStart(() => {
    if (!page) return;
    page.setData({ recording: true, recSec: 0 });
    const t0 = Date.now();
    clearInterval(recTimer);
    recTimer = setInterval(() => {
      const s = Math.floor((Date.now() - t0) / 1000);
      if (page) page.setData({ recSec: s });
      if (s >= 60) rm.stop();   // 最长一分钟
    }, 500);
  });
  rm.onStop((res) => {
    clearInterval(recTimer);
    if (!page) return;
    const sec = Math.max(1, Math.round((res.duration || 0) / 1000));
    page.setData({ recording: false, voice: { path: res.tempFilePath, duration: sec } });
    wx.showToast({ title: '录好了 ' + sec + ' 秒', icon: 'none' });
  });
  rm.onError((err) => {
    clearInterval(recTimer);
    if (!page) return;
    page.setData({ recording: false });
    console.warn('录音失败', err && err.errMsg);
    wx.showModal({
      title: '暂时无法录音',
      content: '当前设备不支持录音，或没有开启麦克风权限。先放一段示例语音（摇篮曲）吗？',
      confirmText: '用示例',
      success: (r) => { if (r.confirm && page) page.setData({ voice: SAMPLE }); }
    });
  });
}

function bind(p) { page = p; init(); }

function unbind(p) {
  stopPlay(p);
  if (p.data.recording) wx.getRecorderManager().stop();
  if (p._audio) { p._audio.destroy(); p._audio = null; }
  if (page === p) page = null;
}

function startRecord(p) {
  page = p;
  wx.getRecorderManager().start({ duration: 60000, format: 'aac', sampleRate: 16000, numberOfChannels: 1, encodeBitRate: 48000 });
}

function stopRecord() { wx.getRecorderManager().stop(); }

function audio(p) {
  if (p._audio) return p._audio;
  const a = wx.createInnerAudioContext();
  a.onPlay(() => p.setData({ playing: true }));
  a.onPause(() => p.setData({ playing: false }));
  a.onStop(() => p.setData({ playing: false }));
  a.onEnded(() => p.setData({ playing: false }));
  a.onError((e) => {
    p.setData({ playing: false });
    wx.showToast({ title: '播放失败：' + (e.errMsg || e.errCode || ''), icon: 'none' });
  });
  p._audio = a;
  return a;
}

function togglePlay(p, v) {
  if (!v) return;
  const a = audio(p);
  if (p.data.playing) { a.stop(); return; }
  if (a.src !== v.path) a.src = v.path;
  a.play();
}

function stopPlay(p) { if (p._audio && p.data.playing) p._audio.stop(); }

module.exports = { SAMPLE, bind, unbind, startRecord, stopRecord, togglePlay, stopPlay };
