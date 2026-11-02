const F = require('../../utils/fmt.js');
const store = require('../../utils/store.js');
const TABS = [
  { id: 'all', name: '全部' }, { id: 'unpaid', name: '待支付' }, { id: 'paid', name: '待观演' }, { id: 'done', name: '退票/取消' }
];
Page({
  data: { tabs: TABS, tab: 'all', list: [], counts: {} },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1, badge: store.orders().filter((o) => o.status === 'unpaid').length });
    this.load();
  },
  load() {
    const now = Date.now();
    const all = store.orders().map((o) => {
      if (o.status === 'unpaid' && now > o.expireAt) { o.status = 'cancelled'; o.cancelReason = '超时未支付'; store.upsertOrder(o); }
      return o;
    });
    const match = (o, t) => t === 'all' || (t === 'done' ? (o.status === 'refunded' || o.status === 'cancelled') : o.status === t);
    const counts = {};
    TABS.forEach((t) => { counts[t.id] = all.filter((o) => match(o, t.id)).length; });
    const list = all.filter((o) => match(o, this.data.tab)).map((o) => ({
      id: o.id, title: o.title, poster: o.poster, ses: o.sessionText, venue: o.city + ' · ' + o.venue,
      n: o.items.length, amount: o.amount, status: o.status, statusText: F.STATUS[o.status]
    }));
    this.setData({ list, counts });
  },
  pick(e) { this.setData({ tab: e.currentTarget.dataset.id }); this.load(); },
  open(e) { wx.navigateTo({ url: '/pages/ticket/ticket?id=' + e.currentTarget.dataset.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
