const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const ST = { paid: '已支付', free: '免费领取', refunded: '已退款' };
Page({
  data: { list: [], sum: '0' },
  onShow() { this.refresh(); },
  refresh() {
    const list = store.getOrders().map(o => {
      const c = data.courseOf(o.cid);
      return Object.assign({}, o, { cover: c.cover, mark: c.mark, st: ST[o.status], canRefund: store.refundable(o), owned: store.owned(o.cid) });
    });
    const sum = list.filter(o => o.status === 'paid').reduce((s, o) => s + o.pay, 0);
    this.setData({ list, sum: store.money(sum) });
  },
  study(e) { wx.navigateTo({ url: '/pages/lesson/lesson?cid=' + e.currentTarget.dataset.cid }); },
  detail(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.cid }); },
  refund(e) {
    const o = this.data.list.find(x => x.id === String(e.currentTarget.dataset.id));
    wx.showModal({ title: '申请退款', content: '「' + o.title + '」退款 ¥' + o.pay + (o.coupon ? '，优惠券退回账户' : '') + '。退款后课程将从学习中心移除。', confirmText: '确认退款', confirmColor: '#f0577a', success: r => {
      if (!r.confirm) return;
      store.refund(o.id);
      this.refresh();
      wx.showToast({ title: '已退款 ¥' + o.pay + '，原路退回', icon: 'none' });
    } });
  }
});
