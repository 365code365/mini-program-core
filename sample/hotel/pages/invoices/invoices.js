const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { select: false, list: [], panel: false, types: ['个人', '企业'], form: { id: '', type: 1, title: '', taxNo: '' }, seq: 0 },
  onLoad(q) {
    const select = String(q.select) === '1';
    this.setData({ select });
    if (select) wx.setNavigationBarTitle({ title: '选择发票抬头' });
  },
  onShow() { this.load(); },
  load() { this.setData({ list: store.getInvoices() }); },
  // 选择模式：点哪个用哪个；管理模式：点了编辑
  onPick(e) {
    if (!this.data.select) { this.onEdit(e); return; }
    wx.setStorageSync(store.K.invPick, String(e.currentTarget.dataset.id));
    wx.navigateBack();
  },
  onAdd() { this.setData({ panel: true, form: { id: '', type: 1, title: '', taxNo: '' }, seq: this.data.seq + 1 }); },
  onEdit(e) {
    const v = store.getInvoices().find(x => x.id === String(e.currentTarget.dataset.id));
    if (v) this.setData({ panel: true, form: Object.assign({ taxNo: '' }, v), seq: this.data.seq + 1 });
  },
  onImport() {
    host.invoiceTitle().then(v => {
      if (!v) return;
      store.upsertInvoice(v);
      wx.showToast({ title: '已导入微信抬头', icon: 'none' });
      this.load();
    });
  },
  onType(e) { this.setData({ 'form.type': Number(e.currentTarget.dataset.i) }); },
  onTitle(e) { this.setData({ 'form.title': e.detail.value }); },
  onTax(e) { this.setData({ 'form.taxNo': e.detail.value }); },
  close() { this.setData({ panel: false }); },
  noop() {},
  onSave() {
    const f = this.data.form;
    const title = f.title.trim();
    const taxNo = f.taxNo.trim().toUpperCase();
    if (title.length < 2) { wx.showToast({ title: f.type === 1 ? '请填写公司全称' : '请填写个人姓名', icon: 'none' }); return; }
    if (f.type === 1 && !/^[0-9A-Z]{15,20}$/.test(taxNo)) { wx.showToast({ title: '请填写 15-20 位纳税人识别号', icon: 'none' }); return; }
    store.upsertInvoice({ id: f.id, type: f.type, title, taxNo: f.type === 1 ? taxNo : '' });
    this.setData({ panel: false });
    wx.showToast({ title: '已保存', icon: 'success' });
    this.load();
  },
  onDel() {
    const f = this.data.form;
    wx.showModal({
      title: '删除抬头', content: '确定删除「' + f.title + '」吗？', confirmText: '删除', confirmColor: '#FF5A36',
      success: r => {
        if (!r.confirm) return;
        store.removeInvoice(f.id);
        this.setData({ panel: false });
        this.load();
      }
    });
  }
});
