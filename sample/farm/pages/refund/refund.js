const store = require('../../utils/store.js');

const TYPES = [{ v: 'full', t: '仅退款（未收到 / 整箱问题）' }, { v: 'part', t: '坏果部分赔付' }];
const REASONS = ['坏果 / 烂果', '与描述不符（大小、口感）', '少件 / 漏发', '物流太慢，已不新鲜', '不想要了'];

Page({
  data: { o: null, types: TYPES, type: 'part', reasons: REASONS, reason: '', amount: '', max: '0.00', desc: '', err: {} },
  onLoad(q) {
    const o = store.orderById(q.id);
    if (!o) return;
    this.setData({ o, max: o.total, amount: store.money(o.total * 0.3) });
  },
  onType(e) { this.applyType(e.detail.value); },
  pickType(e) { this.applyType(e.currentTarget.dataset.v); },
  applyType(v) { this.setData({ type: v, amount: store.money(v === 'full' ? this.data.o.total : this.data.o.total * 0.3), 'err.amount': '' }); },
  onReason(e) { this.setData({ reason: e.detail.value, 'err.reason': '' }); },
  pickReason(e) { this.setData({ reason: e.currentTarget.dataset.v, 'err.reason': '' }); },
  // type="digit"：只留数字和一个小数点、最多两位小数
  onAmount(e) {
    let v = String(e.detail.value).replace(/[^\d.]/g, '');
    const i = v.indexOf('.');
    if (i >= 0) v = v.slice(0, i + 1) + v.slice(i + 1).replace(/\./g, '').slice(0, 2);
    this.setData({ amount: v, 'err.amount': '' });
    return v;
  },
  onDesc(e) { this.setData({ desc: e.detail.value, 'err.desc': '' }); },
  submit() {
    const d = this.data, err = {};
    const amt = parseFloat(d.amount);
    if (!d.reason) err.reason = '请选择退款原因';
    if (!(amt > 0)) err.amount = '请输入退款金额';
    else if (amt > parseFloat(d.max)) err.amount = `不能超过实付 ¥${d.max}`;
    if (d.desc.trim().length < 6) err.desc = '请至少写 6 个字，方便商家核实';
    this.setData({ err });
    if (Object.keys(err).length) return;
    store.updateOrder(d.o.id, { status: 'refunding', refund: { type: d.type, reason: d.reason, amount: store.money(amt), desc: d.desc.trim(), at: Date.now(), from: d.o.status } });
    wx.showToast({ title: '已提交申请', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 700);
  }
});
