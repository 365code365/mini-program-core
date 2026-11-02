// 我的收藏：已卖出的置灰；取消收藏
const store = require('../../utils/store.js');
Page({
  data: { list: [] },
  onShow() { this.setData({ list: store.favs().map(id => store.findItem(id)).filter(Boolean) }); },
  open(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  unfav(e) {
    store.toggleFav(e.currentTarget.dataset.id);
    wx.showToast({ title: '已取消收藏', icon: 'none' });
    this.onShow();
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
