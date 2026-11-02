const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');

Page({
  data: {
    tabs: orders.TABS,
    tab: 'all',
    list: []
  },

  onLoad(q) {
    if (q && q.tab && orders.TABS.some(t => t.key === q.tab)) this.setData({ tab: q.tab });
  },

  onShow() {
    this.load();
  },

  load() {
    const now = Date.now();
    const list = orders.getOrders().map(o => {
      const st = orders.stageOf(o, now);
      return {
        id: o.id,
        st,
        imgs: o.items.slice(0, 4).map(i => i.img),
        summary: o.items.length === 1 ? `${o.items[0].name} ${o.items[0].spec}` : '',
        count: o.count,
        pay: o.pay,
        refund: o.refund || 0,
        time: store.dateTime(o.createdAt),
        match: orders.match(this.data.tab, o, st)
      };
    }).filter(x => x.match);
    this.setData({ list });
  },

  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.load();
  },

  onOpen(e) {
    wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id });
  },

  onPay(e) {
    wx.navigateTo({ url: '/pages/order-detail/order-detail?pay=1&id=' + e.currentTarget.dataset.id });
  },

  onRate(e) {
    wx.navigateTo({ url: '/pages/order-detail/order-detail?rate=1&id=' + e.currentTarget.dataset.id });
  },

  onConfirm(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '确认收货',
      content: '确认已经收到全部商品了吗？',
      confirmText: '确认收货',
      success: r => {
        if (!r.confirm) return;
        orders.confirmOrder(id);
        wx.showToast({ title: '已确认收货，来评价一下吧', icon: 'none' });
        this.load();
      }
    });
  },

  onAgain(e) {
    const o = orders.findOrder(e.currentTarget.dataset.id);
    if (!o) return;
    const r = orders.reorder(o);
    if (!r.lines) {
      wx.showToast({ title: '商品已下架或超出限购', icon: 'none' });
      return;
    }
    wx.switchTab({ url: '/pages/cart/cart' });
    if (r.limited) wx.showToast({ title: '秒杀商品已按限购加入', icon: 'none' });
  },

  onCall() {
    host.call(orders.RIDER.phone);
  },

  onGoShop() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
