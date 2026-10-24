const data = require('../../utils/data.js');

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

  all() {
    const used = wx.getStorageSync('cf_used_coupons') || [];
    const extra = wx.getStorageSync('cf_extra_coupons') || [];
    return extra.concat(data.coupons.map(c => (used.indexOf(c.id) >= 0 ? Object.assign({}, c, { status: 'used' }) : c)));
  },

  load() {
    const all = this.all();
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
    const extra = wx.getStorageSync('cf_extra_coupons') || [];
    if (code !== 'SHIGUANG') {
      wx.showToast({ title: '兑换码无效或已过期', icon: 'none' });
      return;
    }
    if (extra.some(c => c.id === 'x1')) {
      wx.showToast({ title: '该兑换码已使用过', icon: 'none' });
      return;
    }
    extra.push({ id: 'x1', amount: 8, unit: '元', title: '拾光专享 8 元券', cond: '兑换码专享，全场通用', min: 15, expire: '2024-04-30', status: 'valid' });
    wx.setStorageSync('cf_extra_coupons', extra);
    this.setData({ code: '' });
    wx.showToast({ title: '兑换成功', icon: 'success' });
    this.load();
  },

  onUse(e) {
    wx.setStorageSync('cf_pick_coupon', e.currentTarget.dataset.id);
    wx.switchTab({ url: '/pages/menu/menu' });
  }
});
