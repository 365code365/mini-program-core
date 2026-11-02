const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: { a: null, same: [], sameNames: [], sameIdx: 0, quick: [], amount: '', custom: false, payWay: 0, ways: ['微信支付', '零钱通', '招商银行储蓄卡(6621)'] },
  onLoad(q) {
    const all = store.getAccounts().map(store.view);
    const a = all.find(x => x.id === q.id) || all[0];
    const same = all.filter(x => x.type === a.type);
    this.setData({ same, sameNames: same.map(x => x.tag + ' · ' + x.noTx), sameIdx: same.indexOf(same.find(x => x.id === a.id)) });
    this.use(a);
  },
  use(a) {
    const t = data.typeOf(a.type);
    wx.setNavigationBarTitle({ title: t.name + '缴费' });
    // 快捷金额：有欠费时第一个就是欠费金额，其余按整数档位
    const base = a.type === 'net' ? [99, 199, 299, 599] : [50, 100, 200, 300, 500];
    const quick = (a.owe > 0 ? [{ v: a.oweTx, label: '缴清欠费' }] : []).concat(base.filter(v => v > a.owe).slice(0, a.owe > 0 ? 5 : 6).map(v => ({ v: store.money(v), label: '' })));
    this.setData({ a, quick, amount: quick[0].v, custom: false, auto: store.getAuto().ids.indexOf(a.id) >= 0, autoCap: store.getAuto().cap, amtSeq: (this.data.amtSeq || 0) + 1 });
  },
  onSame(e) { const i = Number(e.detail.value); this.setData({ sameIdx: i }); this.use(this.data.same[i]); },
  onQuick(e) { this.setData({ amount: e.currentTarget.dataset.v, custom: false }); },
  onCustom() { this.setData({ custom: true, amount: '', amtSeq: (this.data.amtSeq || 0) + 1 }); },
  goAuto() { wx.navigateTo({ url: '/pages/auto/auto' }); },
  onInput(e) { this.setData({ amount: e.detail.value }); },
  onWay(e) { this.setData({ payWay: Number(e.detail.value) }); },
  bill() { wx.navigateTo({ url: '/pages/bill/bill?id=' + this.data.a.id }); },
  confirm() {
    const a = this.data.a;
    const amt = Number(this.data.amount);
    if (!(amt > 0)) { wx.showToast({ title: '请输入缴费金额', icon: 'none' }); return; }
    if (amt > 5000) { wx.showToast({ title: '单笔最多缴纳 5000 元', icon: 'none' }); return; }
    if (amt < a.owe) {
      wx.showToast({ title: '金额不能少于欠费 ¥' + a.oweTx, icon: 'none' }); return;
    }
    if (this.paying) return;
    this.paying = true;
    const desc = a.tname + ' · 户号 ' + a.noTx + '\n户名 ' + a.owner + ' · ' + this.data.ways[this.data.payWay];
    host.pay(amt, desc).then(ok => {
      this.paying = false;
      if (!ok) return;
      const rec = store.settle(a.id, amt, { way: this.data.ways[this.data.payWay] });
      wx.redirectTo({ url: '/pages/result/result?id=' + rec.id });
    });
  }
});
