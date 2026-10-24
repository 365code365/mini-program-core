const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { city: '杭州市', cities: data.cities, types: data.types, accounts: [], dues: [], dueTotal: '0.00', monthPaid: '0.00', cur: 0 },
  onShow() {
    const accounts = store.getAccounts().map(store.view);
    const dues = accounts.filter(a => a.owe > 0);
    const now = new Date();
    const ym = now.getFullYear() + '-' + (now.getMonth() + 1 < 10 ? '0' : '') + (now.getMonth() + 1);
    const monthPaid = store.getRecords().filter(r => r.time.slice(0, 7) === ym).reduce((s, r) => s + r.paid, 0);
    this.setData({ accounts, dues, dueTotal: store.money(dues.reduce((s, a) => s + a.owe, 0)), monthPaid: store.money(monthPaid),
      city: wx.getStorageSync('lp_city') || '杭州市' });
  },
  onCity(e) {
    const city = data.cities[Number(e.detail.value)];
    wx.setStorageSync('lp_city', city);
    this.setData({ city });
  },
  onType(e) {
    const k = e.currentTarget.dataset.k;
    if (k === 'phone') { wx.navigateTo({ url: '/pages/mobile/mobile' }); return; }
    const list = this.data.accounts.filter(a => a.type === k);
    if (!list.length) {
      wx.showModal({ title: '还没有' + data.typeOf(k).name + '户号', content: '添加户号后即可查询欠费并在线缴纳', confirmText: '去添加', success: r => {
        if (r.confirm) wx.navigateTo({ url: '/pages/accounts/accounts?add=1&type=' + k });
      } });
      return;
    }
    const pick = list.find(a => a.owe > 0) || list[0];
    wx.navigateTo({ url: '/pages/pay/pay?id=' + pick.id });
  },
  goPay(e) { wx.navigateTo({ url: '/pages/pay/pay?id=' + e.currentTarget.dataset.id }); },
  payAll() {
    if (!this.data.dues.length) return;
    wx.navigateTo({ url: '/pages/pay/pay?id=' + this.data.dues[0].id });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  goAccounts() { wx.navigateTo({ url: '/pages/accounts/accounts' }); },
  goAdd() { wx.navigateTo({ url: '/pages/accounts/accounts?add=1' }); },
  goRecords() { wx.switchTab({ url: '/pages/records/records' }); },
  notice() { wx.showModal({ title: '停水通知', content: '因管网改造，西湖区文三路（学院路至教工路段）将于本周六 22:00 至次日 06:00 暂停供水，请提前储水。', showCancel: false, confirmText: '知道了' }); }
});
