// 校园卡：余额、流水、充值（requestPayment，没有宿主时模拟支付确认）、挂失
const store = require('../../utils/store.js');
const kit = require('../../utils/kit.js');
Page({
  data: { c: null, amounts: [20, 50, 100, 200], pick: 1, lost: false },
  onShow() {
    const c = store.card();
    c.balanceText = kit.money(c.balance);
    c.records = c.records.map(r => Object.assign({}, r, { amountText: kit.money(r.amount) }));
    this.setData({ c, lost: !!kit.get('lost', false) });
  },
  pickAmt(e) { this.setData({ pick: Number(e.currentTarget.dataset.i) }); },
  recharge() {
    if (this.data.lost) { kit.toast('卡片已挂失，请先解挂'); return; }
    const amt = this.data.amounts[this.data.pick];
    const done = way => {
      const c = store.card();
      c.balance = Math.round((c.balance + amt) * 100) / 100;
      c.records.unshift({ id: kit.uid('t'), title: way + '充值', amount: amt, time: kit.fmt(Date.now()) });
      store.saveCard(c);
      this.onShow();
      wx.showToast({ title: '充值成功', icon: 'success' });
    };
    wx.requestPayment({
      timeStamp: String(Math.floor(Date.now() / 1000)), nonceStr: kit.uid('n'), package: 'prepay_id=campus', signType: 'RSA', paySign: 'demo',
      success: () => done('微信'),
      fail: () => kit.confirm('模拟支付', '当前环境未接入微信支付，确认向校园卡充值 ¥' + amt + '？', { confirmColor: '#14937f' }).then(ok => { if (ok) done('模拟支付'); })
    });
  },
  toggleLost() {
    const lost = !this.data.lost;
    kit.confirm(lost ? '挂失校园卡' : '解除挂失', lost ? '挂失后卡片立即停用，补卡请到行政楼一楼卡务中心。' : '确认已找回卡片，解除挂失？').then(ok => {
      if (!ok) return;
      kit.set('lost', lost);
      this.setData({ lost });
      kit.toast(lost ? '已挂失' : '已解除挂失');
    });
  },
  copyNo() { wx.setClipboardData({ data: this.data.c.no }); }
});
