const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { CATS, STORES, HOTLINE } = require('../../utils/data.js');

Page({
  data: {
    cars: [], current: 0, car: null, cats: CATS, near: [], locName: '定位中…',
    pending: null
  },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 0 });
    const cars = db.getCars();
    let current = cars.findIndex((c) => c.isDefault);
    if (current < 0) current = 0;
    // 最近一单待服务的预约，首页直接提醒
    const pending = db.getOrders().find((o) => o.status === 'pending' || o.status === 'serving') || null;
    this.setData({ cars, current, car: cars[current] || null, pending });
    util.getLocation((loc) => {
      const near = STORES.map((s) => Object.assign({}, s, { dist: util.distanceKm(loc, s) }))
        .sort((a, b) => a.dist - b.dist).slice(0, 2);
      this.setData({ near, locName: loc.ok ? '当前位置附近' : loc.name });
    });
  },
  onCarChange(e) {
    const i = e.detail.current;
    this.setData({ current: i, car: this.data.cars[i] || null });
  },
  goCar(e) { wx.navigateTo({ url: '/pages/car-edit/car-edit?id=' + e.currentTarget.dataset.id }); },
  addCar() { wx.navigateTo({ url: '/pages/car-edit/car-edit' }); },
  goGarage() { wx.navigateTo({ url: '/pages/garage/garage' }); },
  bookCare() {
    const car = this.data.car;
    wx.navigateTo({ url: '/pages/service/service?cat=care' + (car ? '&carId=' + car.id : '') });
  },
  goCat(e) {
    const car = this.data.car;
    wx.navigateTo({ url: '/pages/service/service?cat=' + e.currentTarget.dataset.id + (car ? '&carId=' + car.id : '') });
  },
  goViolation() { wx.navigateTo({ url: '/pages/violation/violation' + (this.data.car ? '?carId=' + this.data.car.id : '') }); },
  rescue() {
    wx.showActionSheet({
      itemList: ['拨打 24 小时救援热线', '搭电 / 换备胎（就近门店上门）'],
      success: (r) => {
        if (r.tapIndex === 0) util.callPhone(HOTLINE, '24 小时道路救援');
        else util.callPhone(this.data.near.length ? this.data.near[0].phone : STORES[0].phone, '就近门店');
      }
    });
  },
  goStore(e) { wx.navigateTo({ url: '/pages/store/store?id=' + e.currentTarget.dataset.id }); },
  goStores() { wx.switchTab({ url: '/pages/stores/stores' }); },
  goPending() { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + this.data.pending.id }); }
});
