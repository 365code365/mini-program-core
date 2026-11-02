const store = require('../../utils/store.js');

Page({
  data: { id: '', name: '', phone: '', region: [], detail: '', tag: '', isDefault: false, tags: ['家', '公司', '父母家'], err: {} },
  onLoad(q) {
    const a = q.id && store.addresses().find((x) => x.id === q.id);
    if (a) this.setData({ id: a.id, name: a.name, phone: a.phone, region: a.region, detail: a.detail, tag: a.tag || '', isDefault: !!a.isDefault });
    else wx.setNavigationBarTitle({ title: '新增地址' });
    if (!store.addresses().length) this.setData({ isDefault: true });
  },
  onInput(e) { const k = e.currentTarget.dataset.k; this.setData({ [k]: e.detail.value, ['err.' + k]: '' }); },
  onRegion(e) { this.setData({ region: e.detail.value, 'err.region': '' }); },
  onTag(e) { const t = e.currentTarget.dataset.t; this.setData({ tag: this.data.tag === t ? '' : t }); },
  onDefault(e) { this.setData({ isDefault: e.detail.value }); },
  validate() {
    const d = this.data, err = {};
    if (!d.name.trim()) err.name = '请填写收货人';
    else if (d.name.trim().length < 2) err.name = '姓名至少 2 个字';
    if (!/^1[3-9]\d{9}$/.test(d.phone)) err.phone = '请输入正确的 11 位手机号';
    if (d.region.length !== 3) err.region = '请选择所在地区';
    if (d.detail.trim().length < 5) err.detail = '详细地址至少 5 个字，写到门牌号';
    this.setData({ err });
    return !Object.keys(err).length;
  },
  save() {
    if (!this.validate()) { wx.showToast({ title: '请检查标红的项', icon: 'none' }); return; }
    const d = this.data;
    const a = { name: d.name.trim(), phone: d.phone, region: d.region, city: d.region[1], detail: d.detail.trim(), tag: d.tag, isDefault: d.isDefault };
    if (d.id) a.id = d.id;
    store.saveAddress(a);
    wx.showToast({ title: '已保存', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 500);
  },
  remove() {
    wx.showModal({
      title: '删除地址', content: '确定删除这条地址吗？', confirmColor: '#e4572e',
      success: (r) => { if (r.confirm) { store.removeAddress(this.data.id); wx.navigateBack(); } }
    });
  }
});
