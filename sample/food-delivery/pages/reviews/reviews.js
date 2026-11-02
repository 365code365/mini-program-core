const data = require('../../utils/data.js');
const orders = require('../../utils/orders.js');
Page({
  data: {
    list: [],
    pending: 0,
    avg: '',
    starList: [1, 2, 3, 4, 5]
  },
  // 订单详情里刚评完，回到这里要看到
  onShow() {
    const all = orders.getOrders();
    const list = all.filter(o => o.rating).map(o => ({
      id: o.id,
      shop: data.findShop(o.shopId),
      stars: o.rating.stars,
      tags: o.rating.tags || [],
      text: o.rating.text,
      time: o.rating.time,
      dishes: o.items.map(i => i.name).join('、')
    }));
    const avg = list.length ? (list.reduce((n, r) => n + r.stars, 0) / list.length).toFixed(1) : '';
    this.setData({ list, avg, pending: orders.counts().rate });
  },
  onDetail(e) {
    wx.navigateTo({ url: `/pages/order-detail/order-detail?id=${e.currentTarget.dataset.id}` });
  },
  onShop(e) {
    wx.navigateTo({ url: `/pages/shop/shop?id=${e.currentTarget.dataset.shop}` });
  },
  // 待评价的订单在「订单」tab 的「待评价」分类里（switchTab 带不了参数，放在 globalData）
  onPending() {
    getApp().globalData.orderTab = 'rate';
    wx.switchTab({ url: '/pages/orders/orders' });
  }
});
