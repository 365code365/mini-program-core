const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const TYPE = { recharge: '充值', pay: '消费', refund: '退款' };
Page({
  data: {
    balance: 0,
    options: data.recharges,
    pick: 0,
    logs: []
  },
  onShow() {
    this.load();
  },
  load() {
    const w = store.getWallet();
    this.setData({
      balance: w.balance,
      logs: w.logs.map((l, k) => Object.assign({ k }, l, { typeName: TYPE[l.type] || '', sign: l.amount > 0 ? '+' : '', abs: l.amount }))
    });
  },
  onPick(e) {
    this.setData({ pick: Number(e.currentTarget.dataset.i) });
  },
  onRecharge() {
    if (this.paying) return;
    const o = this.data.options[this.data.pick];
    this.paying = true;
    host.pay(o.pay, `咖啡钱包充值（赠 ¥${o.bonus}）`).then(ok => {
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
      title: '钱包说明',
      content: '充值金额与赠送金额都可在下单时直接抵扣，永久有效；赠送金额不可提现。订单取消后钱包支付的款项原路退回钱包。',
      showCancel: false,
      confirmText: '知道了'
    });
  }
});
