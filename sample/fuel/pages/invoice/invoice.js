const store = require('../../utils/store.js');

const MAIL = /^[\w.+-]+@[\w-]+(\.[\w-]+)+$/;

Page({
  data: { r: null, timeText: '', issuedText: '', titles: [], titleId: '', email: '', err: {}, tab: 'todo', todo: [], done: [] },
  onLoad(q) {
    this.id = q.id || '';
    wx.setNavigationBarTitle({ title: this.id ? '开具电子发票' : '电子发票' });
  },
  onShow() { this.refresh(); },
  refresh() {
    if (!this.id) {
      const all = store.records().map((r) => Object.assign({}, r, { timeText: store.fmt(r.createdAt, true) }));
      this.setData({ todo: all.filter((r) => !r.invoice), done: all.filter((r) => r.invoice) });
      return;
    }
    const r = store.recordById(this.id);
    if (!r) return;
    const titles = store.titles();
    const keep = titles.find((t) => t.id === this.data.titleId);
    const def = keep || titles.find((t) => t.isDefault) || titles[0];
    this.setData({
      r, timeText: store.fmt(r.createdAt, true), issuedText: r.invoice ? store.fmt(r.invoice.at, true) : '',
      titles, titleId: def ? def.id : '', email: this.data.email || (def && def.email) || ''
    });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  open(e) { wx.navigateTo({ url: '/pages/invoice/invoice?id=' + e.currentTarget.dataset.id }); },
  pickTitle(e) { this.setData({ titleId: e.currentTarget.dataset.id, 'err.title': '' }); },
  onEmail(e) { this.setData({ email: e.detail.value.trim(), 'err.email': '' }); },
  manage() {
    wx.navigateTo({
      url: '/pages/titles/titles' + (this.id ? '?select=1' : ''),
      events: { picked: (t) => this.setData({ titleId: t.id, 'err.title': '' }) }
    });
  },
  // 微信抬头：真机弹微信保存的发票抬头；桌面 fail → 自建抬头
  fromWechat() {
    wx.chooseInvoiceTitle({
      success: (res) => {
        const list = store.saveTitle({ type: res.type === 0 || res.type === '0' ? 'company' : 'person', name: res.title, taxNo: res.taxNumber || '', address: res.companyAddress || '', phone: res.telephone || '', bank: res.bankName || '', account: res.bankAccount || '', isDefault: false });
        this.setData({ titleId: list.find((t) => t.name === res.title).id });
        this.refresh();
      },
      fail: () => {
        wx.showModal({
          title: '无法读取微信发票抬头', content: '当前环境不支持，可在本小程序里保存常用抬头，下次直接选', confirmText: '去添加',
          success: (r) => { if (r.confirm) wx.navigateTo({ url: '/pages/title-edit/title-edit' }); }
        });
      }
    });
  },
  submit() {
    const d = this.data, err = {};
    const t = d.titles.find((x) => x.id === d.titleId);
    if (!t) err.title = '请选择或新增发票抬头';
    if (d.email && !MAIL.test(d.email)) err.email = '邮箱格式不正确';
    this.setData({ err });
    if (Object.keys(err).length) return;
    wx.showLoading({ title: '开票中', mask: true });
    setTimeout(() => {
      wx.hideLoading();
      const now = Date.now();
      store.updateRecord(this.id, { invoice: { title: t.name, type: t.type, taxNo: t.taxNo || '', at: now, email: d.email, number: '2633' + String(now).slice(-8) } });
      wx.showToast({ title: '开票成功', icon: 'success' });
      this.refresh();
    }, 500);
  },
  resend() {
    const r = this.data.r;
    const send = (mail) => {
      store.updateRecord(r.id, { invoice: Object.assign({}, r.invoice, { email: mail }) });
      wx.showToast({ title: '已发送', icon: 'success' });
      this.refresh();
    };
    if (r.invoice.email) { send(r.invoice.email); return; }
    if (!MAIL.test(this.data.email)) {
      wx.showModal({ title: '还没有邮箱', content: '请在抬头里填写常用邮箱，或复制发票号码自行下载', confirmText: '复制号码', success: (x) => x.confirm && wx.setClipboardData({ data: r.invoice.number || r.no }) });
      return;
    }
    send(this.data.email);
  }
});
