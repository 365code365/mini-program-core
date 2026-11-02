// 我的：发布 / 卖出 / 买到 / 收藏，订单状态入口，曝光统计，清理搜索历史
const store = require('../../utils/store.js');
const D = require('../../utils/data.js');
Page({
  data: { me: D.ME, stat: {}, orderCounts: {}, expose: 0, earned: 0 },
  onShow() {
    store.syncTab(this, 3);
    const mine = store.mine(), orders = store.orders();
    const cnt = s => orders.filter(o => o.status === s).length;
    const sold = mine.filter(m => m.status === 'sold');
    this.setData({
      stat: { on: mine.filter(m => m.status === 'on').length, sold: sold.length, bought: orders.filter(o => o.status === 'done').length, fav: store.favs().length },
      orderCounts: { paid: cnt('paid'), shipped: cnt('shipped'), done: cnt('done'), closed: cnt('closed') },
      expose: store.exposeTotal(), earned: sold.reduce((s, m) => s + m.price, 0)
    });
  },
  goMyItems() { wx.navigateTo({ url: '/pages/my-items/my-items' }); },
  goFavs() { wx.navigateTo({ url: '/pages/favorites/favorites' }); },
  goOrders(e) { wx.navigateTo({ url: '/pages/orders/orders?tab=' + (e.currentTarget.dataset.tab || 'all') }); },
  goMessages() { wx.switchTab({ url: '/pages/messages/messages' }); },
  clearHistory() {
    wx.showModal({
      title: '清除搜索历史', content: '确定清除全部搜索记录吗？',
      success: r => { if (r.confirm) { store.set(store.K.history, []); wx.showToast({ title: '已清除', icon: 'none' }); } }
    });
  },
  rules() {
    wx.showModal({ title: '交易规则', content: '1. 担保交易：确认收货前货款由平台保管。\n2. 宝贝与描述严重不符可申请退款。\n3. 请勿脱离平台私下转账。', showCancel: false });
  },
  onShareAppMessage() { return { title: '闲转 · 闲置一拍就能卖', path: '/pages/index/index' }; }
});
