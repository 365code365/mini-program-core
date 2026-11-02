// 会员卡：开通光影会员（免服务费）、储值充值（充多少送多少）、余额明细；确认页可用余额付款
const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { vip: false, balance: 0, logs: [], opts: D.RECHARGE, pick: 1, vipPrice: D.VIP_PRICE },
  onShow() { this.load(); },
  load() {
    const c = store.card();
    this.setData({
      vip: store.isVip(), balance: c.balance,
      logs: c.logs.map(l => ({ title: l.title, time: store.dateTime(l.ts), amount: (l.amount > 0 ? '+' : '-') + Math.abs(l.amount), plus: l.amount > 0 }))
    });
  },
  onOpen() {
    if (this.data.vip) { wx.showToast({ title: '你已经是光影会员', icon: 'none' }); return; }
    host.pay(D.VIP_PRICE, '开通光影会员（1 个月）').then(ok => {
      if (!ok) return;
      store.openVip();
      wx.showToast({ title: '开通成功', icon: 'success' });
      this.load();
    });
  },
  onPick(e) { this.setData({ pick: Number(e.currentTarget.dataset.i) }); },
  onRecharge() {
    const o = D.RECHARGE[this.data.pick];
    host.pay(o.amount, '会员卡充值 ¥' + o.amount + '（赠 ¥' + o.gift + '）').then(ok => {
      if (!ok) return;
      store.recharge(o);
      wx.showToast({ title: '到账 ¥' + (o.amount + o.gift), icon: 'success' });
      this.load();
    });
  }
});
