// 全局播放器：一个 InnerAudioContext + 播放队列，状态放 app.globalData.player（各页与迷你播放条共用一份）。
// 只用引擎实现了的接口（src/js/prelude/audio.js）：src / play / pause / stop / seek、currentTime / duration / paused，
// 事件 onPlay / onPause / onStop / onEnded / onTimeUpdate / onCanplay / onError / onWaiting。
const RECENT_KEY = 'music_recent';
let ctx = null;
const listeners = [];

function state() {
  const g = getApp().globalData;
  if (!g.player) g.player = { queue: [], index: -1, song: null, playing: false, current: 0, duration: 0, waiting: false };
  return g.player;
}

function emit(kind) {
  const s = state();
  listeners.slice().forEach((fn) => { try { fn(s, kind); } catch (e) { console.error(e); } });
}

function ensure() {
  if (ctx) return ctx;
  ctx = wx.createInnerAudioContext();
  ctx.onPlay(() => { const s = state(); s.playing = true; s.waiting = false; emit('state'); });
  ctx.onPause(() => { state().playing = false; emit('state'); });
  ctx.onStop(() => { state().playing = false; emit('state'); });
  ctx.onWaiting(() => { state().waiting = true; emit('state'); });
  ctx.onCanplay(() => { const s = state(); s.waiting = false; s.duration = ctx.duration || s.duration; emit('state'); });
  ctx.onTimeUpdate(() => {
    const s = state();
    s.current = ctx.currentTime || 0;
    // duration 解码出来之前是 0：先用接口给的试听长度顶着，进度条不会除以 0
    s.duration = ctx.duration || (s.song && s.song.trialSeconds) || 0;
    emit('time');
  });
  // 播完自动下一首；队列只有一首就停在结尾
  ctx.onEnded(() => {
    const s = state();
    s.playing = false;
    s.current = s.duration;
    if (s.queue.length > 1) next();
    else emit('state');
  });
  ctx.onError((e) => {
    state().playing = false;
    emit('state');
    wx.showToast({ title: '播放失败：' + ((e && e.errMsg) || '音频加载出错'), icon: 'none' });
  });
  return ctx;
}

function remember(song) {
  const recent = [song].concat((wx.getStorageSync(RECENT_KEY) || []).filter((x) => x.id !== song.id)).slice(0, 50);
  wx.setStorageSync(RECENT_KEY, recent);
}

function playIndex(i) {
  const s = state();
  if (!s.queue.length) return;
  i = (i + s.queue.length) % s.queue.length;
  const song = s.queue[i];
  const a = ensure();
  s.index = i;
  s.song = song;
  s.current = 0;
  s.duration = song.trialSeconds || 0;
  s.waiting = true;
  a.src = song.audioUrl;
  a.play();
  remember(song);
  emit('song');
}

function playQueue(songs, index) {
  state().queue = songs.slice();
  playIndex(index || 0);
}

function toggle() {
  const s = state();
  if (!s.song) return;
  if (s.playing) ensure().pause();
  else ensure().play();
}

function next() { playIndex(state().index + 1); }
function prev() {
  // 播了 3 秒以上按「上一首」先回到本首开头（与常见播放器一致）
  if (state().current > 3) { seek(0); return; }
  playIndex(state().index - 1);
}
function seek(sec) {
  const s = state();
  s.current = sec;
  ensure().seek(sec);
  emit('time');
}

// 订阅状态变化：立刻回调一次当前状态；返回取消订阅的函数（页面 onUnload / 组件 detached 里调）
function subscribe(fn) {
  listeners.push(fn);
  fn(state(), 'init');
  return () => {
    const i = listeners.indexOf(fn);
    if (i >= 0) listeners.splice(i, 1);
  };
}

function fmt(sec) {
  sec = Math.max(0, Math.floor(sec || 0));
  return Math.floor(sec / 60) + ':' + (sec % 60 < 10 ? '0' : '') + (sec % 60);
}

module.exports = { state, playQueue, playIndex, toggle, next, prev, seek, subscribe, fmt, RECENT_KEY };
