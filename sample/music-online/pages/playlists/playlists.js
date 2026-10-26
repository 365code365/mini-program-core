// 歌单广场：标签横滑 + 三列网格，上拉分页
const { request } = require('../../utils/api.js');
const player = require('../../utils/player.js');
const SIZE = 12;

Page({
  data: { tags: [], tag: '', list: [], status: 'loading', more: 'idle', total: 0, hasSong: false },
  onLoad(q) {
    this.seq = 0;
    this.page = 0;
    const tag = q.tag ? decodeURIComponent(q.tag) : '';
    this.setData({ tag });
    if (tag) wx.setNavigationBarTitle({ title: tag + '歌单' });
    request('/api/demo/tags', { app: 'music' }, { silent: true })
      .then((r) => this.setData({ tags: [{ name: '', label: '推荐' }].concat(r.list.map((t) => ({ name: t.name, label: t.name }))) }))
      .catch(() => {});
    this.fetch(1);
    this.unsub = player.subscribe((s) => { if (!!s.song !== this.data.hasSong) this.setData({ hasSong: !!s.song }); });
  },
  onUnload() { if (this.unsub) this.unsub(); },
  onReachBottom() { if (this.data.more === 'idle') this.fetch(this.page + 1); },
  onPullDownRefresh() { this.fetch(1).then(() => wx.stopPullDownRefresh()); },
  onTag(e) {
    const tag = e.currentTarget.dataset.name;
    if (tag === this.data.tag) return;
    this.setData({ tag, list: [], status: 'loading' });
    wx.setNavigationBarTitle({ title: tag ? tag + '歌单' : '歌单广场' });
    this.fetch(1);
  },
  fetch(page) {
    const seq = ++this.seq;
    if (page > 1) this.setData({ more: 'loading' });
    return request('/api/demo/music/playlists', { tag: this.data.tag, page, size: SIZE }, { silent: true }).then((r) => {
      if (seq !== this.seq) return;
      this.page = r.page;
      const list = page === 1 ? r.list : this.data.list.concat(r.list);
      this.setData({ list, total: r.total, status: list.length ? 'ok' : 'empty', more: r.hasMore ? 'idle' : 'end' });
    }).catch(() => {
      if (seq !== this.seq) return;
      this.setData(page === 1 ? { status: 'error' } : { more: 'error' });
    });
  },
  onRetry() { this.setData({ status: 'loading' }); this.fetch(1); },
  onRetryMore() { this.fetch(this.page + 1); },
  onPlaylist(e) { wx.navigateTo({ url: '/pages/playlist/playlist?id=' + e.currentTarget.dataset.id }); }
});
