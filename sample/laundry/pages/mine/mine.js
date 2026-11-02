const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const tab = require('../../utils/tab.js');

Page({
  data: { status: [], couponN: 0, addrN: 0, washed: 0, saved: 0, phone: data.SERVICE_PHONE },

  onShow() {
    tab.sync(this, 3);
    const all = store.orders();
    const counts = [0, 0, 0, 0];
    let washed = 0, saved = 0;
    all.forEach(o => {
      const m = store.mainStatus(o);
      if (m >= 0) counts[m]++;
      if (m === 3) washed += o.count;
      if (!o.cancelled && o.coupon) saved += o.coupon.amount;
    });
    this.setData({
      status: data.STATUS.map(s => ({ key: s.key, name: s.name, n: counts[s.key] })),
      couponN: store.coupons().filter(c => c.state === 'ok').length,
      addrN: store.addrs().length, washed, saved
    });
  },

  goOrders(e) {
    getApp().globalData.orderTab = Number(e.currentTarget.dataset.k);
    wx.switchTab({ url: '/pages/orders/orders' });
  },

  goCoupons() { wx.navigateTo({ url: '/pages/coupons/coupons' }); },
  goAddr() { wx.navigateTo({ url: '/pages/address/address' }); },

  call() {
    wx.makePhoneCall({
      phoneNumber: data.SERVICE_PHONE,
      fail: () => wx.showModal({ title: '客服热线', content: '当前设备无法直接拨号，请拨打 ' + data.SERVICE_PHONE + '（每天 8:00-22:00）', showCancel: false })
    });
  },

  faq() {
    wx.showModal({
      title: '洗护须知',
      content: '1. 取件前请清空口袋；2. 真丝、羊绒等特殊面料请在备注说明；3. 陈年污渍、褪色、起球属于面料本身问题，无法保证完全处理；4. 签收后 72 小时内可申请一次免费返洗。',
      showCancel: false,
      confirmText: '知道了'
    });
  },

  clearData() {
    wx.showModal({
      title: '清除本地数据',
      content: '订单、地址、优惠券与购物篮都会被删除。',
      confirmText: '清除',
      confirmColor: '#ff4d3a',
      success: res => {
        if (!res.confirm) return;
        store.clearAll();
        wx.showToast({ title: '已清除', icon: 'success' });
        this.onShow();
      }
    });
  }
});
