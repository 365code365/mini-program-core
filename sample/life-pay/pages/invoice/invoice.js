const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const DEF = { kind: 'person', title: '张明远', taxNo: '', email: '' };
Page({
  data: { tab: 'todo', list: [], todoCount: 0, doneCount: 0, panel: '', cur: null, kind: 'person', title: '', taxNo: '', email: '', formSeq: 0 },
  onLoad(q) {
    this.refresh();
    if (q.id) {
      const r = store.getRecord(q.id);
      if (r) { this.setData({ tab: r.invoice ? 'done' : 'todo' }); this.refresh(); this.open(r.id); }
    }
  },
  refresh() {
    const all = store.getRecords().map(r => {
      const t = data.typeOf(r.type);
      return { id: r.id, ch: t.ch, bg: t.bg, title: r.title, targetTx: r.type === 'phone' ? r.target : store.fmtNo(r.target), time: r.time, paidTx: store.money(r.paid), invoice: r.invoice || null };
    });
    const todo = all.filter(r => !r.invoice);
    const done = all.filter(r => r.invoice);
    this.setData({ list: this.data.tab === 'todo' ? todo : done, todoCount: todo.length, doneCount: done.length });
    if (this.data.cur) this.setData({ cur: all.find(r => r.id === this.data.cur.id) || null });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); this.refresh(); },
  onItem(e) { this.open(String(e.currentTarget.dataset.id)); },
  open(id) {
    const r = store.getRecord(id);
    if (!r) return;
    const t = data.typeOf(r.type);
    const cur = { id: r.id, title: r.title, tname: t.name, paidTx: store.money(r.paid), time: r.time, seller: r.type === 'phone' ? (r.unit || '运营商') : (r.unit || '缴费单位'), invoice: r.invoice || null };
    if (r.invoice) { this.setData({ panel: 'view', cur }); return; }
    const p = wx.getStorageSync('lp_invoice_profile') || DEF;
    this.setData({ panel: 'apply', cur, kind: p.kind, title: p.title, taxNo: p.taxNo, email: p.email, formSeq: this.data.formSeq + 1 });
  },
  close() { this.setData({ panel: '' }); },
  noop() {},
  onKind(e) { this.setData({ kind: e.currentTarget.dataset.k }); },
  onTitle(e) { this.setData({ title: e.detail.value }); },
  onTax(e) { this.setData({ taxNo: e.detail.value.toUpperCase().replace(/\s/g, '') }); },
  onEmail(e) { this.setData({ email: e.detail.value.trim() }); },
  submit() {
    const d = this.data;
    const title = (d.title || '').trim();
    if (!title) { wx.showToast({ title: '请填写发票抬头', icon: 'none' }); return; }
    if (d.kind === 'company' && !/^[0-9A-Z]{15,20}$/.test(d.taxNo)) { wx.showToast({ title: '请填写 15~20 位税号', icon: 'none' }); return; }
    if (!/^[^@\s]+@[^@\s]+\.[A-Za-z]{2,}$/.test(d.email)) { wx.showToast({ title: '请填写正确的邮箱', icon: 'none' }); return; }
    const now = new Date();
    const inv = { kind: d.kind, title, taxNo: d.kind === 'company' ? d.taxNo : '', email: d.email, no: String(24330000 + now.getTime() % 100000000).slice(0, 8), at: store.stamp(now), sent: 1 };
    store.updateRecord(d.cur.id, { invoice: inv });
    wx.setStorageSync('lp_invoice_profile', { kind: d.kind, title, taxNo: inv.taxNo, email: d.email });
    wx.showToast({ title: '开票成功，已发送至邮箱', icon: 'none' });
    this.setData({ tab: 'done' });
    this.refresh();
    this.open(d.cur.id);
  },
  copyLink() {
    const inv = this.data.cur.invoice;
    host.copy('https://fp.lifepay.example.com/i/' + inv.no, '下载链接已复制');
  },
  resend() {
    const r = store.getRecord(this.data.cur.id);
    r.invoice.sent = (r.invoice.sent || 1) + 1;
    store.updateRecord(r.id, { invoice: r.invoice });
    this.refresh();
    wx.showToast({ title: '已重新发送至 ' + r.invoice.email, icon: 'none' });
  }
});
