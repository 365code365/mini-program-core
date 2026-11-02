const store = require('../../utils/store.js');
const pay = require('../../utils/pay.js');

const SUB = {
  unpaid: '订单保留 30 分钟，超时自动取消', grouping: '付款成功，等待拼友加入', paid: '果园正在分拣装箱，48 小时内发货',
  shipped: '冷链运输中，签收前请开箱验货', done: '感谢支持产地直供', refunding: '售后申请已提交，等待商家处理',
  refunded: '退款已原路退回', cancelled: '订单已取消'
};
// 每个状态底部能做的事（main = 主按钮）
const OPS = {
  unpaid: [{ k: 'cancel', t: '取消订单' }, { k: 'pay', t: '去支付', main: true }],
  grouping: [{ k: 'group', t: '查看拼团', main: true }],
  paid: [{ k: 'refund', t: '申请退款' }, { k: 'remind', t: '提醒发货', main: true }],
  shipped: [{ k: 'service', t: '联系客服' }, { k: 'receive', t: '确认收货', main: true }],
  done: [{ k: 'refund', t: '申请售后' }, { k: 'again', t: '再来一单', main: true }],
  refunding: [{ k: 'service', t: '联系客服' }, { k: 'revoke', t: '撤销申请' }],
  refunded: [{ k: 'again', t: '再来一单', main: true }],
  cancelled: [{ k: 'remove', t: '删除订单' }, { k: 'again', t: '再来一单', main: true }]
};

Page({
  data: { o: null, statusText: '', statusSub: '', logistics: [], ops: [], createdText: '', paidText: '', cashier: false },

  onLoad(q) { this.id = q.id; this.act = q.act; },
  onShow() {
    this.refresh();
    if (this.act) { const a = this.act; this.act = ''; setTimeout(() => this.run(a), 300); }
  },
  refresh() {
    let o = store.orderById(this.id);
    if (!o) { wx.showToast({ title: '订单不存在', icon: 'none' }); return; }
    if (o.groupId && o.status === 'grouping') { store.settleGroup(o.groupId); o = store.orderById(this.id); }
    // 未付款超过 30 分钟自动取消
    if (o.status === 'unpaid' && Date.now() - o.createdAt > 30 * 60e3) o = store.updateOrder(o.id, { status: 'cancelled' });
    this.setData({
      o, statusText: store.STATUS[o.status], statusSub: SUB[o.status], ops: OPS[o.status] || [],
      logistics: o.shippedAt ? store.logistics(o) : [], createdText: store.fmtTime(o.createdAt), paidText: o.paidAt ? store.fmtTime(o.paidAt) : ''
    });
  },

  onOp(e) { this.run(e.currentTarget.dataset.k); },
  run(k) {
    const o = this.data.o;
    if (!o) return;
    if (k === 'pay') pay.request(o, (unsupported) => (unsupported ? this.setData({ cashier: true }) : null));
    else if (k === 'cancel') this.confirm('取消订单', '确定取消这笔订单吗？', () => { store.updateOrder(o.id, { status: 'cancelled' }); this.refresh(); });
    else if (k === 'group') wx.navigateTo({ url: '/pages/group/group?id=' + o.groupId });
    else if (k === 'remind') {
      // 示例里由「提醒发货」触发商家发货，生成顺丰单号与物流轨迹
      store.updateOrder(o.id, { status: 'shipped', shippedAt: Date.now(), expressNo: 'SF' + String(o.createdAt).slice(-12) });
      wx.showToast({ title: '已提醒，商家已发货', icon: 'success' });
      this.refresh();
    } else if (k === 'receive') {
      this.confirm('确认收货', '请确认已收到商品且无坏果，确认后款项将打给果农', () => {
        wx.vibrateShort({ type: 'light' });
        store.updateOrder(o.id, { status: 'done', doneAt: Date.now() });
        wx.showToast({ title: '已确认收货', icon: 'success' });
        this.refresh();
      });
    } else if (k === 'refund') wx.navigateTo({ url: '/pages/refund/refund?id=' + o.id });
    else if (k === 'revoke') this.confirm('撤销售后', '撤销后订单恢复为原状态', () => { store.updateOrder(o.id, { status: o.refund.from || 'done', refund: null }); this.refresh(); });
    else if (k === 'again') {
      o.items.forEach((it) => store.addCart(it.goodsId, it.skuKey, it.qty));
      wx.showToast({ title: '已加入购物车', icon: 'success' });
      setTimeout(() => wx.switchTab({ url: '/pages/cart/cart' }), 600);
    } else if (k === 'remove') this.confirm('删除订单', '删除后无法恢复', () => {
      wx.setStorageSync('farm_orders', store.orders().filter((x) => x.id !== o.id));
      wx.navigateBack();
    });
    else if (k === 'service') {
      wx.makePhoneCall({ phoneNumber: '4008-826-517', fail: () => wx.showModal({ title: '田园直供客服', content: '客服热线 4008-826-517\n每天 8:00-22:00', showCancel: false }) });
    }
  },
  confirm(title, content, ok) { wx.showModal({ title, content, success: (r) => r.confirm && ok() }); },
  onPaid() { this.setData({ cashier: false }); pay.markPaid(this.id); wx.redirectTo({ url: '/pages/pay-result/pay-result?id=' + this.id }); },
  onPayCancel() { this.setData({ cashier: false }); wx.showToast({ title: '已取消支付', icon: 'none' }); },
  copyNo() { wx.setClipboardData({ data: this.data.o.expressNo }); },
  copyOrderNo() { wx.setClipboardData({ data: this.data.o.no }); },
  openGoods(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); }
});
