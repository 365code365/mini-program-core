// 我的：最近播放 / 我喜欢 / 收藏歌单（都存在 storage，离线也能看）+ 来源小程序信息
const player = require('../../utils/player.js');
const app = getApp();
const store = require('../../utils/store.js');

Page({
  data: { tab: 'recent', recent: [], liked: [], favs: [], list: [], referrer: null, playingId: 0, hasSong: false },
  onLoad() {
    this.unsub = player.subscribe((s) => {
      const id = s.song ? s.song.id : 0;
      if (id !== this.data.playingId) this.setData({ playingId: id, hasSong: !!s.song });
    });
  },
  onUnload() { if (this.unsub) this.unsub(); },
  onShow() {
    const recent = wx.getStorageSync(player.RECENT_KEY) || [];
    this.setData({ recent, liked: store.liked(), favs: store.favPlaylists(), referrer: app.globalData.referrer });
    this.setData({ list: this.listOf(this.data.tab) });
  },
  listOf(tab) { return tab === 'recent' ? this.data.recent : tab === 'liked' ? this.data.liked : this.data.favs; },
  onTab(e) {
    const tab = e.currentTarget.dataset.tab;
    this.setData({ tab, list: this.listOf(tab) });
  },
  onPlaylist(e) { wx.navigateTo({ url: '/pages/playlist/playlist?id=' + e.currentTarget.dataset.id }); },
  onSearch() { wx.navigateTo({ url: '/pages/search/search' }); },
  onPlayAll() {
    if (!this.data.list.length) return;
    player.playQueue(this.data.list, 0);
    wx.navigateTo({ url: '/pages/player/player' });
  },
  onSong(e) {
    player.playQueue(this.data.list, Number(e.currentTarget.dataset.index));
    wx.navigateTo({ url: '/pages/player/player' });
  },
  onClear() {
    const tab = this.data.tab;
    const key = tab === 'recent' ? player.RECENT_KEY : tab === 'liked' ? store.LIKE_KEY : store.FAV_KEY;
    wx.showModal({
      title: '清空列表',
      content: tab === 'recent' ? '清空最近播放？' : tab === 'liked' ? '清空我喜欢的音乐？' : '取消收藏全部歌单？',
      success: (r) => {
        if (!r.confirm) return;
        wx.removeStorageSync(key);
        this.onShow();
      }
    });
  },
  onBack() { app.backToReferrer(); },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
