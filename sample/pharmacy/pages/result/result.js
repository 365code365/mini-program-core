// 支付结果：订单号、实付、预计送达；可直接给订单里的药设用药提醒
const S = require('../../utils/store.js');

Page({
  data: { o: null, eta: '' },
  onLoad(q) {
    const o = S.get('orders', []).find(x => x.id === q.id);
    if (!o) { this.setData({ o: null }); return; }
    const t = new Date(o.createdAt + (o.delivery === '即时达' ? 30 : 24 * 60) * 60000);
    const eta = o.delivery === '即时达'
      ? '预计 ' + ('0' + t.getHours()).slice(-2) + ':' + ('0' + t.getMinutes()).slice(-2) + ' 前送达'
      : '预计 ' + (t.getMonth() + 1) + ' 月 ' + t.getDate() + ' 日送达';
    this.setData({ o, eta, meds: o.items.filter(i => !/器|口罩|血压计|血氧/.test(i.name)) });
  },
  goOrders() { wx.redirectTo({ url: '/pages/orders/orders' }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  remind(e) { wx.navigateTo({ url: '/pages/remind-edit/remind-edit?drug=' + encodeURIComponent(e.currentTarget.dataset.name) }); }
});
