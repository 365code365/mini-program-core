const store = require('../../utils/store.js');
const EMPTY = { name: '', gender: '男', birth: '', idcard: '', phone: '', isDefault: false };

Page({
  data: { list: [], relations: ['本人', '配偶', '子女', '父母', '其他'], relIdx: -1, form: Object.assign({}, EMPTY) },
  onShow() { this.refresh(); },
  refresh() {
    this.setData({ list: store.getPatients().map(p => Object.assign({}, p, { sx: p.name.charAt(0) })) });
  },
  onField(e) { this.setData({ ['form.' + e.currentTarget.dataset.f]: e.detail.value }); },
  onRel(e) { this.setData({ relIdx: Number(e.detail.value) }); },
  onGender(e) { this.setData({ 'form.gender': e.detail.value }); },
  onBirth(e) { this.setData({ 'form.birth': e.detail.value }); },
  onDefault(e) { this.setData({ 'form.isDefault': e.detail.value }); },
  save() {
    const f = this.data.form;
    if (f.name.length < 2) return wx.showToast({ title: '请填写真实姓名', icon: 'none' });
    if (this.data.relIdx < 0) return wx.showToast({ title: '请选择与本人关系', icon: 'none' });
    if (!/^\d{17}[\dXx]$/.test(f.idcard)) return wx.showToast({ title: '身份证号格式不正确', icon: 'none' });
    if (!/^1\d{10}$/.test(f.phone)) return wx.showToast({ title: '请填写 11 位手机号', icon: 'none' });
    let list = store.getPatients();
    if (list.length >= 5) return wx.showToast({ title: '最多添加 5 位就诊人', icon: 'none' });
    if (f.isDefault) list = list.map(p => Object.assign({}, p, { isDefault: false }));
    list.push({
      id: 'p' + Date.now(), name: f.name, relation: this.data.relations[this.data.relIdx], gender: f.gender,
      idcard: f.idcard.slice(0, 6) + '********' + f.idcard.slice(14), phone: f.phone.slice(0, 3) + '****' + f.phone.slice(7),
      card: '0013 ' + f.phone.slice(3, 7) + ' ' + f.idcard.slice(14), isDefault: f.isDefault
    });
    store.savePatients(list);
    this.setData({ form: Object.assign({}, EMPTY), relIdx: -1 });
    this.refresh();
    wx.showToast({ title: '添加成功', icon: 'success' });
  },
  onItem(e) {
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({
      itemList: ['设为默认就诊人', '删除就诊人'],
      success: (r) => {
        let list = store.getPatients();
        if (r.tapIndex === 0) {
          list = list.map(p => Object.assign({}, p, { isDefault: p.id === id }));
          wx.showToast({ title: '已设为默认', icon: 'none' });
        } else {
          if (list.length <= 1) return wx.showToast({ title: '至少保留一位就诊人', icon: 'none' });
          list = list.filter(p => p.id !== id);
        }
        store.savePatients(list);
        this.refresh();
      }
    });
  }
});
