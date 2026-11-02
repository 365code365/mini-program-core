// 钱包：余额充值（充得多送得多）、骑行卡购买（余额或微信支付）、账单明细
const fee = require('../../utils/fee.js');
const store = require('../../utils/store.js');
const pay = require('../../utils/pay.js');

const PLANS = [
  { amount: 10, bonus: 0 },
  { amount: 20, bonus: 1 },
  { amount: 50, bonus: 5 },
  { amount: 100, bonus: 15 }
];

Page({
  data: {
    balance: '0.00',
    plans: PLANS,
    plan: 1,
    cards: [],
    txns: [],
    showAll: false,
    cashier: { show: false, amount: 0, title: '' }
  },

  onShow() { this.load(); },

  load() {
    const w = store.wallet();
    const now = Date.now();
    const cards = fee.CARDS.map(c => {
      const own = w.cards.find(x => x.id === c.id && x.expireAt > now);
      return Object.assign({}, c, {
        priceTx: c.price.toFixed(2),
        own: !!own,
        left: own ? Math.ceil((own.expireAt - now) / 86400000) : 0
      });
    });
    const txns = w.txns.map(t => ({
      id: t.id, title: t.title, kind: t.kind,
      amount: (t.amount > 0 ? '+' : '') + t.amount.toFixed(2),
      at: store.fmtTime(t.at)
    }));
    this.setData({ balance: w.balance.toFixed(2), cards, txns });
  },

  onPlan(e) { this.setData({ plan: Number(e.currentTarget.dataset.i) }); },

  onRecharge() {
    const p = PLANS[this.data.plan];
    pay.pay(this, { amount: p.amount, title: '余额充值' }, () => {
      const w = store.wallet();
      w.balance += p.amount + p.bonus;
      store.addTxn(w, '余额充值' + (p.bonus ? '（赠 ' + p.bonus + ' 元）' : ''), p.amount + p.bonus);
      store.saveWallet(w);
      wx.showToast({ title: '充值成功，到账 ¥' + (p.amount + p.bonus), icon: 'none' });
      this.load();
    });
  },

  onBuyCard(e) {
    const c = fee.CARDS.find(x => x.id === e.currentTarget.dataset.id);
    const w = store.wallet();
    const items = ['余额支付（可用 ¥' + w.balance.toFixed(2) + '）', '微信支付'];
    wx.showActionSheet({
      itemList: items,
      success: (res) => {
        if (res.tapIndex === 0) {
          if (w.balance < c.price) { wx.showToast({ title: '余额不足，请先充值或选择微信支付', icon: 'none' }); return; }
          w.balance -= c.price;
          this.grant(w, c, '余额');
        } else {
          pay.pay(this, { amount: c.price, title: c.name }, () => this.grant(store.wallet(), c, '微信'));
        }
      }
    });
  },

  // 发卡：已有同款卡就顺延有效期
  grant(w, c, by) {
    const now = Date.now();
    const old = w.cards.find(x => x.id === c.id && x.expireAt > now);
    if (old) old.expireAt += c.days * 86400000;
    else w.cards.push({ id: c.id, name: c.name, type: c.type, expireAt: now + c.days * 86400000 });
    w.cards = w.cards.filter(x => x.expireAt > now);
    store.addTxn(w, '购买' + c.name + '（' + by + '支付）', -c.price);
    store.saveWallet(w);
    wx.showToast({ title: old ? '已续期 ' + c.days + ' 天' : '购买成功，下次骑行自动抵扣', icon: 'none' });
    this.load();
  },

  onToggleAll() { this.setData({ showAll: !this.data.showAll }); },
  onCashierConfirm() { pay.confirm(this); },
  onCashierCancel() { pay.cancel(this); }
});
