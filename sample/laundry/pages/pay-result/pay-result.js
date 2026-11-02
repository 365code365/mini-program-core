const store = require('../../utils/store.js');

// 由下单页 redirectTo 过来：返回不会回到已提交的下单页
Page({
  data: { o: {}, pickText: '' },

  onLoad(q) {
    const o = store.order(q.id);
    if (!o) {
      wx.showToast({ title: '订单不存在', icon: 'none' });
      return;
    }
    this.id = o.id;
    this.setData({ o, pickText: o.pickup.label.split(' ').slice(0, 2).join(' ') + ' ' + o.pickup.slot });
  },

  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  goDetail() { wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + this.id }); }
});
