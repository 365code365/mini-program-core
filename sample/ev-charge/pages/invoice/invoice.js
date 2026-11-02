// 电子发票：多选已支付、未开票的充电订单合开一张 → 抬头（企业 / 个人，可导入微信抬头）+ 邮箱 → 开票记录。
// 从订单详情带 ids=EV1,EV2 进来时预先勾上
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const EMAIL = /^[\w.+-]+@[\w-]+(\.[\w-]+)+$/;
const TAX = /^[0-9A-Z]{15,20}$/;
Page({
  data: {
    tab: 'order', orders: [], picked: {}, count: 0, amount: '0.00', allOn: false,
    kind: 'company', title: '', taxNo: '', email: '', formSeq: 0, records: []
  },
  onLoad(q) {
    const picked = {};
    (q.ids ? String(q.ids).split(',') : []).filter(Boolean).forEach(id => { picked[id] = true; });
    this.setData({ picked, tab: q.tab === 'list' ? 'list' : 'order' });
  },
  onShow() { this.load(); },
  load() {
    const list = store.invoiceable();
    const picked = {};
    list.forEach(o => { if (this.data.picked[o.id]) picked[o.id] = true; });
    const orders = list.map(o => ({ id: o.id, name: data.getStation(o.station).name, date: o.date, kwh: o.kwh, pay: o.pay.toFixed(2), on: !!picked[o.id] }));
    const on = list.filter(o => picked[o.id]);
    this.setData({
      orders, picked, count: on.length, amount: store.round2(on.reduce((s, o) => s + o.pay, 0)).toFixed(2),
      allOn: orders.length > 0 && on.length === orders.length,
      records: store.getInvoices().map(r => Object.assign({}, r, { n: r.orderIds.length, kindName: r.kind === 'person' ? '个人' : '企业', amountText: r.amount.toFixed(2) }))
    });
  },
  onTab(e) { this.setData({ tab: String(e.currentTarget.dataset.k) }); },
  onToggle(e) {
    const id = String(e.currentTarget.dataset.id);
    this.setData({ ['picked.' + id]: !this.data.picked[id] });
    this.load();
  },
  onAll() {
    const picked = {};
    if (!this.data.allOn) this.data.orders.forEach(o => { picked[o.id] = true; });
    this.setData({ picked });
    this.load();
  },
  onKind(e) { this.setData({ kind: String(e.currentTarget.dataset.k) }); },
  onTitle(e) { this.setData({ title: e.detail.value }); },
  onTaxNo(e) { this.setData({ taxNo: e.detail.value }); },
  onEmail(e) { this.setData({ email: e.detail.value }); },
  onImportTitle() {
    host.invoiceTitle().then(r => {
      if (r) this.setData({ kind: r.type, title: r.title, taxNo: r.taxNo, formSeq: this.data.formSeq + 1 });
    });
  },
  onSubmit() {
    const d = this.data;
    const title = d.title.trim();
    const taxNo = d.taxNo.trim().toUpperCase();
    const email = d.email.trim();
    let err = '';
    if (!d.count) err = '请勾选要开票的订单';
    else if (!title) err = d.kind === 'company' ? '请填写公司抬头' : '请填写个人抬头（姓名）';
    else if (d.kind === 'company' && !TAX.test(taxNo)) err = '税号为 15-20 位数字或字母';
    else if (!EMAIL.test(email)) err = '请填写正确的邮箱';
    if (err) { wx.showToast({ title: err, icon: 'none' }); return; }
    wx.showModal({
      title: '确认开票',
      content: (d.kind === 'company' ? '企业抬头：' + title + '\n税号：' + taxNo : '个人抬头：' + title) +
        '\n' + d.count + ' 笔订单，合计 ¥' + d.amount + '\n电子发票发送至 ' + email,
      confirmText: '开票',
      confirmColor: '#00B87A',
      success: r => {
        if (!r.confirm) return;
        const inv = store.addInvoice(Object.keys(d.picked), { kind: d.kind, title, taxNo: d.kind === 'company' ? taxNo : '', email });
        if (!inv) { wx.showToast({ title: '所选订单已开过票', icon: 'none' }); this.load(); return; }
        this.setData({ picked: {}, tab: 'list' });
        this.load();
        wx.showToast({ title: '开票成功，已发送至邮箱', icon: 'none' });
      }
    });
  },
  onRecord(e) {
    const r = this.data.records[Number(e.currentTarget.dataset.i)];
    if (!r) return;
    wx.showModal({
      title: '电子发票 ' + r.id,
      content: r.kindName + '抬头：' + r.title + (r.taxNo ? '\n税号：' + r.taxNo : '') + '\n金额：¥' + r.amountText + '（' + r.n + ' 笔订单）\n开票时间：' + r.time + '\n接收邮箱：' + r.email,
      confirmText: '重发邮件',
      cancelText: '关闭',
      confirmColor: '#00B87A',
      success: m => { if (m.confirm) wx.showToast({ title: '已重新发送至 ' + r.email, icon: 'none' }); }
    });
  },
  onGoOrders() { wx.switchTab({ url: '/pages/orders/orders' }); }
});
