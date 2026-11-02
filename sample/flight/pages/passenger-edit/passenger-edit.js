const ID = require('../../utils/idcard');
const store = require('../../utils/store');

// 证件号最大长度随证件类型变：身份证 18、护照 9、港澳 9、台湾 8、永居 18
const MAXLEN = [18, 9, 9, 8, 18];

Page({
  data: { id: '', name: '', idType: 0, idNo: '', phone: '', self: false, types: ID.TYPES, maxlen: 18, errors: {}, age: '', typeName: ID.TYPES[0] },
  onLoad(q) {
    const p = q.id ? store.passengers().find(x => x.id === q.id) : null;
    if (p) {
      this.setData({ id: p.id, name: p.name, idType: p.idType, idNo: p.idNo, phone: p.phone || '', self: !!p.self, maxlen: MAXLEN[p.idType], typeName: ID.TYPES[p.idType], age: ID.ageType(p.idType, p.idNo) });
      wx.setNavigationBarTitle({ title: '编辑乘机人' });
    } else {
      wx.setNavigationBarTitle({ title: '新增乘机人' });
    }
  },
  onName(e) { this.setData({ name: e.detail.value, 'errors.name': '' }); },
  onType(e) {
    const t = +e.detail.value;
    this.setData({ idType: t, typeName: ID.TYPES[t], maxlen: MAXLEN[t], idNo: this.data.idNo.slice(0, MAXLEN[t]), 'errors.idNo': '' });
  },
  onIdNo(e) {
    const v = e.detail.value.toUpperCase();
    this.setData({ idNo: v, 'errors.idNo': '', age: v.length === 18 && !ID.check(this.data.idType, v) ? ID.ageType(this.data.idType, v) : '' });
  },
  checkIdNo() { if (this.data.idNo) this.setData({ 'errors.idNo': ID.check(this.data.idType, this.data.idNo) }); },
  onPhone(e) { this.setData({ phone: e.detail.value, 'errors.phone': '' }); },
  save() {
    const d = this.data, errors = {};
    const ne = ID.checkName(d.name); if (ne) errors.name = ne;
    const ie = ID.check(d.idType, d.idNo); if (ie) errors.idNo = ie;
    if (d.phone && ID.checkPhone(d.phone)) errors.phone = ID.checkPhone(d.phone);
    const dup = store.passengers().find(p => p.idNo === d.idNo.trim().toUpperCase() && p.id !== d.id);
    if (!ie && dup) errors.idNo = '该证件已添加为乘机人「' + dup.name + '」';
    this.setData({ errors });
    if (Object.keys(errors).length) return wx.showToast({ title: '请检查标红的信息', icon: 'none' });
    const p = { name: d.name.trim(), idType: d.idType, idNo: d.idNo.trim().toUpperCase(), phone: d.phone, self: d.self };
    if (d.id) p.id = d.id;
    store.savePassenger(p);
    wx.showToast({ title: '已保存', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 500);
  },
  remove() {
    wx.showModal({
      title: '删除乘机人', content: '确定删除「' + this.data.name + '」吗？已下的订单不受影响。', confirmColor: '#f04438',
      success: (r) => { if (r.confirm) { store.removePassenger(this.data.id); wx.navigateBack(); } }
    });
  }
});
