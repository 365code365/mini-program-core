const store = require('../../utils/store.js');
Page({
  data: { points: 0, cash: 0, log: [], signed: false },
  onShow() { this.load(); },
  load() {
    const points = store.getPoints();
    this.setData({ points, cash: Math.floor(points / 100), log: store.pointsLog().map((x, i) => Object.assign({ k: 'p' + i }, x)), signed: store.signedToday() });
  },
  onSign() {
    if (!store.sign()) { wx.showToast({ title: '今天已经签过了', icon: 'none' }); return; }
    wx.showToast({ title: '签到成功 +10 积分', icon: 'none' });
    this.load();
  },
  onUse() { wx.switchTab({ url: '/pages/index/index' }); }
});
