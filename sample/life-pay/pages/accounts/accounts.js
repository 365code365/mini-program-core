const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const TAGS = ['我家', '父母家', '公司', '出租房'];
const PAY_TYPES = data.types.filter(t => t.k !== 'phone');
Page({
  data: {
    list: [], adding: false,
    cities: data.cities, cityIdx: 0,
    payTypes: PAY_TYPES, typeIdx: 0,
    unitList: [], unitIdx: 0,
    no: '', tags: TAGS, tag: '我家', setDefault: true
  },
  onLoad(q) {
    if (q.add) {
      const ti = Math.max(0, PAY_TYPES.findIndex(t => t.k === q.type));
      const ci = Math.max(0, data.cities.indexOf(wx.getStorageSync('lp_city') || '杭州市'));
      this.setData({ adding: true, typeIdx: ti, cityIdx: ci });
      wx.setNavigationBarTitle({ title: '添加户号' });
    }
    this.syncUnits();
    this.refresh();
  },
  refresh() { this.setData({ list: store.getAccounts().map(store.view) }); },
  syncUnits() {
    const list = data.units[data.cities[this.data.cityIdx]][PAY_TYPES[this.data.typeIdx].k];
    this.setData({ unitList: list, unitIdx: 0 });
  },
  openAdd() { this.setData({ adding: true }); wx.setNavigationBarTitle({ title: '添加户号' }); },
  closeAdd() { this.setData({ adding: false, no: '' }); wx.setNavigationBarTitle({ title: '户号管理' }); },
  onCity(e) { this.setData({ cityIdx: Number(e.detail.value) }); this.syncUnits(); },
  onType(e) { this.setData({ typeIdx: Number(e.currentTarget.dataset.i) }); this.syncUnits(); },
  onUnit(e) { this.setData({ unitIdx: Number(e.detail.value) }); },
  onNo(e) { this.setData({ no: e.detail.value.replace(/\s/g, '') }); },
  onTag(e) { this.setData({ tag: e.currentTarget.dataset.t }); },
  onDefault(e) { this.setData({ setDefault: e.detail.value }); },
  help() { wx.showModal({ title: '如何查找户号', content: '户号一般印在纸质账单右上角，或营业厅缴费回执的「用户编号」一栏，通常为 10~12 位数字。', showCancel: false }); },
  submit() {
    const d = this.data;
    if (!/^\d{8,12}$/.test(d.no)) { wx.showToast({ title: '请输入 8~12 位数字户号', icon: 'none' }); return; }
    const list = store.getAccounts();
    if (list.some(a => a.no === d.no)) { wx.showToast({ title: '该户号已添加', icon: 'none' }); return; }
    wx.showLoading({ title: '查询户号中' });
    setTimeout(() => {
      wx.hideLoading();
      const type = PAY_TYPES[d.typeIdx].k;
      // 离线演示：按户号末位模拟查到一笔欠费
      const owe = Number(d.no.slice(-1)) % 2 === 1 ? Math.round((40 + Number(d.no.slice(-3)) / 10) * 100) / 100 : 0;
      const acc = { id: 'a' + Date.now(), type, city: d.cities[d.cityIdx], unit: d.unitList[d.unitIdx], no: d.no, owner: '*明远', addr: d.cities[d.cityIdx] + '（以缴费单位登记为准）', tag: d.tag, owe, balance: 0, period: owe ? '上月' + PAY_TYPES[d.typeIdx].name : '' };
      if (d.setDefault) list.unshift(acc); else list.push(acc);
      store.saveAccounts(list);
      this.closeAdd();
      this.refresh();
      wx.showToast({ title: owe ? '添加成功，查到 1 笔欠费' : '添加成功', icon: 'none' });
    }, 500);
  },
  more(e) {
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({ itemList: ['去缴费', '复制户号', '删除户号'], success: r => {
      if (r.tapIndex === 0) wx.navigateTo({ url: '/pages/pay/pay?id=' + id });
      else if (r.tapIndex === 1) wx.showToast({ title: '户号已复制', icon: 'none' });
      else this.remove(id);
    } });
  },
  remove(id) {
    wx.showModal({ title: '删除户号', content: '删除后将不再提醒该户号的欠费账单，确定删除？', confirmColor: '#f2603d', success: r => {
      if (!r.confirm) return;
      store.saveAccounts(store.getAccounts().filter(a => a.id !== id));
      this.refresh();
      wx.showToast({ title: '已删除', icon: 'none' });
    } });
  }
});
