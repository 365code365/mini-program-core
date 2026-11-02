// 行程记录：全部 / 单车 / 电单车，待支付的行程点进去直接结算
const fee = require('../../utils/fee.js');
const store = require('../../utils/store.js');

Page({
  data: {
    tabs: [{ k: 'all', t: '全部' }, { k: 'bike', t: '单车' }, { k: 'ebike', t: '电单车' }],
    tab: 'all',
    loading: true,
    list: [],
    sum: { count: 0, km: '0.0', spend: '0.00' }
  },

  onShow() {
    this.setData({ loading: true });
    // 模拟从服务端拉取的等待，给加载态一个出场机会
    clearTimeout(this._t);
    this._t = setTimeout(() => this.load(), 350);
  },
  onUnload() { clearTimeout(this._t); },

  load() {
    const all = store.trips();
    const list = all.filter(t => this.data.tab === 'all' || t.type === this.data.tab).map(t => ({
      id: t.id, type: t.type, status: t.status,
      name: fee.PRICING[t.type].name,
      date: store.fmtTime(t.startAt),
      from: t.from, to: t.to,
      dur: fee.fmtMin(t.sec),
      km: (t.meters / 1000).toFixed(2),
      paid: t.paid.toFixed(2)
    }));
    const km = all.reduce((s, t) => s + t.meters, 0) / 1000;
    const spend = all.filter(t => t.status === 'paid').reduce((s, t) => s + t.paid, 0);
    this.setData({ loading: false, list, sum: { count: all.length, km: km.toFixed(1), spend: spend.toFixed(2) } });
  },

  onTab(e) {
    const k = e.currentTarget.dataset.k;
    if (k === this.data.tab) return;
    this.setData({ tab: k });
    this.load();
  },

  onItem(e) {
    const { id, status } = e.currentTarget.dataset;
    wx.navigateTo({ url: (status === 'unpaid' ? '/pages/settle/settle?id=' : '/pages/trip/trip?id=') + id });
  },

  onPullDownRefresh() { this.load(); wx.stopPullDownRefresh(); },
  goHome() { wx.reLaunch({ url: '/pages/index/index' }); }
});
