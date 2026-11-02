const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { tabs: ['全部', '待支付', '已完成'], tIdx: 0, all: [], shown: [], session: null, sum: { count: 0, kwh: '0', co2: '0' }, canInvoice: 0 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    const all = store.getOrders().map(o => Object.assign({}, o, {
      stationName: data.getStation(o.station).name,
      payText: o.pay.toFixed(2),
      stText: o.status === 'unpaid' ? '待支付' : o.invoiced ? '已开票' : '已完成'
    }));
    const s = store.getSession();
    this.setData({
      all,
      session: s ? { no: s.no, stationName: data.getStation(s.station).name } : null,
      sum: store.stats(),
      canInvoice: store.invoiceable().length
    });
    this.apply();
  },
  apply() {
    const t = this.data.tIdx;
    const shown = this.data.all.filter(o => t === 0 || (t === 1 ? o.status === 'unpaid' : o.status === 'paid'));
    this.setData({ shown });
  },
  onTab(e) { this.setData({ tIdx: Number(e.currentTarget.dataset.idx) }); this.apply(); },
  onSession() { wx.navigateTo({ url: '/pages/charging/charging' }); },
  onOrder(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  onInvoice() { wx.navigateTo({ url: '/pages/invoice/invoice' }); }
});
