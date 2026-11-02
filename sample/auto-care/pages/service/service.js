const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { STORES, SERVICES, CATS } = require('../../utils/data.js');

Page({
  data: { cats: CATS, cat: 'care', list: [], picked: {}, count: 0, total: 0, mins: 0, store: null, car: null },
  onLoad(q) {
    const picked = {};
    let cat = q.cat || 'care';
    if (q.sid) { picked[q.sid] = true; const s = SERVICES.find((v) => v.id === q.sid); if (s) cat = s.cat; }
    const car = (q.carId && db.getCar(q.carId)) || db.defaultCar();
    this.setData({ cat, picked, car });
    if (q.storeId) this.setData({ store: STORES.find((s) => s.id === q.storeId) || STORES[0] });
    else util.getLocation((loc) => {
      // 没指定门店：默认离你最近的那家
      const near = STORES.slice().sort((a, b) => util.distanceKm(loc, a) - util.distanceKm(loc, b))[0];
      this.setData({ store: near });
    });
    this.refresh();
  },
  onShow() {
    // 从「添加爱车」回来：带上新车
    if (!this.data.car) this.setData({ car: db.defaultCar() });
  },
  refresh() {
    const p = this.data.picked;
    const list = SERVICES.filter((v) => v.cat === this.data.cat).map((v) => Object.assign({}, v, { on: !!p[v.id] }));
    const sel = SERVICES.filter((v) => p[v.id]);
    this.setData({
      list, count: sel.length,
      total: sel.reduce((s, v) => s + v.price, 0), mins: sel.reduce((s, v) => s + v.mins, 0)
    });
  },
  onCat(e) { this.setData({ cat: e.currentTarget.dataset.id }); this.refresh(); },
  toggle(e) {
    const id = e.currentTarget.dataset.id;
    const picked = Object.assign({}, this.data.picked);
    if (picked[id]) delete picked[id]; else picked[id] = true;
    this.setData({ picked });
    this.refresh();
  },
  switchStore() {
    wx.showActionSheet({
      itemList: STORES.map((s) => s.short),
      success: (r) => this.setData({ store: STORES[r.tapIndex] })
    });
  },
  switchCar() {
    const cars = db.getCars();
    if (!cars.length) { wx.navigateTo({ url: '/pages/car-edit/car-edit' }); return; }
    wx.showActionSheet({
      itemList: cars.map((c) => c.plateText + ' ' + c.name).concat(['添加新车辆']),
      success: (r) => {
        if (r.tapIndex === cars.length) wx.navigateTo({ url: '/pages/car-edit/car-edit' });
        else this.setData({ car: cars[r.tapIndex] });
      }
    });
  },
  next() {
    if (!this.data.count) { wx.showToast({ title: '请至少选择一个服务项目', icon: 'none' }); return; }
    if (!this.data.car) {
      wx.showModal({ title: '还没有爱车', content: '预约前请先添加车辆，门店会按车型备好配件', confirmText: '去添加',
        success: (r) => { if (r.confirm) wx.navigateTo({ url: '/pages/car-edit/car-edit' }); } });
      return;
    }
    const ids = Object.keys(this.data.picked).join(',');
    wx.navigateTo({ url: '/pages/booking/booking?storeId=' + this.data.store.id + '&carId=' + this.data.car.id + '&ids=' + ids });
  }
});
