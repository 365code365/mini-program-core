const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { STORES, SERVICES } = require('../../utils/data.js');

// 营业时间 "08:30-20:00" → 可选的最早 / 最晚到店时间（最晚 = 打烊前 1 小时）
function hoursRange(h) {
  const p = h.split('-');
  const end = p[1].split(':');
  return { start: p[0], end: util.pad(+end[0] - 1) + ':' + end[1] };
}
const toMin = (t) => { const p = t.split(':'); return +p[0] * 60 + +p[1]; };

Page({
  data: {
    store: null, items: [], total: 0, mins: 0,
    cars: [], carIdx: 0,
    days: [], dayIdx: 0, time: '', range: { start: '08:30', end: '19:00' },
    contact: '', phone: '', pickup: false, pickupAddr: '', manualAddr: false, remark: '',
    errors: {}
  },
  onLoad(q) {
    const store = STORES.find((s) => s.id === q.storeId) || STORES[0];
    const ids = (q.ids || '').split(',').filter(Boolean);
    const items = SERVICES.filter((v) => ids.indexOf(v.id) >= 0);
    const cars = db.getCars();
    const carIdx = Math.max(0, cars.findIndex((c) => c.id === q.carId));
    const last = db.getOrders()[0];
    const addr = db.getPickupAddr();
    this.setData({
      store, items, cars, carIdx, range: hoursRange(store.hours), days: util.nextDays(7),
      total: items.reduce((s, v) => s + v.price, 0), mins: items.reduce((s, v) => s + v.mins, 0),
      // 联系人沿用上一单
      contact: last ? last.contact : '', phone: last ? last.phone : '', pickupAddr: addr ? addr.name : ''
    });
  },
  onDay(e) { this.setData({ dayIdx: +e.currentTarget.dataset.i, 'errors.time': '' }); this.checkTime(this.data.time); },
  onTime(e) { this.setData({ time: e.detail.value }); this.checkTime(e.detail.value); },
  // 今天的预约至少提前 30 分钟
  checkTime(t) {
    if (!t) return true;
    let msg = '';
    if (this.data.dayIdx === 0) {
      const now = new Date();
      if (toMin(t) < now.getHours() * 60 + now.getMinutes() + 30) msg = '今天的预约请至少提前 30 分钟';
    }
    this.setData({ 'errors.time': msg });
    return !msg;
  },
  onCar(e) { this.setData({ carIdx: +e.detail.value }); },
  onInput(e) { this.setData({ [e.currentTarget.dataset.k]: e.detail.value, ['errors.' + e.currentTarget.dataset.k]: '' }); },
  onPickup(e) {
    this.setData({ pickup: e.detail.value });
    if (e.detail.value && !this.data.pickupAddr) this.chooseAddr();
  },
  chooseAddr() {
    wx.chooseLocation({
      latitude: this.data.store.latitude, longitude: this.data.store.longitude,
      success: (r) => {
        const a = { name: r.name || r.address, address: r.address, latitude: r.latitude, longitude: r.longitude };
        db.setPickupAddr(a);
        this.setData({ pickupAddr: a.name, manualAddr: false, 'errors.pickupAddr': '' });
      },
      // 选点不可用：改成手填地址
      fail: () => {
        this.setData({ manualAddr: true });
        wx.showToast({ title: '无法打开地图选点，请手动填写取车地址', icon: 'none' });
      }
    });
  },
  validate() {
    const d = this.data, errors = {};
    if (!d.time) errors.time = '请选择到店时间';
    else if (!this.checkTime(d.time)) errors.time = '今天的预约请至少提前 30 分钟';
    if (!/^[\u4e00-\u9fa5A-Za-z·]{2,12}$/.test(d.contact.trim())) errors.contact = '请填写 2~12 个字的联系人姓名';
    if (!/^1[3-9]\d{9}$/.test(d.phone)) errors.phone = '请填写 11 位手机号';
    if (d.pickup && !d.pickupAddr.trim()) errors.pickupAddr = '请填写取车地址';
    this.setData({ errors });
    return Object.keys(errors).length === 0;
  },
  submit() {
    if (!this.validate()) { wx.showToast({ title: '请检查标红的信息', icon: 'none' }); return; }
    const d = this.data, car = d.cars[d.carIdx], s = d.store, now = new Date();
    const id = 'AC' + util.ymd(now).replace(/-/g, '') + util.pad(now.getHours()) + util.pad(now.getMinutes()) + util.pad(now.getSeconds());
    const order = {
      id, storeId: s.id, storeName: s.name, storeAddr: s.address, storePhone: s.phone,
      carId: car.id, carName: car.name, plate: car.plate,
      items: d.items.map((v) => ({ id: v.id, name: v.name, price: v.price, mins: v.mins })),
      total: d.total, date: d.days[d.dayIdx].date, time: d.time, contact: d.contact.trim(), phone: d.phone,
      pickup: d.pickup, pickupAddr: d.pickup ? d.pickupAddr.trim() : '', remark: d.remark.trim(),
      status: 'pending', createdAt: util.stamp(now), cancelReason: '', review: null,
      timeline: [{ t: util.stamp(now), text: '预约成功，门店已确认工位' }]
    };
    if (d.pickup && d.manualAddr) db.setPickupAddr({ name: order.pickupAddr, address: order.pickupAddr });
    wx.showLoading({ title: '提交预约中', mask: true });
    setTimeout(() => {
      db.saveOrder(order);
      wx.hideLoading();
      wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + id + '&new=1' });
    }, 500);
  }
});
