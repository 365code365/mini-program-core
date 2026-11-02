// 订单详情：待发货 →（卖家发货，演示里付款 8 秒后自动发货）待收货 → 确认收货 / 评价；待发货可申请退款
const store = require('../../utils/store.js');
const D = require('../../utils/data.js');
const STEPS = ['已付款', '卖家发货', '确认收货'];
Page({
  data: { o: null, seller: null, steps: STEPS, cur: 0, rate: 5, review: '', created: '', stars: [1, 2, 3, 4, 5] },
  onLoad(q) { this.id = q.id; this.refresh(); this.timer = setInterval(() => this.autoShip(), 2000); },
  onUnload() { clearInterval(this.timer); },
  autoShip() {
    const o = store.orderOf(this.id);
    if (o && o.status === 'paid' && Date.now() - o.created > 8000) {
      store.updateOrder(o.id, { status: 'shipped', shippedAt: Date.now(), express: o.way === 'express' ? '顺丰 SF' + String(Date.now()).slice(-12) : '' });
      store.tradeNote(o.item.id, o.way === 'express' ? '卖家已发货，请留意物流' : '卖家已确认面交时间，验货后记得确认收货');
      this.refresh();
    }
  },
  refresh() {
    const o = store.orderOf(this.id);
    if (!o) return;
    const cur = { paid: 0, shipped: 1, done: 2, closed: 0 }[o.status];
    const d = new Date(o.created);
    this.setData({ o: Object.assign({}, o, { statusText: store.STATUS[o.status] }), seller: D.SELLERS[o.item.seller] || D.ME, cur,
      created: d.getFullYear() + '-' + (d.getMonth() + 1) + '-' + d.getDate() + ' ' + d.getHours() + ':' + ('0' + d.getMinutes()).slice(-2) });
  },
  confirmRecv() {
    wx.showModal({
      title: '确认收货', content: '确认后货款 ¥' + this.data.o.total + ' 将打给卖家，请确保已收到宝贝且无问题。', confirmText: '确认收货',
      success: r => {
        if (!r.confirm) return;
        store.updateOrder(this.id, { status: 'done', doneAt: Date.now() });
        store.tradeNote(this.data.o.item.id, '买家已确认收货，交易完成');
        this.refresh();
      }
    });
  },
  refund() {
    wx.showActionSheet({
      itemList: ['不想要了', '和卖家协商一致', '卖家迟迟不发货'],
      success: r => {
        store.updateOrder(this.id, { status: 'closed', refundReason: ['不想要了', '和卖家协商一致', '卖家迟迟不发货'][r.tapIndex] });
        const sold = store.get(store.K.sold, {}); delete sold[this.data.o.item.id]; store.set(store.K.sold, sold);
        store.tradeNote(this.data.o.item.id, '买家申请退款，¥' + this.data.o.total + ' 已原路退回');
        wx.showToast({ title: '退款成功', icon: 'success' });
        this.refresh();
      }
    });
  },
  setRate(e) { this.setData({ rate: Number(e.currentTarget.dataset.v) }); },
  onReview(e) { this.setData({ review: e.detail.value }); },
  submitReview() {
    store.updateOrder(this.id, { rate: this.data.rate, review: this.data.review.trim() || '好评！' });
    wx.showToast({ title: '评价成功', icon: 'success' });
    this.refresh();
  },
  chat() {
    const c = store.convForItem(this.data.o.item.id);
    wx.navigateTo({ url: '/pages/chat/chat?id=' + c.id });
  },
  copy() { wx.setClipboardData({ data: this.data.o.express || this.data.o.id }); }
});
