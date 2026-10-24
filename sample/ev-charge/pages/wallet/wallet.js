const data = require('../../utils/data.js');

const BASE_LOGS = [
  { id: 'l1', title: '充电消费·国网文一西路站', date: '10-22 19:08', v: -70.72 },
  { id: 'l2', title: '微信支付充值', date: '10-20 12:31', v: 100 },
  { id: 'l3', title: '充电消费·星联西溪银泰城站', date: '10-18 08:44', v: -52.10 }
];

Page({
  data: {
    balance: '0.00',
    amounts: [{ v: 20 }, { v: 50 }, { v: 100 }, { v: 200, gift: 10 }, { v: 300, gift: 20 }, { v: 500, gift: 40 }],
    aIdx: 2,
    custom: '',
    pays: [
      { name: '微信支付', glyph: '微', bg: 'linear-gradient(135deg, #3FD779, #07C160)' },
      { name: '云闪付', glyph: '云', bg: 'linear-gradient(135deg, #FF7A7A, #E5383B)' }
    ],
    pIdx: 0,
    autoPay: false,
    payText: '100',
    logs: []
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.setData({ autoPay: !!wx.getStorageSync('ev_autopay') });
    this.load();
  },
  load() {
    const mine = wx.getStorageSync('ev_logs') || [];
    const orders = (wx.getStorageSync('ev_orders') || []).map(o => ({ id: o.id, title: '充电消费·' + data.getStation(o.station).name.split('·').pop(), date: o.date, v: -o.fee }));
    const logs = mine.concat(orders).sort((a, b) => (a.date < b.date ? 1 : -1)).concat(BASE_LOGS)
      .map(l => Object.assign({}, l, { vText: (l.v > 0 ? '+' : '-') + Math.abs(l.v).toFixed(2) }));
    this.setData({ balance: data.getBalance().toFixed(2), logs });
  },
  amount() {
    if (this.data.aIdx >= 0) { const a = this.data.amounts[this.data.aIdx]; return a.v + (a.gift || 0); }
    return Number(this.data.custom) || 0;
  },
  pay() { return this.data.aIdx >= 0 ? this.data.amounts[this.data.aIdx].v : (Number(this.data.custom) || 0); },
  onAmount(e) { this.setData({ aIdx: e.currentTarget.dataset.idx, custom: '' }); this.setData({ payText: String(this.pay()) }); },
  onCustomFocus() { this.setData({ aIdx: -1, payText: String(Number(this.data.custom) || 0) }); },
  onCustom(e) { this.setData({ aIdx: -1, custom: e.detail.value }); this.setData({ payText: String(Number(e.detail.value) || 0) }); },
  onPay(e) { this.setData({ pIdx: e.currentTarget.dataset.idx }); },
  onAuto(e) { wx.setStorageSync('ev_autopay', e.detail.value); this.setData({ autoPay: e.detail.value }); },
  onRecharge() {
    const pay = this.pay();
    if (pay < 10 || pay > 2000) { wx.showToast({ title: '充值金额需在 10~2000 元', icon: 'none' }); return; }
    const get = this.amount();
    wx.showModal({
      title: '确认充值',
      content: '使用' + this.data.pays[this.data.pIdx].name + '支付 ¥' + pay + (get > pay ? '，到账 ¥' + get : ''),
      confirmText: '支付',
      confirmColor: '#00B87A',
      success: (r) => {
        if (!r.confirm) return;
        data.setBalance(data.getBalance() + get);
        const now = new Date();
        const p = n => (n < 10 ? '0' + n : '' + n);
        const logs = wx.getStorageSync('ev_logs') || [];
        logs.unshift({ id: 'r' + now.getTime(), title: this.data.pays[this.data.pIdx].name + '充值', date: p(now.getMonth() + 1) + '-' + p(now.getDate()) + ' ' + p(now.getHours()) + ':' + p(now.getMinutes()), v: get });
        wx.setStorageSync('ev_logs', logs);
        this.load();
        wx.showToast({ title: '充值成功', icon: 'success' });
      }
    });
  }
});
