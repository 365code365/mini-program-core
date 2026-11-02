const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');
Page({
  data: {
    phone: data.SERVICE_PHONE,
    faqs: data.faqs.map((f, i) => ({ i, q: f.q, a: f.a, open: false })),
    recent: null
  },
  onShow() {
    // 最近一单：客服最常处理的就是它（催单、退款、改地址）
    const o = orders.getOrders()[0];
    if (!o) {
      this.setData({ recent: null });
      return;
    }
    const shop = data.findShop(o.shopId);
    const st = orders.stageOf(o);
    this.setData({
      recent: {
        id: o.id, logo: shop.logo, c1: shop.c1, c2: shop.c2, name: shop.name, status: st.text,
        summary: o.items.map(i => `${i.name} ×${i.qty}`).join('、'), time: store.fmtTime(o.createdAt), pay: o.pay
      }
    });
  },
  onFaq(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ [`faqs[${i}].open`]: !this.data.faqs[i].open });
  },
  onOrder() {
    wx.navigateTo({ url: `/pages/order-detail/order-detail?id=${this.data.recent.id}` });
  },
  onAllOrders() {
    getApp().globalData.orderTab = 'all';
    wx.switchTab({ url: '/pages/orders/orders' });
  },
  onCall() {
    host.call(this.data.phone, '饱了么客服');
  }
});
