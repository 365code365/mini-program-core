// 榜单：四个榜横向切换，每个榜 20 首（带排名升降），播放全部 / 点歌播放
const { request } = require('../../utils/api.js');
const player = require('../../utils/player.js');
const app = getApp();

Page({
  data: { charts: [], cur: 0, songs: [], status: 'loading', playingId: 0, hasSong: false },
  onLoad() {
    this.load();
    this.unsub = player.subscribe((s) => {
      const id = s.song ? s.song.id : 0;
      if (id !== this.data.playingId) this.setData({ playingId: id, hasSong: !!s.song });
    });
  },
  onUnload() { if (this.unsub) this.unsub(); },
  // 首页点了某个榜再切过来：tab 页不会重新 onLoad，所以在 onShow 里读要看哪个榜
  onShow() { if (this.data.charts.length) this.pick(this.indexOf(app.globalData.chartId)); },
  onPullDownRefresh() { this.load().then(() => wx.stopPullDownRefresh()); },
  load() {
    return request('/api/demo/music/charts', null, { silent: true }).then((r) => {
      this.setData({ charts: r.list, status: 'ok' });
      this.pick(this.indexOf(app.globalData.chartId));
    }).catch(() => this.setData({ status: 'error' }));
  },
  indexOf(id) {
    const i = this.data.charts.findIndex((c) => c.id === id);
    return i < 0 ? 0 : i;
  },
  pick(i) {
    const c = this.data.charts[i];
    app.globalData.chartId = c.id;
    const songs = c.songs.map((s) => Object.assign({}, s, {
      trendText: s.trend > 0 ? '↑' + s.trend : (s.trend < 0 ? '↓' + (-s.trend) : '—'),
      trendCls: s.trend > 0 ? 'up' : (s.trend < 0 ? 'down' : 'flat')
    }));
    this.setData({ cur: i, songs, color: c.color });
  },
  onTab(e) { this.pick(e.currentTarget.dataset.index); },
  onRetry() { this.setData({ status: 'loading' }); this.load(); },
  onPlayAll() {
    player.playQueue(this.data.songs, 0);
    wx.navigateTo({ url: '/pages/player/player' });
  },
  onSong(e) {
    player.playQueue(this.data.songs, e.currentTarget.dataset.index);
    wx.navigateTo({ url: '/pages/player/player' });
  }
});
