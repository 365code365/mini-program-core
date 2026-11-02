const D = require('../../utils/date');
const store = require('../../utils/store');
const order = require('../../utils/order');
const app = getApp();

const TABS = ['全部', '待支付', '待出行', '已取消/退票'];

Page({
  data: { tabs: TABS, tab: 0, list: [], loading: true },
  onShow() {
    app.syncTab(this, 1);
    this.setData({ loading: true });
    clearTimeout(this._t);
    this._t = setTimeout(() => this.load(), 250);
  },
  onHide() { clearTimeout(this._t); },
  load() {
    // 超时未付的先关掉，再按 tab 过滤
    store.orders().forEach(o => { if (order.expired(o)) order.cancel(o.id); });
    const t = this.data.tab, today = D.today();
    const list = store.orders().filter(o => t === 0 ||
      (t === 1 && o.status === 'unpaid') ||
      (t === 2 && (o.status === 'paid' || o.status === 'changed') && o.legs[0].flight.date >= today) ||
      (t === 3 && (o.status === 'refunded' || o.status === 'cancelled'))
    ).map(o => {
      const f = o.legs[0].flight;
      return { id: o.id, status: o.status, statusText: order.STATUS[o.status], total: o.total,
        route: f.fromName + (o.legs.length > 1 ? ' ⇌ ' : ' → ') + f.toName, trip: o.legs.length > 1 ? '往返' : '单程',
        when: D.md(f.date) + ' ' + D.weekLabel(f.date) + ' ' + f.depText, no: f.airlineName + f.no, pax: o.pax.map(p => p.name).join('、') };
    });
    this.setData({ list, loading: false });
    app.syncTab(this, 1);
  },
  setTab(e) { this.setData({ tab: +e.currentTarget.dataset.i }); this.load(); },
  open(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  goBook() { wx.switchTab({ url: '/pages/index/index' }); }
});
