Page({
  data: {
    balance: '126.50', couponCount: 6, autoShare: true,
    menus: [
      { name: '我的优惠券', glyph: '券', bg: 'linear-gradient(135deg, #FFB067, #FF7A1A)', extra: '6 张可用', url: '/pages/coupons/coupons' },
      { name: '我的行程', glyph: '程', bg: 'linear-gradient(135deg, #6BA8FF, #3D7BFF)', extra: '', tab: '/pages/trips/trips' },
      { name: '常用地址', glyph: '址', bg: 'linear-gradient(135deg, #5FD99A, #19BE6B)', extra: '家 · 公司' },
      { name: '紧急联系人', glyph: '安', bg: 'linear-gradient(135deg, #FF8A8A, #F5455C)', extra: '1 位' },
      { name: '电子发票', glyph: '票', bg: 'linear-gradient(135deg, #B28CFF, #7B5CFF)', extra: '' },
      { name: '客服中心', glyph: '服', bg: 'linear-gradient(135deg, #7ED6E8, #26A8C7)', extra: '7×24 小时' }
    ]
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    const used = wx.getStorageSync('ride_coupon_used') || [];
    this.setData({ couponCount: 6 - used.length, autoShare: wx.getStorageSync('ride_autoshare') !== false });
  },
  onMenu(e) {
    const m = this.data.menus[e.currentTarget.dataset.idx];
    if (m.url) wx.navigateTo({ url: m.url });
    else if (m.tab) wx.switchTab({ url: m.tab });
    else wx.showToast({ title: m.name, icon: 'none' });
  },
  onCoupons() { wx.navigateTo({ url: '/pages/coupons/coupons' }); },
  onWallet() {
    wx.showActionSheet({ itemList: ['充值 50 元', '充值 100 元', '充值 200 元'], success: (r) => {
      const add = [50, 100, 200][r.tapIndex];
      this.setData({ balance: (parseFloat(this.data.balance) + add).toFixed(2) });
      wx.showToast({ title: '充值成功', icon: 'success' });
    } });
  },
  onShare(e) { wx.setStorageSync('ride_autoshare', e.detail.value); }
});
