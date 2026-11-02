const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');

Page({
  data: {
    ok: false,
    payTo: '微信',
    items: [],
    reasons: data.afterReasons,
    reason: '',
    note: '',
    amount: 0,
    picked: 0
  },

  onLoad(q) {
    this.id = q.id;
    const o = orders.findOrder(this.id);
    if (!o) return;
    this.setData({
      ok: orders.stageOf(o).canAfter,
      payTo: o.payBy === 'wallet' ? '余额' : '微信',
      items: orders.refundable(o).map(i => Object.assign({}, i, { on: false }))
    });
  },

  sum() {
    const on = this.data.items.filter(i => i.on && !i.refunded);
    this.setData({ amount: store.round2(on.reduce((n, i) => n + i.refund, 0)), picked: on.length });
  },

  onItem(e) {
    const i = Number(e.currentTarget.dataset.i);
    const it = this.data.items[i];
    if (it.refunded) {
      wx.showToast({ title: '这件商品已经退过款了', icon: 'none' });
      return;
    }
    this.setData({ [`items[${i}].on`]: !it.on });
    this.sum();
  },

  onReason(e) {
    this.setData({ reason: e.currentTarget.dataset.r });
  },

  onNote(e) {
    this.setData({ note: e.detail.value });
  },

  onSubmit() {
    const d = this.data;
    if (!d.picked) {
      wx.showToast({ title: '请勾选要售后的商品', icon: 'none' });
      return;
    }
    if (!d.reason) {
      wx.showToast({ title: '请选择售后原因', icon: 'none' });
      return;
    }
    wx.showModal({
      title: '确认申请售后',
      content: `${d.reason}：退款 ¥${d.amount}，将原路退回${d.payTo}，无需退回商品`,
      confirmText: '提交',
      success: r => {
        if (!r.confirm) return;
        const keys = d.items.filter(i => i.on && !i.refunded).map(i => i.key);
        const rec = orders.applyAfterSale(this.id, keys, d.reason, d.note.trim());
        if (!rec) {
          wx.showToast({ title: '已超过售后时效', icon: 'none' });
          return;
        }
        wx.showToast({ title: `退款成功 ¥${rec.amount}`, icon: 'none' });
        setTimeout(() => wx.navigateBack(), 600);
      }
    });
  },

  onBack() {
    wx.navigateBack();
  }
});
