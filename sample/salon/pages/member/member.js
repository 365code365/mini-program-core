// 会员卡：等级进度（progress active）、储值充值（requestPayment，没有宿主时模拟支付确认）、积分 / 消费流水
const store = require('../../utils/store.js');
const data = require('../../utils/data.js');
Page({
  data: { m: null, lv: null, packages: data.packages, pick: 1, levels: data.levels, records: [] },
  onShow() { this.refresh(); },
  refresh() {
    const m = store.member();
    const lv = store.levelOf(m.growth);
    this.setData({ m, lv, records: m.records.slice(0, 15) });
  },
  pickPkg(e) { this.setData({ pick: Number(e.currentTarget.dataset.i) }); },
  recharge() {
    const k = this.data.packages[this.data.pick];
    const done = way => {
      const m = store.member();
      m.balance += k.pay + k.gift;
      m.growth += k.pay;
      store.addRecord(m, 'recharge', '储值 ¥' + k.pay + ' 赠 ¥' + k.gift + '（' + way + '）', k.pay + k.gift);
      store.saveMember(m);
      const before = this.data.lv.cur.name;
      this.refresh();
      const after = this.data.lv.cur.name;
      wx.showToast({ title: after !== before ? '充值成功，升级为' + after : '充值成功', icon: 'success' });
    };
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: 'n' + Date.now(), package: 'prepay_id=salon' + k.id, signType: 'RSA', paySign: 'demo',
      success: () => done('微信支付'),
      fail: () => wx.showModal({
        title: '模拟支付', content: '当前环境未接入微信支付。确认支付 ¥' + k.pay + '，到账 ¥' + (k.pay + k.gift) + '？', confirmText: '确认支付', confirmColor: '#c0846a',
        success: r => { if (r.confirm) done('模拟支付'); }
      })
    });
  },
  copyCard() { wx.setClipboardData({ data: this.data.m.cardNo.replace(/\s/g, '') }); }
});
