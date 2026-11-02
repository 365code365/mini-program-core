const store = require('../../utils/store.js');

Page({
  data: { o: null },
  onLoad(q) {
    const o = store.order(q && q.no);
    this.setData({ o });
    if (o) wx.vibrateShort({ type: 'light' });
  },
  goTicket() { wx.redirectTo({ url: '/pages/ticket/ticket?no=' + this.data.o.no }); },
  goOrder() { wx.redirectTo({ url: '/pages/order-detail/order-detail?no=' + this.data.o.no }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  goGuide() { wx.switchTab({ url: '/pages/guide/guide' }); }
});
