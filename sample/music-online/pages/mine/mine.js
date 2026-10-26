// 我的：最近播放 / 我喜欢（都存在 storage，离线也能看）+ 来源小程序信息
const player = require('../../utils/player.js');
const app = getApp();
const LIKE_KEY = 'music_liked';

Page({
  data: { tab: 'recent', recent: [], liked: [], list: [], referrer: null, playingId: 0, hasSong: false },
  onLoad() {
    this.unsub = player.subscribe((s) => {
      const id = s.song ? s.song.id : 0;
      if (id !== this.data.playingId) this.setData({ playingId: id, hasSong: !!s.song });
    });
  },
  onUnload() { if (this.unsub) this.unsub(); },
  onShow() {
    const recent = wx.getStorageSync(player.RECENT_KEY) || [];
    const liked = wx.getStorageSync(LIKE_KEY) || [];
    this.setData({ recent, liked, list: this.data.tab === 'recent' ? recent : liked, referrer: app.globalData.referrer });
  },
  onTab(e) {
    const tab = e.currentTarget.dataset.tab;
    this.setData({ tab, list: tab === 'recent' ? this.data.recent : this.data.liked });
  },
  onPlayAll() {
    if (!this.data.list.length) return;
    player.playQueue(this.data.list, 0);
    wx.navigateTo({ url: '/pages/player/player' });
  },
  onSong(e) {
    player.playQueue(this.data.list, e.currentTarget.dataset.index);
    wx.navigateTo({ url: '/pages/player/player' });
  },
  onClear() {
    const key = this.data.tab === 'recent' ? player.RECENT_KEY : LIKE_KEY;
    wx.showModal({
      title: '清空列表',
      content: this.data.tab === 'recent' ? '清空最近播放？' : '清空我喜欢的音乐？',
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
