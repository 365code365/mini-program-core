const db = require('../../utils/db.js');
const util = require('../../utils/util.js');
const { HOTLINE } = require('../../utils/data.js');
const ENTRIES = [
  { key: 'waiting', text: '待上门', icon: '/images/svc/daily.png' },
  { key: 'serving', text: '服务中', icon: '/images/svc/electric.png' },
  { key: 'review', text: '待评价', icon: '/images/svc/deep.png' },
  { key: 'done', text: '已完成', icon: '/images/svc/newhome.png' }
];
Page({
  data: { user: null, entries: ENTRIES, counts: {}, couponN: 0, invoiceN: 0, canInvoice: 0, addrN: 0, saved: 0 },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    const a = db.defaultAddr();
    const orders = db.getOrders();
    const counts = {};
    orders.forEach((o) => { counts[o.status] = (counts[o.status] || 0) + 1; });
    // 累计节省 = 已用券面额（取消的单券已退回，不算）
    const saved = orders.filter((o) => o.status !== 'cancelled').reduce((s, o) => s + (o.discount || 0), 0);
    this.setData({
      // 模板里不能对字符串取下标（引擎问题 9），首字在 JS 里算好
      user: a ? { name: a.name, initial: a.name.slice(0, 1), phone: util.maskPhone(a.phone) } : null,
      counts, saved,
      couponN: db.getCoupons().filter((c) => c.state === 'unused').length,
      invoiceN: db.getInvoices().length,
      canInvoice: orders.filter((o) => (o.status === 'done' || o.status === 'review') && !o.invoiced).length,
      addrN: db.getAddrs().length
    });
  },
  goOrders(e) {
    getApp().globalData.orderTab = e.currentTarget.dataset.k || 'all';
    wx.switchTab({ url: '/pages/orders/orders' });
  },
  goAddr() { wx.navigateTo({ url: '/pages/address/address' }); },
  goCoupons() { wx.navigateTo({ url: '/pages/coupons/coupons' }); },
  goInvoice() { wx.navigateTo({ url: '/pages/invoice/invoice' }); },
  callService() { util.callPhone(HOTLINE, '好管家客服（8:00-21:00）'); },
  guarantee() {
    wx.showModal({
      title: '好管家服务保障',
      content: '① 阿姨师傅实名认证、持证上岗；② 服务不满意 24 小时内免费返工；③ 服务中造成财物损坏，经核实照价赔付（单次最高 5000 元）。',
      showCancel: false, confirmText: '知道了'
    });
  },
  reset() {
    wx.showModal({
      title: '恢复演示数据', content: '会清空本机的订单、地址、优惠券与开票记录，恢复成初始示例数据。', confirmColor: '#e5484d',
      success: (r) => {
        if (!r.confirm) return;
        wx.clearStorageSync();
        db.seed();
        this.onShow();
        wx.showToast({ title: '已恢复', icon: 'success' });
      }
    });
  }
});
