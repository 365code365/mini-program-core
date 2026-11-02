const db = require('../../utils/db.js');
const { SERVICES, STAFF } = require('../../utils/data.js');

// 限时优惠：深度保洁券当月底截止
const PROMO_END = new Date(2026, 10, 30, 23, 59, 59);

Page({
  data: { services: SERVICES, staff: STAFF, promo: '', active: null, couponGot: false },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 });
    const active = db.getOrders().find((o) => o.status === 'waiting' || o.status === 'serving') || null;
    const got = db.getCoupons().some((c) => c.id === 'cp3');
    this.setData({ active, couponGot: got });
    this.tick();
    this.timer = setInterval(() => this.tick(), 60000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },
  tick() {
    const ms = PROMO_END - Date.now();
    if (ms <= 0) { this.setData({ promo: '活动已结束' }); return; }
    const d = Math.floor(ms / 86400000), h = Math.floor(ms / 3600000) % 24;
    this.setData({ promo: '距结束 ' + d + ' 天 ' + h + ' 小时' });
  },
  goSvc(e) { wx.navigateTo({ url: '/pages/service/service?id=' + e.currentTarget.dataset.id }); },
  goPromo() { wx.navigateTo({ url: '/pages/service/service?id=deep' }); },
  // 指定阿姨：进她最擅长的服务，下单时优先派给她
  goStaff(e) {
    const s = STAFF.find((x) => x.id === e.currentTarget.dataset.id);
    wx.navigateTo({ url: '/pages/service/service?id=' + s.skills[0] + '&staff=' + s.id });
  },
  goActive() { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + this.data.active.id }); },
  goCoupons() { wx.navigateTo({ url: '/pages/coupons/coupons' }); }
});
