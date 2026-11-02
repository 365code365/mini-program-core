// 展厅详情：page-meta 里的 navigation-bar 设标题；内置 <audio> 组件放展厅导览（wx.createAudioContext 控制从头播 / 暂停）
const { HALLS } = require('../../utils/data.js');
const U = require('../../utils/util.js');

Page({
  data: { hall: null, items: [], playing: false, played: false },
  onLoad(q) {
    const hall = HALLS.find(h => h.id === q.id) || HALLS[0];
    this.setData({ hall });
    this.ctx = wx.createAudioContext('hallAudio', this);
  },
  onShow() {
    const fav = U.load('fav', []);
    const items = this.data.hall.exhibits.map(id => Object.assign(U.exhibit(id), { faved: fav.indexOf(id) >= 0 }));
    this.setData({ items });
  },
  onHide() { this.ctx && this.ctx.pause(); },
  onPlay() { this.setData({ playing: true, played: true }); },
  onPause() { this.setData({ playing: false }); },
  onEnded() {
    this.setData({ playing: false });
    wx.showToast({ title: '导览播放完毕，去看看展品吧', icon: 'none' });
  },
  onAudioError(e) { wx.showToast({ title: '音频加载失败：' + (e.detail.errMsg || ''), icon: 'none' }); },
  replay() {
    this.ctx.seek(0);
    this.ctx.play();
  },
  toggle() { if (this.data.playing) this.ctx.pause(); else this.ctx.play(); },
  openExhibit(e) { wx.navigateTo({ url: '/pages/exhibit/exhibit?id=' + e.currentTarget.dataset.id }); },
  goBook() { wx.navigateTo({ url: '/pages/booking/booking' }); }
});
