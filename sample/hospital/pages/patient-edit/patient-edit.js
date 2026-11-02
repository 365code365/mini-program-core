const store = require('../../utils/store.js');
const RELATIONS = ['本人', '配偶', '子女', '父母', '其他'];
// 新增 / 编辑就诊人。编辑时姓名、身份证（以及由身份证推出的性别、生日）已实名，不能改；能改关系、手机号、默认
// 身份证号填满 18 位就按号码自动带出出生日期与性别（第 17 位单数为男）
function fromIdcard(id) {
  if (!/^\d{17}[\dXx]$/.test(id)) return null;
  const y = +id.slice(6, 10), m = +id.slice(10, 12), d = +id.slice(12, 14);
  const t = new Date(y, m - 1, d);
  if (t.getFullYear() !== y || t.getMonth() !== m - 1 || t.getDate() !== d || t.getTime() > Date.now() || y < 1900) return null;
  return { birth: store.dateStr(t.getTime()), gender: Number(id.charAt(16)) % 2 ? '男' : '女' };
}
Page({
  data: {
    editing: false, id: '', relations: RELATIONS, relIdx: -1, today: '',
    form: { name: '', gender: '男', birth: '', idcard: '', phone: '', isDefault: false },
    idMasked: '', oldPhone: ''
  },
  onLoad(q) {
    const p = q.id ? store.findPatient(q.id) : null;
    this.setData({ today: store.today() });
    if (p) {
      this.setData({
        editing: true, id: p.id, relIdx: RELATIONS.indexOf(p.relation), idMasked: p.idcard, oldPhone: p.phone,
        form: { name: p.name, gender: p.gender, birth: p.birth || '', idcard: '', phone: '', isDefault: !!p.isDefault }
      });
      wx.setNavigationBarTitle({ title: '编辑就诊人' });
    } else {
      wx.setNavigationBarTitle({ title: '添加就诊人' });
    }
  },
  onField(e) { this.setData({ ['form.' + e.currentTarget.dataset.f]: e.detail.value }); },
  onIdcard(e) {
    const v = e.detail.value.trim();
    const auto = fromIdcard(v);
    this.setData(auto ? { 'form.idcard': v, 'form.birth': auto.birth, 'form.gender': auto.gender } : { 'form.idcard': v });
  },
  onRel(e) { this.setData({ relIdx: Number(e.detail.value) }); },
  onGender(e) { this.setData({ 'form.gender': e.detail.value }); },
  onBirth(e) { this.setData({ 'form.birth': e.detail.value }); },
  onDefault(e) { this.setData({ 'form.isDefault': e.detail.value }); },
  check() {
    const f = this.data.form;
    const phone = f.phone.trim();
    if (!this.data.editing) {
      if (!/^[\u4e00-\u9fa5·]{2,15}$/.test(f.name.trim())) return '请填写与身份证一致的中文姓名';
      if (!/^\d{17}[\dXx]$/.test(f.idcard)) return '身份证号应为 18 位';
      if (!fromIdcard(f.idcard)) return '身份证号里的出生日期不正确';
      if (!f.birth) return '请选择出生日期';
    }
    if (this.data.relIdx < 0) return '请选择与本人关系';
    if ((phone || !this.data.editing) && !/^1\d{10}$/.test(phone)) return '请填写 11 位手机号';
    return '';
  },
  save() {
    const err = this.check();
    if (err) return wx.showToast({ title: err, icon: 'none' });
    const f = this.data.form;
    const phone = f.phone.trim();
    const p = { id: this.data.id, relation: RELATIONS[this.data.relIdx], isDefault: f.isDefault };
    if (phone) p.phone = phone.slice(0, 3) + '****' + phone.slice(7);
    if (!this.data.editing) {
      const id = f.idcard.toUpperCase();
      Object.assign(p, {
        id: 'p' + Date.now(), name: f.name.trim(), gender: f.gender, birth: f.birth,
        idcard: id.slice(0, 6) + '********' + id.slice(14), card: '0013 ' + phone.slice(3, 7) + ' ' + id.slice(14)
      });
    }
    const msg = store.upsertPatient(p);
    if (msg) return wx.showToast({ title: msg, icon: 'none' });
    wx.showToast({ title: this.data.editing ? '已保存修改' : '添加成功', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 500);
  },
  remove() {
    const name = this.data.form.name;
    wx.showModal({
      title: '删除就诊人',
      content: '删除后「' + name + '」的挂号、缴费记录仍会保留，确定删除吗？',
      confirmText: '删除',
      confirmColor: '#ff5a36',
      success: r => {
        if (!r.confirm) return;
        const msg = store.removePatient(this.data.id);
        if (msg) {
          wx.showModal({ title: '暂时不能删除', content: msg, showCancel: false, confirmText: '知道了' });
          return;
        }
        wx.showToast({ title: '已删除', icon: 'none' });
        setTimeout(() => wx.navigateBack(), 500);
      }
    });
  }
});
