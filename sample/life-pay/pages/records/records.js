const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { filters: [{ k: 'all', name: '全部' }].concat(data.types.map(t => ({ k: t.k, name: t.name }))), filter: 'all', groups: [], total: '0.00', count: 0 },
  onShow() { this.refresh(); },
  refresh() {
    const f = this.data.filter;
    const list = store.getRecords().filter(r => f === 'all' || r.type === f).sort((a, b) => (a.time < b.time ? 1 : -1));
    // 按「年-月」分组，组内带月合计
    const groups = [];
    const byKey = {};
    list.forEach(r => {
      const key = r.time.slice(0, 7);
      if (!byKey[key]) {
        byKey[key] = { key, title: Number(key.slice(0, 4)) + ' 年 ' + Number(key.slice(5, 7)) + ' 月', sum: 0, items: [] };
        groups.push(byKey[key]);
      }
      const t = data.typeOf(r.type);
      byKey[key].sum += r.paid;
      byKey[key].items.push(Object.assign({}, r, { ch: t.ch, bg: t.bg, day: r.time.slice(5, 16),
        targetTx: r.type === 'phone' ? r.target : store.fmtNo(r.target), paidTx: store.money(r.paid), saved: r.amount > r.paid ? store.money(r.amount - r.paid) : '' }));
    });
    groups.forEach(g => { g.sumTx = store.money(g.sum); });
    this.setData({ groups, count: list.length, total: store.money(list.reduce((s, r) => s + r.paid, 0)) });
  },
  onFilter(e) { this.setData({ filter: e.currentTarget.dataset.k }); this.refresh(); },
  detail(e) { wx.navigateTo({ url: '/pages/result/result?id=' + e.currentTarget.dataset.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
