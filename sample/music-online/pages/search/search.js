// 搜索：歌名 / 歌手（GET /api/demo/music/songs?q=），搜索历史存本地，热搜词点了直接搜；结果点歌即播（整页结果作为播放队列）
const { request } = require('../../utils/api.js');
const player = require('../../utils/player.js');
const store = require('../../utils/store.js');
const SIZE = 20;
const HOT = ['晚风', '夏夜', '城南', '海', '星', '温言', '南风乐队', '梦'];
Page({
  data: { kw: '', status: 'idle', list: [], total: 0, more: 'idle', history: [], hot: HOT, playingId: 0, hasSong: false },
  onLoad() {
    this.seq = 0;
    this.page = 0;
    this.unsub = player.subscribe((s) => {
      const id = s.song ? s.song.id : 0;
      if (id !== this.data.playingId) this.setData({ playingId: id, hasSong: !!s.song });
    });
  },
  onUnload() { if (this.unsub) this.unsub(); },
  onShow() { this.setData({ history: store.history() }); },
  onInput(e) {
    const kw = e.detail.value;
    this.setData({ kw });
    if (!kw.trim()) this.setData({ status: 'idle', list: [] });
  },
  onConfirm() { this.search(this.data.kw); },
  onPick(e) {
    const kw = String(e.currentTarget.dataset.kw);
    this.setData({ kw });
    this.search(kw);
  },
  onClear() { this.setData({ kw: '', status: 'idle', list: [] }); },
  onClearHistory() {
    store.clearHistory();
    this.setData({ history: [] });
  },
  search(kw) {
    kw = String(kw || '').trim();
    if (!kw) { wx.showToast({ title: '输入歌名或歌手', icon: 'none' }); return; }
    store.addHistory(kw);
    this.setData({ history: store.history(), status: 'loading', list: [] });
    this.kw = kw;
    this.fetch(1);
  },
  fetch(page) {
    const seq = ++this.seq;
    if (page > 1) this.setData({ more: 'loading' });
    return request('/api/demo/music/songs', { q: this.kw, page, size: SIZE }, { silent: true }).then((r) => {
      if (seq !== this.seq) return;
      this.page = r.page;
      const list = page === 1 ? r.list : this.data.list.concat(r.list);
      this.setData({ list, total: r.total, status: list.length ? 'ok' : 'empty', more: r.hasMore ? 'idle' : 'end' });
    }).catch(() => {
      if (seq !== this.seq) return;
      this.setData(page === 1 ? { status: 'error' } : { more: 'error' });
    });
  },
  onReachBottom() { if (this.data.status === 'ok' && this.data.more === 'idle') this.fetch(this.page + 1); },
  onRetry() { this.search(this.kw); },
  onRetryMore() { this.fetch(this.page + 1); },
  onSong(e) {
    player.playQueue(this.data.list, Number(e.currentTarget.dataset.index));
    wx.navigateTo({ url: '/pages/player/player' });
  }
});
