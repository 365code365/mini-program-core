const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const KIND = { reg: '挂号', clinic: '门诊', consult: '问诊', deposit: '住院', settle: '住院' };
// 能开电子发票的：挂号费、门诊处方、问诊费、出院结算补缴（预交金只给收据，退款没有发票）
const CAN_INVOICE = { reg: true, clinic: true, consult: true, settle: true };
Page({
  data: {
    tabs: [{ key: 'unpaid', name: '待缴费' }, { key: 'paid', name: '缴费记录' }],
    tab: 'unpaid', list: [], unpaidCount: 0, openId: '',
    inv: null, invTypes: ['个人', '单位'], invType: 0, invTitle: '', invTax: '', invEmail: ''
  },
  onLoad(q) {
    if (q.tab === 'paid') this.setData({ tab: 'paid' });
  },
  onShow() { this.refresh(); },
  refresh() {
    store.tick();
    const all = store.getBills();
    const unpaid = all.filter(b => b.status === 'unpaid');
    const rows = this.data.tab === 'unpaid' ? unpaid : all.filter(b => b.status !== 'unpaid');
    this.setData({
      unpaidCount: unpaid.length,
      list: rows.map(b => Object.assign({}, b, {
        kindName: KIND[b.kind] || '门诊',
        time: store.dateTime(b.paidAt || b.createdAt),
        canInvoice: b.status === 'paid' && !!CAN_INVOICE[b.kind]
      }))
    });
  },
  setTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key, openId: '' });
    this.refresh();
  },
  onToggle(e) {
    const id = e.currentTarget.dataset.id;
    this.setData({ openId: this.data.openId === id ? '' : id });
  },
  onPay(e) {
    const b = store.findBill(e.currentTarget.dataset.id);
    if (!b || this.paying) return;
    this.paying = true;
    host.pay(b.self, b.patient + ' · ' + b.title + (b.insured ? '\n医保统筹已支付 ¥' + b.insured : '')).then(ok => {
      this.paying = false;
      if (!ok) {
        wx.showToast({ title: '已取消支付', icon: 'none' });
        return;
      }
      const paid = store.payBill(b.id);
      const reports = (paid.items || []).filter(x => x.report).length;
      wx.showToast({ title: reports ? '缴费成功，检查报告已生成' : '缴费成功，请到药房取药', icon: 'none' });
      this.refresh();
    });
  },
  goReports() { wx.navigateTo({ url: '/pages/reports/reports' }); },
  // ───────── 电子发票 ─────────
  onInvoice(e) {
    const b = store.findBill(e.currentTarget.dataset.id);
    if (b.invoice) {
      const v = b.invoice;
      wx.showModal({ title: '电子发票 ' + v.no, content: '抬头：' + v.title + (v.tax ? '\n税号：' + v.tax : '') + '\n金额：¥' + b.self + '\n已发送至 ' + v.email, showCancel: false, confirmText: '知道了' });
      return;
    }
    this.setData({ inv: { id: b.id, amount: b.self, name: b.title }, invType: 0, invTitle: b.patient, invTax: '', invEmail: '' });
  },
  closeInvoice() { this.setData({ inv: null }); },
  noop() {},
  onInvType(e) { this.setData({ invType: Number(e.currentTarget.dataset.i) }); },
  onInvField(e) { this.setData({ [e.currentTarget.dataset.f]: e.detail.value }); },
  saveInvoice() {
    const d = this.data;
    const title = d.invTitle.trim();
    const tax = d.invTax.trim().toUpperCase();
    const email = d.invEmail.trim();
    let err = '';
    if (!title) err = '请填写发票抬头';
    else if (d.invType === 1 && !/^[0-9A-Z]{15,20}$/.test(tax)) err = '请填写 15~20 位纳税人识别号';
    else if (!/^[\w.-]+@[\w-]+(\.[\w-]+)+$/.test(email)) err = '请填写正确的邮箱';
    if (err) {
      wx.showToast({ title: err, icon: 'none' });
      return;
    }
    const no = 'FP' + String(Date.now()).slice(-10);
    store.updateBill(d.inv.id, { invoice: { no, type: d.invTypes[d.invType], title, tax: d.invType === 1 ? tax : '', email, at: Date.now() } });
    this.setData({ inv: null });
    wx.showToast({ title: '发票已开具，已发送至邮箱', icon: 'none' });
    this.refresh();
  }
});
