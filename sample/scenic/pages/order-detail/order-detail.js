// 订单详情：票种明细、游客、入园日；待使用可看电子票 / 退票（入园前一天 23:59 前免费退），已退款 / 已使用可删除
const store = require('../../utils/store.js');
const data = require('../../utils/data.js');
const D = require('../../utils/date.js');
Page({
  data: { o: null, s: null, canRefund: false, daysLeft: 0 },
  onLoad(q) { this.no = q.no; },
  onShow() {
    const o = store.order(this.no);
    if (!o) { this.setData({ o: null }); return; }
    const daysLeft = D.diff(D.today(), o.date);
    this.setData({ o, s: data.find(o.scenic), daysLeft, canRefund: o.status === 'paid' && daysLeft >= 1 });
  },
  goTicket() { wx.navigateTo({ url: '/pages/ticket/ticket?no=' + this.no }); },
  goScenic() { wx.navigateTo({ url: '/pages/detail/detail?id=' + this.data.o.scenic }); },
  rebuy() { wx.navigateTo({ url: '/pages/booking/booking?id=' + this.data.o.scenic }); },
  refund() {
    const o = this.data.o;
    if (!this.data.canRefund) {
      wx.showModal({ title: '无法退票', content: '入园当天及之后不支持在线退票，如有特殊情况请联系客服 ' + data.SERVICE_TEL + '。', showCancel: false });
      return;
    }
    wx.showModal({
      title: '申请退票', content: '退还 ' + o.count + ' 张门票，¥' + o.total + ' 将原路退回，预计 1-3 个工作日到账。', confirmText: '确认退票', confirmColor: '#e5484d',
      success: (r) => { if (r.confirm) { store.setStatus(o.no, 'refunded'); wx.showToast({ title: '退票成功', icon: 'success' }); this.onShow(); } }
    });
  },
  remove() {
    wx.showModal({
      title: '删除订单', content: '删除后不可恢复', confirmColor: '#e5484d',
      success: (r) => { if (r.confirm) { store.removeOrder(this.no); wx.navigateBack(); } }
    });
  },
  call() {
    const tel = data.SERVICE_TEL;
    wx.makePhoneCall({ phoneNumber: tel, fail: () => wx.showModal({ title: '客服电话', content: tel, confirmText: '复制号码', success: (r) => { if (r.confirm) wx.setClipboardData({ data: tel }); } }) });
  },
  copyNo() { wx.setClipboardData({ data: this.no }); }
});
