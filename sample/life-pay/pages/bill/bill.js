const store = require('../../utils/store.js');
// 账单明细 / 查询欠费：当期账单拆分（用量 × 单价）+ 近 6 期账单柱状图 + 本户号的缴费记录
Page({
  data: { a: null, bills: [], cur: null, recs: [], maxAmt: 1, queriedAt: '' },
  onLoad(q) { this.aid = q.id; },
  onShow() { this.refresh(); },
  refresh() {
    const all = store.getAccounts().map(store.view);
    const a = all.find(x => x.id === this.aid) || all[0];
    if (!a) { this.setData({ a: null }); return; }
    this.aid = a.id;
    wx.setNavigationBarTitle({ title: a.tname + '账单' });
    const bills = store.bills(a);
    const max = Math.max.apply(null, bills.map(b => b.amt)) || 1;
    bills.forEach(b => { b.h = Math.max(8, Math.round(b.amt / max * 100)); b.short = b.ym.replace(/^\d+年/, ''); });
    const recs = store.getRecords().filter(r => r.accId === a.id || (r.type === a.type && r.target === a.no)).slice(0, 5)
      .map(r => ({ id: r.id, time: r.time, paidTx: store.money(r.paid) }));
    this.setData({ a, bills: bills.slice().reverse(), cur: bills[0], recs, queriedAt: wx.getStorageSync('lp_query_' + a.id) || '' });
  },
  // 重新查询：去缴费单位拉一次最新欠费（离线演示：结果不变，记下查询时间）
  query() {
    wx.showLoading({ title: '查询中' });
    setTimeout(() => {
      wx.hideLoading();
      const t = store.stamp(new Date());
      wx.setStorageSync('lp_query_' + this.aid, t);
      this.refresh();
      const a = this.data.a;
      wx.showToast({ title: a.owe > 0 ? '查到欠费 ¥' + a.oweTx : '已是最新，暂无欠费', icon: 'none' });
    }, 400);
  },
  pay() { wx.redirectTo({ url: '/pages/pay/pay?id=' + this.aid }); },
  rec(e) { wx.navigateTo({ url: '/pages/result/result?from=list&id=' + e.currentTarget.dataset.id }); }
});
