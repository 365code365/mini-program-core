const S = require('../../utils/store.js');
const M = require('../../utils/money.js');

function wan(v) { return v >= 10000 ? v / 10000 + '万' : String(v); }
function addDays(t, n) { return M.dateStr(t + n * 86400000); }

Page({
  data: {
    p: {}, rateText: '', rateLabel: '', riskName: '', minText: '', periodNames: [], periodHint: '', pIdx: 0, periodName: '',
    amount: 50000, amountText: '50000', sliderVal: 50000, upperShort: '', sMin: 0, sMax: 300000, sStep: 1000, sMinText: '', sMaxText: '',
    presets: [10000, 50000, 100000], incomeText: '', totalText: '', cmp: [], dates: {}, acctNames: [], acctIdx: 0
  },

  onLoad(options) {
    const p = S.PRODUCTS.find(x => x.id === options.id) || S.PRODUCTS[0];
    this.p = p;
    const sMax = Math.min(p.maxBuy, 300000);
    const sMin = p.minBuy < 1000 ? 0 : p.minBuy;
    this.accounts = S.accounts();
    this.setData({
      p: { name: p.name, tag: p.tag, type: p.type, desc: p.desc },
      riskName: p.risk + ' ' + S.RISK_NAME[p.risk],
      minText: wan(p.minBuy) + '元',
      periodNames: p.periods.map(x => (x.days === 1 ? '随存随取' : x.days + '天') + ' · ' + x.rate.toFixed(2) + '%'),
      periodHint: p.periods.length > 1 ? '可选' : '固定',
      pIdx: p.periods.length - 1,
      sMin, sMax, sMinText: wan(Math.max(sMin, p.minBuy)), sMaxText: wan(sMax),
      presets: [10000, 50000, 100000].filter(v => v >= p.minBuy && v <= sMax),
      acctNames: this.accounts.map(a => a.name + '（' + a.no.slice(-4) + '）')
    });
    wx.setNavigationBarTitle({ title: p.name.split(' · ')[0] });
    this.setAmount(Math.max(50000, p.minBuy), true);
  },

  setAmount(v, syncSlider) {
    const amount = Math.max(0, Math.round(Number(v) || 0));
    const patch = { amount, upperShort: amount >= 10000 ? (amount / 10000).toFixed(amount % 10000 ? 2 : 0) + ' 万元' : '' };
    if (syncSlider) {
      patch.amountText = String(amount);
      patch.sliderVal = Math.min(this.data.sMax, Math.max(this.data.sMin, amount));
    }
    this.setData(patch);
    this.calc();
  },

  calc() {
    const per = this.p.periods[this.data.pIdx];
    const days = per.days === 1 ? 365 : per.days;
    const amt = this.data.amount;
    const income = amt * per.rate / 100 * days / 365;
    const rows = [
      { t: '本产品', r: per.rate, c: 'linear-gradient(90deg, #F0534A, #C8162E)' },
      { t: '一年定期', r: 1.45, c: '#F7A23B' },
      { t: '活期存款', r: 0.15, c: '#c4c7cc' }
    ];
    const max = rows[0].r > 1.45 ? rows[0].r : 1.45;
    const now = Date.now();
    this.setData({
      pIdx: this.data.pIdx,
      rateText: per.rate.toFixed(2),
      rateLabel: per.days === 1 ? '七日年化' : '业绩比较基准',
      periodName: per.days === 1 ? '按 1 年测算' : per.days + '天',
      incomeText: M.fmt(income), totalText: M.fmt(amt + income),
      cmp: rows.map(x => ({ t: x.t, c: x.c, w: Math.max(4, Math.round(x.r / max * 100)), v: '+' + M.fmt(amt * x.r / 100 * days / 365) })),
      dates: { buy: addDays(now, 0).slice(5), start: addDays(now, 1).slice(5), end: per.days === 1 ? '随时' : addDays(now, 1 + per.days).slice(5) }
    });
  },

  onSlide(e) {
    const v = Math.max(this.p.minBuy, Number(e.detail.value));
    this.setData({ amountText: String(v), sliderVal: Number(e.detail.value) });
    this.setAmount(v, false);
  },
  onAmountInput(e) {
    const s = String(e.detail.value).replace(/[^\d]/g, '').slice(0, 9);
    this.setData({ amountText: s, sliderVal: Math.min(this.data.sMax, Math.max(this.data.sMin, Number(s) || 0)) });
    this.setAmount(s, false);
    return s;
  },
  onAmountBlur() {
    const a = this.data.amount;
    if (a < this.p.minBuy) {
      wx.showToast({ title: '最低起购 ' + wan(this.p.minBuy) + ' 元', icon: 'none' });
      this.setAmount(this.p.minBuy, true);
    } else if (a > this.p.maxBuy) {
      wx.showToast({ title: '单笔最高 ' + wan(this.p.maxBuy) + ' 元', icon: 'none' });
      this.setAmount(this.p.maxBuy, true);
    }
  },
  preset(e) { this.setAmount(e.currentTarget.dataset.v, true); },
  onPeriod(e) { this.setData({ pIdx: Number(e.detail.value) }); this.calc(); },
  onAcct(e) { this.setData({ acctIdx: Number(e.detail.value) }); },

  buy() {
    const amt = this.data.amount;
    const acct = this.accounts[this.data.acctIdx];
    if (amt < this.p.minBuy) return wx.showToast({ title: '最低起购 ' + wan(this.p.minBuy) + ' 元', icon: 'none' });
    if (amt > this.p.maxBuy) return wx.showToast({ title: '超过单笔购买上限', icon: 'none' });
    if (amt > acct.balance) return wx.showToast({ title: acct.name + '余额不足', icon: 'none' });
    const per = this.p.periods[this.data.pIdx];
    wx.showModal({
      title: '确认购买',
      content: this.p.name + '，' + this.data.periodNames[this.data.pIdx] + '，买入 ¥' + M.fmt(amt) + '，从' + acct.name + '扣款，预计收益 ¥' + this.data.incomeText,
      confirmText: '确认买入',
      confirmColor: '#D7263D',
      success: r => {
        if (!r.confirm) return;
        wx.showLoading({ title: '处理中', mask: true });
        setTimeout(() => {
          const accounts = S.accounts();
          accounts[this.data.acctIdx].balance = Math.round((accounts[this.data.acctIdx].balance - amt) * 100) / 100;
          S.set('accounts', accounts);
          const hs = S.holdings();
          hs.unshift({ pid: this.p.id, name: this.p.name, amount: amt, days: per.days, rate: per.rate, income: Math.round(amt * per.rate / 100 * (per.days === 1 ? 365 : per.days) / 365 * 100) / 100, time: Date.now() });
          S.set('holdings', hs);
          S.addBill({ id: 'b' + Date.now(), acct: accounts[this.data.acctIdx].id, type: 'out', title: '购买 · ' + this.p.name, cat: '理财', amount: amt, time: Date.now(), counter: '招财理财子公司', remark: '' });
          this.accounts = accounts;
          wx.hideLoading();
          wx.showToast({ title: '购买成功', icon: 'success' });
          setTimeout(() => {
            if (getCurrentPages().length > 1) wx.navigateBack(); else wx.switchTab({ url: '/pages/mine/mine' });
          }, 1200);
        }, 600);
      }
    });
  }
});
