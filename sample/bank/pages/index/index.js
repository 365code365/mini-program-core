const S = require('../../utils/store.js');
const M = require('../../utils/money.js');

const ENTRIES = [
  { k: 'transfer', t: '转账', g: '转', c: '#F0534A' },
  { k: 'bills', t: '收支明细', g: '明', c: '#3E7BFA' },
  { k: 'wealth', t: '理财', g: '财', c: '#F7A23B' },
  { k: 'payees', t: '收款人', g: '人', c: '#22B07D' },
  { k: 'credit', t: '信用卡', g: '信', c: '#8B5CF6' },
  { k: 'phone', t: '手机充值', g: '充', c: '#06B6D4' },
  { k: 'life', t: '生活缴费', g: '缴', c: '#EC4899' },
  { k: 'mine', t: '安全中心', g: '安', c: '#64748B' }
];

Page({
  data: { greet: '你好', hide: false, accounts: [], cur: 0, curName: '', recent: [], entries: ENTRIES, pick: {} },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    const h = new Date().getHours();
    const greet = h < 6 ? '夜深了' : h < 12 ? '上午好' : h < 18 ? '下午好' : '晚上好';
    const accounts = S.accounts().map(a => Object.assign({}, a, { masked: M.maskCard(a.no), balText: M.fmt(a.balance) }));
    const holds = S.holdings();
    const hold = holds.reduce((s, x) => s + x.amount, 0);
    const total = accounts.reduce((s, a) => s + a.balance, 0) + hold;
    const now = new Date();
    const monthOut = S.bills().filter(b => b.type === 'out' && new Date(b.time).getMonth() === now.getMonth() && new Date(b.time).getFullYear() === now.getFullYear())
      .reduce((s, b) => s + b.amount, 0);
    const p = S.PRODUCTS[2];
    const cur = Math.min(S.get('curAcct', 0), accounts.length - 1);
    this.setData({
      greet, accounts, cur,
      hide: !!S.get('hide', false),
      totalText: M.fmt(total), holdText: M.fmt(hold), monthOutText: M.fmt(monthOut),
      yesterdayText: M.fmt(accounts[2].balance * 0.0205 / 365 + hold * 0.035 / 365),
      pick: { id: p.id, name: p.name, tag: p.tag, rateText: p.periods[p.periods.length - 1].rate.toFixed(2), minText: M.fmt(p.minBuy).replace('.00', '') + '元', riskName: S.RISK_NAME[p.risk] }
    });
    this.refreshRecent(cur);
  },

  refreshRecent(cur) {
    const acct = this.data.accounts[cur];
    const recent = S.bills().filter(b => b.acct === acct.id).slice(0, 3).map(b => ({
      id: b.id, title: b.title, type: b.type, g: b.cat.slice(0, 1),
      timeText: M.timeStr(b.time).slice(5),
      amtText: (b.type === 'in' ? '+' : '-') + M.fmt(b.amount)
    }));
    this.setData({ recent, curName: acct.name });
  },

  toggleEye() {
    const hide = !this.data.hide;
    S.set('hide', hide);
    this.setData({ hide });
  },

  onSwiper(e) {
    const cur = e.detail.current;
    S.set('curAcct', cur);
    this.setData({ cur });
    this.refreshRecent(cur);
  },

  go(e) {
    const k = e.currentTarget.dataset.page;
    if (k === 'bills' || k === 'wealth' || k === 'mine') return wx.switchTab({ url: '/pages/' + k + '/' + k });
    if (k === 'transfer') return wx.navigateTo({ url: '/pages/transfer/transfer?acct=' + this.data.cur });
    if (k === 'payees') return wx.navigateTo({ url: '/pages/payees/payees?mode=go' });
    wx.showToast({ title: '演示版暂未开放', icon: 'none' });
  },

  openBill(e) { wx.navigateTo({ url: '/pages/bill-detail/bill-detail?id=' + e.currentTarget.dataset.id }); },
  openProduct(e) { wx.navigateTo({ url: '/pages/product/product?id=' + e.currentTarget.dataset.id }); }
});
