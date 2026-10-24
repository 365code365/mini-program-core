const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

const STATUS = { delivering: '骑手配送中', done: '已完成', refund: '已退款' };

Page({
  data: {
    tabs: [
      { key: 'all', name: '全部' },
      { key: 'delivering', name: '进行中' },
      { key: 'done', name: '已完成' },
      { key: 'refund', name: '退款' }
    ],
    tab: 'all',
    list: []
  },

  onShow() {
    this.load();
  },

  load() {
    const tab = this.data.tab;
    const list = cart.getOrders()
      .filter(o => tab === 'all' || o.status === tab)
      .map(o => ({
        id: o.id,
        shopId: o.shopId,
        shop: data.findShop(o.shopId),
        status: o.status,
        statusText: STATUS[o.status] || '已完成',
        summary: o.items.map(i => `${i.name} ×${i.qty}`).join('、'),
        count: o.items.reduce((n, i) => n + i.qty, 0),
        pay: o.pay,
        time: cart.fmtTime(o.createdAt)
      }));
    this.setData({ list });
  },

  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.load();
  },

  onDetail(e) {
    wx.navigateTo({ url: `/pages/order-detail/order-detail?id=${e.currentTarget.dataset.id}` });
  },

  onAgain(e) {
    wx.navigateTo({ url: `/pages/shop/shop?id=${e.currentTarget.dataset.shop}` });
  },

  onReview() {
    wx.showActionSheet({
      itemList: ['★★★★★ 超赞', '★★★★ 满意', '★★★ 一般', '★★ 不太满意'],
      success: () => wx.showToast({ title: '感谢评价，红包 +1', icon: 'none' })
    });
  },

  onGoHome() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
