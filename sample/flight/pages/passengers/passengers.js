const ID = require('../../utils/idcard');
const store = require('../../utils/store');

// 乘机人管理；mode=select 时多选，确定后经 EventChannel 的 pickPassengers 回传给填写订单页
Page({
  data: { mode: 'manage', list: [], count: 0 },
  onLoad(q) {
    this.ids = [];
    this.setData({ mode: q.mode || 'manage' });
    if (q.mode === 'select') wx.setNavigationBarTitle({ title: '选择乘机人' });
    this.channel = this.getOpenerEventChannel ? this.getOpenerEventChannel() : null;
    if (this.channel && this.channel.on) this.channel.on('init', (d) => { this.ids = (d.ids || []).slice(); this.refresh(); });
  },
  onShow() { this.refresh(); },
  refresh() {
    const all = store.passengers();
    this.ids = this.ids.filter(id => all.some(p => p.id === id));
    const list = all.map(p => ({ id: p.id, name: p.name, self: p.self, age: ID.ageType(p.idType, p.idNo), type: ID.TYPES[p.idType],
      masked: ID.mask(p.idNo), phone: p.phone ? p.phone.slice(0, 3) + '****' + p.phone.slice(-4) : '', on: this.ids.indexOf(p.id) >= 0 }));
    this.setData({ list, count: this.ids.length });
  },
  tap(e) {
    const id = e.currentTarget.dataset.id;
    if (this.data.mode !== 'select') return this.edit(e);
    const i = this.ids.indexOf(id);
    if (i >= 0) this.ids.splice(i, 1);
    else if (this.ids.length >= 9) return wx.showToast({ title: '一个订单最多 9 位乘机人', icon: 'none' });
    else this.ids.push(id);
    this.refresh();
  },
  edit(e) { wx.navigateTo({ url: '/pages/passenger-edit/passenger-edit?id=' + e.currentTarget.dataset.id }); },
  add() { wx.navigateTo({ url: '/pages/passenger-edit/passenger-edit' }); },
  confirm() {
    if (!this.ids.length) return wx.showToast({ title: '请选择乘机人', icon: 'none' });
    if (this.channel && this.channel.emit) this.channel.emit('pickPassengers', { ids: this.ids.slice() });
    wx.navigateBack();
  }
});
