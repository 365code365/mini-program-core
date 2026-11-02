const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { HOTLINE } = require('../../utils/data.js');

Page({
  data: { cars: 0, orders: 0, spent: 0, counts: {}, addr: '', car: null },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 3 });
    const orders = db.getOrders();
    const counts = {};
    orders.forEach((o) => { counts[o.status] = (counts[o.status] || 0) + 1; });
    const spent = orders.filter((o) => o.status === 'done' || o.status === 'reviewed').reduce((s, o) => s + o.total, 0);
    const a = db.getPickupAddr();
    this.setData({ cars: db.getCars().length, orders: orders.length, spent, counts, addr: a ? a.name : '', car: db.defaultCar() });
  },
  goOrders(e) {
    getApp().globalData.orderTab = e.currentTarget.dataset.k;
    wx.switchTab({ url: '/pages/orders/orders' });
  },
  goGarage() { wx.navigateTo({ url: '/pages/garage/garage' }); },
  goViolation() { wx.navigateTo({ url: '/pages/violation/violation' }); },
  setAddr() {
    wx.chooseLocation({
      success: (r) => {
        db.setPickupAddr({ name: r.name || r.address, address: r.address, latitude: r.latitude, longitude: r.longitude });
        this.setData({ addr: r.name || r.address });
      },
      fail: () => {
        wx.showModal({ title: '无法打开地图选点', content: '可以在预约时打开「上门取送车」并手动填写地址，提交后会保存为常用地址。', showCancel: false });
      }
    });
  },
  hotline() { util.callPhone(HOTLINE, '车管家客服（8:00-22:00）'); },
  about() {
    wx.showModal({ title: '车管家 v2.6.0', content: '杭州 4 家直营门店，原厂配件，明码标价，服务不满意 7 天内免费返工。', showCancel: false });
  }
});
