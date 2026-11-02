const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const DAYS = ['每月 5 日', '每月 10 日', '每月 15 日', '每月 20 日'];
Page({
  data: { city: '杭州市', cities: data.cities, types: data.types, accounts: [], dues: [], dueTotal: '0.00', monthPaid: '0.00', cur: 0 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    const accounts = store.getAccounts().map(store.view);
    const dues = accounts.filter(a => a.owe > 0);
    const now = new Date();
    const ym = now.getFullYear() + '-' + (now.getMonth() + 1 < 10 ? '0' : '') + (now.getMonth() + 1);
    const monthPaid = store.getRecords().filter(r => r.time.slice(0, 7) === ym).reduce((s, r) => s + r.paid, 0);
    this.setData({ accounts, dues, dueTotal: store.money(dues.reduce((s, a) => s + a.owe, 0)), monthPaid: store.money(monthPaid),
      city: wx.getStorageSync('lp_city') || '杭州市' });
    this.syncNotice();
    const auto = store.getAuto();
    this.setData({ remindTx: wx.getStorageSync('lp_remind') === false ? '欠费提醒已关闭' : '每月提醒：' + DAYS[Number(wx.getStorageSync('lp_remind_day')) || 0].replace('每月 ', ''),
      autoIds: auto.ids });
  },
  syncNotice() {
    const city = wx.getStorageSync('lp_city') || '杭州市';
    const list = data.notices.filter(n => n.city === city);
    const read = wx.getStorageSync('lp_notice_read');
    const unread = list.filter(n => !(Array.isArray(read) && read.indexOf(n.id) >= 0)).length;
    this.setData({ noticeTx: list.length ? list[0].title + ' · ' + list[0].time : city + '近期没有停水停电公告', noticeCount: list.length, noticeNew: unread > 0 });
  },
  onCity(e) {
    const city = data.cities[Number(e.detail.value)];
    wx.setStorageSync('lp_city', city);
    this.setData({ city });
    this.syncNotice();
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
  // 一键缴清：所有欠费一次付清，每个户号各记一笔
  payAll() {
    const dues = this.data.dues;
    if (!dues.length || this.paying) return;
    this.paying = true;
    const total = this.data.dueTotal;
    host.pay(Number(total), dues.length + ' 笔生活缴费：' + dues.map(a => a.tname + ' ¥' + a.oweTx).join('、')).then(ok => {
      this.paying = false;
      if (!ok) return;
      dues.forEach(a => store.settle(a.id, a.owe, { way: '微信支付' }));
      this.onShow();
      wx.showToast({ title: '已缴清 ' + dues.length + ' 笔，共 ¥' + total, icon: 'none' });
    });
  },
  onSwiper(e) { this.setData({ cur: e.detail.current }); },
  goAccounts() { wx.navigateTo({ url: '/pages/accounts/accounts' }); },
  goAdd() { wx.navigateTo({ url: '/pages/accounts/accounts?add=1' }); },
  goRecords() { wx.switchTab({ url: '/pages/records/records' }); },
  notice() { wx.navigateTo({ url: '/pages/notice/notice' }); }
});
