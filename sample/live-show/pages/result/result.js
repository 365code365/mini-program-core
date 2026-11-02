const store = require('../../utils/store.js');
Page({
  data: { o: null, count: 0 },
  onLoad(q) {
    const o = store.getOrder(q.id);
    this.setData({ o, count: o ? o.items.length : 0 });
  },
  toTicket() {
    if (!this.data.o) return; wx.redirectTo({ url: '/pages/ticket/ticket?id=' + this.data.o.id }); },
  toWallet() { wx.switchTab({ url: '/pages/wallet/wallet' }); },
  toHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
