const store = require('../../utils/store.js');
Page({
  data: {
    tabs: [],
    tab: 'valid',
    list: [],
    code: ''
  },
  onShow() {
    this.load();
  },
  load() {
    const all = store.allCoupons();
    const n = k => all.filter(c => c.status === k).length;
    this.setData({
      tabs: [
        { key: 'valid', name: `可使用(${n('valid')})` },
        { key: 'used', name: `已使用(${n('used')})` },
        { key: 'expired', name: `已过期(${n('expired')})` }
      ],
      list: all.filter(c => c.status === this.data.tab)
    });
  },
  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.load();
  },
  onCode(e) {
    this.setData({ code: e.detail.value });
  },
  onRedeem() {
    const code = this.data.code.trim().toUpperCase();
    if (!code) return;
    if (code !== 'SHIGUANG') {
      wx.showToast({ title: '兑换码无效或已过期', icon: 'none' });
      return;
    }
    if (store.allCoupons().some(c => c.id === 'x1')) {
      wx.showToast({ title: '该兑换码已使用过', icon: 'none' });
      return;
    }
    store.grantCoupon({ id: 'x1', amount: 8, title: '拾光专享 8 元券', cond: '兑换码专享，全场通用', min: 15, expire: '2024-04-30' });
    this.setData({ code: '' });
    wx.showToast({ title: '兑换成功', icon: 'success' });
    this.load();
  },
  onUse(e) {
    wx.setStorageSync('cf_pick_coupon', e.currentTarget.dataset.id);
    // 购物车里已经有饮品就直接去结算，否则先去点单
    if (store.getCart().length) wx.navigateTo({ url: '/pages/cart/cart' });
    else wx.switchTab({ url: '/pages/menu/menu' });
  }
});
