// 背景音频：优先 wx.getBackgroundAudioManager（锁屏 / 通知栏有控件，切页继续播）。
// 本引擎桌面端调用时抛「property is not configurable」（prelude 里对 src 二次 defineProperty），
// 拿不到时退回一个全局的 InnerAudioContext，接口照 BackgroundAudioManager 包一层：设 src 即开播。
let mgr = null;

function fallback() {
  const a = wx.createInnerAudioContext();
  const meta = { title: '', singer: '', epname: '', coverImgUrl: '' };
  let src = '';
  const m = {
    fallback: true,
    play: () => a.play(),
    pause: () => a.pause(),
    stop: () => a.stop(),
    seek: p => a.seek(p)
  };
  Object.keys(meta).forEach(k => {
    Object.defineProperty(m, k, { get: () => meta[k], set: v => { meta[k] = String(v == null ? '' : v); } });
  });
  Object.defineProperty(m, 'src', {
    get: () => src,
    set: v => { src = String(v || ''); a.src = src; if (src) a.play(); }
  });
  ['paused', 'currentTime', 'duration'].forEach(k => Object.defineProperty(m, k, { get: () => a[k] }));
  ['Play', 'Pause', 'Stop', 'Ended', 'TimeUpdate', 'Error', 'Canplay', 'Waiting'].forEach(N => {
    m['on' + N] = cb => a['on' + N](cb);
  });
  return m;
}

function get() {
  if (mgr) return mgr;
  try {
    mgr = wx.getBackgroundAudioManager();
  } catch (e) {
    console.warn('getBackgroundAudioManager 不可用，退回 InnerAudioContext：' + (e && e.message));
    mgr = fallback();
  }
  return mgr;
}

module.exports = { get };
