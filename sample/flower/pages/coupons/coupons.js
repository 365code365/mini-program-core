const store = require('../../utils/store.js');
Page({
  data: { tabs: [], tab: 'valid', list: [] },
  onShow() { this.load(); },
  load() {
    const all = store.allCoupons();
    const n = k => all.filter(c => c.status === k).length;
    this.setData({
      tabs: [{ k: 'valid', n: '可使用(' + n('valid') + ')' }, { k: 'used', n: '已使用(' + n('used') + ')' }, { k: 'expired', n: '已过期(' + n('expired') + ')' }],
      list: all.filter(c => c.status === this.data.tab)
    });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.load(); },
  // 结算时会自动挑最省的一张；购物车里有花就直接去购物车，否则先去挑花
  onUse() {
    if (store.getCart().length) wx.switchTab({ url: '/pages/cart/cart' });
    else wx.switchTab({ url: '/pages/index/index' });
  }
});
