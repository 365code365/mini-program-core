const store = require('../../utils/store.js');
const util = require('../../utils/util.js');
Page({
  data: { p: null, premiumText: '' },
  onLoad(q) {
    const p = store.getPolicy(q.id);
    if (p) this.setData({ p, premiumText: util.money(p.premium) });
    this.animate('.rs-ic', [{ scale: [0.3, 0.3], opacity: 0 }, { scale: [1.15, 1.15], opacity: 1 }, { scale: [1, 1], opacity: 1 }], 500);
  },
  viewPolicy() { wx.redirectTo({ url: '/pages/policy/policy?id=' + this.data.p.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  copyNo() { wx.setClipboardData({ data: this.data.p.policyNo }); }
});
