const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
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
    this.setData({ a, quick, amount: quick[0].v, custom: false });
  },
  onSame(e) { const i = Number(e.detail.value); this.setData({ sameIdx: i }); this.use(this.data.same[i]); },
  onQuick(e) { this.setData({ amount: e.currentTarget.dataset.v, custom: false }); },
  onCustom() { this.setData({ custom: true, amount: '' }); },
  onInput(e) { this.setData({ amount: e.detail.value }); },
  onWay(e) { this.setData({ payWay: Number(e.detail.value) }); },
  bill() {
    const a = this.data.a;
    wx.showModal({ title: '账单明细', content: a.owe > 0 ? a.period + '\n用量费用 ¥' + a.oweTx + '\n违约金 ¥0.00\n出账日期：每月 5 日' : '当前无欠费账单', showCancel: false });
  },
  confirm() {
    const a = this.data.a;
    const amt = Number(this.data.amount);
    if (!(amt > 0)) { wx.showToast({ title: '请输入缴费金额', icon: 'none' }); return; }
    if (amt > 5000) { wx.showToast({ title: '单笔最多缴纳 5000 元', icon: 'none' }); return; }
    if (amt < a.owe) {
      wx.showToast({ title: '金额不能少于欠费 ¥' + a.oweTx, icon: 'none' }); return;
    }
    wx.showModal({
      title: '确认缴费',
      content: a.tname + ' · 户号 ' + a.noTx + '\n户名 ' + a.owner + '\n缴费金额 ¥' + store.money(amt) + '\n支付方式 ' + this.data.ways[this.data.payWay],
      confirmText: '确认支付', confirmColor: '#14a06f',
      success: r => {
        if (!r.confirm) return;
        wx.showLoading({ title: '支付中' });
        setTimeout(() => {
          wx.hideLoading();
          const accs = store.getAccounts();
          const raw = accs.find(x => x.id === a.id);
          const extra = amt - raw.owe;
          raw.owe = 0;
          raw.balance = Math.round(((raw.balance || 0) + extra) * 100) / 100;
          raw.period = '';
          store.saveAccounts(accs);
          const rec = store.addRecord({ type: a.type, title: a.tname, target: a.no, amount: amt, paid: amt, unit: a.unit, owner: a.owner });
          wx.setStorageSync('lp_last', rec);
          wx.redirectTo({ url: '/pages/result/result?id=' + rec.id });
        }, 600);
      }
    });
  }
});
