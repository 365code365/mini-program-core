const store = require('../../utils/store.js');
Page({
  data: { select: false, max: 1, list: [], picked: [], panel: false, form: { id: '', name: '', phone: '' }, seq: 0 },
  onLoad(q) {
    const select = String(q.select) === '1';
    this.setData({ select, max: Number(q.max) || 1 });
    if (select) wx.setNavigationBarTitle({ title: '选择入住人' });
  },
  onShow() { this.load(); },
  load() {
    const picked = this.data.picked;
    this.setData({ list: store.getGuests().map(g => Object.assign({}, g, { av: g.name.slice(0, 1), phoneMask: store.maskPhone(g.phone) || '未填手机号', on: picked.indexOf(g.id) >= 0 })) });
  },
  // 选择模式：勾选（每间 1 位，最多 max 位）；管理模式：点了编辑
  onPick(e) {
    const id = String(e.currentTarget.dataset.id);
    if (!this.data.select) { this.onEdit(e); return; }
    let picked = this.data.picked.slice();
    if (picked.indexOf(id) >= 0) picked = picked.filter(x => x !== id);
    else if (picked.length >= this.data.max) { wx.showToast({ title: '最多选 ' + this.data.max + ' 位（每间 1 位）', icon: 'none' }); return; }
    else picked.push(id);
    this.setData({ picked });
    this.load();
  },
  onConfirm() {
    if (!this.data.picked.length) { wx.showToast({ title: '请至少选择 1 位入住人', icon: 'none' }); return; }
    const all = store.getGuests();
    wx.setStorageSync(store.K.guestPick, this.data.picked.map(id => (all.find(g => g.id === id) || {}).name).filter(Boolean));
    wx.navigateBack();
  },
  onAdd() { this.setData({ panel: true, form: { id: '', name: '', phone: '' }, seq: this.data.seq + 1 }); },
  onEdit(e) {
    const g = store.getGuests().find(x => x.id === String(e.currentTarget.dataset.id));
    if (g) this.setData({ panel: true, form: { id: g.id, name: g.name, phone: g.phone || '' }, seq: this.data.seq + 1 });
  },
  onName(e) { this.setData({ 'form.name': e.detail.value }); },
  onPhone(e) { this.setData({ 'form.phone': e.detail.value }); },
  close() { this.setData({ panel: false }); },
  noop() {},
  onSave() {
    const f = this.data.form;
    const name = f.name.trim();
    const phone = f.phone.trim();
    if (name.length < 2) { wx.showToast({ title: '请填写与证件一致的姓名', icon: 'none' }); return; }
    if (phone && !/^1\d{10}$/.test(phone)) { wx.showToast({ title: '手机号格式不对', icon: 'none' }); return; }
    if (store.getGuests().some(g => g.name === name && g.id !== f.id)) { wx.showToast({ title: '已有同名入住人', icon: 'none' }); return; }
    const g = store.upsertGuest({ id: f.id, name, phone });
    // 选择模式下新增的人顺手勾上
    if (this.data.select && !f.id && this.data.picked.length < this.data.max) this.setData({ picked: this.data.picked.concat([g.id]) });
    this.setData({ panel: false });
    wx.showToast({ title: '已保存', icon: 'success' });
    this.load();
  },
  onDel() {
    const f = this.data.form;
    wx.showModal({
      title: '删除入住人', content: '确定删除「' + f.name + '」吗？', confirmText: '删除', confirmColor: '#FF5A36',
      success: r => {
        if (!r.confirm) return;
        store.removeGuest(f.id);
        this.setData({ panel: false, picked: this.data.picked.filter(x => x !== f.id) });
        this.load();
      }
    });
  }
});
