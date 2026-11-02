const store = require('../../utils/store.js');

// 税号：统一社会信用代码 18 位（数字 + 大写字母，不含 I O Z S V），老税号 15 / 20 位
const TAX = /^[0-9A-HJ-NPQRTUWXY]{15}$|^[0-9A-HJ-NPQRTUWXY]{18}$|^[0-9A-HJ-NPQRTUWXY]{20}$/;
const MAIL = /^[\w.+-]+@[\w-]+(\.[\w-]+)+$/;

Page({
  data: { id: '', type: 'company', name: '', taxNo: '', bank: '', account: '', email: '', isDefault: false, err: {} },
  onLoad(q) {
    const t = q.id && store.titles().find((x) => x.id === q.id);
    if (t) this.setData(Object.assign({ bank: '', account: '', email: '' }, t, { err: {} }));
    else wx.setNavigationBarTitle({ title: '新增抬头' });
    if (!store.titles().length) this.setData({ isDefault: true });
  },
  onType(e) { this.setData({ type: e.detail.value, err: {} }); },
  pickType(e) { this.setData({ type: e.currentTarget.dataset.v, err: {} }); },
  onInput(e) {
    const k = e.currentTarget.dataset.k;
    let v = e.detail.value;
    if (k === 'taxNo') v = v.toUpperCase().replace(/\s/g, '');
    this.setData({ [k]: v, ['err.' + k]: '' });
    return k === 'taxNo' ? v : undefined;
  },
  onDefault(e) { this.setData({ isDefault: e.detail.value }); },
  save() {
    const d = this.data, err = {};
    const name = d.name.trim();
    if (!name) err.name = '请填写抬头名称';
    else if (d.type === 'company' && name.length < 4) err.name = '企业名称至少 4 个字';
    if (d.type === 'company' && !TAX.test(d.taxNo)) err.taxNo = '税号应为 15、18 或 20 位数字和大写字母';
    if (d.email && !MAIL.test(d.email.trim())) err.email = '邮箱格式不正确';
    this.setData({ err });
    if (Object.keys(err).length) { wx.showToast({ title: '请检查标红的项', icon: 'none' }); return; }
    const t = { type: d.type, name, taxNo: d.type === 'company' ? d.taxNo : '', bank: d.type === 'company' ? d.bank.trim() : '', account: d.type === 'company' ? d.account : '', email: d.email.trim(), isDefault: d.isDefault };
    if (d.id) t.id = d.id;
    store.saveTitle(t);
    wx.showToast({ title: '已保存', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 500);
  }
});
