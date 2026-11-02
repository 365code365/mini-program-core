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
      logs: w.logs.map((l, k) => Object.assign({ k }, l, { typeName: TYPE[l.type] || '其他', sign: l.amount > 0 ? '+' : '' }))
    });
  },
  onPick(e) {
    this.setData({ pick: Number(e.currentTarget.dataset.i) });
  },
  // 充值：先付款（宿主没有支付能力时退回页内确认框），付成功才入账
  onRecharge() {
    if (this.paying) return;
    const o = this.data.options[this.data.pick];
    this.paying = true;
    host.pay(o.pay, `饱了么余额充值${o.bonus ? `（赠 ¥${o.bonus}）` : ''}`).then(ok => {
      this.paying = false;
      if (!ok) {
        wx.showToast({ title: '已取消充值', icon: 'none' });
        return;
      }
      store.changeWallet(o.pay + o.bonus, o.bonus ? `余额充值 ¥${o.pay} 赠 ¥${o.bonus}` : `余额充值 ¥${o.pay}`, 'recharge');
      this.load();
      wx.showToast({ title: `到账 ¥${o.pay + o.bonus}`, icon: 'success' });
    });
  },
  onRule() {
    wx.showModal({
      title: '余额说明',
      content: '充值金额与赠送金额都可在下单时选「余额支付」直接抵扣，长期有效；订单取消或退款，余额支付的款项原路退回余额。赠送金额不可提现。',
      showCancel: false,
      confirmText: '知道了'
    });
  }
});
