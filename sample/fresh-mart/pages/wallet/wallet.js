const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');

const TYPE = { recharge: '充值', pay: '消费', refund: '退款' };

Page({
  data: {
    balance: 0,
    options: data.recharges,
    pick: 1,
    logs: []
  },

  onShow() {
    this.load();
  },

  load() {
    const w = store.getWallet();
    this.setData({
      balance: w.balance,
      logs: w.logs.map((l, k) => Object.assign({ k }, l, { typeName: TYPE[l.type] || '', sign: l.amount > 0 ? '+' : '' }))
    });
  },

  onPick(e) {
    this.setData({ pick: Number(e.currentTarget.dataset.i) });
  },

  onRecharge() {
    if (this.paying) return;
    const o = this.data.options[this.data.pick];
    this.paying = true;
    host.pay(o.pay, `鲜到家余额充值（赠 ¥${o.bonus}）`).then(ok => {
      this.paying = false;
      if (!ok) {
        wx.showToast({ title: '已取消充值', icon: 'none' });
        return;
      }
      store.changeWallet(o.pay + o.bonus, `充值 ¥${o.pay} 赠 ¥${o.bonus}`, 'recharge');
      this.load();
      wx.showToast({ title: `到账 ¥${o.pay + o.bonus}`, icon: 'success' });
    });
  },

  onRule() {
    wx.showModal({
      title: '余额说明',
      content: '充值与赠送金额都可在结算时选择「余额支付」直接抵扣，长期有效；赠送金额不可提现。用余额支付的订单取消或售后退款，款项原路退回余额。',
      showCancel: false,
      confirmText: '知道了'
    });
  }
});
