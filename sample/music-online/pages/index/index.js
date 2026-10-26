// 发现页：来源提示条 + 轮播 + 快捷入口 + 推荐歌单 + 榜单速览 + 新歌速递
const { request } = require('../../utils/api.js');
const player = require('../../utils/player.js');
const app = getApp();

Page({
  data: { referrer: null, banners: [], playlists: [], charts: [], songs: [], status: 'loading', hasSong: false },
  onLoad() {
    this.load();
    this.unsub = player.subscribe((s) => { if (!!s.song !== this.data.hasSong) this.setData({ hasSong: !!s.song }); });
  },
  onUnload() { if (this.unsub) this.unsub(); },
  onShow() {
    const ref = app.globalData.referrer;
    this.setData({ referrer: ref });
    // 被别的小程序带着歌单打开：直接进歌单（只消费一次，返回首页不会再跳）
    const pid = app.globalData.pendingPlaylistId;
    if (pid) {
      app.globalData.pendingPlaylistId = 0;
      wx.navigateTo({ url: '/pages/playlist/playlist?id=' + pid + '&from=referrer' });
    }
  },
  onPullDownRefresh() {
    this.load().then(() => wx.stopPullDownRefresh());
  },
  load() {
    return Promise.all([
      request('/api/demo/banners', { app: 'music' }, { silent: true }),
      request('/api/demo/music/playlists', { page: 1, size: 6 }, { silent: true }),
      request('/api/demo/music/charts', null, { silent: true }),
      request('/api/demo/music/songs', { page: 1, size: 6 }, { silent: true })
    ]).then(([b, p, c, s]) => {
      this.charts = c.list;
      this.setData({
        status: 'ok', banners: b.list, playlists: p.list, songs: s.list,
        charts: c.list.map((x) => ({ id: x.id, name: x.name, color: x.color, updateText: x.updateText, cover: x.cover, top: x.songs.slice(0, 3) }))
      });
    }).catch(() => {
      if (!this.data.playlists.length) this.setData({ status: 'error' });
      wx.showToast({ title: '加载失败，下拉重试', icon: 'none' });
    });
  },
  onRetry() { this.setData({ status: 'loading' }); this.load(); },
  onBanner(e) {
    const d = e.currentTarget.dataset;
    if (d.type === 'playlist') wx.navigateTo({ url: '/pages/playlist/playlist?id=' + d.id });
    else if (d.type === 'song') this.playSongs(this.data.songs, 0);
  },
  onPlaylist(e) { wx.navigateTo({ url: '/pages/playlist/playlist?id=' + e.currentTarget.dataset.id }); },
  onMorePlaylists() { wx.navigateTo({ url: '/pages/playlists/playlists' }); },
  onOst() { wx.navigateTo({ url: '/pages/playlists/playlists?tag=' + encodeURIComponent('影视原声') }); },
  onDaily() {
    if (this.data.songs.length) this.playSongs(this.data.songs, 0);
  },
  onCharts(e) {
    app.globalData.chartId = Number(e.currentTarget.dataset.id) || 1;
    wx.switchTab({ url: '/pages/charts/charts' });
  },
  onSong(e) { this.playSongs(this.data.songs, e.currentTarget.dataset.index); },
  playSongs(list, i) {
    player.playQueue(list, i);
    wx.navigateTo({ url: '/pages/player/player' });
  },
  onBack() { app.backToReferrer(); },
  onCloseRef() {
    app.globalData.referrer = null;
    this.setData({ referrer: null });
  }
});
