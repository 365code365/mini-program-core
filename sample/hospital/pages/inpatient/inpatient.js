const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const LOW = 1000; // 预交金余额低于这个数提醒补缴
Page({
  // formSeq：打过字的输入框之后 setData 清空画面不更新，补缴成功后换 id 让它重建
  data: { ip: null, sum: {}, low: false, days: [], openK: -1, deposits: [], amounts: [500, 1000, 2000, 5000], amtIdx: 1, custom: '', formSeq: 0 },
  onShow() { this.load(); },
  load() {
    const ip = store.getInpatient();
    const sum = store.inpatientSummary(ip);
    this.setData({
      ip,
      sx: ip.patient.charAt(0),
      sum,
      low: ip.status === 'in' && sum.balance < LOW,
      days: store.inpatientDays(ip),
      deposits: ip.deposits.map(d => ({ amount: d.amount, time: store.dateTime(d.at) })).reverse(),
      settleTime: ip.settle ? store.dateTime(ip.settle.at) : ''
    });
  },
  onToggleDay(e) {
    const k = Number(e.currentTarget.dataset.k);
    this.setData({ openK: this.data.openK === k ? -1 : k });
  },
  // 点了固定金额就清掉「其他金额」：输入框打过字的话要换 id 重建，画面才会清空
  onAmt(e) { this.setData({ amtIdx: Number(e.currentTarget.dataset.i), custom: '', formSeq: this.data.custom ? this.data.formSeq + 1 : this.data.formSeq }); },
  onCustom(e) { this.setData({ custom: e.detail.value, amtIdx: -1 }); },
  onTopup() {
    if (this.paying) return;
    const d = this.data;
    const amount = d.custom ? Number(d.custom) : d.amounts[d.amtIdx];
    if (!amount || amount % 1 || amount < 100 || amount > 50000) {
      wx.showToast({ title: '补缴金额为 100~50000 元的整数', icon: 'none' });
      return;
    }
    this.paying = true;
    host.pay(amount, d.ip.patient + ' · 住院预交金').then(ok => {
      this.paying = false;
      if (!ok) {
        wx.showToast({ title: '已取消支付', icon: 'none' });
        return;
      }
      store.addDeposit(amount);
      this.setData({ custom: '', amtIdx: 1, formSeq: this.data.formSeq + 1 });
      wx.showToast({ title: '预交金 ¥' + amount + ' 已到账', icon: 'none' });
      this.load();
    });
  },
  // 出院结算：预交金多退少补
  onSettle() {
    const s = this.data.sum;
    const ip = this.data.ip;
    const need = s.balance < 0 ? store.round2(-s.balance) : 0;
    wx.showModal({
      title: '出院结算',
      content: '住院 ' + s.days + ' 天，总费用 ¥' + s.total + '，已交预交金 ¥' + s.deposit + '\n' + (need ? '需补缴 ¥' + need : '结算后退还 ¥' + s.balance + '（原路退回）'),
      // 按钮最多 4 个汉字（多了 showModal 直接 fail，弹窗出不来）
      confirmText: need ? '补缴结算' : '确认结算',
      success: r => {
        if (!r.confirm) return;
        const done = () => {
          store.settleInpatient(need);
          wx.showToast({ title: need ? '结算完成，祝早日康复' : '结算完成，¥' + s.balance + ' 原路退回', icon: 'none' });
          this.load();
        };
        if (!need) return done();
        host.pay(need, ip.patient + ' · 出院结算补缴').then(ok => (ok ? done() : wx.showToast({ title: '已取消支付', icon: 'none' })));
      }
    });
  },
  onCall() { host.call('021-6438 2212', '心内科病区护士站'); },
  goBills() { wx.navigateTo({ url: '/pages/bills/bills?tab=paid' }); }
});
