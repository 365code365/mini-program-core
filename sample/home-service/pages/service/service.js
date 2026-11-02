const db = require('../../utils/db.js');
const { SERVICES, STAFF } = require('../../utils/data.js');

Page({
  data: { svc: null, specIdx: 0, staff: null, prefer: '' },
  onLoad(q) {
    const svc = SERVICES.find((s) => s.id === q.id) || SERVICES[0];
    const staff = (q.staff && STAFF.find((s) => s.id === q.staff)) || db.staffFor(svc.id);
    this.setData({ svc, staff, prefer: q.staff || '' });
    wx.setNavigationBarTitle({ title: svc.name });
  },
  pickSpec(e) { this.setData({ specIdx: +e.currentTarget.dataset.i }); },
  callStaff() { wx.showToast({ title: '下单后可直接联系' + this.data.staff.name, icon: 'none' }); },
  book() {
    const s = this.data.svc;
    wx.navigateTo({ url: '/pages/order-form/order-form?id=' + s.id + '&spec=' + s.specs[this.data.specIdx].id + (this.data.prefer ? '&staff=' + this.data.prefer : '') });
  },
  onShareAppMessage() { return { title: '好管家 · ' + this.data.svc.name + ' ¥' + this.data.svc.price + ' 起', path: '/pages/service/service?id=' + this.data.svc.id }; }
});
