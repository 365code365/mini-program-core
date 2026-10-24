const app = getApp();
const DEFAULT_FROM = { name: '望京SOHO T1-东门', addr: '北京市朝阳区阜通东大街6号', km: 0 };

Page({
  data: {
    types: app.globalData.CAR_TYPES,
    carType: 'express',
    typeName: '快车',
    from: DEFAULT_FROM,
    to: null,
    km: 0,
    min: 0,
    price: 0,
    services: [
      { name: '代驾', glyph: '代', bg: 'linear-gradient(135deg, #6BA8FF, #3D7BFF)' },
      { name: '顺风车', glyph: '顺', bg: 'linear-gradient(135deg, #5FD99A, #19BE6B)' },
      { name: '企业用车', glyph: '企', bg: 'linear-gradient(135deg, #B28CFF, #7B5CFF)' },
      { name: '租车', glyph: '租', bg: 'linear-gradient(135deg, #FFC35C, #FF9A1A)' }
    ]
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    const from = wx.getStorageSync('ride_from') || DEFAULT_FROM;
    const to = wx.getStorageSync('ride_to') || null;
    this.setData({ from, to });
    this.recalc();
  },
  recalc() {
    const to = this.data.to;
    if (!to) return;
    const km = Math.max(1.5, Math.abs((to.km || 8) - (this.data.from.km || 0)));
    const kmR = Math.round(km * 10) / 10;
    const min = Math.round(km * 2.2 + 6);
    const price = Math.max(0, app.estimate(this.data.carType, kmR, min) - 5).toFixed(1);
    const t = this.data.types.find(x => x.key === this.data.carType);
    this.setData({ km: kmR, min, price, typeName: t.name });
  },
  onType(e) {
    this.setData({ carType: e.currentTarget.dataset.key });
    this.recalc();
    if (!this.data.to) {
      const t = this.data.types.find(x => x.key === e.currentTarget.dataset.key);
      this.setData({ typeName: t.name });
    }
  },
  onPick(e) {
    wx.navigateTo({ url: '/pages/place/place?type=' + e.currentTarget.dataset.type });
  },
  onCall() {
    wx.setStorageSync('ride_order', {
      from: this.data.from, to: this.data.to, type: this.data.typeName,
      km: this.data.km, min: this.data.min, price: this.data.price
    });
    wx.navigateTo({ url: '/pages/calling/calling' });
  },
  onLocate() {
    wx.setStorageSync('ride_from', DEFAULT_FROM);
    this.setData({ from: DEFAULT_FROM });
    this.recalc();
    wx.showToast({ title: '已定位到当前位置', icon: 'none' });
  },
  onService(e) {
    wx.showToast({ title: e.currentTarget.dataset.name + '即将开放', icon: 'none' });
  },
  onCoupons() {
    wx.navigateTo({ url: '/pages/coupons/coupons' });
  }
});
