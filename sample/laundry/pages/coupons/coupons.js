const store = require('../../utils/store.js');

function claimables() {
  const y = new Date().getFullYear();
  return [
    { id: 'cp5', title: '换季满 149 减 30', amount: 30, min: 149, expire: y + '-12-31', scope: '全场通用' },
    { id: 'cp6', title: '无门槛 5 元券', amount: 5, min: 0, expire: y + '-12-31', scope: '全场通用' }
  ];
}

Page({
  data: { cur: 'ok', tabs: [], list: [], claim: [] },

  onShow() { this.render(); },

  render() {
    const all = store.coupons();
    const has = {};
    all.forEach(c => { has[c.id] = true; });
    const names = { ok: '可使用', used: '已使用', expired: '已过期' };
    this.setData({
      tabs: ['ok', 'used', 'expired'].map(k => ({ key: k, name: names[k], n: all.filter(c => c.state === k).length })),
      list: all.filter(c => c.state === this.data.cur).sort((a, b) => b.amount - a.amount),
      claim: claimables().filter(c => !has[c.id])
    });
  },

  onTab(e) { this.setData({ cur: e.currentTarget.dataset.key }); this.render(); },

  take(e) {
    const c = claimables().find(x => x.id === e.currentTarget.dataset.id);
    if (c && store.addCoupon(c)) wx.showToast({ title: '领取成功', icon: 'success' });
    this.render();
  },

  use() { wx.switchTab({ url: '/pages/price/price' }); }
});
