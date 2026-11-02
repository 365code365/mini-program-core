// 钱包：余额、充值（满赠，走 host.pay，宿主没有支付能力时页内确认）、收支明细（充值 / 行程支付 / 取消费）
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const TYPE = { recharge: '充值', pay: '支付', refund: '退款', gift: '礼金' };

Page({
  data: { balance: '0.00', options: data.RECHARGES, pick: 1, logs: [], income: 0, spend: 0 },
  onShow() {
    this.load();
  },
  load() {
    const w = store.getWallet();
    let income = 0;
    let spend = 0;
    w.logs.forEach(l => {
      if (l.amount > 0) income += l.amount;
      else spend -= l.amount;
    });
    this.setData({
      balance: w.balance.toFixed(2),
      income: store.round2(income),
      spend: store.round2(spend),
      logs: w.logs.map((l, k) => Object.assign({ k }, l, { typeName: TYPE[l.type] || '其他', sign: l.amount > 0 ? '+' : '', abs: l.amount.toFixed(2) }))
    });
  },
  onPick(e) {
    this.setData({ pick: Number(e.currentTarget.dataset.i) });
  },
  onRecharge() {
    if (this.paying) return;
    const o = this.data.options[this.data.pick];
    this.paying = true;
    host.pay(o.pay, '滴达出行钱包充值' + (o.bonus ? '（赠 ¥' + o.bonus + '）' : '')).then(ok => {
      this.paying = false;
      if (!ok) {
        wx.showToast({ title: '已取消充值', icon: 'none' });
        return;
      }
      store.changeWallet(o.pay + o.bonus, o.bonus ? '充值 ¥' + o.pay + ' 赠 ¥' + o.bonus : '充值 ¥' + o.pay, 'recharge');
      this.load();
      wx.showToast({ title: '到账 ¥' + (o.pay + o.bonus), icon: 'success' });
    });
  },
  onRule() {
    wx.showModal({
      title: '钱包说明',
      content: '钱包余额可直接支付车费与取消费，免输密码；充值赠送的金额与本金一样使用，永久有效，赠送部分不可提现。',
      showCancel: false,
      confirmText: '知道了'
    });
  }
});
