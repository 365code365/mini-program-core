const store = require('../../utils/store.js');
const ORD = require('../../utils/order.js');
Page({
  data: { o: null },
  onLoad(q) {
    const o = store.getOrder(q.id);
    if (o) this.setData({ o: ORD.view(o) });
  },
  toPass() { wx.redirectTo({ url: '/pages/pass/pass?id=' + this.data.o.id }); },
  toOrder() { wx.redirectTo({ url: '/pages/order/order?id=' + this.data.o.id }); },
  toTeam() { wx.navigateTo({ url: '/pages/team-create/team-create?order=' + this.data.o.id }); },
  toHome() { wx.switchTab({ url: '/pages/index/index' }); },
  toOrders() { wx.redirectTo({ url: '/pages/orders/orders' }); },
  onShareAppMessage() {
    const o = this.data.o;
    if (!o) return { title: '约场：附近球馆空场一键订', path: '/pages/index/index' };
    return { title: '我订了' + o.venueName + ' ' + o.dateText + '，一起来打' + o.sportName + '！', path: '/pages/venue/venue?id=' + o.venueId, imageUrl: o.photo };
  }
});
