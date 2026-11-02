const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const { qrDots } = require('../../utils/qr.js');
const SERVICE_PHONE = '021-6438 2200';
const PAGES = {
  patients: '/pages/patients/patients',
  report: '/pages/reports/reports',
  bills: '/pages/bills/bills',
  invoice: '/pages/bills/bills?tab=paid',
  consult: '/pages/consult/consult',
  inpatient: '/pages/inpatient/inpatient'
};
Page({
  data: {
    me: {}, waitCount: 0, patientCount: 0, followCount: 0, reportCount: 0, remind: true,
    menus: [
      { key: 'orders', name: '我的挂号单', ch: '单', bg: 'linear-gradient(135deg, #5aa2ff, #1f7cf0)', extra: '' },
      { key: 'patients', name: '就诊人管理', ch: '人', bg: 'linear-gradient(135deg, #7fd88e, #3cb35a)', extra: '' },
      { key: 'report', name: '检查检验报告', ch: '报', bg: 'linear-gradient(135deg, #a18cff, #7b6cf6)', extra: '' },
      { key: 'bills', name: '门诊缴费', ch: '缴', bg: 'linear-gradient(135deg, #ffb26b, #ff8240)', extra: '' },
      { key: 'invoice', name: '电子发票', ch: '票', bg: 'linear-gradient(135deg, #ff8e9e, #f2557a)', extra: '缴费记录里开具' },
      { key: 'consult', name: '在线问诊记录', ch: '诊', bg: 'linear-gradient(135deg, #37d2c0, #13a8a8)', extra: '' },
      { key: 'inpatient', name: '住院服务', ch: '住', bg: 'linear-gradient(135deg, #63c2ff, #2a9df4)', extra: '' }
    ],
    cardOpen: false, cardList: [], cardIdx: 0, qr: []
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    store.tick();
    const ps = store.getPatients();
    const me = ps.find(p => p.relation === '本人') || ps[0];
    const active = store.getOrders().filter(store.isActive).length;
    const unread = store.unreadReports();
    const unpaid = store.unpaidBills().length;
    const replied = store.getConsults().filter(c => c.status === 'reply').length;
    const ip = store.getInpatient();
    this.setData({
      me: Object.assign({}, me, { sx: me.name.charAt(0) }),
      waitCount: active,
      patientCount: ps.length,
      followCount: store.getFollows().length,
      reportCount: store.getReports().length,
      remind: wx.getStorageSync('hs_remind') !== false,
      'menus[0].extra': active ? active + ' 个待就诊' : '',
      'menus[1].extra': ps.length + '/' + store.MAX_PATIENTS + ' 位',
      'menus[2].extra': unread ? unread + ' 份未读' : '',
      'menus[3].extra': unpaid ? unpaid + ' 笔待缴' : '',
      'menus[5].extra': replied ? replied + ' 条新回复' : '',
      'menus[6].extra': ip.status === 'in' ? ip.patient + ' 住院中' : '已出院'
    });
  },
  onMenu(e) {
    const k = e.currentTarget.dataset.key;
    if (k === 'orders') this.goOrders();
    else if (PAGES[k]) wx.navigateTo({ url: PAGES[k] });
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  goPatients() { wx.navigateTo({ url: PAGES.patients }); },
  goFollow() { wx.navigateTo({ url: '/pages/doctors/doctors?follow=1' }); },
  goReports() { wx.navigateTo({ url: PAGES.report }); },
  // 电子就诊卡：每位就诊人一张，切换着出示
  showCard() {
    const list = store.getPatients().map(p => ({ id: p.id, name: p.name, relation: p.relation, card: p.card }));
    const i = Math.max(0, list.findIndex(p => p.id === this.data.me.id));
    this.setData({ cardOpen: true, cardList: list, cardIdx: i, qr: qrDots(list[i].card, 18, 14) });
  },
  pickCard(e) {
    const i = Number(e.currentTarget.dataset.i);
    this.setData({ cardIdx: i, qr: qrDots(this.data.cardList[i].card, 18, 14) });
  },
  closeCard() { this.setData({ cardOpen: false }); },
  noop() {},
  onRemind(e) {
    wx.setStorageSync('hs_remind', e.detail.value);
    this.setData({ remind: e.detail.value });
    wx.showToast({ title: e.detail.value ? '已开启，就诊前一天 18:00 提醒' : '已关闭就诊提醒', icon: 'none' });
  },
  callService() { host.call(SERVICE_PHONE, '客服热线（工作日 08:00-18:00）'); },
  goNavi() { wx.navigateTo({ url: '/pages/navi/navi' }); }
});
