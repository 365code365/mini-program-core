// 歌单详情：封面头 + 播放全部 + 歌曲列表；正在播的那首标红。被别的小程序带过来时顶部给「返回」
const { request } = require('../../utils/api.js');
const player = require('../../utils/player.js');
const store = require('../../utils/store.js');
const app = getApp();

Page({
  data: { status: 'loading', p: null, fav: false, favCount: 0, songs: [], playingId: 0, hasSong: false, referrer: null, descOpen: false },
  onLoad(q) {
    this.id = Number(q.id) || 1;
    // 只有从来源小程序直接带进来时才显示返回条；平常从首页点进来的不显示
    if (q.from === 'referrer' || (app.globalData.referrer && Number(app.globalData.referrer.extraData.playlistId) === this.id)) {
      this.setData({ referrer: app.globalData.referrer });
    }
    this.load();
    this.unsub = player.subscribe((s) => {
      const id = s.song ? s.song.id : 0;
      if (id !== this.data.playingId || !!s.song !== this.data.hasSong) this.setData({ playingId: id, hasSong: !!s.song });
    });
  },
  onUnload() { if (this.unsub) this.unsub(); },
  load() {
    this.setData({ status: 'loading' });
    request('/api/demo/music/playlists/' + this.id, null, { silent: true }).then((p) => {
      const fav = store.isFav(p.id);
      // 接口的收藏数不含自己：本地收藏了就 +1
      this.setData({ status: 'ok', p, songs: p.songs, fav, favCount: p.subscribed + (fav ? 1 : 0) });
      wx.setNavigationBarTitle({ title: p.name.length > 12 ? '歌单' : p.name });
    }).catch((err) => {
      this.setData({ status: 'error', errMsg: err.statusCode === 404 ? '歌单不存在或已下架' : '加载失败，请检查网络' });
    });
  },
  onRetry() { this.load(); },
  onPlayAll() {
    if (!this.data.songs.length) return;
    player.playQueue(this.data.songs, 0);
    wx.navigateTo({ url: '/pages/player/player' });
  },
  onSong(e) {
    const i = Number(e.currentTarget.dataset.index);
    const s = player.state();
    // 点正在播的那首：不重头播，直接进播放页
    if (s.song && s.song.id === this.data.songs[i].id) {
      wx.navigateTo({ url: '/pages/player/player' });
      return;
    }
    player.playQueue(this.data.songs, i);
    wx.navigateTo({ url: '/pages/player/player' });
  },
  onDesc() { this.setData({ descOpen: !this.data.descOpen }); },
  onCollect() {
    const on = store.toggleFav(this.data.p);
    this.setData({ fav: on, favCount: this.data.p.subscribed + (on ? 1 : 0) });
    wx.showToast({ title: on ? '已收藏到「我的 - 收藏歌单」' : '已取消收藏', icon: 'none' });
  },
  onSongsTap() { wx.pageScrollTo({ selector: '.list', duration: 200 }); },
  onBack() { app.backToReferrer(); }
});
