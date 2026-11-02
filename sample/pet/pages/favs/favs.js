const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');
const store = require('../../utils/store.js');
// 我的收藏：商品详情页点的「收藏」都在这里，可直接加购或取消收藏
Page({
  data: { list: [] },
  onShow() { this.refresh(); },
  refresh() {
    this.setData({ list: store.getFavs().map(id => data.findGoods(id)).filter(Boolean) });
  },
  goGoods(e) { wx.navigateTo({ url: '/pages/goods/goods?id=' + e.currentTarget.dataset.id }); },
  addCart(e) {
    const g = data.findGoods(e.currentTarget.dataset.id);
    cart.add(g.id, g.specs[0], 1);
    wx.showToast({ title: '已加入购物车（' + g.specs[0] + '）', icon: 'none' });
  },
  unfav(e) {
    store.toggleFav(e.currentTarget.dataset.id);
    this.refresh();
    wx.showToast({ title: '已取消收藏', icon: 'none' });
  },
  goMall() { wx.switchTab({ url: '/pages/mall/mall' }); }
});
