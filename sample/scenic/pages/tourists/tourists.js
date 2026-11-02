// 常用游客：新增 / 编辑 / 删除，身份证按 GB 11643 校验位校验；从预订页 ?pick=1 进来时保存后直接返回
const store = require('../../utils/store.js');
const idc = require('../../utils/idcard.js');
Page({
  data: { list: [], pick: false, form: null, errors: {} },
  onLoad(q) { this.setData({ pick: q && q.pick === '1' }); },
  onShow() { this.refresh(); },
  refresh() {
    this.setData({ list: store.tourists().map((t) => Object.assign({}, t, { idMask: idc.mask(t.idcard), age: idc.check(t.idcard).age })) });
  },
  add() { this.setData({ form: { id: '', name: '', idcard: '', phone: '' }, errors: {} }); },
  edit(e) {
    const t = store.tourists().find((x) => x.id === e.currentTarget.dataset.id);
    this.setData({ form: Object.assign({}, t), errors: {} });
  },
  close() { this.setData({ form: null }); },
  onField(e) { this.setData({ ['form.' + e.currentTarget.dataset.k]: e.detail.value, ['errors.' + e.currentTarget.dataset.k]: '' }); },
  save() {
    const f = this.data.form, errors = {};
    if (!/^[\u4e00-\u9fa5·]{2,15}$/.test((f.name || '').trim())) errors.name = '请填写与证件一致的中文姓名';
    const r = idc.check(f.idcard);
    if (!r.ok) errors.idcard = r.msg;
    if (f.phone && !/^1[3-9]\d{9}$/.test(f.phone)) errors.phone = '手机号格式不对';
    const dup = store.tourists().find((x) => x.idcard === r.id && x.id !== f.id);
    if (r.ok && dup) errors.idcard = '这位游客已经添加过了（' + dup.name + '）';
    this.setData({ errors });
    if (Object.keys(errors).length) return;
    store.saveTourist({ id: f.id || 't' + Date.now(), name: f.name.trim(), idcard: r.id, phone: f.phone, idTail: r.id.slice(-4) });
    this.setData({ form: null });
    wx.showToast({ title: '已保存', icon: 'success' });
    if (this.data.pick) setTimeout(() => wx.navigateBack(), 400); else this.refresh();
  },
  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除游客', content: '删除后预订时需要重新填写', confirmColor: '#e5484d',
      success: (r) => { if (r.confirm) { store.removeTourist(id); this.refresh(); } }
    });
  }
});
