// 电子发票：按行程开票（多选已支付、未开过票的行程，合开一张）→ 抬头（企业 / 个人，可导入微信抬头）+ 邮箱 → 开票记录。
// 从行程详情带 ids=T1,T2 进来时预先勾上
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const EMAIL = /^[\w.+-]+@[\w-]+(\.[\w-]+)+$/;
const TAX = /^[0-9A-Z]{15,20}$/;

Page({
  data: {
    tab: 'trip', trips: [], picked: {}, count: 0, amount: 0, allOn: false,
    kind: 'company', title: '', taxNo: '', email: '', formSeq: 0, records: []
  },
  onLoad(q) {
    const ids = q.ids ? String(q.ids).split(',').filter(Boolean) : [];
    const picked = {};
    ids.forEach(id => { picked[id] = true; });
    this.setData({ picked, tab: q.tab === 'list' ? 'list' : 'trip' });
  },
  onShow() {
    this.load();
  },
  load() {
    const list = store.invoiceable();
    // 已经开过 / 不再可开的勾选项去掉
    const picked = {};
    list.forEach(t => {
      if (this.data.picked[t.id]) picked[t.id] = true;
    });
    const trips = list.map(t => ({ id: t.id, date: t.date, type: t.type, fromName: t.from.name, toName: t.to.name, pay: t.pay, on: !!picked[t.id] }));
    const on = trips.filter(t => t.on);
    this.setData({
      trips, picked, count: on.length, amount: store.round2(on.reduce((s, t) => s + t.pay, 0)),
      allOn: trips.length > 0 && on.length === trips.length,
      records: store.getInvoices().map(r => Object.assign({}, r, { n: r.tripIds.length, kindName: r.kind === 'person' ? '个人' : '企业' }))
    });
  },
  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.k });
  },
  onToggle(e) {
    const id = String(e.currentTarget.dataset.id);
    this.setData({ ['picked.' + id]: !this.data.picked[id] });
    this.load();
  },
  onAll() {
    const picked = {};
    if (!this.data.allOn) this.data.trips.forEach(t => { picked[t.id] = true; });
    this.setData({ picked });
    this.load();
  },
  onKind(e) {
    this.setData({ kind: e.currentTarget.dataset.k });
  },
  onTitle(e) {
    this.setData({ title: e.detail.value });
  },
  onTaxNo(e) {
    this.setData({ taxNo: e.detail.value });
  },
  onEmail(e) {
    this.setData({ email: e.detail.value });
  },
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
    if (!d.count) err = '请勾选要开票的行程';
    else if (!title) err = d.kind === 'company' ? '请填写公司抬头' : '请填写个人抬头（姓名）';
    else if (d.kind === 'company' && !TAX.test(taxNo)) err = '税号为 15-20 位数字或字母';
    else if (!EMAIL.test(email)) err = '请填写正确的邮箱';
    if (err) {
      wx.showToast({ title: err, icon: 'none' });
      return;
    }
    wx.showModal({
      title: '确认开票',
      content: (d.kind === 'company' ? '企业抬头：' + title + '\n税号：' + taxNo : '个人抬头：' + title) +
        '\n' + d.count + ' 个行程，合计 ¥' + d.amount + '\n电子发票发送至 ' + email,
      confirmText: '开票',
      success: r => {
        if (!r.confirm) return;
        const inv = store.addInvoice(Object.keys(d.picked), { kind: d.kind, title, taxNo: d.kind === 'company' ? taxNo : '', email });
        if (!inv) {
          wx.showToast({ title: '所选行程已开过票', icon: 'none' });
          this.load();
          return;
        }
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
      content: r.kindName + '抬头：' + r.title + (r.taxNo ? '\n税号：' + r.taxNo : '') + '\n金额：¥' + r.amount + '（' + r.n + ' 个行程）\n开票时间：' + r.time + '\n接收邮箱：' + r.email,
      confirmText: '重发邮件',
      cancelText: '关闭',
      success: m => {
        if (m.confirm) wx.showToast({ title: '已重新发送至 ' + r.email, icon: 'none' });
      }
    });
  },
  onGoTrips() {
    wx.switchTab({ url: '/pages/trips/trips' });
  }
});
