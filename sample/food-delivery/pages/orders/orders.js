const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');

const TABS = [
  { key: 'all', name: '全部' },
  { key: 'unpaid', name: '待付款' },
  { key: 'going', name: '进行中' },
  { key: 'rate', name: '待评价' },
  { key: 'refund', name: '取消/退款' }
];

function match(tab, o) {
  if (tab === 'unpaid') return o.status === 'unpaid';
  if (tab === 'going') return o.status === 'paid';
  if (tab === 'rate') return o.status === 'done' && !o.rating;
  if (tab === 'refund') return o.status === 'refund' || o.status === 'cancelled';
  return true;
}

Page({
  data: {
    tabs: TABS,
    tab: 'all',
    list: []
  },

  onLoad(q) {
    if (q && q.tab && TABS.some(t => t.key === q.tab)) this.setData({ tab: q.tab });
  },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    // 「我的」页的订单入口是 switchTab 过来的（带不了参数），要看的分类放在 globalData 里
    const app = getApp();
    if (app.globalData.orderTab) {
      this.setData({ tab: app.globalData.orderTab });
      app.globalData.orderTab = '';
    }
    this.load();
    clearInterval(this.timer);
    // 有待支付的单要走倒计时，有配送中的单要跟着推进
    this.timer = setInterval(() => { if (this.live) this.load(); }, 1000);
  },

  onHide() {
    clearInterval(this.timer);
  },

  onUnload() {
    clearInterval(this.timer);
  },

  load() {
    const now = Date.now();
    const all = orders.getOrders();
    this.live = all.some(o => o.status === 'unpaid' || o.status === 'paid');
    const list = all.filter(o => match(this.data.tab, o)).map(o => {
      const st = orders.stageOf(o, now);
      return {
        id: o.id,
        shopId: o.shopId,
        shop: data.findShop(o.shopId),
        status: o.status,
        statusText: st.text,
        summary: o.items.map(i => `${i.name} ×${i.qty}`).join('、'),
        count: o.count,
        pay: o.pay,
        time: store.fmtTime(o.createdAt),
        left: st.canPay ? orders.mmss(st.left) : '',
        canRate: st.canRate
      };
    });
    this.setData({ list });
  },

  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.load();
  },

  onDetail(e) {
    wx.navigateTo({ url: `/pages/order-detail/order-detail?id=${e.currentTarget.dataset.id}` });
  },

  onRate(e) {
    wx.navigateTo({ url: `/pages/order-detail/order-detail?rate=1&id=${e.currentTarget.dataset.id}` });
  },

  onAgain(e) {
    const o = orders.findOrder(e.currentTarget.dataset.id);
    if (!o) return;
    const n = orders.reorder(o);
    wx.showToast({ title: n ? `已把 ${n} 件商品放回购物车` : '商品已下架', icon: 'none' });
    wx.navigateTo({ url: `/pages/shop/shop?id=${o.shopId}` });
  },

  onGoHome() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
