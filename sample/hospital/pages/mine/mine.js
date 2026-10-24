const store = require('../../utils/store.js');

Page({
  data: {
    me: {}, waitCount: 0, patientCount: 0, followCount: 0, remind: true,
    menus: [
      { key: 'orders', name: '我的挂号单', ch: '单', bg: 'linear-gradient(135deg, #5aa2ff, #1f7cf0)', extra: '' },
      { key: 'patients', name: '就诊人管理', ch: '人', bg: 'linear-gradient(135deg, #7fd88e, #3cb35a)', extra: '' },
      { key: 'report', name: '检查检验报告', ch: '报', bg: 'linear-gradient(135deg, #a18cff, #7b6cf6)', extra: '2 份新报告' },
      { key: 'invoice', name: '电子发票', ch: '票', bg: 'linear-gradient(135deg, #ff8e9e, #f2557a)', extra: '' }
    ]
  },
  onShow() {
    const ps = store.getPatients();
    const me = ps.find(p => p.relation === '本人') || ps[0];
    this.setData({
      me: Object.assign({}, me, { sx: me.name.charAt(0) }),
      waitCount: store.getOrders().filter(o => o.status === 'wait').length,
      patientCount: ps.length,
      followCount: (wx.getStorageSync('hs_follow') || []).length,
      remind: wx.getStorageSync('hs_remind') !== false
    });
  },
  onMenu(e) {
    const k = e.currentTarget.dataset.key;
    if (k === 'orders') this.goOrders();
    else if (k === 'patients') this.goPatients();
    else if (k === 'report') wx.showModal({ title: '检查检验报告', content: '血常规（2024-10-15）\n胸部 CT 平扫（2024-10-15）', showCancel: false });
    else wx.showToast({ title: '暂无可开具的发票', icon: 'none' });
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  goPatients() { wx.navigateTo({ url: '/pages/patients/patients' }); },
  showCard() { wx.showModal({ title: '电子就诊卡', content: this.data.me.name + '\n' + this.data.me.card, showCancel: false }); },
  onRemind(e) {
    wx.setStorageSync('hs_remind', e.detail.value);
    wx.showToast({ title: e.detail.value ? '已开启就诊提醒' : '已关闭就诊提醒', icon: 'none' });
  },
  callService() { wx.showModal({ title: '客服热线', content: '021-6438 2200（工作日 08:00-18:00）', confirmText: '呼叫' }); }
});
