// 播放页：旋转封面、进度条拖动 seek、上一首 / 播放暂停 / 下一首、歌词随时间高亮滚动、队列面板、喜欢
const { request } = require('../../utils/api.js');
const player = require('../../utils/player.js');
const LIKE_KEY = 'music_liked';
const SLIDER_MAX = 1000;   // 进度条按千分比走：试听只有 20 秒，按秒走一格太粗

Page({
  data: {
    song: null, playing: false, waiting: false, pos: 0, posText: '0:00', durText: '0:00',
    lyrics: [], curLine: -1, toView: '', liked: false, showQueue: false, queue: [], index: -1, max: SLIDER_MAX
  },
  onLoad() {
    this.seeking = false;
    this.lyricSongId = 0;
    this.unsub = player.subscribe((s, kind) => this.sync(s, kind));
  },
  onUnload() { if (this.unsub) this.unsub(); },
  sync(s, kind) {
    if (!s.song) return;
    const patch = {};
    if (!this.data.song || this.data.song.id !== s.song.id || kind === 'init') {
      Object.assign(patch, { song: s.song, liked: this.isLiked(s.song.id), queue: s.queue, index: s.index, lyrics: [], curLine: -1, toView: '' });
      wx.setNavigationBarTitle({ title: s.song.name });
      this.loadLyrics(s.song.id);
    }
    if (s.playing !== this.data.playing) patch.playing = s.playing;
    if (s.waiting !== this.data.waiting) patch.waiting = s.waiting;
    // 拖着进度条时别让播放进度把滑块拽回去
    if (!this.seeking) {
      const pos = s.duration ? Math.round((s.current / s.duration) * SLIDER_MAX) : 0;
      const posText = player.fmt(s.current);
      if (pos !== this.data.pos) patch.pos = pos;
      if (posText !== this.data.posText) patch.posText = posText;
      const line = this.lineAt(s.current, patch.lyrics || this.data.lyrics);
      if (line !== this.data.curLine) {
        patch.curLine = line;
        // 高亮行上面留两行，滚动时高亮行停在视野中部
        patch.toView = line >= 0 ? 'l' + Math.max(0, line - 2) : '';
      }
    }
    const durText = player.fmt(s.duration);
    if (durText !== this.data.durText) patch.durText = durText;
    if (Object.keys(patch).length) this.setData(patch);
  },
  loadLyrics(id) {
    this.lyricSongId = id;
    request('/api/demo/music/songs/' + id, null, { silent: true }).then((d) => {
      if (this.lyricSongId !== id) return;   // 切歌了，丢掉旧歌的歌词
      this.setData({ lyrics: d.lyrics, curLine: -1 });
      this.sync(player.state(), 'lyrics');
    }).catch(() => {
      if (this.lyricSongId === id) this.setData({ lyrics: [{ time: 0, text: '歌词加载失败' }] });
    });
  },
  lineAt(t, lyrics) {
    let k = -1;
    for (let i = 0; i < lyrics.length; i++) {
      if (lyrics[i].time <= t + 0.05) k = i;
      else break;
    }
    return k;
  },
  onToggle() { player.toggle(); },
  onPrev() { player.prev(); },
  onNext() { player.next(); },
  onSeeking(e) {
    this.seeking = true;
    const s = player.state();
    this.setData({ posText: player.fmt((e.detail.value / SLIDER_MAX) * s.duration) });
  },
  onSeek(e) {
    const s = player.state();
    this.seeking = false;
    player.seek((e.detail.value / SLIDER_MAX) * s.duration);
  },
  // 点歌词跳到那一句
  onLine(e) {
    const l = this.data.lyrics[e.currentTarget.dataset.index];
    if (l) player.seek(l.time);
  },
  isLiked(id) { return (wx.getStorageSync(LIKE_KEY) || []).some((x) => x.id === id); },
  onLike() {
    const song = this.data.song;
    let list = wx.getStorageSync(LIKE_KEY) || [];
    const on = !list.some((x) => x.id === song.id);
    list = on ? [song].concat(list) : list.filter((x) => x.id !== song.id);
    wx.setStorageSync(LIKE_KEY, list);
    this.setData({ liked: on });
    wx.showToast({ title: on ? '已添加到我喜欢的音乐' : '已取消喜欢', icon: 'none' });
  },
  onQueue() { this.setData({ showQueue: true, queue: player.state().queue, index: player.state().index }); },
  onCloseQueue() { this.setData({ showQueue: false }); },
  onPick(e) {
    player.playIndex(e.currentTarget.dataset.index);
    this.setData({ showQueue: false });
  },
  noop() {}
});
